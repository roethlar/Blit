//! Local transfers on the unified session (otp-11,
//! `docs/plan/OTP11_LOCAL_SESSION.md`).
//!
//! [`run_local_session`] joins both role drivers over
//! [`super::transport::in_process_pair`] — the same choreography as every
//! remote session (manifest streaming, destination-owned diff, the one
//! mirror delete rule, the destination-computed summary) — with the
//! LOCAL byte-carrier: a [`LocalApply`] extension on the destination
//! config under which needed files are applied in-process through the
//! shared payload planner and [`FsTransferSink`] (clonefile /
//! block-clone / copy_file_range where the platform has them), so no
//! payload byte rides any transport. `LocalApply` is process-local
//! config with no wire representation: only a caller holding BOTH
//! roots — this entry — can construct it (D-2026-07-05-3's
//! capability-selected write strategy, never role or initiator).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Instant;

use async_trait::async_trait;
use eyre::{eyre, Context, Result};
use tokio::sync::mpsc;

use crate::fs_enum::FileFilter;
use crate::generated::{FileHeader, MirrorMode, SessionOpen, TransferRole};
use crate::path_posix::relative_path_to_posix;
use crate::remote::transfer::payload::{TransferPayload, DEFAULT_PAYLOAD_PREFETCH};
use crate::remote::transfer::pipeline::execute_sink_pipeline_streaming;
use crate::remote::transfer::sink::{
    FsSinkConfig, FsTransferSink, NullSink, SinkOutcome, TransferSink,
};
use crate::remote::transfer::source::{
    FilteredSource, FsTransferSource, SourceScan, TransferSource,
};
use crate::remote::transfer::{RemoteTransferProgress, SmallFileProbe};
use crate::transfer_plan::PlanOptions;

use super::transport::in_process_pair;
use super::{
    run_destination, run_source, DestinationInstruments, DestinationSessionConfig,
    DestinationTarget, HelloConfig, SessionEndpoint, SourceInstruments, SourceSessionConfig,
};

// ---------------------------------------------------------------------------
// The local option/summary surface (re-homed from the deleted
// engine/options.rs + engine/summary.rs at otp-11b — the engine died;
// these types are the app-facing local contract, D2 of the slice doc).
// ---------------------------------------------------------------------------

/// Scope of mirror deletions. Matches the wire-side `MirrorMode` enum
/// (FilteredSubset / All). R58-F6 brought local up to parity with the
/// remote paths' wire `MirrorMode` scope.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LocalMirrorDeleteScope {
    /// Default: only delete destination entries that the source-side
    /// filter would have allowed. Files matching `--exclude` patterns
    /// at the destination are left alone, because they're not in
    /// scope for this mirror operation.
    #[default]
    FilteredSubset,
    /// Delete every destination entry not present at the source,
    /// regardless of filter scope. Selected via `--delete-scope all`.
    All,
}

/// Local comparison policy. Mirrors the wire-side `ComparisonMode` enum
/// so local copy/mirror behaves the same as a same-options remote run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LocalCompareMode {
    /// Default size + mtime. Skip if both match.
    #[default]
    SizeMtime,
    /// Compare by Blake3 checksum. Slow but content-accurate.
    Checksum,
    /// Compare by size only. Mtime differences are ignored.
    SizeOnly,
    /// Transfer regardless of target state.
    Force,
    /// Transfer all files unconditionally (--ignore-times). Same
    /// outcome as Force at the planner level; kept as a separate
    /// variant so the user's intent is preserved in summaries.
    IgnoreTimes,
}

impl LocalCompareMode {
    /// Resolve onto the unified wire-side `ComparisonMode`, honoring
    /// the legacy `checksum: bool` under the default `SizeMtime`
    /// (back-compat: `--checksum` callers that haven't migrated to
    /// `compare_mode` keep their behavior). ue-r2-1c: the single home
    /// for this translation.
    pub fn resolve_comparison_mode(
        self,
        legacy_checksum: bool,
    ) -> crate::generated::ComparisonMode {
        use crate::generated::ComparisonMode;
        match self {
            LocalCompareMode::Checksum => ComparisonMode::Checksum,
            LocalCompareMode::SizeOnly => ComparisonMode::SizeOnly,
            LocalCompareMode::Force => ComparisonMode::Force,
            LocalCompareMode::IgnoreTimes => ComparisonMode::IgnoreTimes,
            LocalCompareMode::SizeMtime => {
                if legacy_checksum {
                    ComparisonMode::Checksum
                } else {
                    ComparisonMode::SizeMtime
                }
            }
        }
    }

    /// Same resolution, onto the perf-history snapshot enum (tuning
    /// buckets key on the full comparison policy — R59 finding #5).
    pub(crate) fn resolve_compare_snapshot(
        self,
        legacy_checksum: bool,
    ) -> crate::perf_history::CompareModeSnapshot {
        use crate::perf_history::CompareModeSnapshot;
        match self {
            LocalCompareMode::Checksum => CompareModeSnapshot::Checksum,
            LocalCompareMode::SizeOnly => CompareModeSnapshot::SizeOnly,
            LocalCompareMode::Force => CompareModeSnapshot::Force,
            LocalCompareMode::IgnoreTimes => CompareModeSnapshot::IgnoreTimes,
            LocalCompareMode::SizeMtime => {
                if legacy_checksum {
                    CompareModeSnapshot::Checksum
                } else {
                    CompareModeSnapshot::SizeMtime
                }
            }
        }
    }
}

/// Options for executing a local mirror/copy operation. The dead
/// engine-era axes (`force_tar`, `preserve_symlinks`,
/// `include_symlinks`, `skip_unchanged`) retired with the engine at
/// otp-11b — none was reachable from any production caller (slice doc
/// D2/F6 adjudication).
#[derive(Clone, Debug)]
pub struct LocalMirrorOptions {
    pub filter: FileFilter,
    pub mirror: bool,
    pub dry_run: bool,
    pub progress: bool,
    pub verbose: bool,
    pub perf_history: bool,
    /// Skip any file the destination already has, regardless of
    /// comparison mode. Orthogonal to `checksum`; matches the wire
    /// `ignore_existing` for full route parity.
    pub ignore_existing: bool,
    /// Explicitly discard Windows attributes and named data streams at the
    /// SOURCE. False preserves strictly.
    pub drop_windows_metadata: bool,
    pub checksum: bool,
    /// R58-F7: comparison policy — `--size-only` / `--ignore-times` /
    /// `--force` honored on local copy/mirror the same way the remote
    /// routes honor them.
    pub compare_mode: LocalCompareMode,
    /// R58-F6: delete-scope policy for mirror. Only consulted when
    /// `mirror == true`.
    pub delete_scope: LocalMirrorDeleteScope,
    /// The hidden `--workers` debug limiter (always paired with
    /// `debug_mode`); bounds the apply pipeline's worker count.
    pub workers: usize,
    pub preserve_times: bool,
    pub debug_mode: bool,
    /// Resume interrupted transfers using block-level comparison (the
    /// local carrier's sink-level block phase).
    pub resume: bool,
    /// Discard writes (NullSink). Measures source read + pipeline
    /// throughput.
    pub null_sink: bool,
}

impl Default for LocalMirrorOptions {
    fn default() -> Self {
        Self {
            filter: FileFilter::default(),
            mirror: false,
            dry_run: false,
            progress: false,
            verbose: false,
            perf_history: true,
            ignore_existing: false,
            drop_windows_metadata: false,
            checksum: false,
            compare_mode: LocalCompareMode::default(),
            delete_scope: LocalMirrorDeleteScope::default(),
            workers: num_cpus::get().max(1),
            preserve_times: true,
            debug_mode: false,
            resume: false,
            null_sink: false,
        }
    }
}

/// Why a transfer copied zero files. `JournalSkip` retired at otp-11b
/// with the unsound engine journal fast path (proven silent data loss
/// — see `docs/bench/otp11-local-2026-07-11/README.md`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TransferOutcome {
    /// Normal case: some work was attempted (files examined, possibly copied).
    #[default]
    Transferred,
    /// The run examined the source and found it up to date with the dest.
    UpToDate,
    /// The run examined the source and it contained zero files.
    SourceEmpty,
}

/// Summary of a local transfer execution.
///
///   - `scanned_files` / `scanned_bytes`: the source-side workload
///     observed by enumeration (post-filter, pre-diff).
///   - `planned_files`: entries the diff decided to transfer.
///   - `copied_files`: what the apply pipeline actually wrote.
///   - `total_bytes`: bytes the pipeline wrote — distinct from
///     `scanned_bytes` on incremental runs.
#[derive(Clone, Debug, Default)]
pub struct LocalMirrorSummary {
    pub planned_files: usize,
    pub copied_files: usize,
    pub total_bytes: u64,
    pub scanned_files: usize,
    pub scanned_bytes: u64,
    pub deleted_files: usize,
    pub deleted_dirs: usize,
    pub dry_run: bool,
    pub duration: std::time::Duration,
    pub tar_shard_tasks: usize,
    pub tar_shard_files: usize,
    pub tar_shard_bytes: u64,
    pub raw_bundle_tasks: usize,
    pub raw_bundle_files: usize,
    pub raw_bundle_bytes: u64,
    pub large_tasks: usize,
    pub large_bytes: u64,
    /// Classifier for the CLI summary line.
    pub outcome: TransferOutcome,
    /// R47-F4: source-side paths that couldn't be scanned or read.
    /// Destructive follow-ups in the caller — most importantly
    /// `blit move`'s source-side delete — MUST inspect this and
    /// refuse when non-empty.
    pub unreadable_paths: Vec<String>,
}

/// Process-local destination extension: apply needed files in-process
/// instead of requesting them from the source. Constructed only by
/// [`run_local_session`] — the fields are crate-private, so no caller
/// outside this crate (and no wire peer, which has no representation
/// for it at all) can select the local carrier.
pub struct LocalApply {
    /// Source root for the payload planner (absolute paths =
    /// `src_root.join(relative_path)`).
    pub(super) src_root: PathBuf,
    /// The pre-built local write backend (FsTransferSink with the full
    /// user config, or NullSink under `--null`).
    pub(super) sink: Arc<dyn TransferSink>,
    /// Unfiltered source used by the apply pipeline to prepare
    /// payloads (tar builds, availability checks). Filtering already
    /// happened at scan time; prepare only reads planned entries.
    pub(super) prepare_source: Arc<dyn TransferSource>,
    /// Planner knobs for grouping needed headers into payloads —
    /// the same planner the session source uses for its needs.
    pub(super) plan_options: PlanOptions,
    /// Mirror delete scope under `MirrorMode::FilteredSubset`: the
    /// user's `FileFilter` directly (process-local twin of deriving it
    /// from the wire `SessionOpen.filter` — same type, same delete
    /// pass).
    pub(super) mirror_scope_filter: FileFilter,
    /// `--dry-run`: the sink already refuses writes; the mirror delete
    /// pass runs in plan-only mode (counts, deletes nothing).
    pub(super) dry_run: bool,
    /// Pipeline worker count: 1 (the old streaming pipeline's default
    /// shape) unless the hidden `--workers` debug limiter set
    /// `debug_mode` (codex otp-11a F7).
    pub(super) sink_workers: usize,
    /// Shared unreadable-path accumulator (same Arc the source scan
    /// feeds): apply-side availability failures land here too, so
    /// `blit move`'s source-delete gate sees one merged list.
    pub(super) unreadable: Arc<StdMutex<Vec<String>>>,
    /// Counters the entry folds into [`LocalMirrorSummary`] afterward.
    pub(super) stats: Arc<LocalApplyStats>,
}

/// Destination-side counters for the local summary. Atomics because
/// the diff loop (control lane) and the delete pass (SourceDone arm)
/// write them at different points of the session.
#[derive(Default)]
pub struct LocalApplyStats {
    pub(super) scanned_files: AtomicU64,
    pub(super) scanned_bytes: AtomicU64,
    pub(super) tar_shard_tasks: AtomicU64,
    pub(super) tar_shard_files: AtomicU64,
    pub(super) tar_shard_bytes: AtomicU64,
    pub(super) large_tasks: AtomicU64,
    pub(super) large_bytes: AtomicU64,
    pub(super) deleted_files: AtomicU64,
    pub(super) deleted_dirs: AtomicU64,
}

/// A running local-apply pipeline: the destination diff queues
/// payloads, `finish()` closes the queue and joins the pipeline for
/// the write totals (the same join discipline as the data-plane
/// receive). A run dropped WITHOUT `finish()` — a session error or a
/// cancelled future — aborts the pipeline task at its next payload
/// boundary (codex otp-11a F3): the in-flight `spawn_blocking` write
/// completes, queued payloads are dropped, and no write continues
/// behind an operation that already returned.
pub(super) struct LocalApplyRun {
    payload_tx: Option<mpsc::Sender<TransferPayload>>,
    pipeline: Option<tokio::task::JoinHandle<Result<SinkOutcome>>>,
}

impl Drop for LocalApplyRun {
    fn drop(&mut self) {
        if let Some(handle) = &self.pipeline {
            handle.abort();
        }
    }
}

impl LocalApply {
    /// Spawn the apply pipeline — the shared streaming sink pipeline
    /// (prefetched prepares, blocking-pool writes) over this config's
    /// sink.
    pub(super) fn start(&self, progress: Option<RemoteTransferProgress>) -> LocalApplyRun {
        let (payload_tx, payload_rx) =
            mpsc::channel::<TransferPayload>(DEFAULT_PAYLOAD_PREFETCH.max(1));
        let source = Arc::clone(&self.prepare_source);
        // One pipeline worker per sink handle — the old streaming
        // pipeline's default shape is one; the hidden `--workers`
        // debug limiter (which always sets debug_mode) widens it
        // (codex otp-11a F7).
        let sinks: Vec<Arc<dyn TransferSink>> = (0..self.sink_workers.max(1))
            .map(|_| Arc::clone(&self.sink))
            .collect();
        let pipeline = tokio::spawn(async move {
            execute_sink_pipeline_streaming(
                source,
                sinks,
                payload_rx,
                DEFAULT_PAYLOAD_PREFETCH,
                progress.as_ref(),
            )
            .await
        });
        LocalApplyRun {
            payload_tx: Some(payload_tx),
            pipeline: Some(pipeline),
        }
    }

    /// Group one diff chunk's needed headers into payloads, folding
    /// the planner-mix counters. Unavailable (unreadable) entries are
    /// dropped into the shared accumulator and skipped — the old local
    /// pipeline's copy-what-is-readable posture; the caller-side move
    /// gate refuses the source delete when the list is non-empty.
    pub(super) async fn plan_chunk(&self, needed: Vec<FileHeader>) -> Result<Vec<TransferPayload>> {
        if needed.is_empty() {
            return Ok(Vec::new());
        }
        let available = self
            .prepare_source
            .check_availability(needed, Arc::clone(&self.unreadable))
            .await?;
        let payloads = crate::remote::transfer::payload::plan_transfer_payloads(
            available,
            &self.src_root,
            self.plan_options,
        )?;
        for payload in &payloads {
            match payload {
                TransferPayload::TarShard { headers } => {
                    self.stats.tar_shard_tasks.fetch_add(1, Ordering::Relaxed);
                    self.stats
                        .tar_shard_files
                        .fetch_add(headers.len() as u64, Ordering::Relaxed);
                    self.stats.tar_shard_bytes.fetch_add(
                        headers.iter().map(|h| h.size).sum::<u64>(),
                        Ordering::Relaxed,
                    );
                }
                TransferPayload::File(header) => {
                    self.stats.large_tasks.fetch_add(1, Ordering::Relaxed);
                    self.stats
                        .large_bytes
                        .fetch_add(header.size, Ordering::Relaxed);
                }
                // The local planner emits only File/TarShard; resume
                // block payloads are receive-side wire shapes.
                _ => {}
            }
        }
        Ok(payloads)
    }
}

impl LocalApplyRun {
    /// Queue one payload (bounded — the diff loop inherits the
    /// pipeline's backpressure, exactly as the wire carriers lean on
    /// transport backpressure).
    pub(super) async fn queue(&self, payload: TransferPayload) -> Result<()> {
        self.payload_tx
            .as_ref()
            .expect("local apply queue used after finish")
            .send(payload)
            .await
            .map_err(|_| eyre!("local apply pipeline stopped early"))
    }

    /// Close the queue and join the pipeline. Returns the write
    /// totals; surfaces the pipeline's own error as the root cause.
    pub(super) async fn finish(mut self) -> Result<SinkOutcome> {
        self.payload_tx.take();
        let pipeline = self
            .pipeline
            .take()
            .expect("local apply pipeline joined twice");
        pipeline
            .await
            .map_err(|err| eyre!("local apply pipeline panicked: {err}"))?
    }
}

/// Source wrapper that drops manifest entries under the destination
/// subtree when the destination sits inside the source — the session
/// twin of the old engine's `exclude_dest_subtree` (pinned by
/// `nested_destination_does_not_self_copy`: without it, each run
/// re-copies the destination into itself one level deeper).
struct DestSubtreeExcludedSource {
    inner: Arc<dyn TransferSource>,
    /// POSIX-form relative path of the destination under the source
    /// root (no trailing slash).
    exclude_rel: String,
}

#[async_trait]
impl TransferSource for DestSubtreeExcludedSource {
    fn scan(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<StdMutex<Vec<String>>>,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        self.scan_with_metadata_policy(filter, unreadable_paths, true)
    }

    fn scan_without_windows_metadata(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<StdMutex<Vec<String>>>,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        self.scan_with_metadata_policy(filter, unreadable_paths, false)
    }

    async fn prepare_payload(
        &self,
        payload: TransferPayload,
    ) -> Result<crate::remote::transfer::payload::PreparedPayload> {
        self.inner.prepare_payload(payload).await
    }

    async fn check_availability(
        &self,
        headers: Vec<FileHeader>,
        unreadable_paths: Arc<StdMutex<Vec<String>>>,
    ) -> Result<Vec<FileHeader>> {
        self.inner
            .check_availability(headers, unreadable_paths)
            .await
    }

    async fn open_file(
        &self,
        header: &FileHeader,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        self.inner.open_file(header).await
    }

    fn root(&self) -> &Path {
        self.inner.root()
    }
}

impl DestSubtreeExcludedSource {
    fn scan_with_metadata_policy(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<StdMutex<Vec<String>>>,
        preserve_windows_metadata: bool,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        let (mut inner_rx, mut scan) = if preserve_windows_metadata {
            self.inner.scan(filter, unreadable_paths)
        } else {
            self.inner
                .scan_without_windows_metadata(filter, unreadable_paths)
        };
        let (tx, rx) = mpsc::channel(64);
        let exact = self.exclude_rel.clone();
        let prefix = format!("{}/", self.exclude_rel);
        let handle = tokio::spawn(async move {
            let mut forwarded = 0u64;
            while let Some(header) = inner_rx.recv().await {
                if header.relative_path == exact || header.relative_path.starts_with(&prefix) {
                    continue;
                }
                forwarded += 1;
                if tx.send(header).await.is_err() {
                    break;
                }
            }
            Ok(forwarded)
        });
        scan.replace_primary(handle);
        (rx, scan)
    }
}

/// The destination's POSIX relative path under the source root, when
/// (and only when) it nests inside it. Same lexical check the old
/// engine used.
fn dest_subtree_rel(src_root: &Path, dst_root: &Path) -> Option<String> {
    match dst_root.strip_prefix(src_root) {
        Ok(rel) if !rel.as_os_str().is_empty() => Some(relative_path_to_posix(rel)),
        _ => None,
    }
}

/// Run one LOCAL transfer as a full session: both role drivers joined
/// over the in-process pair, bytes applied through [`LocalApply`].
/// This is the ONLY local transfer entry (D-2026-07-05-1) — the
/// `blit_app::transfers::local::run` chokepoint (CLI + TUI) rides it.
pub async fn run_local_session(
    src_root: &Path,
    dst_root: &Path,
    options: LocalMirrorOptions,
) -> Result<LocalMirrorSummary> {
    let started = Instant::now();

    if !src_root.exists() {
        return Err(eyre!("source path does not exist: {}", src_root.display()));
    }
    if !options.dry_run {
        if let Some(parent) = dst_root.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create destination parent {}", parent.display())
            })?;
        }
    }

    let compare_mode = options
        .compare_mode
        .resolve_comparison_mode(options.checksum);
    let mirror_kind = if options.mirror {
        match options.delete_scope {
            LocalMirrorDeleteScope::FilteredSubset => MirrorMode::FilteredSubset,
            LocalMirrorDeleteScope::All => MirrorMode::All,
        }
    } else {
        MirrorMode::Off
    };
    let open = SessionOpen {
        initiator_role: TransferRole::Source as i32,
        compare_mode: compare_mode as i32,
        ignore_existing: options.ignore_existing,
        drop_windows_metadata: options.drop_windows_metadata,
        // The local carrier moves no bytes on any lane; in-stream keeps
        // the responder from binding a TCP data plane.
        in_stream_bytes: true,
        mirror_enabled: options.mirror,
        mirror_kind: mirror_kind as i32,
        ..Default::default()
    };

    // One merged unreadable list: scan-side (source instruments) and
    // apply-side (availability checks) — the move gate reads it whole.
    let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();

    // Source chain: fs source → user filter (the universal
    // FilteredSource chokepoint, same as push/pull) → dest-subtree
    // exclusion when dst nests inside src.
    let fs_source: Arc<dyn TransferSource> =
        Arc::new(FsTransferSource::new(src_root.to_path_buf()));
    let filtered: Arc<dyn TransferSource> = Arc::new(FilteredSource::new(
        Arc::clone(&fs_source),
        options.filter.clone_without_cache(),
    ));
    let scan_source: Arc<dyn TransferSource> = match dest_subtree_rel(src_root, dst_root) {
        Some(exclude_rel) => Arc::new(DestSubtreeExcludedSource {
            inner: filtered,
            exclude_rel,
        }),
        None => filtered,
    };

    // Local write backend — the old orchestrator's exact construction.
    let sink: Arc<dyn TransferSink> = if options.null_sink {
        Arc::new(NullSink::new())
    } else {
        Arc::new(FsTransferSink::new(
            src_root.to_path_buf(),
            dst_root.to_path_buf(),
            FsSinkConfig {
                preserve_times: options.preserve_times,
                dry_run: options.dry_run,
                checksum: if options.checksum {
                    Some(crate::checksum::ChecksumType::Blake3)
                } else {
                    None
                },
                resume: options.resume,
                compare_mode,
            },
        ))
    };

    let stats = Arc::new(LocalApplyStats::default());
    let local_apply = LocalApply {
        src_root: src_root.to_path_buf(),
        sink,
        prepare_source: Arc::clone(&fs_source),
        plan_options: PlanOptions::default(),
        mirror_scope_filter: options.filter.clone_without_cache(),
        dry_run: options.dry_run,
        sink_workers: if options.debug_mode {
            options.workers.max(1)
        } else {
            1
        },
        unreadable: Arc::clone(&unreadable),
        stats: Arc::clone(&stats),
    };

    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open),
        plan_options: PlanOptions::default(),
        data_plane_host: None,
        instruments: SourceInstruments {
            progress: None,
            unreadable: Some(Arc::clone(&unreadable)),
            trace_data_plane: false,
            session_phase_trace: Default::default(),
            small_file_probe: SmallFileProbe::disabled(),
            #[cfg(test)]
            dial_test_samples: None,
            #[cfg(test)]
            dial_terminal_test_gate: None,
            #[cfg(test)]
            dial_proposal_test_gate: None,
            #[cfg(test)]
            dial_membership_test_gate: None,
        },
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder,
        data_plane_host: None,
        receiver_capacity: None,
        instruments: DestinationInstruments {
            small_file_probe: SmallFileProbe::disabled(),
            ..Default::default()
        },
        local_apply: Some(local_apply),
    };

    let (a, b) = in_process_pair();
    let (source_result, dest_result) = tokio::join!(
        run_source(source_cfg, a, scan_source),
        run_destination(
            dest_cfg,
            b,
            DestinationTarget::Fixed(dst_root.to_path_buf())
        ),
    );
    // The destination is the scorer and holds the primary fault
    // (refusals, apply failures, delete failures); a source-only
    // failure (scan abort) surfaces when the destination succeeded.
    let outcome = match dest_result {
        Ok(outcome) => {
            source_result?;
            outcome
        }
        Err(err) => return Err(err),
    };

    let scanned_files = stats.scanned_files.load(Ordering::Relaxed) as usize;
    let scanned_bytes = stats.scanned_bytes.load(Ordering::Relaxed);
    let unreadable_paths = unreadable
        .lock()
        .map_err(|err| eyre!("unreadable-path accumulator poisoned: {err}"))?
        .clone();

    // Outcome classification mirrors the old fast-path gate (strategy
    // gate: mirror / checksum / non-SizeMtime compare all
    // forced streaming, which always reported Transferred).
    let fast_path_shape = !options.mirror
        && !options.checksum
        && matches!(options.compare_mode, LocalCompareMode::SizeMtime);
    let copied_files = outcome.summary.files_transferred as usize;
    let outcome_class = if fast_path_shape && scanned_files == 0 {
        TransferOutcome::SourceEmpty
    } else if fast_path_shape && copied_files == 0 {
        TransferOutcome::UpToDate
    } else {
        TransferOutcome::Transferred
    };

    let summary = LocalMirrorSummary {
        planned_files: outcome.needed_paths.len(),
        copied_files,
        total_bytes: outcome.summary.bytes_transferred,
        scanned_files,
        scanned_bytes,
        deleted_files: stats.deleted_files.load(Ordering::Relaxed) as usize,
        deleted_dirs: stats.deleted_dirs.load(Ordering::Relaxed) as usize,
        dry_run: options.dry_run,
        duration: started.elapsed(),
        tar_shard_tasks: stats.tar_shard_tasks.load(Ordering::Relaxed) as usize,
        tar_shard_files: stats.tar_shard_files.load(Ordering::Relaxed) as usize,
        tar_shard_bytes: stats.tar_shard_bytes.load(Ordering::Relaxed),
        raw_bundle_tasks: 0,
        raw_bundle_files: 0,
        raw_bundle_bytes: 0,
        large_tasks: stats.large_tasks.load(Ordering::Relaxed) as usize,
        large_bytes: stats.large_bytes.load(Ordering::Relaxed),
        outcome: outcome_class,
        unreadable_paths,
    };

    record_local_history(&summary, &options);

    Ok(summary)
}

/// Perf-history row for a local session run (D3 in the slice doc:
/// `blit profile` keeps its local data feed; the predictor and its
/// planner/transfer split retired with the engine, so the whole wall
/// time lands in `transfer_duration_ms`).
fn record_local_history(summary: &LocalMirrorSummary, options: &LocalMirrorOptions) {
    if !options.perf_history {
        return;
    }
    let record = build_local_record(summary, options);
    if let Err(err) = crate::perf_history::append_local_record(&record) {
        if options.verbose {
            eprintln!("Failed to update performance history: {err:?}");
        }
    }
}

/// Construct the local session's [`PerformanceRecord`] without
/// touching disk — split from the writer so the record-shape contract
/// (R44-F1's "train and query on the same feature vector" invariant,
/// carried forward as "record scanned features") stays unit-testable,
/// the same rationale the engine's `build_performance_record` had.
fn build_local_record(
    summary: &LocalMirrorSummary,
    options: &LocalMirrorOptions,
) -> crate::perf_history::PerformanceRecord {
    use crate::perf_history::{OptionSnapshot, PerformanceRecord, TransferMode};
    let snapshot = OptionSnapshot {
        dry_run: options.dry_run,
        // The engine-era option axes retired at otp-11b; the persisted
        // snapshot schema keeps the fields — record the historical
        // defaults (the only values production ever produced).
        preserve_symlinks: true,
        include_symlinks: true,
        skip_unchanged: true,
        checksum: options.checksum,
        compare_mode: options
            .compare_mode
            .resolve_compare_snapshot(options.checksum),
        workers: options.workers,
    };
    let mode = if options.mirror {
        TransferMode::Mirror
    } else {
        TransferMode::Copy
    };
    // `--null` runs keep the old `null_sink` tag: RunKind derivation
    // keys on it (perf_history.rs), and a `"session"` tag would
    // classify diagnostics runs as Real and contaminate profiling
    // (codex otp-11a F9).
    let fast_path = if options.null_sink {
        "null_sink"
    } else {
        "session"
    };
    let mut record = PerformanceRecord::new(
        mode,
        None,
        None,
        summary.scanned_files,
        summary.scanned_bytes,
        snapshot,
        Some(fast_path.to_string()),
        0,
        summary.duration.as_millis(),
        0,
        0,
    );
    record.tar_shard_tasks = summary.tar_shard_tasks as u32;
    record.tar_shard_files = summary.tar_shard_files as u32;
    record.tar_shard_bytes = summary.tar_shard_bytes;
    record.large_tasks = summary.large_tasks as u32;
    record.large_bytes = summary.large_bytes;
    record
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::{ComparisonMode, TransferSummary};
    use crate::transfer_session::DestinationOutcome;

    /// Delegates scan/prepare/open to a real fs source but drops one
    /// path at `check_availability`, recording it unreadable — the
    /// deterministic stand-in for a file vanishing between a CLEAN
    /// scan and the apply (the window the SourceDone mirror guard
    /// exists for; a mode-000 fixture is caught at scan time instead).
    struct VanishingSource {
        inner: Arc<dyn TransferSource>,
        vanish: String,
    }

    #[async_trait]
    impl TransferSource for VanishingSource {
        fn scan(
            &self,
            filter: Option<FileFilter>,
            unreadable_paths: Arc<StdMutex<Vec<String>>>,
        ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
            self.inner.scan(filter, unreadable_paths)
        }

        async fn prepare_payload(
            &self,
            payload: TransferPayload,
        ) -> eyre::Result<crate::remote::transfer::payload::PreparedPayload> {
            self.inner.prepare_payload(payload).await
        }

        async fn check_availability(
            &self,
            headers: Vec<FileHeader>,
            unreadable_paths: Arc<StdMutex<Vec<String>>>,
        ) -> eyre::Result<Vec<FileHeader>> {
            let (gone, available): (Vec<_>, Vec<_>) = headers
                .into_iter()
                .partition(|h| h.relative_path == self.vanish);
            if !gone.is_empty() {
                unreadable_paths
                    .lock()
                    .expect("accumulator lock")
                    .push(self.vanish.clone());
            }
            Ok(available)
        }

        async fn open_file(
            &self,
            header: &FileHeader,
        ) -> eyre::Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
            self.inner.open_file(header).await
        }

        fn root(&self) -> &Path {
            self.inner.root()
        }
    }

    /// R46-F2 carried onto the local carrier (codex otp-11a F4): a
    /// source entry that vanishes AFTER a clean scan (recorded
    /// unreadable by the apply's availability check) must refuse the
    /// mirror at SourceDone, before any deletion — the old engine
    /// refused mirror deletions on ANY unreadable entry.
    #[tokio::test]
    async fn mirror_refuses_when_availability_drops_after_clean_scan() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).expect("mkdir src");
        std::fs::create_dir_all(&dst_root).expect("mkdir dst");
        std::fs::write(src_root.join("ok.txt"), b"fine").expect("write");
        std::fs::write(src_root.join("gone.txt"), b"vanishes").expect("write");
        std::fs::write(dst_root.join("extraneous.txt"), b"would die").expect("write");

        let open = SessionOpen {
            initiator_role: TransferRole::Source as i32,
            compare_mode: ComparisonMode::SizeMtime as i32,
            in_stream_bytes: true,
            mirror_enabled: true,
            mirror_kind: MirrorMode::All as i32,
            ..Default::default()
        };
        let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
        let fs_source: Arc<dyn TransferSource> = Arc::new(FsTransferSource::new(src_root.clone()));
        let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
            src_root.clone(),
            dst_root.clone(),
            FsSinkConfig::default(),
        ));
        let local_apply = LocalApply {
            src_root: src_root.clone(),
            sink,
            prepare_source: Arc::new(VanishingSource {
                inner: fs_source,
                vanish: "gone.txt".to_string(),
            }),
            plan_options: PlanOptions::default(),
            mirror_scope_filter: FileFilter::default(),
            dry_run: false,
            sink_workers: 1,
            unreadable: Arc::clone(&unreadable),
            stats: Arc::new(LocalApplyStats::default()),
        };
        let source_cfg = SourceSessionConfig {
            hello: HelloConfig::default(),
            endpoint: SessionEndpoint::initiator(open),
            plan_options: PlanOptions::default(),
            data_plane_host: None,
            instruments: SourceInstruments {
                progress: None,
                unreadable: Some(Arc::clone(&unreadable)),
                trace_data_plane: false,
                session_phase_trace: Default::default(),
                small_file_probe: SmallFileProbe::disabled(),
                #[cfg(test)]
                dial_test_samples: None,
                #[cfg(test)]
                dial_terminal_test_gate: None,
                #[cfg(test)]
                dial_proposal_test_gate: None,
                #[cfg(test)]
                dial_membership_test_gate: None,
            },
        };
        let dest_cfg = DestinationSessionConfig {
            hello: HelloConfig::default(),
            endpoint: SessionEndpoint::Responder,
            data_plane_host: None,
            receiver_capacity: None,
            instruments: DestinationInstruments {
                small_file_probe: SmallFileProbe::disabled(),
                ..Default::default()
            },
            local_apply: Some(local_apply),
        };
        let (a, b) = in_process_pair();
        let scan_source: Arc<dyn TransferSource> =
            Arc::new(FsTransferSource::new(src_root.clone()));
        let (_, dest_result): (
            eyre::Result<TransferSummary>,
            eyre::Result<DestinationOutcome>,
        ) = tokio::join!(
            run_source(source_cfg, a, scan_source),
            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
        );

        let err = dest_result.expect_err("apply-time unreadable must refuse the mirror");
        assert!(
            format!("{err:#}").contains("could not be read during the transfer"),
            "unexpected error: {err:#}"
        );
        assert!(
            dst_root.join("extraneous.txt").exists(),
            "a refused mirror must not have deleted anything"
        );
    }

    #[test]
    fn dest_subtree_rel_detects_nesting() {
        assert_eq!(
            dest_subtree_rel(Path::new("/a/src"), Path::new("/a/src/nested/dst")),
            Some("nested/dst".to_string())
        );
        assert_eq!(
            dest_subtree_rel(Path::new("/a/src"), Path::new("/a/dst")),
            None
        );
        // dst == src is not a nested-subtree shape (strip yields empty).
        assert_eq!(
            dest_subtree_rel(Path::new("/a/src"), Path::new("/a/src")),
            None
        );
    }

    /// R44-F1 carried forward: the record's `(file_count, total_bytes)`
    /// are the SCANNED features, not the copied counts.
    #[test]
    fn local_record_uses_scanned_features_not_copied() {
        let summary = LocalMirrorSummary {
            scanned_files: 1000,
            scanned_bytes: 10 * 1024 * 1024,
            planned_files: 5,
            copied_files: 5,
            total_bytes: 100 * 1024,
            duration: std::time::Duration::from_millis(200),
            ..LocalMirrorSummary::default()
        };
        let record = build_local_record(&summary, &LocalMirrorOptions::default());
        assert_eq!(record.file_count, 1000);
        assert_eq!(record.total_bytes, summary.scanned_bytes);
        assert_eq!(record.transfer_duration_ms, 200);
        assert_eq!(record.fast_path.as_deref(), Some("session"));
    }

    /// Bucket-shape fields still reflect actual apply activity.
    #[test]
    fn local_record_carries_bucket_counters() {
        let summary = LocalMirrorSummary {
            scanned_files: 100,
            scanned_bytes: 1_000_000,
            tar_shard_tasks: 2,
            tar_shard_files: 7,
            tar_shard_bytes: 30_000,
            large_tasks: 1,
            large_bytes: 5_000,
            ..LocalMirrorSummary::default()
        };
        let record = build_local_record(&summary, &LocalMirrorOptions::default());
        assert_eq!(record.tar_shard_tasks, 2);
        assert_eq!(record.tar_shard_files, 7);
        assert_eq!(record.tar_shard_bytes, 30_000);
        assert_eq!(record.large_tasks, 1);
        assert_eq!(record.large_bytes, 5_000);
    }

    /// codex otp-11a F9: `--null` runs keep the `null_sink` tag so
    /// RunKind derivation classifies them as diagnostics, and dry-run
    /// records carry `dry_run` for the same lane split.
    #[test]
    fn local_record_null_and_dry_run_lanes() {
        use crate::perf_history::RunKind;
        let summary = LocalMirrorSummary::default();
        let null = build_local_record(
            &summary,
            &LocalMirrorOptions {
                null_sink: true,
                ..LocalMirrorOptions::default()
            },
        );
        assert_eq!(null.fast_path.as_deref(), Some("null_sink"));
        assert_eq!(null.run_kind, RunKind::NullSink);
        let dry = build_local_record(
            &summary,
            &LocalMirrorOptions {
                dry_run: true,
                ..LocalMirrorOptions::default()
            },
        );
        assert_eq!(dry.run_kind, RunKind::DryRun);
    }

    /// ue-r2-1c single-home mapping: every `LocalCompareMode` variant
    /// resolves onto its wire `ComparisonMode`, and the legacy
    /// `--checksum` bool upgrades the SizeMtime default only.
    #[test]
    fn compare_mode_resolves_onto_wire_enum() {
        assert_eq!(
            LocalCompareMode::SizeMtime.resolve_comparison_mode(false),
            ComparisonMode::SizeMtime
        );
        assert_eq!(
            LocalCompareMode::SizeMtime.resolve_comparison_mode(true),
            ComparisonMode::Checksum
        );
        assert_eq!(
            LocalCompareMode::Checksum.resolve_comparison_mode(false),
            ComparisonMode::Checksum
        );
        assert_eq!(
            LocalCompareMode::SizeOnly.resolve_comparison_mode(true),
            ComparisonMode::SizeOnly,
            "legacy checksum must not override an explicit non-default mode"
        );
        assert_eq!(
            LocalCompareMode::Force.resolve_comparison_mode(false),
            ComparisonMode::Force
        );
        assert_eq!(
            LocalCompareMode::IgnoreTimes.resolve_comparison_mode(false),
            ComparisonMode::IgnoreTimes
        );
    }

    /// The perf-history snapshot mapping mirrors the wire mapping
    /// (tuning buckets key on the full comparison policy, R59 #5).
    #[test]
    fn compare_mode_resolves_onto_snapshot_enum() {
        use crate::perf_history::CompareModeSnapshot;
        assert_eq!(
            LocalCompareMode::SizeMtime.resolve_compare_snapshot(true),
            CompareModeSnapshot::Checksum
        );
        assert_eq!(
            LocalCompareMode::IgnoreTimes.resolve_compare_snapshot(false),
            CompareModeSnapshot::IgnoreTimes
        );
        assert_eq!(
            LocalCompareMode::SizeMtime.resolve_compare_snapshot(false),
            CompareModeSnapshot::SizeMtime
        );
    }

    /// The dest-subtree exclusion wrapper forwards everything outside
    /// the excluded prefix and drops everything under it (the manifest
    /// the destination diff sees never contains the destination).
    #[tokio::test]
    async fn dest_subtree_excluded_source_filters_the_stream() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        std::fs::create_dir_all(src_root.join("backup")).expect("mkdir");
        std::fs::write(src_root.join("a.txt"), b"keep").expect("write");
        std::fs::write(src_root.join("b.txt"), b"keep").expect("write");
        std::fs::write(src_root.join("backup/old.txt"), b"drop").expect("write");

        let wrapper = DestSubtreeExcludedSource {
            inner: Arc::new(FsTransferSource::new(src_root.clone())),
            exclude_rel: "backup".to_string(),
        };
        let (mut rx, mut scan) = wrapper.scan(None, Arc::default());
        let mut forwarded = Vec::new();
        while let Some(h) = rx.recv().await {
            forwarded.push(h.relative_path);
        }
        forwarded.sort();
        assert_eq!(forwarded, vec!["a.txt".to_string(), "b.txt".to_string()]);
        let count = scan.finish().await.expect("scan");
        assert_eq!(count, 2, "the forwarded count excludes the subtree");
    }

    /// The streaming-overlap property, ported from the engine's
    /// `first_work_lands_before_enumeration_completes`: with more than
    /// one diff chunk of files, the first destination writes land
    /// while the source scan is still running. A gating source holds
    /// the manifest stream open after `DEST_DIFF_CHUNK` + a few
    /// entries until the test observes a file at the destination.
    #[tokio::test]
    async fn first_apply_lands_before_enumeration_completes() {
        use tokio::sync::oneshot;

        struct GatedSource {
            inner: Arc<dyn TransferSource>,
            gate: StdMutex<Option<oneshot::Receiver<()>>>,
        }

        #[async_trait]
        impl TransferSource for GatedSource {
            fn scan(
                &self,
                filter: Option<FileFilter>,
                unreadable_paths: Arc<StdMutex<Vec<String>>>,
            ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
                let (mut inner_rx, mut scan) = self.inner.scan(filter, unreadable_paths);
                let (tx, rx) = mpsc::channel(8);
                let gate = self
                    .gate
                    .lock()
                    .expect("gate lock")
                    .take()
                    .expect("scan called once");
                let handle = tokio::spawn(async move {
                    let mut forwarded = 0u64;
                    let mut gate = Some(gate);
                    while let Some(h) = inner_rx.recv().await {
                        forwarded += 1;
                        if tx.send(h).await.is_err() {
                            break;
                        }
                        // Hold the manifest open once a full diff chunk
                        // (plus slack) is out, until the gate fires.
                        if forwarded == 160 {
                            if let Some(g) = gate.take() {
                                let _ = g.await;
                            }
                        }
                    }
                    Ok(forwarded)
                });
                scan.replace_primary(handle);
                (rx, scan)
            }

            async fn prepare_payload(
                &self,
                payload: TransferPayload,
            ) -> eyre::Result<crate::remote::transfer::payload::PreparedPayload> {
                self.inner.prepare_payload(payload).await
            }

            async fn check_availability(
                &self,
                headers: Vec<FileHeader>,
                unreadable_paths: Arc<StdMutex<Vec<String>>>,
            ) -> eyre::Result<Vec<FileHeader>> {
                self.inner
                    .check_availability(headers, unreadable_paths)
                    .await
            }

            async fn open_file(
                &self,
                header: &FileHeader,
            ) -> eyre::Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
                self.inner.open_file(header).await
            }

            fn root(&self) -> &Path {
                self.inner.root()
            }
        }

        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).expect("mkdir");
        std::fs::create_dir_all(&dst_root).expect("mkdir");
        for i in 0..200 {
            std::fs::write(src_root.join(format!("f{i:03}.txt")), b"payload").expect("write");
        }

        let (gate_tx, gate_rx) = oneshot::channel();
        // Watcher: fire the gate as soon as ANY file lands at the dest —
        // proof that apply work started before the scan completed.
        let dst_watch = dst_root.clone();
        let watcher = tokio::spawn(async move {
            for _ in 0..1000 {
                let landed = std::fs::read_dir(&dst_watch)
                    .map(|d| d.count())
                    .unwrap_or(0);
                if landed > 0 {
                    let _ = gate_tx.send(());
                    return true;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            false
        });

        let scan_source: Arc<dyn TransferSource> = Arc::new(GatedSource {
            inner: Arc::new(FsTransferSource::new(src_root.clone())),
            gate: StdMutex::new(Some(gate_rx)),
        });
        let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
            src_root.clone(),
            dst_root.clone(),
            FsSinkConfig::default(),
        ));
        let local_apply = LocalApply {
            src_root: src_root.clone(),
            sink,
            prepare_source: Arc::new(FsTransferSource::new(src_root.clone())),
            plan_options: PlanOptions::default(),
            mirror_scope_filter: FileFilter::default(),
            dry_run: false,
            sink_workers: 1,
            unreadable: Arc::default(),
            stats: Arc::new(LocalApplyStats::default()),
        };
        let open = SessionOpen {
            initiator_role: TransferRole::Source as i32,
            compare_mode: ComparisonMode::SizeMtime as i32,
            in_stream_bytes: true,
            ..Default::default()
        };
        let source_cfg = SourceSessionConfig {
            hello: HelloConfig::default(),
            endpoint: SessionEndpoint::initiator(open),
            plan_options: PlanOptions::default(),
            data_plane_host: None,
            instruments: SourceInstruments::default(),
        };
        let dest_cfg = DestinationSessionConfig {
            hello: HelloConfig::default(),
            endpoint: SessionEndpoint::Responder,
            data_plane_host: None,
            receiver_capacity: None,
            instruments: DestinationInstruments::default(),
            local_apply: Some(local_apply),
        };
        let (a, b) = in_process_pair();
        let (source_result, dest_result) =
            tokio::time::timeout(std::time::Duration::from_secs(30), async {
                tokio::join!(
                    run_source(source_cfg, a, scan_source),
                    run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
                )
            })
            .await
            .expect("session timed out — apply never overlapped the gated scan");
        source_result.expect("source");
        let outcome = dest_result.expect("destination");
        assert_eq!(outcome.summary.files_transferred, 200);
        assert!(
            watcher.await.expect("watcher"),
            "a destination write must land before enumeration completes"
        );
    }
}
