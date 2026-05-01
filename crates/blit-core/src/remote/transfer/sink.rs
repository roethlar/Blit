//! Pluggable write backends for the transfer pipeline.
//!
//! Every src→dst combination flows through `TransferSource → plan → prepare → TransferSink`.
//! Implementations handle the actual write: local filesystem, TCP data plane, etc.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use eyre::{Context, Result};
use filetime::FileTime;
use tar::Archive;

use crate::buffer::BufferSizer;
use crate::checksum::ChecksumType;
use crate::copy::{copy_file, file_needs_copy_with_checksum_type, resume_copy_file};
use crate::generated::FileHeader;
use crate::logger::NoopLogger;
use crate::remote::transfer::payload::PreparedPayload;
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
    /// double-buffering into a `'static` reader. Sinks that don't
    /// support inbound streaming (e.g. `GrpcFallbackSink`) inherit the
    /// default error implementation.
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
}

/// Writes files directly to a local filesystem using zero-copy primitives
/// (copy_file_range, sendfile, clonefile, block clone) where available.
pub struct FsTransferSink {
    src_root: PathBuf,
    dst_root: PathBuf,
    config: FsSinkConfig,
    /// Optional collector for relative paths of successfully-written
    /// files. Used by remote pull's mirror flow to know which files to
    /// keep when purging extraneous local entries. Each successful
    /// `write_payload`/`write_file_stream` pushes its `relative_path`.
    path_tracker: Option<Arc<std::sync::Mutex<Vec<PathBuf>>>>,
}

impl FsTransferSink {
    pub fn new(src_root: PathBuf, dst_root: PathBuf, config: FsSinkConfig) -> Self {
        Self {
            src_root,
            dst_root,
            config,
            path_tracker: None,
        }
    }

    /// Enable path tracking. After each successful write, the relative
    /// path of the written file is pushed onto the supplied collector.
    /// Lets receive callers (e.g. mirror) discover which files survived
    /// without re-implementing the record dispatch loop.
    pub fn with_path_tracker(
        mut self,
        tracker: Arc<std::sync::Mutex<Vec<PathBuf>>>,
    ) -> Self {
        self.path_tracker = Some(tracker);
        self
    }

    fn track(&self, rel: &str) {
        if let Some(tracker) = &self.path_tracker {
            if let Ok(mut guard) = tracker.lock() {
                guard.push(PathBuf::from(rel));
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
        match payload {
            PreparedPayload::FileBlock {
                relative_path,
                offset,
                bytes,
            } => write_file_block_payload(&self.dst_root, &relative_path, offset, bytes).await,
            PreparedPayload::FileBlockComplete {
                relative_path,
                total_size,
            } => {
                let outcome =
                    write_file_block_complete(&self.dst_root, &relative_path, total_size).await?;
                if outcome.files_written > 0 {
                    self.track(&relative_path);
                }
                Ok(outcome)
            }
            PreparedPayload::File(_) | PreparedPayload::TarShard { .. } => {
                // Capture paths for tracking before payload moves into
                // the spawn_blocking closure.
                let tracked_paths: Vec<String> = match &payload {
                    PreparedPayload::File(h) => vec![h.relative_path.clone()],
                    PreparedPayload::TarShard { headers, .. } => {
                        headers.iter().map(|h| h.relative_path.clone()).collect()
                    }
                    _ => Vec::new(),
                };
                let src_root = self.src_root.clone();
                let dst_root = self.dst_root.clone();
                let config = self.config.clone();
                let outcome = tokio::task::spawn_blocking(move || match payload {
                    PreparedPayload::File(header) => {
                        write_file_payload(&src_root, &dst_root, &header, &config)
                    }
                    PreparedPayload::TarShard { headers, data } => {
                        write_tar_shard_payload(&dst_root, &headers, &data, &config)
                    }
                    _ => unreachable!("outer match guarantees File or TarShard"),
                })
                .await
                .context("sink worker panicked")??;
                if outcome.files_written > 0 {
                    for path in tracked_paths {
                        self.track(&path);
                    }
                }
                Ok(outcome)
            }
        }
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

        // Single-file source emits relative_path="" — destination is
        // already the full target path. PathBuf::join("") on Unix
        // appends a trailing separator that File::create rejects with
        // ENOTDIR, so handle empty as identity.
        let dst = if header.relative_path.is_empty() {
            self.dst_root.clone()
        } else {
            self.dst_root.join(&header.relative_path)
        };
        if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("creating directory {}", parent.display()))?;
        }

        if self.config.dry_run {
            // Drain the wire so the protocol stream stays aligned, but
            // discard the bytes.
            let mut sink = tokio::io::sink();
            receive_stream_double_buffered(reader, &mut sink, header.size, RECEIVE_CHUNK_SIZE)
                .await
                .with_context(|| format!("draining {} (dry-run)", header.relative_path))?;
            return Ok(SinkOutcome {
                files_written: 1,
                bytes_written: 0,
            });
        }

        let mut file = tokio::fs::File::create(&dst)
            .await
            .with_context(|| format!("creating {}", dst.display()))?;
        receive_stream_double_buffered(reader, &mut file, header.size, RECEIVE_CHUNK_SIZE)
            .await
            .with_context(|| format!("writing {}", dst.display()))?;
        file.sync_all()
            .await
            .with_context(|| format!("syncing {}", dst.display()))?;

        if self.config.preserve_times && header.mtime_seconds > 0 {
            let ft = FileTime::from_unix_time(header.mtime_seconds, 0);
            let _ = filetime::set_file_mtime(&dst, ft);
        }

        // Permissions arrive on the wire (Unix mode bits). Apply best-
        // effort; ignore failures (cross-fs, root-owned dst, etc.).
        #[cfg(unix)]
        if header.permissions != 0 {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(
                &dst,
                std::fs::Permissions::from_mode(header.permissions),
            );
        }
        #[cfg(not(unix))]
        let _ = header.permissions;

        self.track(&header.relative_path);

        Ok(SinkOutcome {
            files_written: 1,
            bytes_written: header.size,
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
    header: &FileHeader,
    config: &FsSinkConfig,
) -> Result<SinkOutcome> {
    let src = src_root.join(&header.relative_path);
    let dst = dst_root.join(&header.relative_path);

    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    if config.dry_run {
        return Ok(SinkOutcome {
            files_written: 1,
            bytes_written: 0,
        });
    }

    let mut did_copy = false;
    let mut clone_succeeded = false;

    if config.resume {
        let outcome = resume_copy_file(&src, &dst, 0)
            .with_context(|| format!("resume copy {}", header.relative_path))?;
        did_copy = outcome.bytes_transferred > 0;
    } else if file_needs_copy_with_checksum_type(&src, &dst, config.checksum)? {
        let sizer = BufferSizer::default();
        let logger = NoopLogger;
        let outcome = copy_file(&src, &dst, &sizer, false, &logger)
            .with_context(|| format!("copy {}", header.relative_path))?;
        did_copy = true;
        clone_succeeded = outcome.clone_succeeded;
    }

    if config.preserve_times && did_copy && !clone_succeeded {
        if let Ok(meta) = std::fs::metadata(&src) {
            if let Ok(modified) = meta.modified() {
                let ft = FileTime::from_system_time(modified);
                let _ = filetime::set_file_mtime(&dst, ft);
            }
        }
    }

    Ok(SinkOutcome {
        files_written: 1,
        bytes_written: if did_copy { header.size } else { 0 },
    })
}

/// Extract an in-memory tar shard to the destination directory.
fn write_tar_shard_payload(
    dst_root: &Path,
    headers: &[FileHeader],
    data: &[u8],
    config: &FsSinkConfig,
) -> Result<SinkOutcome> {
    if config.dry_run {
        return Ok(SinkOutcome {
            files_written: headers.len(),
            bytes_written: 0,
        });
    }

    let mut archive = Archive::new(Cursor::new(data));
    let entries = archive
        .entries()
        .context("reading tar shard entries")?;

    let mut files_written = 0usize;
    let mut bytes_written = 0u64;

    for entry_result in entries {
        let mut entry = entry_result.context("tar shard entry")?;
        if entry.header().entry_type().is_dir() {
            continue;
        }

        let rel_path = entry.path().context("tar shard path")?;
        let rel_string = rel_path.to_string_lossy().replace('\\', "/");

        // Security: reject paths with .. components
        if rel_string.contains("..") {
            eyre::bail!("tar shard contains path traversal: {}", rel_string);
        }

        let dest_path = dst_root.join(&*rel_string);
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dir {}", parent.display()))?;
        }

        let size = entry.size();
        entry
            .unpack(&dest_path)
            .with_context(|| format!("unpack {}", dest_path.display()))?;

        // Apply mtime from headers if available
        if config.preserve_times {
            if let Some(h) = headers.iter().find(|h| h.relative_path == rel_string) {
                if h.mtime_seconds > 0 {
                    let ft = FileTime::from_unix_time(h.mtime_seconds, 0);
                    let _ = filetime::set_file_mtime(&dest_path, ft);
                }
            }
        }

        files_written += 1;
        bytes_written += size;
    }

    Ok(SinkOutcome {
        files_written,
        bytes_written,
    })
}

/// Resume protocol: overwrite a block of an existing file at the given offset.
async fn write_file_block_payload(
    dst_root: &Path,
    relative_path: &str,
    offset: u64,
    bytes: Vec<u8>,
) -> Result<SinkOutcome> {
    use tokio::io::{AsyncSeekExt, AsyncWriteExt};

    let dst = dst_root.join(relative_path);
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
    Ok(SinkOutcome {
        files_written: 0, // Resume blocks patch in-place; finalization counts the file.
        bytes_written: bytes_len,
    })
}

/// Resume protocol: finalize a resumed file by truncating to total_size + fsync.
async fn write_file_block_complete(
    dst_root: &Path,
    relative_path: &str,
    total_size: u64,
) -> Result<SinkOutcome> {
    let dst = dst_root.join(relative_path);
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
    Ok(SinkOutcome {
        files_written: 1,
        bytes_written: 0,
    })
}

// ---------------------------------------------------------------------------
// DataPlaneSink — TCP data plane writer
// ---------------------------------------------------------------------------

/// Writes payloads to a remote daemon via the TCP data plane binary protocol.
///
/// Each instance wraps a single TCP stream (DataPlaneSession). For multi-stream
/// transfers, the pipeline executor creates multiple DataPlaneSink instances.
pub struct DataPlaneSink {
    session: tokio::sync::Mutex<DataPlaneSession>,
    source: Arc<dyn TransferSource>,
    dst_root: PathBuf,
}

impl DataPlaneSink {
    pub fn new(
        session: DataPlaneSession,
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
impl TransferSink for DataPlaneSink {
    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
        let mut session = self.session.lock().await;
        match payload {
            PreparedPayload::File(header) => {
                let size = header.size;
                session
                    .send_file(self.source.clone(), &header)
                    .await
                    .with_context(|| format!("sending {}", header.relative_path))?;
                Ok(SinkOutcome {
                    files_written: 1,
                    bytes_written: size,
                })
            }
            PreparedPayload::TarShard { headers, data } => {
                let bytes: u64 = headers.iter().map(|h| h.size).sum();
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
            bytes_written: size,
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
                bytes_written: header.size,
            }),
            PreparedPayload::TarShard { headers, data } => Ok(SinkOutcome {
                files_written: headers.len(),
                bytes_written: data.len() as u64,
            }),
            PreparedPayload::FileBlock { bytes, .. } => Ok(SinkOutcome {
                files_written: 0,
                bytes_written: bytes.len() as u64,
            }),
            PreparedPayload::FileBlockComplete { .. } => Ok(SinkOutcome::default()),
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
        let n = receive_stream_double_buffered(reader, &mut sink, header.size, RECEIVE_CHUNK_SIZE)
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

// ---------------------------------------------------------------------------
// GrpcFallbackSink — stream payloads over the gRPC control plane
// ---------------------------------------------------------------------------

/// Streams payloads to a remote daemon over the gRPC control plane channel.
///
/// Used when the TCP data plane is unavailable (`--force-grpc`) or when
/// negotiation fails. Slower than `DataPlaneSink` but works in restrictive
/// network environments.
pub struct GrpcFallbackSink {
    source: Arc<dyn TransferSource>,
    tx: tokio::sync::mpsc::Sender<crate::generated::ClientPushRequest>,
    chunk_bytes: usize,
    dst_label: PathBuf,
}

impl GrpcFallbackSink {
    pub fn new(
        source: Arc<dyn TransferSource>,
        tx: tokio::sync::mpsc::Sender<crate::generated::ClientPushRequest>,
        chunk_bytes: usize,
        dst_label: PathBuf,
    ) -> Self {
        Self {
            source,
            tx,
            chunk_bytes,
            dst_label,
        }
    }
}

#[async_trait]
impl TransferSink for GrpcFallbackSink {
    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
        use crate::generated::client_push_request::Payload as ClientPayload;
        use crate::generated::{
            ClientPushRequest, FileData, TarShardChunk, TarShardComplete, TarShardHeader,
        };
        use tokio::io::AsyncReadExt;

        let chunk_size = self
            .chunk_bytes
            .max(super::data_plane::CONTROL_PLANE_CHUNK_SIZE);

        match payload {
            PreparedPayload::File(header) => {
                let size = header.size;

                self.tx
                    .send(ClientPushRequest {
                        payload: Some(ClientPayload::FileManifest(header.clone())),
                    })
                    .await
                    .map_err(|_| eyre::eyre!("gRPC channel closed"))?;

                if size > 0 {
                    let mut file = self
                        .source
                        .open_file(&header)
                        .await
                        .with_context(|| format!("opening {}", header.relative_path))?;

                    let mut buffer = vec![0u8; chunk_size];
                    let mut remaining = size;
                    while remaining > 0 {
                        let to_read = buffer.len().min(remaining as usize);
                        let n = file
                            .read(&mut buffer[..to_read])
                            .await
                            .with_context(|| format!("reading {}", header.relative_path))?;
                        if n == 0 {
                            eyre::bail!(
                                "unexpected EOF reading {} ({} bytes remaining)",
                                header.relative_path,
                                remaining
                            );
                        }
                        self.tx
                            .send(ClientPushRequest {
                                payload: Some(ClientPayload::FileData(FileData {
                                    content: buffer[..n].to_vec(),
                                })),
                            })
                            .await
                            .map_err(|_| eyre::eyre!("gRPC channel closed"))?;
                        remaining -= n as u64;
                    }
                }

                Ok(SinkOutcome {
                    files_written: 1,
                    bytes_written: size,
                })
            }
            PreparedPayload::TarShard { headers, data } => {
                let bytes: u64 = headers.iter().map(|h| h.size).sum();
                let count = headers.len();

                self.tx
                    .send(ClientPushRequest {
                        payload: Some(ClientPayload::TarShardHeader(TarShardHeader {
                            files: headers,
                            archive_size: data.len() as u64,
                        })),
                    })
                    .await
                    .map_err(|_| eyre::eyre!("gRPC channel closed"))?;

                for chunk in data.chunks(chunk_size) {
                    self.tx
                        .send(ClientPushRequest {
                            payload: Some(ClientPayload::TarShardChunk(TarShardChunk {
                                content: chunk.to_vec(),
                            })),
                        })
                        .await
                        .map_err(|_| eyre::eyre!("gRPC channel closed"))?;
                }

                self.tx
                    .send(ClientPushRequest {
                        payload: Some(ClientPayload::TarShardComplete(TarShardComplete {})),
                    })
                    .await
                    .map_err(|_| eyre::eyre!("gRPC channel closed"))?;

                Ok(SinkOutcome {
                    files_written: count,
                    bytes_written: bytes,
                })
            }
            // gRPC fallback is outbound only; receive-side payloads
            // shouldn't reach this sink.
            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
                eyre::bail!(
                    "GrpcFallbackSink does not handle FileBlock payloads (outbound only)"
                );
            }
        }
    }

    async fn finish(&self) -> Result<()> {
        use crate::generated::client_push_request::Payload as ClientPayload;
        use crate::generated::{ClientPushRequest, UploadComplete};

        self.tx
            .send(ClientPushRequest {
                payload: Some(ClientPayload::UploadComplete(UploadComplete {})),
            })
            .await
            .map_err(|_| eyre::eyre!("gRPC channel closed"))?;
        Ok(())
    }

    fn root(&self) -> &Path {
        &self.dst_label
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
        }
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

    #[tokio::test]
    async fn grpc_sink_sends_file() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);

        // Create a real source with a file to read
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("hello.txt"), b"world").unwrap();

        let source = Arc::new(crate::remote::transfer::source::FsTransferSource::new(
            src,
        ));
        let sink = GrpcFallbackSink::new(
            source,
            tx,
            1024 * 1024,
            PathBuf::from("remote:/test/"),
        );

        let header = make_file_header("hello.txt", 5);
        let outcome = sink
            .write_payload(PreparedPayload::File(header))
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.bytes_written, 5);

        // Verify messages sent: FileManifest + FileData
        let msg1 = rx.recv().await.unwrap();
        assert!(
            matches!(
                msg1.payload,
                Some(crate::generated::client_push_request::Payload::FileManifest(_))
            ),
            "expected FileManifest"
        );
        let msg2 = rx.recv().await.unwrap();
        assert!(
            matches!(
                msg2.payload,
                Some(crate::generated::client_push_request::Payload::FileData(_))
            ),
            "expected FileData"
        );
    }

    #[tokio::test]
    async fn grpc_sink_finish_sends_upload_complete() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        let tmp = tempdir().unwrap();
        let source = Arc::new(crate::remote::transfer::source::FsTransferSource::new(
            tmp.path().to_path_buf(),
        ));
        let sink = GrpcFallbackSink::new(
            source,
            tx,
            1024 * 1024,
            PathBuf::from("remote:/test/"),
        );

        sink.finish().await.unwrap();

        let msg = rx.recv().await.unwrap();
        assert!(
            matches!(
                msg.payload,
                Some(crate::generated::client_push_request::Payload::UploadComplete(_))
            ),
            "expected UploadComplete"
        );
    }
}
