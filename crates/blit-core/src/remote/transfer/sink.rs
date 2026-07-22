//! Pluggable write backends for the transfer pipeline.
//!
//! Every src→dst combination flows through `TransferSource → plan → prepare → TransferSink`.
//! Implementations handle the actual write: local filesystem, TCP data plane, etc.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use eyre::{Context, Result};
use filetime::FileTime;

use crate::buffer::BufferSizer;
use crate::checksum::ChecksumType;
use crate::copy::{copy_file, resume_copy_file};
use crate::generated::{ComparisonMode, FileHeader};
use crate::remote::transfer::payload::PreparedPayload;
use crate::remote::transfer::progress::{ByteProgressSink, NoProbe, Probe};
use crate::remote::transfer::small_file_probe::{BoundSmallFileProbe, MemberTimingReport};
use crate::remote::transfer::source::TransferSource;

// Re-export for consumers.
pub use super::data_plane::DataPlaneSession;

/// Outcome of writing payload(s) to a sink.
#[derive(Debug, Default, Clone)]
pub struct SinkOutcome {
    pub files_written: usize,
    pub bytes_written: u64,
}

impl SinkOutcome {
    pub fn merge(&mut self, other: &SinkOutcome) {
        self.files_written += other.files_written;
        self.bytes_written += other.bytes_written;
    }
}

/// A pluggable write backend for the transfer pipeline.
///
/// Implementations receive [`PreparedPayload`] items produced by a [`TransferSource`]
/// and write them to a destination (local filesystem, TCP stream, etc.).
#[async_trait]
pub trait TransferSink: Send + Sync {
    /// Write a single prepared payload to the destination.
    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome>;

    /// Stream a file payload from a borrowed async reader.
    ///
    /// Used by the receive pipeline so file bytes that arrive on a TCP
    /// wire can be written through the same sink as local copies — no
    /// double-buffering into a `'static` reader. Outbound-only sinks
    /// (e.g. `DataPlaneSink`) inherit the default error implementation.
    async fn write_file_stream(
        &self,
        header: &FileHeader,
        _reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
    ) -> Result<SinkOutcome> {
        eyre::bail!(
            "{} does not support write_file_stream (called for {})",
            std::any::type_name::<Self>(),
            header.relative_path
        )
    }

    /// Signal that all payloads have been sent. Flushes buffers, sends terminators, etc.
    /// Default implementation is a no-op.
    async fn finish(&self) -> Result<()> {
        Ok(())
    }

    /// Destination root path (if applicable).
    fn root(&self) -> &Path;
}

// ---------------------------------------------------------------------------
// FsTransferSink — local filesystem writer
// ---------------------------------------------------------------------------

/// Configuration for filesystem sink writes.
#[derive(Debug, Clone)]
pub struct FsSinkConfig {
    pub preserve_times: bool,
    pub dry_run: bool,
    pub checksum: Option<ChecksumType>,
    pub resume: bool,
    /// R58-followup: comparison policy the sink uses when deciding
    /// whether to copy a `PreparedPayload::File`. The diff_planner
    /// upstream already filters by `compare_mode`, but
    /// `write_file_payload` re-checks before copying as a defense
    /// layer; pre-fix it called `file_needs_copy_with_checksum_type`
    /// which only knows SizeMtime + Checksum, so `Force` and
    /// `IgnoreTimes` were silently downgraded to SizeMtime and
    /// dropped at the sink layer. The default `SizeMtime` keeps
    /// pre-fix behavior for callers that haven't migrated.
    pub compare_mode: ComparisonMode,
}

impl Default for FsSinkConfig {
    fn default() -> Self {
        Self {
            preserve_times: true,
            dry_run: false,
            checksum: None,
            resume: false,
            compare_mode: ComparisonMode::SizeMtime,
        }
    }
}

/// Writes files directly to a local filesystem using zero-copy primitives
/// (copy_file_range, sendfile, clonefile, block clone) where available.
pub struct FsTransferSink {
    src_root: PathBuf,
    dst_root: PathBuf,
    /// Canonical form of `dst_root` (or its deepest existing
    /// ancestor) captured once at sink construction time. Every
    /// per-entry write resolves the lexical path under `dst_root`
    /// and then verifies it stays inside `canonical_dst_root`
    /// post-symlink. R46-F3: pre-fix the sink only ran lexical
    /// `safe_join`, so a peer-controlled relative path joined under
    /// a `dst_root/link → /outside` symlink would write outside
    /// the destination root.
    canonical_dst_root: Option<PathBuf>,
    config: FsSinkConfig,
    /// Optional byte-level progress sink. When set,
    /// `write_file_stream` passes it into
    /// `receive_stream_double_buffered` so chunk-granularity
    /// writes report cumulative byte progress against the
    /// daemon's per-transfer counter (c-1a). Unset on the CLI
    /// side; the daemon side sets it via
    /// [`FsTransferSink::with_byte_progress`] from
    /// `ActiveJobGuard::bytes_counter()`.
    byte_progress: Option<ByteProgressSink>,
    /// Separate otp-12 high-volume observer. `None` is the exact normal
    /// sink path: no clocks, per-member timing, or output.
    small_file_probe: Option<BoundSmallFileProbe>,
}

impl FsTransferSink {
    pub fn new(src_root: PathBuf, dst_root: PathBuf, config: FsSinkConfig) -> Self {
        // Best-effort canonical root capture. We don't fail
        // construction if canonicalize fails (e.g. dst_root is a
        // not-yet-created path under a deeply unusual filesystem) —
        // instead we leave canonical_dst_root as None and the
        // per-write check degrades to lexical-only with a warn.
        // R46-F3: in the common case (dst_root or its ancestor
        // exists) this captures the canonical form needed for
        // symlink-escape rejection.
        let canonical_dst_root = crate::path_safety::canonical_dest_root(&dst_root).ok();
        Self {
            src_root,
            dst_root,
            canonical_dst_root,
            config,
            byte_progress: None,
            small_file_probe: None,
        }
    }

    /// Attach a byte-level progress sink. When set,
    /// `write_file_stream` reports every chunk the data plane
    /// writes against this sink. Used by the daemon side of
    /// remote→remote transfers so `GetState.active[].bytes_completed`
    /// tracks live progress; CLI-side callers omit it.
    pub fn with_byte_progress(mut self, sink: ByteProgressSink) -> Self {
        self.byte_progress = Some(sink);
        self
    }

    pub(crate) fn with_small_file_probe(mut self, probe: Option<BoundSmallFileProbe>) -> Self {
        self.small_file_probe = probe;
        self
    }

    /// R46-F3: lexical resolve + canonical containment check in one
    /// call. Used by every per-entry write site on this sink so a
    /// peer-controlled relative path can't escape the destination
    /// root via a pre-existing symlink. Falls back to lexical-only
    /// (with a warn) if `canonical_dst_root` was None at
    /// construction time — that path remains exposed but is
    /// extremely unusual in practice.
    fn resolve_destination(&self, wire_path: &str) -> Result<PathBuf> {
        match self.canonical_dst_root.as_ref() {
            Some(canonical) => {
                crate::path_safety::safe_join_contained(canonical, &self.dst_root, wire_path)
            }
            None => {
                log::warn!(
                    "FsTransferSink at '{}' has no canonical root; \
                     receive falls back to lexical-only path check \
                     (R46-F3 escape protection unavailable)",
                    self.dst_root.display()
                );
                crate::path_safety::safe_join(&self.dst_root, wire_path)
            }
        }
    }
}

#[async_trait]
impl TransferSink for FsTransferSink {
    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
        // Resume payloads need async I/O (file open + seek + write
        // through tokio). Local-source payloads (File / TarShard) stay
        // on a blocking thread so the zero-copy cascade and tar
        // extraction can use std::fs.
        let outcome = match payload {
            PreparedPayload::FileBlock {
                relative_path,
                offset,
                bytes,
            } => {
                write_file_block_payload(
                    &self.dst_root,
                    self.canonical_dst_root.as_deref(),
                    &relative_path,
                    offset,
                    bytes,
                )
                .await?
            }
            PreparedPayload::FileBlockComplete {
                relative_path,
                total_size,
                mtime_seconds,
                permissions,
                windows_metadata,
            } => {
                let outcome = write_file_block_complete(
                    &self.dst_root,
                    self.canonical_dst_root.as_deref(),
                    &relative_path,
                    total_size,
                    mtime_seconds,
                    permissions,
                    windows_metadata,
                )
                .await?;
                outcome
            }
            // otp-7b: the composite resume payload is send-side only
            // (DataPlaneSink); the receive pipeline decodes per-block
            // FileBlock/FileBlockComplete, never this shape.
            PreparedPayload::ResumeFile { .. } => {
                eyre::bail!("FsTransferSink does not consume composite ResumeFile payloads")
            }
            PreparedPayload::File(_) | PreparedPayload::TarShard { .. } => {
                let src_root = self.src_root.clone();
                let dst_root = self.dst_root.clone();
                let canonical_dst_root = self.canonical_dst_root.clone();
                let config = self.config.clone();
                let tar_probe = self
                    .small_file_probe
                    .as_ref()
                    .and_then(|probe| match &payload {
                        PreparedPayload::TarShard { headers, .. } => {
                            let shard_id = probe.shard_id(headers);
                            Some((probe.clone(), shard_id, probe.start()))
                        }
                        _ => None,
                    });
                let outcome = tokio::task::spawn_blocking(move || match payload {
                    PreparedPayload::File(header) => write_file_payload(
                        &src_root,
                        &dst_root,
                        canonical_dst_root.as_deref(),
                        &header,
                        &config,
                    ),
                    PreparedPayload::TarShard { headers, data } => {
                        let worker_started = tar_probe.as_ref().map(|_| std::time::Instant::now());
                        let blocking_pool_wait = tar_probe.as_ref().zip(worker_started).map(
                            |((_, _, queued), worker)| worker.saturating_duration_since(*queued),
                        );
                        write_tar_shard_payload(
                            &src_root,
                            &dst_root,
                            canonical_dst_root.as_deref(),
                            &headers,
                            &data,
                            &config,
                            tar_probe.as_ref().zip(blocking_pool_wait).map(
                                |((probe, shard_id, queued), wait)| {
                                    (probe, shard_id.as_str(), *queued, wait)
                                },
                            ),
                        )
                    }
                    _ => unreachable!("outer match guarantees File or TarShard"),
                })
                .await
                .context("sink worker panicked")??;
                outcome
            }
        };
        // c-1b round 2: tar shards and resume blocks land via
        // write_payload, not write_file_stream, so the chunk-
        // granular `receive_stream_double_buffered` hook never
        // fires for them. Report `outcome.bytes_written` here so
        // `GetState.active[].bytes_completed` reflects bytes
        // landed on disk for ALL payload shapes, not just
        // streamed files. Dry-run write paths return
        // `bytes_written: 0` (see `write_file_payload` and
        // `write_tar_shard_payload`'s dry-run early returns), so
        // adding 0 is a no-op for previews — same semantics as
        // `write_file_stream`'s dry-run branch.
        if let Some(bp) = &self.byte_progress {
            bp.report(outcome.bytes_written);
        }
        Ok(outcome)
    }

    /// Stream file bytes from the wire to the destination filesystem
    /// using the same double-buffered helper the send side uses. This
    /// is what makes push and pull receive symmetric on the FsTransferSink.
    async fn write_file_stream(
        &self,
        header: &FileHeader,
        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
    ) -> Result<SinkOutcome> {
        use crate::remote::transfer::data_plane::{
            receive_stream_double_buffered, RECEIVE_CHUNK_SIZE,
        };

        // R46-F3: lexical resolve + canonical containment check via
        // resolve_destination. Pre-fix this was a bare safe_join,
        // which rejected lexical traversal (`../`) but didn't catch
        // the case where dst_root contained a pre-existing symlink
        // pointing outside (`dst_root/link → /outside`); a peer-
        // controlled relative path `link/file` would then write to
        // `/outside/file`.
        let dst = self
            .resolve_destination(&header.relative_path)
            .with_context(|| format!("validating receive path {:?}", header.relative_path))?;

        // R58-F4: dry-run must be side-effect-free. Drain the wire
        // for protocol-stream alignment, but skip the parent-mkdir
        // and the file write. Pre-fix the parent-mkdir ran before
        // the dry-run check below, so `--dry-run` over a remote
        // transfer would create destination directories.
        if self.config.dry_run {
            let mut sink = tokio::io::sink();
            // Dry-run: drain wire bytes for protocol alignment.
            // Do NOT report against `byte_progress` — by contract
            // dry-run is side-effect-free and these bytes never
            // hit user disk; we don't want a daemon-side bytes_completed
            // counter to advance for an aborted preview.
            receive_stream_double_buffered(
                reader,
                &mut sink,
                header.size,
                RECEIVE_CHUNK_SIZE,
                None,
            )
            .await
            .with_context(|| format!("draining {} (dry-run)", header.relative_path))?;
            return Ok(SinkOutcome {
                files_written: 1,
                bytes_written: 0,
            });
        }

        if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("creating directory {}", parent.display()))?;
        }

        crate::windows_metadata::validate_payload(header.windows_metadata.as_ref())
            .with_context(|| format!("validating Windows metadata for {}", header.relative_path))?;
        crate::windows_metadata::prepare_destination(&dst, header.windows_metadata.as_ref())?;

        {
            use tokio::io::AsyncWriteExt as _;
            let mut file = tokio::fs::File::create(&dst)
                .await
                .with_context(|| format!("creating {}", dst.display()))?;
            receive_stream_double_buffered(
                reader,
                &mut file,
                header.size,
                RECEIVE_CHUNK_SIZE,
                self.byte_progress.as_ref(),
            )
            .await
            .with_context(|| format!("writing {}", dst.display()))?;
            // Flush the tokio File's internal buffer state (does NOT
            // fsync — just ensures user-space buffering is drained
            // before we drop the handle and apply mtime). Without
            // this, set_file_mtime races with deferred writes from
            // tokio's blocking-thread pool: 5/8 of mtimes were
            // observed silently bumped to "now" on the receive side.
            //
            // POST_REVIEW_FIXES §1.1: flush failure is a data-loss
            // signal — the user believes the file is durable when it
            // isn't. Propagate, don't swallow.
            file.flush()
                .await
                .with_context(|| format!("flushing {}", dst.display()))?;
        }
        // Handle dropped → kernel close() complete → no further
        // metadata churn from this file. Now safe to set mtime by path.

        // Intentionally no sync_all: ZFS commits per fsync are
        // multi-second on spinning rust and crater throughput
        // (9.3 → 3.3 Gbps observed). The transfer's durability signal
        // is its END marker plus the OS's own flush; matches rsync's
        // default behavior. Add a config flag if a caller needs sync.

        let windows_bytes =
            crate::windows_metadata::replace_streams(&dst, header.windows_metadata.as_ref())?;

        if self.config.preserve_times && header.mtime_seconds > 0 {
            let ft = FileTime::from_unix_time(header.mtime_seconds, 0);
            // Best-effort: cross-fs, root-owned, or ACL-protected
            // destinations can refuse mtime updates. Surface via
            // `log::warn!` so the failure is visible without making
            // it a hard transfer error. POST_REVIEW_FIXES §1.1.
            if let Err(e) = filetime::set_file_mtime(&dst, ft) {
                log::warn!("set mtime on {}: {}", dst.display(), e);
            }
        }

        // Permissions arrive on the wire (Unix mode bits). Apply best-
        // effort; ignore failures (cross-fs, root-owned dst, etc.).
        #[cfg(unix)]
        if header.permissions != 0 {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) =
                std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(header.permissions))
            {
                log::warn!("set permissions on {}: {}", dst.display(), e);
            }
        }
        #[cfg(not(unix))]
        let _ = header.permissions;
        crate::windows_metadata::apply_attributes(&dst, header.windows_metadata.as_ref())?;

        Ok(SinkOutcome {
            files_written: 1,
            bytes_written: header.size.saturating_add(windows_bytes),
        })
    }

    fn root(&self) -> &Path {
        &self.dst_root
    }
}

/// Copy a single file using the zero-copy cascade in `copy::file_copy`.
fn write_file_payload(
    src_root: &Path,
    dst_root: &Path,
    canonical_dst_root: Option<&Path>,
    header: &FileHeader,
    config: &FsSinkConfig,
) -> Result<SinkOutcome> {
    // An empty relative_path means "the root itself" — the enumeration
    // root was a single file (same rule as FsTransferSource::open_file):
    // joining "" can yield a trailing-slash form the OS reads as
    // "descend into", which fails with ENOTDIR on a regular file. The
    // local session route (otp-11) is the first caller to send a
    // file-root File payload through here.
    if header.relative_path.is_empty() {
        return copy_root_file_payload(src_root, dst_root, header, config);
    }
    let src = src_root.join(&header.relative_path);
    // R47-F1: the FsTransferSink::write_payload arm for
    // PreparedPayload::File hit this helper, which previously
    // joined dst_root + header.relative_path lexically. A peer-
    // controlled `link/file` with a pre-existing `dst/link →
    // /outside` symlink would write outside the destination root.
    // Route through the same canonical-containment chokepoint that
    // write_file_stream uses.
    let dst = match canonical_dst_root {
        Some(canonical) => {
            crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
                .with_context(|| {
                    format!("validating file payload path {:?}", header.relative_path)
                })?
        }
        None => {
            log::warn!(
                "write_file_payload at '{}' has no canonical root; \
                 falls back to lexical-only path check (R47-F1 \
                 escape protection unavailable)",
                dst_root.display()
            );
            crate::path_safety::safe_join(dst_root, &header.relative_path).with_context(|| {
                format!("validating file payload path {:?}", header.relative_path)
            })?
        }
    };

    copy_resolved_file_payload(&src, &dst, header, config)
}

/// The file-root identity case of [`write_file_payload`]: `src_root`
/// IS the file and `dst_root` IS the exact target path, so there is
/// nothing to join and nothing to containment-check — the configured
/// root cannot escape itself.
fn copy_root_file_payload(
    src_root: &Path,
    dst_root: &Path,
    header: &FileHeader,
    config: &FsSinkConfig,
) -> Result<SinkOutcome> {
    copy_resolved_file_payload(src_root, dst_root, header, config)
}

/// Shared tail of the File-payload write: dry-run gate, parent mkdir,
/// resume/compare/copy cascade, mtime preservation.
fn copy_resolved_file_payload(
    src: &Path,
    dst: &Path,
    header: &FileHeader,
    config: &FsSinkConfig,
) -> Result<SinkOutcome> {
    // R58-F4: dry-run must be side-effect-free. Bail before the
    // parent-mkdir so a dry-run doesn't create destination
    // directories on disk.
    if config.dry_run {
        return Ok(SinkOutcome {
            files_written: 1,
            bytes_written: 0,
        });
    }

    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    crate::windows_metadata::validate_payload(header.windows_metadata.as_ref())
        .with_context(|| format!("validating Windows metadata for {}", header.relative_path))?;
    crate::windows_metadata::prepare_destination(dst, header.windows_metadata.as_ref())?;

    let mut did_copy = false;
    if config.resume {
        let outcome = resume_copy_file(src, dst, 0)
            .with_context(|| format!("resume copy {}", header.relative_path))?;
        did_copy = outcome.bytes_transferred > 0;
    } else if crate::copy::file_needs_copy_with_mode(src, dst, config.compare_mode)? {
        let sizer = BufferSizer::default();
        copy_file(src, dst, &sizer, false)
            .with_context(|| format!("copy {}", header.relative_path))?;
        did_copy = true;
    }

    let windows_bytes =
        crate::windows_metadata::replace_streams(dst, header.windows_metadata.as_ref())?;

    if config.preserve_times {
        let fallback =
            (header.mtime_seconds > 0).then(|| FileTime::from_unix_time(header.mtime_seconds, 0));
        if let Some(ft) = source_file_mtime(src, fallback) {
            if let Err(e) = filetime::set_file_mtime(dst, ft) {
                log::warn!("set mtime on {}: {}", dst.display(), e);
            }
        }
    }
    crate::windows_metadata::apply_attributes(dst, header.windows_metadata.as_ref())?;

    Ok(SinkOutcome {
        files_written: 1,
        bytes_written: (if did_copy { header.size } else { 0 }).saturating_add(windows_bytes),
    })
}

/// Read the local source timestamp at apply time so local copies retain the
/// precision that the second-granularity wire header cannot represent. The
/// header value remains a fallback when the source timestamp cannot be read.
fn source_file_mtime(source: &Path, fallback: Option<FileTime>) -> Option<FileTime> {
    std::fs::metadata(source)
        .and_then(|metadata| metadata.modified())
        .map(FileTime::from_system_time)
        .ok()
        .or(fallback)
}

/// Replace wire-derived tar timestamps with local source timestamps. An empty
/// source root is the transfer-session convention for a wire receive, which
/// must keep using the timestamp carried by its header.
fn restamp_local_tar_mtimes(src_root: &Path, files: &mut [super::tar_safety::ExtractedFile]) {
    if src_root.as_os_str().is_empty() {
        return;
    }
    for file in files {
        let source = if file.rel.is_empty() {
            src_root.to_path_buf()
        } else {
            src_root.join(&file.rel)
        };
        file.mtime = source_file_mtime(&source, file.mtime);
    }
}

/// Extract an in-memory tar shard to the destination directory.
fn write_tar_shard_payload(
    src_root: &Path,
    dst_root: &Path,
    canonical_dst_root: Option<&Path>,
    headers: &[FileHeader],
    data: &[u8],
    config: &FsSinkConfig,
    probe: Option<(
        &BoundSmallFileProbe,
        &str,
        std::time::Instant,
        std::time::Duration,
    )>,
) -> Result<SinkOutcome> {
    if config.dry_run {
        return Ok(SinkOutcome {
            files_written: headers.len(),
            bytes_written: 0,
        });
    }

    // Two-phase extraction:
    //   1. Validate + parse the tar serially via the shared
    //      `tar_safety` helper. Tar is a sequential format — entries
    //      can't be read in parallel out of one Archive — and this
    //      is also where R5-F2 / R6-F1 / R6-F3 safety checks live.
    //   2. Write files to disk in parallel via rayon. Inode creation
    //      and write are the bottleneck for many-small-files shards;
    //      4–8 worker cores can saturate ZFS' inode pipeline.
    //
    // Empirically, sequential extraction was ~62 MiB/s on ZFS-on-HDD
    // for 10k × 4 KiB; parallel raises the disk's small-file ceiling
    // toward CPU-or-fs limits.
    use rayon::prelude::*;

    use super::tar_safety::{safe_extract_tar_shard, ExtractedFile, TarShardExtractOptions};

    let parse_started = probe.map(|_| std::time::Instant::now());
    let opts = TarShardExtractOptions::default();
    let mut extracted = safe_extract_tar_shard(data, headers.to_vec(), dst_root, &opts)?;

    // R47-F1: tar shards arriving on FsTransferSink::write_payload
    // (push-receive on the daemon flows through here too) only had
    // lexical safe_join inside safe_extract_tar_shard. A pre-
    // existing dst/link → /outside escape symlink would let an
    // entry path like `link/victim` write through the symlink.
    // Verify each extracted entry's destination against the
    // canonical root before writing.
    if let Some(canonical) = canonical_dst_root {
        for f in &extracted {
            crate::path_safety::verify_contained(canonical, &f.dest_path).with_context(|| {
                format!("tar shard entry {:?} escapes destination root", f.dest_path)
            })?;
        }
    } else {
        log::warn!(
            "write_tar_shard_payload at '{}' has no canonical root; \
             tar-shard receive falls back to lexical-only path \
             checks (R47-F1 escape protection unavailable)",
            dst_root.display()
        );
    }

    // Honor the sink's preserve_times toggle by stripping mtimes that
    // the helper would otherwise apply. Permissions are best-effort
    // either way (matches the historical FsTransferSink policy).
    if config.preserve_times {
        restamp_local_tar_mtimes(src_root, &mut extracted);
    } else {
        for f in &mut extracted {
            f.mtime = None;
        }
    }

    let parse_validate = parse_started.map(|started| started.elapsed());

    // Write in parallel. Each closure does its own create_dir_all +
    // fs::write + best-effort mtime/permission application — same
    // policy as `tar_safety::write_extracted_file` but inlined so we
    // can return per-file byte counts for the SinkOutcome.
    if probe.is_none() {
        let results: Vec<Result<u64>> = extracted
            .into_par_iter()
            .map(|f: ExtractedFile| -> Result<u64> {
                if let Some(parent) = f.dest_path.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("create dir {}", parent.display()))?;
                }
                crate::windows_metadata::prepare_destination(
                    &f.dest_path,
                    f.windows_metadata.as_ref(),
                )?;
                std::fs::write(&f.dest_path, &f.contents)
                    .with_context(|| format!("write {}", f.dest_path.display()))?;
                let windows_bytes = crate::windows_metadata::replace_streams(
                    &f.dest_path,
                    f.windows_metadata.as_ref(),
                )?;
                if let Some(ft) = f.mtime {
                    if let Err(e) = filetime::set_file_mtime(&f.dest_path, ft) {
                        log::warn!("set mtime on {}: {}", f.dest_path.display(), e);
                    }
                }
                #[cfg(unix)]
                if let Some(perms) = f.permissions {
                    use std::os::unix::fs::PermissionsExt;
                    if let Err(e) = std::fs::set_permissions(
                        &f.dest_path,
                        std::fs::Permissions::from_mode(perms),
                    ) {
                        log::warn!("set permissions on {}: {}", f.dest_path.display(), e);
                    }
                }
                crate::windows_metadata::apply_attributes(
                    &f.dest_path,
                    f.windows_metadata.as_ref(),
                )?;
                Ok(f.size.saturating_add(windows_bytes))
            })
            .collect();
        let mut files_written = 0usize;
        let mut bytes_written = 0u64;
        for result in results {
            bytes_written += result?;
            files_written += 1;
        }
        return Ok(SinkOutcome {
            files_written,
            bytes_written,
        });
    }

    type MemberSample = (
        std::time::Duration,
        std::time::Duration,
        std::time::Duration,
        std::time::Duration,
        std::time::Duration,
        std::time::Duration,
    );
    let members_started = probe.map(|_| std::time::Instant::now());
    let results: Vec<Result<(u64, Option<MemberSample>)>> = extracted
        .into_par_iter()
        .map(|f: ExtractedFile| -> Result<(u64, Option<MemberSample>)> {
            use std::io::Write as _;

            let total_started = std::time::Instant::now();
            let mkdir_started = std::time::Instant::now();
            if let Some(parent) = f.dest_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("create dir {}", parent.display()))?;
            }
            let mkdir = mkdir_started.elapsed();

            crate::windows_metadata::prepare_destination(
                &f.dest_path,
                f.windows_metadata.as_ref(),
            )?;

            let open_started = std::time::Instant::now();
            let mut file = std::fs::File::create(&f.dest_path)
                .with_context(|| format!("open {}", f.dest_path.display()))?;
            let open = open_started.elapsed();
            let write_started = std::time::Instant::now();
            file.write_all(&f.contents)
                .with_context(|| format!("write {}", f.dest_path.display()))?;
            let write = write_started.elapsed();
            let close_started = std::time::Instant::now();
            drop(file);
            let close = close_started.elapsed();

            let metadata_started = std::time::Instant::now();
            let windows_bytes = crate::windows_metadata::replace_streams(
                &f.dest_path,
                f.windows_metadata.as_ref(),
            )?;
            if let Some(ft) = f.mtime {
                if let Err(e) = filetime::set_file_mtime(&f.dest_path, ft) {
                    log::warn!("set mtime on {}: {}", f.dest_path.display(), e);
                }
            }
            #[cfg(unix)]
            if let Some(perms) = f.permissions {
                use std::os::unix::fs::PermissionsExt;
                if let Err(e) =
                    std::fs::set_permissions(&f.dest_path, std::fs::Permissions::from_mode(perms))
                {
                    log::warn!("set permissions on {}: {}", f.dest_path.display(), e);
                }
            }
            crate::windows_metadata::apply_attributes(&f.dest_path, f.windows_metadata.as_ref())?;
            let metadata = metadata_started.elapsed();
            Ok((
                f.size.saturating_add(windows_bytes),
                Some((mkdir, open, write, close, metadata, total_started.elapsed())),
            ))
        })
        .collect();
    let member_parallel_wall = members_started.map(|started| started.elapsed());

    let mut files_written = 0usize;
    let mut bytes_written = 0u64;
    let mut member_timings = MemberTimingReport::default();
    for r in results {
        let (bytes, sample) = r?;
        bytes_written += bytes;
        files_written += 1;
        if let Some((mkdir, open, write, close, metadata, total)) = sample {
            member_timings.record(mkdir, open, write, close, metadata, total);
        }
    }

    if let Some((probe, shard_id, started, blocking_pool_wait)) = probe {
        probe.note_shard_sink(
            shard_id.to_owned(),
            probe.carrier(),
            headers.len(),
            data.len() as u64,
            started,
            blocking_pool_wait,
            parse_validate.unwrap_or_default(),
            member_parallel_wall.unwrap_or_default(),
            started.elapsed(),
            member_timings,
        );
    }

    Ok(SinkOutcome {
        files_written,
        bytes_written,
    })
}

/// Resume protocol: overwrite a block of an existing file at the given offset.
async fn write_file_block_payload(
    dst_root: &Path,
    canonical_dst_root: Option<&Path>,
    relative_path: &str,
    offset: u64,
    bytes: Vec<u8>,
) -> Result<SinkOutcome> {
    use tokio::io::{AsyncSeekExt, AsyncWriteExt};

    // R46-F3: contained resolve when canonical root is available.
    let dst = match canonical_dst_root {
        Some(canonical) => {
            crate::path_safety::safe_join_contained(canonical, dst_root, relative_path)
                .with_context(|| format!("validating block-write path {:?}", relative_path))?
        }
        None => crate::path_safety::safe_join(dst_root, relative_path)
            .with_context(|| format!("validating block-write path {:?}", relative_path))?,
    };
    let bytes_len = bytes.len() as u64;
    // Resume blocks patch existing files at offset; we want to create
    // if missing but never truncate (subsequent block records share
    // the file).
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&dst)
        .await
        .with_context(|| format!("opening {} for block write", dst.display()))?;
    file.seek(std::io::SeekFrom::Start(offset))
        .await
        .with_context(|| format!("seeking {} to offset {}", dst.display(), offset))?;
    file.write_all(&bytes)
        .await
        .with_context(|| format!("writing block to {}", dst.display()))?;
    // tokio::fs::File buffers writes and performs them on the blocking
    // pool in the background — `write_all` returning does NOT mean the
    // bytes reached the OS. Without this flush an acknowledged block can
    // land arbitrarily late (or race the finalization truncate, which
    // runs on a separate handle) — observed as the otp-7b-2 flake where
    // a faulted session's already-applied block was missing from the
    // partial under full-suite load. Flush before reporting the write
    // done, so "record applied" means the OS has the bytes.
    file.flush()
        .await
        .with_context(|| format!("flushing block write to {}", dst.display()))?;
    Ok(SinkOutcome {
        files_written: 0, // Resume blocks patch in-place; finalization counts the file.
        bytes_written: bytes_len,
    })
}

/// Resume protocol: finalize a resumed file by truncating to total_size,
/// then stamp mtime + perms from the wire. The mtime stamp is what makes
/// the "mtime touched, content identical" mirror case correct — block-hash
/// compare sends zero blocks, but BLOCK_COMPLETE still updates the dest
/// mtime to match the source.
async fn write_file_block_complete(
    dst_root: &Path,
    canonical_dst_root: Option<&Path>,
    relative_path: &str,
    total_size: u64,
    mtime_seconds: i64,
    permissions: u32,
    windows_metadata: Option<crate::generated::WindowsFileMetadata>,
) -> Result<SinkOutcome> {
    // R46-F3: contained resolve when canonical root is available.
    let dst = match canonical_dst_root {
        Some(canonical) => {
            crate::path_safety::safe_join_contained(canonical, dst_root, relative_path)
                .with_context(|| format!("validating block-complete path {:?}", relative_path))?
        }
        None => crate::path_safety::safe_join(dst_root, relative_path)
            .with_context(|| format!("validating block-complete path {:?}", relative_path))?,
    };
    crate::windows_metadata::prepare_destination(&dst, windows_metadata.as_ref())?;
    {
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .open(&dst)
            .await
            .with_context(|| format!("opening {} for truncation", dst.display()))?;
        file.set_len(total_size)
            .await
            .with_context(|| format!("truncating {} to {}", dst.display(), total_size))?;
        file.sync_all()
            .await
            .with_context(|| format!("syncing {}", dst.display()))?;
    }
    // Stamp mtime + perms after the file handle is closed (same race
    // dance as write_file_stream — see commit 946bd77).
    let windows_bytes = crate::windows_metadata::replace_streams(&dst, windows_metadata.as_ref())?;
    if mtime_seconds > 0 {
        let ft = FileTime::from_unix_time(mtime_seconds, 0);
        if let Err(e) = filetime::set_file_mtime(&dst, ft) {
            log::warn!("set mtime on {}: {}", dst.display(), e);
        }
    }
    #[cfg(unix)]
    if permissions != 0 {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(permissions))
        {
            log::warn!("set permissions on {}: {}", dst.display(), e);
        }
    }
    #[cfg(not(unix))]
    let _ = permissions;
    crate::windows_metadata::apply_attributes(&dst, windows_metadata.as_ref())?;
    Ok(SinkOutcome {
        files_written: 1,
        bytes_written: windows_bytes,
    })
}

// ---------------------------------------------------------------------------
// DataPlaneSink — TCP data plane writer
// ---------------------------------------------------------------------------

/// Writes payloads to a remote daemon via the TCP data plane binary protocol.
///
/// Each instance wraps a single TCP stream (DataPlaneSession). For multi-stream
/// transfers, the pipeline executor creates multiple DataPlaneSink instances.
pub struct DataPlaneSink<P: Probe = NoProbe> {
    session: tokio::sync::Mutex<DataPlaneSession<P>>,
    source: Arc<dyn TransferSource>,
    dst_root: PathBuf,
}

impl<P: Probe> DataPlaneSink<P> {
    pub fn new(
        session: DataPlaneSession<P>,
        source: Arc<dyn TransferSource>,
        dst_root: PathBuf,
    ) -> Self {
        Self {
            session: tokio::sync::Mutex::new(session),
            source,
            dst_root,
        }
    }
}

#[async_trait]
impl<P: Probe> TransferSink for DataPlaneSink<P> {
    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
        let mut session = self.session.lock().await;
        match payload {
            PreparedPayload::File(header) => {
                let size = header
                    .size
                    .saturating_add(crate::windows_metadata::payload_bytes(&header));
                // otp-7b-2: name the file structurally on failure, so a
                // mid-record fault reaches the end-of-operation summary.
                session
                    .send_file(self.source.clone(), &header)
                    .await
                    .with_context(|| format!("sending {}", header.relative_path))
                    .map_err(|e| {
                        e.wrap_err(crate::remote::transfer::faulted_path::FaultedPath(
                            header.relative_path.clone(),
                        ))
                    })?;
                Ok(SinkOutcome {
                    files_written: 1,
                    bytes_written: size,
                })
            }
            PreparedPayload::TarShard { headers, data } => {
                let bytes: u64 = headers
                    .iter()
                    .map(|header| {
                        header
                            .size
                            .saturating_add(crate::windows_metadata::payload_bytes(header))
                    })
                    .sum();
                let count = headers.len();
                session
                    .send_prepared_tar_shard(headers, &data)
                    .await
                    .context("sending tar shard")?;
                Ok(SinkOutcome {
                    files_written: count,
                    bytes_written: bytes,
                })
            }
            // Resume payloads can't be relayed without a reverse-resume
            // protocol on the next hop. Reject explicitly.
            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
                eyre::bail!("DataPlaneSink does not relay resume-block payloads")
            }
            // otp-7b: one resume-flagged file's whole block phase. The
            // session lock is held across the record, so every BLOCK and
            // the closing BLOCK_COMPLETE ride THIS socket in order —
            // the same strict serialization the in-stream carrier gets
            // from its single control lane. The complete record carries
            // mtime+perms from the manifest header so a zero-block
            // resume still stamps metadata at the destination.
            PreparedPayload::ResumeFile {
                header,
                block_size,
                dest_hashes,
            } => {
                use crate::remote::transfer::resume_diff::{ResumeBlockDiff, ResumeDiffEvent};
                let path = header.relative_path.clone();
                let record = async {
                    // codex otp-7b-1 F1: a mostly-matching scan is a
                    // long SILENT read+hash — arm keepalive ticks well
                    // inside the receiver's stall window and answer each
                    // with a zero-length BLOCK (a no-op in-place write),
                    // so a healthy scan never reads as a stalled peer.
                    let mut diff = ResumeBlockDiff::open(
                        &self.source,
                        &header,
                        block_size as usize,
                        dest_hashes,
                    )
                    .await?
                    .with_keepalive(
                        crate::remote::transfer::stall_guard::TRANSFER_STALL_TIMEOUT / 3,
                    );
                    let mut bytes_written: u64 = 0;
                    while let Some(event) = diff.next_event().await? {
                        match event {
                            ResumeDiffEvent::Stale { offset, bytes } => {
                                session
                                    .send_block(&header.relative_path, offset, bytes)
                                    .await
                                    .context("sending resume block")?;
                                bytes_written += bytes.len() as u64;
                            }
                            ResumeDiffEvent::KeepAlive { offset } => {
                                session
                                    .send_block(&header.relative_path, offset, &[])
                                    .await
                                    .context("sending resume keepalive block")?;
                            }
                        }
                    }
                    session
                        .send_block_complete(
                            &header.relative_path,
                            header.size,
                            header.mtime_seconds,
                            header.permissions,
                            header.windows_metadata.as_ref(),
                        )
                        .await
                        .context("sending resume block complete")?;
                    Ok(SinkOutcome {
                        files_written: 1,
                        bytes_written: bytes_written
                            .saturating_add(crate::windows_metadata::payload_bytes(&header)),
                    })
                }
                .await;
                // otp-7b-2: any failure inside the record names its file
                // structurally (the end-of-operation summary's identity).
                record.map_err(|e: eyre::Report| {
                    e.wrap_err(crate::remote::transfer::faulted_path::FaultedPath(path))
                })
            }
        }
    }

    /// Relay case: bytes arrive on `reader` (e.g. from a DataPlaneSource
    /// during a remote→remote transfer) and forward to the next hop.
    async fn write_file_stream(
        &self,
        header: &FileHeader,
        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
    ) -> Result<SinkOutcome> {
        let size = header.size;
        let mut session = self.session.lock().await;
        session
            .send_file_from_reader(header, reader)
            .await
            .with_context(|| format!("relaying {}", header.relative_path))?;
        Ok(SinkOutcome {
            files_written: 1,
            bytes_written: size.saturating_add(crate::windows_metadata::payload_bytes(header)),
        })
    }

    async fn finish(&self) -> Result<()> {
        let mut session = self.session.lock().await;
        session.finish().await
    }

    fn root(&self) -> &Path {
        &self.dst_root
    }
}

// ---------------------------------------------------------------------------
// NullSink — discard data, count bytes (for benchmarking)
// ---------------------------------------------------------------------------

/// Discards all payload data, counting files and bytes.
///
/// Useful for benchmarking source + network throughput without destination
/// I/O as a bottleneck. The pipeline still prepares payloads (reading source
/// files, building tar shards) so this measures everything except the write.
pub struct NullSink {
    label: PathBuf,
}

impl Default for NullSink {
    fn default() -> Self {
        Self {
            label: PathBuf::from("/dev/null"),
        }
    }
}

impl NullSink {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TransferSink for NullSink {
    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
        match payload {
            PreparedPayload::File(header) => Ok(SinkOutcome {
                files_written: 1,
                bytes_written: header
                    .size
                    .saturating_add(crate::windows_metadata::payload_bytes(&header)),
            }),
            PreparedPayload::TarShard { headers, data } => Ok(SinkOutcome {
                files_written: headers.len(),
                bytes_written: (data.len() as u64).saturating_add(
                    headers
                        .iter()
                        .map(crate::windows_metadata::payload_bytes)
                        .sum(),
                ),
            }),
            PreparedPayload::FileBlock { bytes, .. } => Ok(SinkOutcome {
                files_written: 0,
                bytes_written: bytes.len() as u64,
            }),
            PreparedPayload::FileBlockComplete { .. } => Ok(SinkOutcome::default()),
            // Send-side composite (otp-7b); the receive path this sink
            // benchmarks never produces it.
            PreparedPayload::ResumeFile { .. } => {
                eyre::bail!("NullSink does not consume composite ResumeFile payloads")
            }
        }
    }

    /// Drain the wire so the protocol stream stays aligned, then count
    /// the bytes. Lets `--null` benchmark the receive path end-to-end
    /// without paying for disk writes.
    async fn write_file_stream(
        &self,
        header: &FileHeader,
        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
    ) -> Result<SinkOutcome> {
        use crate::remote::transfer::data_plane::{
            receive_stream_double_buffered, RECEIVE_CHUNK_SIZE,
        };
        let mut sink = tokio::io::sink();
        // --null benchmark: bytes never land on user disk; do
        // not advance a daemon-side progress counter for these
        // drains. Same reasoning as the dry-run path on
        // FsTransferSink.
        let n = receive_stream_double_buffered(
            reader,
            &mut sink,
            header.size,
            RECEIVE_CHUNK_SIZE,
            None,
        )
        .await
        .with_context(|| format!("draining {} (null sink)", header.relative_path))?;
        Ok(SinkOutcome {
            files_written: 1,
            bytes_written: n,
        })
    }

    fn root(&self) -> &Path {
        &self.label
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_file_header(rel: &str, size: u64) -> FileHeader {
        FileHeader {
            relative_path: rel.to_string(),
            size,
            mtime_seconds: 0,
            permissions: 0o644,
            checksum: Vec::new(),
            windows_metadata: None,
        }
    }

    /// otp-11a: a file-root File payload (empty relative_path — the
    /// enumeration root was a single file) writes dst_root itself;
    /// the joins would otherwise produce trailing-slash paths that
    /// fail ENOTDIR on a regular file. The local carrier is the only
    /// producer of this shape.
    #[test]
    fn file_root_payload_copies_root_to_root() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");
        std::fs::write(&src, b"root payload").unwrap();
        let header = make_file_header("", b"root payload".len() as u64);
        let outcome =
            write_file_payload(&src, &dst, None, &header, &FsSinkConfig::default()).unwrap();
        assert_eq!(outcome.files_written, 1);
        assert_eq!(std::fs::read(&dst).unwrap(), b"root payload");
    }

    #[test]
    fn source_mtime_keeps_subsecond_precision_over_wire_fallback() {
        let tmp = tempdir().unwrap();
        let source = tmp.path().join("source.bin");
        std::fs::write(&source, b"x").unwrap();
        let requested = FileTime::from_unix_time(1_700_000_000, 123_456_700);
        filetime::set_file_mtime(&source, requested).unwrap();
        let expected = FileTime::from_last_modification_time(&std::fs::metadata(&source).unwrap());
        assert_ne!(expected.nanoseconds(), 0, "fixture lost sub-second mtime");

        let actual = source_file_mtime(
            &source,
            Some(FileTime::from_unix_time(expected.unix_seconds(), 0)),
        )
        .unwrap();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn fs_sink_copies_file() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        let content = b"hello world";
        std::fs::write(src.join("file.txt"), content).unwrap();

        let sink = FsTransferSink::new(
            src.clone(),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let header = make_file_header("file.txt", content.len() as u64);
        let outcome = sink
            .write_payload(PreparedPayload::File(header))
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.bytes_written, content.len() as u64);
        assert_eq!(std::fs::read(dst.join("file.txt")).unwrap(), content);
    }

    #[tokio::test]
    async fn fs_sink_dry_run_does_not_write() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        std::fs::write(src.join("file.txt"), b"data").unwrap();

        let sink = FsTransferSink::new(
            src,
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: true,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let header = make_file_header("file.txt", 4);
        let outcome = sink
            .write_payload(PreparedPayload::File(header))
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.bytes_written, 0);
        assert!(!dst.join("file.txt").exists());
    }

    /// R58-F4 regression: dry-run for `write_payload` must NOT
    /// create destination subdirectories. Pre-fix `write_file_payload`
    /// mkdir'd `dst/sub/` for a header `sub/file.txt` before
    /// returning from the dry-run check.
    #[tokio::test]
    async fn fs_sink_dry_run_does_not_create_destination_dirs() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("sub/file.txt"), b"data").unwrap();

        let sink = FsTransferSink::new(
            src,
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: true,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let header = make_file_header("sub/file.txt", 4);
        let _ = sink
            .write_payload(PreparedPayload::File(header))
            .await
            .unwrap();

        assert!(
            !dst.join("sub").exists(),
            "dry-run must not create destination subdirectories \
             (R58-F4 — pre-fix mkdir ran before the dry-run check)"
        );
    }

    /// R58-F4 regression for the streaming receive path. `write_file_stream`
    /// is used by remote pull receive on the CLI side and by daemon push
    /// receive — the pre-fix create_dir_all ran above the dry-run
    /// short-circuit on both.
    #[tokio::test]
    async fn fs_sink_dry_run_write_file_stream_does_not_create_dirs() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        let sink = FsTransferSink::new(
            src,
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: true,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let header = make_file_header("nested/dir/file.txt", 4);
        let mut reader: &[u8] = b"data";
        let outcome = sink.write_file_stream(&header, &mut reader).await.unwrap();

        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.bytes_written, 0);
        assert!(
            !dst.join("nested").exists(),
            "dry-run streaming receive must not create destination dirs"
        );
    }

    #[tokio::test]
    async fn fs_sink_skips_unchanged_file() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        let content = b"identical content";
        std::fs::write(src.join("same.txt"), content).unwrap();
        std::fs::write(dst.join("same.txt"), content).unwrap();

        let sink = FsTransferSink::new(
            src,
            dst,
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let header = make_file_header("same.txt", content.len() as u64);
        let outcome = sink
            .write_payload(PreparedPayload::File(header))
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.bytes_written, 0); // skipped — no copy needed
    }

    #[tokio::test]
    async fn fs_sink_extracts_tar_shard() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();

        // Build a tar archive in memory
        let mut builder = tar::Builder::new(Vec::new());
        let content_a = b"file a content";
        let content_b = b"file b content";

        let mut header_a = tar::Header::new_gnu();
        header_a.set_size(content_a.len() as u64);
        header_a.set_mode(0o644);
        header_a.set_cksum();
        builder
            .append_data(&mut header_a, "a.txt", &content_a[..])
            .unwrap();

        let mut header_b = tar::Header::new_gnu();
        header_b.set_size(content_b.len() as u64);
        header_b.set_mode(0o644);
        header_b.set_cksum();
        builder
            .append_data(&mut header_b, "sub/b.txt", &content_b[..])
            .unwrap();

        let tar_data = builder.into_inner().unwrap();

        let headers = vec![
            make_file_header("a.txt", content_a.len() as u64),
            make_file_header("sub/b.txt", content_b.len() as u64),
        ];

        // Use a dummy src_root (not used for tar shards)
        let sink = FsTransferSink::new(
            tmp.path().to_path_buf(),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let outcome = sink
            .write_payload(PreparedPayload::TarShard {
                headers,
                data: tar_data,
            })
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 2);
        assert_eq!(std::fs::read(dst.join("a.txt")).unwrap(), content_a);
        assert_eq!(std::fs::read(dst.join("sub/b.txt")).unwrap(), content_b);
    }

    #[tokio::test]
    async fn local_tar_shard_preserves_source_subsecond_mtime() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        let content = b"timestamped tar member";
        let source = src.join("member.bin");
        std::fs::write(&source, content).unwrap();
        filetime::set_file_mtime(
            &source,
            FileTime::from_unix_time(1_700_000_000, 123_456_700),
        )
        .unwrap();
        let source_time =
            FileTime::from_last_modification_time(&std::fs::metadata(&source).unwrap());
        assert_ne!(
            source_time.nanoseconds(),
            0,
            "fixture lost sub-second mtime"
        );

        let mut builder = tar::Builder::new(Vec::new());
        let mut tar_header = tar::Header::new_gnu();
        tar_header.set_size(content.len() as u64);
        tar_header.set_mode(0o644);
        tar_header.set_cksum();
        builder
            .append_data(&mut tar_header, "member.bin", &content[..])
            .unwrap();
        let data = builder.into_inner().unwrap();

        let mut file_header = make_file_header("member.bin", content.len() as u64);
        file_header.mtime_seconds = source_time.unix_seconds();
        let sink = FsTransferSink::new(src, dst.clone(), FsSinkConfig::default());
        sink.write_payload(PreparedPayload::TarShard {
            headers: vec![file_header],
            data,
        })
        .await
        .unwrap();

        let destination_time = FileTime::from_last_modification_time(
            &std::fs::metadata(dst.join("member.bin")).unwrap(),
        );
        assert_eq!(destination_time, source_time);
    }

    #[tokio::test]
    async fn fs_sink_creates_nested_directories() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(src.join("a/b/c")).unwrap();

        let content = b"deep file";
        std::fs::write(src.join("a/b/c/deep.txt"), content).unwrap();

        let sink = FsTransferSink::new(
            src,
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let header = make_file_header("a/b/c/deep.txt", content.len() as u64);
        sink.write_payload(PreparedPayload::File(header))
            .await
            .unwrap();

        assert_eq!(std::fs::read(dst.join("a/b/c/deep.txt")).unwrap(), content);
    }

    #[tokio::test]
    async fn null_sink_counts_file() {
        let sink = NullSink::new();
        let header = make_file_header("test.bin", 1024);
        let outcome = sink
            .write_payload(PreparedPayload::File(header))
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.bytes_written, 1024);
    }

    #[tokio::test]
    async fn null_sink_counts_tar_shard() {
        let sink = NullSink::new();
        let headers = vec![
            make_file_header("a.txt", 100),
            make_file_header("b.txt", 200),
            make_file_header("c.txt", 300),
        ];
        let data = vec![0u8; 4096]; // fake tar data

        let outcome = sink
            .write_payload(PreparedPayload::TarShard { headers, data })
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 3);
        assert_eq!(outcome.bytes_written, 4096);
    }

    #[tokio::test]
    async fn null_sink_root_is_dev_null() {
        let sink = NullSink::new();
        assert_eq!(sink.root(), Path::new("/dev/null"));
    }

    // ─── Path-safety end-to-end (F1) ──────────────────────────────────
    //
    // The shared `path_safety` module has its own unit tests covering the
    // validator's surface. These tests exercise the FsTransferSink end of
    // the chain to confirm a malicious wire path is rejected before any
    // filesystem write happens. They protect against future regressions
    // where a sink-level call site bypasses `safe_join`.

    async fn assert_sink_rejects(rel: &str) {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        let sink = FsTransferSink::new(
            src,
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );
        let header = make_file_header(rel, 4);
        // Use write_file_stream so we exercise the sink.rs:218 site that
        // F1 hardens. An empty reader is fine — validation happens before
        // any byte is consumed.
        let mut empty: &[u8] = b"";
        let result = sink.write_file_stream(&header, &mut empty).await;
        assert!(
            result.is_err(),
            "expected rejection for malicious wire path {:?}, but got Ok",
            rel
        );
        // Sibling-of-dst guard: nothing was written to a sibling
        // directory under tmp.
        let sibling_attack = tmp.path().join("evil");
        assert!(
            !sibling_attack.exists(),
            "malicious path {:?} caused write outside dst_root",
            rel
        );
    }

    #[tokio::test]
    async fn fs_sink_rejects_parent_dir_traversal() {
        assert_sink_rejects("../evil").await;
    }

    #[tokio::test]
    async fn fs_sink_rejects_nested_parent_dir() {
        assert_sink_rejects("subdir/../../../evil").await;
    }

    #[tokio::test]
    async fn fs_sink_rejects_unix_absolute() {
        assert_sink_rejects("/tmp/evil").await;
    }

    #[tokio::test]
    async fn fs_sink_rejects_windows_drive() {
        assert_sink_rejects("C:\\evil").await;
    }

    #[tokio::test]
    async fn fs_sink_rejects_unc() {
        assert_sink_rejects("\\\\server\\share\\evil").await;
    }

    #[tokio::test]
    async fn fs_sink_rejects_nul_byte() {
        assert_sink_rejects("foo\0bar").await;
    }

    #[tokio::test]
    async fn fs_sink_accepts_filename_containing_dot_dot() {
        // `foo..bar` is a valid filename — only `..` as a *component* is
        // dangerous. Confirms the new validator is precise enough to not
        // reject legitimate names (the previous `rel.contains("..")`
        // check was too aggressive here).
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        let sink = FsTransferSink::new(
            src,
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let content = b"valid";
        let header = make_file_header("foo..bar.txt", content.len() as u64);
        let mut reader: &[u8] = content;
        let outcome = sink
            .write_file_stream(&header, &mut reader)
            .await
            .expect("filename containing literal `..` must be accepted");

        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.bytes_written, content.len() as u64);
        assert_eq!(std::fs::read(dst.join("foo..bar.txt")).unwrap(), content);
    }

    #[tokio::test]
    async fn fs_sink_accepts_empty_path_for_single_file_dest() {
        // Single-file destination case: dst_root is itself the final
        // file path, header.relative_path == "" by convention. This
        // path must remain working even with the safe_join chokepoint.
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        // dst_root is the file path itself, not a directory.
        let dst_root = tmp.path().join("output.bin");

        let sink = FsTransferSink::new(
            src,
            dst_root.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let content = b"single-file content";
        let header = make_file_header("", content.len() as u64);
        let mut reader: &[u8] = content;
        let outcome = sink
            .write_file_stream(&header, &mut reader)
            .await
            .expect("empty relative_path must use dst_root verbatim");

        assert_eq!(outcome.bytes_written, content.len() as u64);
        assert_eq!(std::fs::read(&dst_root).unwrap(), content);
    }

    /// R46-F3 regression: a destination root containing a pre-
    /// existing escape symlink must reject any peer-controlled
    /// wire path that would write through it. Pre-fix the sink ran
    /// only `safe_join` (lexical), which accepts `link/file.txt`
    /// because lexically that's just two components — the symlink
    /// resolution happens at write time and would land outside the
    /// destination root. unix-only because the test relies on
    /// `std::os::unix::fs::symlink`.
    #[cfg(unix)]
    #[tokio::test]
    async fn fs_sink_rejects_write_through_pre_existing_escape_symlink() {
        use std::os::unix::fs::symlink;

        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        // Pre-existing escape symlink inside dst.
        symlink(&outside, dst.join("link")).unwrap();

        let sink = FsTransferSink::new(
            src,
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        // Wire path joining through `link` resolves to /outside/victim.txt.
        let content = b"would-be exfiltration";
        let header = make_file_header("link/victim.txt", content.len() as u64);
        let mut reader: &[u8] = content;
        let err = sink
            .write_file_stream(&header, &mut reader)
            .await
            .expect_err("R46-F3: write through escape symlink must be rejected");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("escape") || msg.contains("escapes"),
            "expected canonical-escape rejection, got: {msg}"
        );
        assert!(
            !outside.join("victim.txt").exists(),
            "victim file should NOT have been written outside dst"
        );
    }

    /// R47-F1 regression: the `write_payload` arm for
    /// `PreparedPayload::File` must reject a wire-controlled path
    /// that would write through a pre-existing dst escape symlink.
    /// Pre-fix `write_file_payload` lexically joined dst_root +
    /// header.relative_path, so `dst/link → /outside` plus a
    /// payload header for `link/victim` would land outside dst.
    /// The daemon's push-receive path flows through this same
    /// helper via `execute_receive_pipeline`, so this also closes
    /// the daemon-side push escape vector.
    #[cfg(unix)]
    #[tokio::test]
    async fn fs_sink_write_payload_file_rejects_escape() {
        use std::os::unix::fs::symlink;

        let tmp = tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        // Source file the planner would have prepared.
        std::fs::write(src_root.join("link/victim.txt"), b"payload").ok();
        std::fs::create_dir_all(src_root.join("link")).unwrap();
        std::fs::write(src_root.join("link/victim.txt"), b"payload").unwrap();

        // Pre-existing escape symlink in the destination.
        symlink(&outside, dst.join("link")).unwrap();

        let sink = FsTransferSink::new(
            src_root.clone(),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let header = make_file_header("link/victim.txt", 7);
        let payload = PreparedPayload::File(header);
        let err = sink
            .write_payload(payload)
            .await
            .expect_err("R47-F1: PreparedPayload::File through escape symlink must reject");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("escape") || msg.contains("escapes"),
            "expected canonical-escape rejection, got: {msg}"
        );
        assert!(
            !outside.join("victim.txt").exists(),
            "file payload must not write outside dst"
        );
    }

    /// R47-F1 regression: the `write_payload` arm for
    /// `PreparedPayload::TarShard` must reject any extracted entry
    /// whose destination path resolves outside dst via a pre-
    /// existing dst escape symlink. Pre-fix `write_tar_shard_payload`
    /// used `safe_extract_tar_shard` which does lexical
    /// validation but not canonical containment, so a tar with
    /// entry path `link/victim` plus `dst/link → /outside` would
    /// land bytes in /outside/victim.
    #[cfg(unix)]
    #[tokio::test]
    async fn fs_sink_write_payload_tar_shard_rejects_escape() {
        use std::os::unix::fs::symlink;
        use tar::{Builder, EntryType, Header as TarHeader};

        let tmp = tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        // Pre-existing escape symlink in destination.
        symlink(&outside, dst.join("link")).unwrap();

        // Build a tar with a single entry pointing through `link/`.
        let content = b"tar-shard payload";
        let mut tar_buf: Vec<u8> = Vec::new();
        {
            let mut builder = Builder::new(&mut tar_buf);
            let mut hdr = TarHeader::new_gnu();
            hdr.set_entry_type(EntryType::Regular);
            hdr.set_size(content.len() as u64);
            hdr.set_mode(0o644);
            hdr.set_path("link/victim.txt").unwrap();
            hdr.set_cksum();
            builder.append(&hdr, &content[..]).unwrap();
            builder.finish().unwrap();
        }

        let headers = vec![make_file_header("link/victim.txt", content.len() as u64)];

        let sink = FsTransferSink::new(
            src_root.clone(),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let payload = PreparedPayload::TarShard {
            headers,
            data: tar_buf,
        };
        let err = sink
            .write_payload(payload)
            .await
            .expect_err("R47-F1: tar shard entry through escape symlink must reject");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("escape") || msg.contains("escapes"),
            "expected canonical-escape rejection, got: {msg}"
        );
        assert!(
            !outside.join("victim.txt").exists(),
            "tar shard must not write outside dst"
        );
    }

    /// c-1b round 2 regression: tar shards land via `write_payload`,
    /// not `write_file_stream`, so the chunk-granular byte hook
    /// inside `receive_stream_double_buffered` never fires for them.
    /// `write_payload` now reports `outcome.bytes_written` against
    /// the sink's byte counter for non-streamed records.
    #[tokio::test]
    async fn write_payload_reports_tar_shard_bytes_against_byte_progress() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();

        let mut builder = tar::Builder::new(Vec::new());
        let content_a = b"alpha shard content";
        let content_b = b"beta shard content!";
        let mut header_a = tar::Header::new_gnu();
        header_a.set_size(content_a.len() as u64);
        header_a.set_mode(0o644);
        header_a.set_cksum();
        builder
            .append_data(&mut header_a, "a.txt", &content_a[..])
            .unwrap();
        let mut header_b = tar::Header::new_gnu();
        header_b.set_size(content_b.len() as u64);
        header_b.set_mode(0o644);
        header_b.set_cksum();
        builder
            .append_data(&mut header_b, "b.txt", &content_b[..])
            .unwrap();
        let tar_data = builder.into_inner().unwrap();
        let headers = vec![
            make_file_header("a.txt", content_a.len() as u64),
            make_file_header("b.txt", content_b.len() as u64),
        ];

        let byte_progress = ByteProgressSink::new();
        let probe_counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        // Use `from_counter` so we can read the atomic directly
        // for the assertion. Cloning the sink would also work but
        // requires re-exposing a load() — `from_counter` is the
        // cleaner observer pattern.
        let sink_progress = ByteProgressSink::from_counter(std::sync::Arc::clone(&probe_counter));
        let _ = byte_progress; // keep `new()` covered too

        let sink = FsTransferSink::new(
            tmp.path().to_path_buf(),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        )
        .with_byte_progress(sink_progress);

        let outcome = sink
            .write_payload(PreparedPayload::TarShard {
                headers,
                data: tar_data,
            })
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 2);
        let expected = (content_a.len() + content_b.len()) as u64;
        assert_eq!(outcome.bytes_written, expected);
        assert_eq!(
            probe_counter.load(std::sync::atomic::Ordering::Relaxed),
            expected,
            "tar shard byte progress must equal outcome.bytes_written"
        );
    }

    /// c-1b round 2 regression: resume `FileBlock` payloads
    /// also land via `write_payload`. Their `bytes_written`
    /// reflects the bytes seeked-and-written; the byte counter
    /// must see them too.
    #[tokio::test]
    async fn write_payload_reports_file_block_bytes_against_byte_progress() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        // FileBlock writes seek into an existing destination file.
        // Pre-create the target with a placeholder of the right size.
        std::fs::write(dst.join("resume.bin"), vec![0u8; 64]).unwrap();

        let probe_counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let sink_progress = ByteProgressSink::from_counter(std::sync::Arc::clone(&probe_counter));

        let sink = FsTransferSink::new(
            tmp.path().to_path_buf(),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        )
        .with_byte_progress(sink_progress);

        let block_bytes = vec![0xABu8; 32];
        let outcome = sink
            .write_payload(PreparedPayload::FileBlock {
                relative_path: "resume.bin".to_string(),
                offset: 16,
                bytes: block_bytes.clone(),
            })
            .await
            .expect("block write succeeds against pre-created file");

        // FileBlock's outcome.bytes_written reflects bytes
        // landed on disk for this block.
        assert_eq!(outcome.bytes_written, block_bytes.len() as u64);
        assert_eq!(
            probe_counter.load(std::sync::atomic::Ordering::Relaxed),
            block_bytes.len() as u64,
            "FileBlock byte progress must equal outcome.bytes_written"
        );
    }
}
