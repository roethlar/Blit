use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use base64::{engine::general_purpose, Engine as _};
use eyre::{bail, eyre, Context, Result};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

use crate::generated::blit_client::BlitClient;
use crate::generated::{
    client_pull_message, pull_chunk, server_pull_message, BlockHashList, ClientPullMessage,
    ComparisonMode, DataTransferNegotiation, FileData, FileHeader, ManifestComplete, MirrorMode,
    PeerCapabilities, PullChunk, PullRequest, PullSummary, ResumeSettings, TransferOperationSpec,
};
use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
use crate::remote::transfer::progress::RemoteTransferProgress;

/// Options for pull synchronization operations.
#[derive(Debug, Default, Clone)]
pub struct PullSyncOptions {
    /// Force gRPC data plane (no TCP fallback).
    pub force_grpc: bool,
    /// Mirror mode: report files to delete.
    pub mirror_mode: bool,
    /// Mirror scope policy: when true, deletions extend across the
    /// full destination tree (`MirrorMode::All`). Default false →
    /// `MirrorMode::FilteredSubset` so files outside the source
    /// filter scope are never purged.
    pub delete_all_scope: bool,
    /// Filter rules to apply at the daemon's source enumeration.
    /// `None` means no filtering. The daemon converts this to a
    /// `FileFilter` via `NormalizedTransferOperation::from_spec`.
    pub filter: Option<crate::generated::FilterSpec>,
    /// Compare only by size, ignore modification time.
    pub size_only: bool,
    /// Transfer all files unconditionally.
    pub ignore_times: bool,
    /// Skip files that already exist on target.
    pub ignore_existing: bool,
    /// Overwrite even if target is newer (dangerous).
    pub force: bool,
    /// Force checksum comparison (slower but more accurate).
    pub checksum: bool,
    /// Enable block-level resume for partial/changed files.
    pub resume: bool,
    /// Block size for resume (0 = default 1 MiB).
    pub block_size: u32,
}

#[derive(Debug, Default, Clone)]
pub struct RemotePullReport {
    pub files_transferred: usize,
    pub bytes_transferred: u64,
    pub downloaded_paths: Vec<PathBuf>,
    pub summary: Option<PullSummary>,
    /// Authoritative deletion list from the daemon (mirror mode only).
    /// `None` means the daemon never sent one (e.g. mirror=Off, or
    /// older daemon — but per the no-back-compat policy the latter
    /// shouldn't reach here). Empty `Some` means "daemon agrees
    /// nothing should be deleted." The CLI deletes exactly these
    /// relative paths and never walks the dest tree on its own.
    ///
    /// Stored as raw wire strings — the consumer routes each through
    /// `path_safety::safe_join` before performing any filesystem op,
    /// so a hostile daemon can't escape the destination via `..`,
    /// absolute paths, or Windows-shaped roots (R5-F1).
    pub paths_to_delete: Option<Vec<String>>,
    /// Daemon's `server_checksums_enabled` advertisement from the
    /// PullSyncAck. `None` means no ack arrived (legacy daemon or
    /// pre-spec wire shape). Set by the receive loop and read by
    /// the CLI to honor F11 of the 2026-05-01 baseline review:
    /// when the client asked for `--checksum` mode but the daemon
    /// has checksums disabled, the comparison would silently
    /// degrade to size+mtime — a real footgun for users expecting
    /// byte-level equality. The pull_sync handshake errors out
    /// before any data flows when this mismatch is detected.
    pub server_checksums_enabled: Option<bool>,
}

pub type RemotePullProgress = RemoteTransferProgress;

struct PullWorkerStats {
    start: Instant,
    files_transferred: u64,
    bytes_transferred: u64,
    downloaded_paths: Vec<PathBuf>,
    bytes: u64,
}

impl PullWorkerStats {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            files_transferred: 0,
            bytes_transferred: 0,
            downloaded_paths: Vec::new(),
            bytes: 0,
        }
    }
}

/// Result from data plane receiver, used to merge with control plane report.
struct DataPlaneResult {
    files_transferred: usize,
    bytes_transferred: u64,
    downloaded_paths: Vec<PathBuf>,
}

#[derive(Clone)]
pub struct RemotePullClient {
    endpoint: RemoteEndpoint,
    client: BlitClient<tonic::transport::Channel>,
}

impl RemotePullClient {
    pub async fn connect(endpoint: RemoteEndpoint) -> Result<Self> {
        let uri = endpoint.control_plane_uri();
        let client = BlitClient::connect(uri.clone())
            .await
            .map_err(|err| eyre!("failed to connect to {}: {}", uri, err))?;

        Ok(Self { endpoint, client })
    }

    pub async fn pull(
        &mut self,
        dest_root: &Path,
        force_grpc: bool,
        track_paths: bool,
        progress: Option<&RemotePullProgress>,
    ) -> Result<RemotePullReport> {
        // dest_root is the fully-resolved target. For a directory-source
        // pull, it's the container dir; for a single-file pull, it's the
        // final file path. Creating dest_root unconditionally would turn
        // a file target into a directory. Only ensure the parent exists —
        // handle_file_record will mkdir sub-directories as files arrive.
        if let Some(parent) = dest_root.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("creating destination parent {}", parent.display()))?;
            }
        }

        let (module, rel_path) = match &self.endpoint.path {
            RemotePath::Module { module, rel_path } => (module.clone(), rel_path.clone()),
            RemotePath::Root { rel_path } => (String::new(), rel_path.clone()),
            RemotePath::Discovery => {
                bail!("remote source must specify a module (server:/module/...)");
            }
        };

        let path_str = if rel_path.as_os_str().is_empty() {
            ".".to_string()
        } else {
            normalize_for_request(&rel_path)
        };

        let pull_request = PullRequest {
            module,
            path: path_str,
            force_grpc,
            metadata_only: false,
        };

        let mut stream = self
            .client
            .pull(pull_request)
            .await
            .map_err(|status| eyre!(status.message().to_string()))?
            .into_inner();

        let mut report = RemotePullReport::default();
        let mut active_file: Option<(File, PathBuf)> = None;
        // Store data plane task handle - spawned as background task so control plane can continue
        let mut data_plane_handle: Option<JoinHandle<Result<DataPlaneResult>>> = None;

        while let Some(chunk) = stream
            .message()
            .await
            .map_err(|status| eyre!(status.message().to_string()))?
        {
            match chunk.payload {
                Some(pull_chunk::Payload::FileHeader(header)) => {
                    finalize_active_file(&mut active_file, progress).await?;

                    let relative_path = sanitize_relative_path(&header.relative_path)?;
                    let dest_path = resolve_pull_dest(dest_root, &relative_path);
                    if let Some(parent) = dest_path.parent() {
                        fs::create_dir_all(parent)
                            .await
                            .with_context(|| format!("creating directory {}", parent.display()))?;
                    }

                    let file = File::create(&dest_path)
                        .await
                        .with_context(|| format!("creating {}", dest_path.display()))?;

                    if track_paths {
                        report.downloaded_paths.push(relative_path.clone());
                    }

                    active_file = Some((file, dest_path));
                    report.files_transferred += 1;
                }
                Some(pull_chunk::Payload::FileData(FileData { content })) => {
                    let (file, path) = active_file
                        .as_mut()
                        .ok_or_else(|| eyre!("received file data without a preceding header"))?;
                    file.write_all(&content)
                        .await
                        .with_context(|| format!("reading {}", path.display()))?;
                    report.bytes_transferred += content.len() as u64;
                    if let Some(progress) = progress {
                        progress.report_payload(0, content.len() as u64);
                    }
                }
                Some(pull_chunk::Payload::Negotiation(neg)) => {
                    if neg.tcp_fallback {
                        continue;
                    }
                    // Spawn data plane as background task so we can continue processing
                    // ManifestBatch messages on the control plane
                    data_plane_handle = Some(self.spawn_data_plane_receiver(
                        neg,
                        dest_root,
                        track_paths,
                        progress,
                    )?);
                }
                Some(pull_chunk::Payload::Summary(summary)) => {
                    report.summary = Some(summary);
                }
                Some(pull_chunk::Payload::ManifestBatch(batch)) => {
                    if let Some(progress) = progress {
                        progress.report_manifest_batch(batch.file_count as usize);
                    }
                }
                None => {}
            }
        }

        finalize_active_file(&mut active_file, progress).await?;

        // Wait for data plane to complete and merge results
        if let Some(handle) = data_plane_handle {
            let dp_result = handle
                .await
                .map_err(|err| eyre!("data plane task panicked: {}", err))??;
            report.files_transferred = report
                .files_transferred
                .saturating_add(dp_result.files_transferred);
            report.bytes_transferred = report
                .bytes_transferred
                .saturating_add(dp_result.bytes_transferred);
            if track_paths {
                report.downloaded_paths.extend(dp_result.downloaded_paths);
            }
            if report.summary.is_none() {
                eprintln!("[pull] data plane completed without summary payload");
            }
        }

        Ok(report)
    }

    /// Spawn data plane receiver as background task, returning JoinHandle.
    /// This allows the control plane to continue processing ManifestBatch messages.
    fn spawn_data_plane_receiver(
        &self,
        negotiation: DataTransferNegotiation,
        dest_root: &Path,
        track_paths: bool,
        progress: Option<&RemotePullProgress>,
    ) -> Result<JoinHandle<Result<DataPlaneResult>>> {
        if negotiation.tcp_port == 0 {
            bail!("server provided zero data-plane port for pull");
        }
        let token = general_purpose::STANDARD_NO_PAD
            .decode(negotiation.one_time_token.as_bytes())
            .map_err(|err| eyre!("failed to decode pull data-plane token: {err}"))?;

        // Clone/own all values for the spawned task
        let host = self.endpoint.host.clone();
        let port = negotiation.tcp_port;
        let stream_count = negotiation.stream_count.max(1) as usize;
        let dest_root = dest_root.to_path_buf();
        let progress = progress.cloned();

        Ok(tokio::spawn(async move {
            receive_data_plane_streams_owned(
                host,
                port,
                token,
                stream_count,
                dest_root,
                track_paths,
                progress,
            )
            .await
        }))
    }
    pub async fn scan_remote_files(&mut self, path: &Path) -> Result<Vec<FileHeader>> {
        let (module, rel_path) = match &self.endpoint.path {
            RemotePath::Module { module, rel_path } => (module.clone(), rel_path.join(path)),
            RemotePath::Root { rel_path } => (String::new(), rel_path.join(path)),
            RemotePath::Discovery => bail!("remote source must specify a module"),
        };

        let path_str = normalize_for_request(&rel_path);
        let pull_request = PullRequest {
            module,
            path: path_str,
            force_grpc: true, // Force gRPC to get headers in the control stream
            metadata_only: true,
        };

        let mut stream = self
            .client
            .pull(pull_request)
            .await
            .map_err(|status| eyre!(status.message().to_string()))?
            .into_inner();

        let mut headers = Vec::new();
        while let Some(chunk) = stream
            .message()
            .await
            .map_err(|status| eyre!(status.message().to_string()))?
        {
            if let Some(pull_chunk::Payload::FileHeader(header)) = chunk.payload {
                headers.push(header);
            }
        }
        Ok(headers)
    }

    pub async fn open_remote_file(
        &self,
        path: &Path,
    ) -> Result<impl tokio::io::AsyncRead + Unpin + Send> {
        let (module, rel_path) = match &self.endpoint.path {
            RemotePath::Module { module, rel_path } => (module.clone(), rel_path.join(path)),
            RemotePath::Root { rel_path } => (String::new(), rel_path.join(path)),
            RemotePath::Discovery => bail!("remote source must specify a module"),
        };

        let path_str = normalize_for_request(&rel_path);
        let pull_request = PullRequest {
            module,
            path: path_str,
            force_grpc: true, // Force gRPC to get data in the control stream for single file
            metadata_only: false,
        };

        // Clone client to use in async block if needed, but here we need to return a stream.
        // We can't easily return the stream directly because it's a gRPC stream.
        // We need to wrap it in an AsyncRead adapter.
        let mut client = self.client.clone();
        let stream = client
            .pull(pull_request)
            .await
            .map_err(|status| eyre!(status.message().to_string()))?
            .into_inner();

        Ok(RemoteFileStream::new(stream))
    }

    /// Pull with manifest synchronization - sends local manifest to server,
    /// server compares and only sends files that need updating.
    pub async fn pull_sync(
        &mut self,
        dest_root: &Path,
        local_manifest: Vec<FileHeader>,
        options: &PullSyncOptions,
        track_paths: bool,
        progress: Option<&RemotePullProgress>,
    ) -> Result<RemotePullReport> {
        let force_grpc = options.force_grpc;
        let mirror_mode = options.mirror_mode;
        use tokio_stream::wrappers::ReceiverStream;

        // Ensure the parent exists; do NOT mkdir dest_root itself — for a
        // single-file pull it's the target file path, and creating it as
        // a directory here would cause the subsequent File::create to fail.
        if let Some(parent) = dest_root.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("creating destination parent {}", parent.display()))?;
            }
        }

        let (module, rel_path) = match &self.endpoint.path {
            RemotePath::Module { module, rel_path } => (module.clone(), rel_path.clone()),
            RemotePath::Root { rel_path } => (String::new(), rel_path.clone()),
            RemotePath::Discovery => {
                bail!("remote source must specify a module (server:/module/...)");
            }
        };

        let path_str = if rel_path.as_os_str().is_empty() {
            ".".to_string()
        } else {
            normalize_for_request(&rel_path)
        };

        // Create channel for sending messages to server. Capacity is
        // small (32) — adequate because the gRPC stream is opened
        // BEFORE we push manifest entries, so the daemon is consuming
        // continuously and the channel never fills.
        //
        // History: for a long time this code pushed all manifest
        // entries into the channel BEFORE opening the gRPC stream.
        // For any local manifest with >30 entries that deadlocked at
        // entry 33 (channel full, no consumer because stream wasn't
        // open yet). Mirror noop on a populated dest hung silently.
        let (tx, rx) = tokio::sync::mpsc::channel::<ClientPullMessage>(32);

        // Open the bidirectional stream FIRST so the daemon starts
        // consuming our messages as we push them.
        let request_stream = ReceiverStream::new(rx);
        let mut response_stream = self
            .client
            .pull_sync(request_stream)
            .await
            .map_err(|status| eyre!(status.message().to_string()))?
            .into_inner();

        // Build the unified TransferOperationSpec from the client's
        // CLI args. ComparisonMode now covers only the "given the
        // file is being considered, what counts as a match?" axis;
        // the orthogonal "skip if dst exists" axis travels in the
        // top-level `ignore_existing` spec field. The CLI rejects
        // `--force --ignore-existing` (contradictory) before reaching
        // here — but the spec normalizer also rejects it defensively.
        let compare_mode = if options.ignore_times {
            ComparisonMode::IgnoreTimes
        } else if options.force {
            ComparisonMode::Force
        } else if options.size_only {
            ComparisonMode::SizeOnly
        } else if options.checksum {
            ComparisonMode::Checksum
        } else {
            ComparisonMode::SizeMtime
        };
        let mirror = if mirror_mode {
            if options.delete_all_scope {
                MirrorMode::All
            } else {
                // Default — files outside the filter scope are not
                // purged from the destination, since the source
                // filter excluded them on purpose.
                MirrorMode::FilteredSubset
            }
        } else {
            MirrorMode::Off
        };
        let filter_spec = options.filter.clone().unwrap_or_default();
        tx.send(ClientPullMessage {
            payload: Some(client_pull_message::Payload::Spec(TransferOperationSpec {
                spec_version: 1,
                module,
                source_path: path_str,
                filter: Some(filter_spec),
                compare_mode: compare_mode as i32,
                mirror_mode: mirror as i32,
                resume: Some(ResumeSettings {
                    enabled: options.resume,
                    block_size: options.block_size,
                }),
                client_capabilities: Some(PeerCapabilities {
                    supports_resume: true,
                    supports_tar_shards: true,
                    supports_data_plane_tcp: true,
                    supports_filter_spec: true,
                }),
                force_grpc,
                ignore_existing: options.ignore_existing,
            })),
        })
        .await
        .map_err(|_| eyre!("failed to send pull sync spec"))?;

        // Send local manifest. Send in a separate task so we can also
        // drive response_stream concurrently — for large manifests the
        // daemon may start emitting need-list / data-plane responses
        // before we finish enumerating, and we must not block sending
        // the manifest just because we haven't started reading
        // responses yet.
        let local_manifest_clone = local_manifest.clone();
        let tx_for_manifest = tx.clone();
        let manifest_send_task = tokio::spawn(async move {
            for header in &local_manifest_clone {
                if tx_for_manifest
                    .send(ClientPullMessage {
                        payload: Some(client_pull_message::Payload::LocalFile(header.clone())),
                    })
                    .await
                    .is_err()
                {
                    return Err(eyre!("failed to send local file header"));
                }
            }
            tx_for_manifest
                .send(ClientPullMessage {
                    payload: Some(client_pull_message::Payload::ManifestDone(
                        ManifestComplete {},
                    )),
                })
                .await
                .map_err(|_| eyre!("failed to send manifest done"))?;
            Ok::<(), eyre::Report>(())
        });

        let mut report = RemotePullReport::default();
        let mut active_file: Option<(File, PathBuf)> = None;
        let mut active_shard: Option<InProgressShard> = None;
        let mut data_plane_handle: Option<JoinHandle<Result<DataPlaneResult>>> = None;
        let mut files_to_delete = 0u64;

        while let Some(msg) = response_stream
            .message()
            .await
            .map_err(|status| eyre!(status.message().to_string()))?
        {
            match msg.payload {
                Some(server_pull_message::Payload::Ack(_)) => {
                    // Header acknowledged, continue (deprecated, use PullSyncAck)
                }
                Some(server_pull_message::Payload::PullSyncAck(ack)) => {
                    // F11: store the capability and reject early if
                    // the user explicitly asked for `--checksum` but
                    // the daemon has checksums disabled. Silently
                    // degrading to size+mtime would lie to the user
                    // about the comparison strength they requested.
                    report.server_checksums_enabled = Some(ack.server_checksums_enabled);
                    if options.checksum && !ack.server_checksums_enabled {
                        bail!(
                            "client requested checksum comparison (--checksum) but the daemon \
                             has checksums disabled; aborting before transfer to avoid silent \
                             fallback to size+mtime comparison"
                        );
                    }
                }
                Some(server_pull_message::Payload::ManifestBatch(batch)) => {
                    if let Some(progress) = progress {
                        progress.report_manifest_batch(batch.file_count as usize);
                    }
                }
                Some(server_pull_message::Payload::FilesToDownload(_files)) => {
                    // Server tells us which files will be sent - for progress tracking
                    // The actual file count is already handled in ManifestBatch
                }
                Some(server_pull_message::Payload::DeleteList(list)) => {
                    // Daemon authoritative mirror-purge list (closes F4).
                    // Stored as wire strings — the CLI consumer is
                    // responsible for routing each through safe_join
                    // before touching the filesystem (R5-F1).
                    report.paths_to_delete = Some(list.relative_paths);
                }
                Some(server_pull_message::Payload::FileHeader(header)) => {
                    finalize_active_file(&mut active_file, progress).await?;

                    let relative_path = sanitize_relative_path(&header.relative_path)?;
                    let dest_path = resolve_pull_dest(dest_root, &relative_path);
                    if let Some(parent) = dest_path.parent() {
                        fs::create_dir_all(parent)
                            .await
                            .with_context(|| format!("creating directory {}", parent.display()))?;
                    }

                    let file = File::create(&dest_path)
                        .await
                        .with_context(|| format!("creating {}", dest_path.display()))?;

                    if track_paths {
                        report.downloaded_paths.push(relative_path.clone());
                    }

                    active_file = Some((file, dest_path));
                    report.files_transferred += 1;
                }
                Some(server_pull_message::Payload::FileData(FileData { content })) => {
                    let (file, path) = active_file
                        .as_mut()
                        .ok_or_else(|| eyre!("received file data without a preceding header"))?;
                    file.write_all(&content)
                        .await
                        .with_context(|| format!("writing {}", path.display()))?;
                    report.bytes_transferred += content.len() as u64;
                    if let Some(progress) = progress {
                        progress.report_payload(0, content.len() as u64);
                    }
                }
                Some(server_pull_message::Payload::TarShardHeader(header)) => {
                    finalize_active_file(&mut active_file, progress).await?;
                    if active_shard.is_some() {
                        bail!("received TarShardHeader while a previous shard was open");
                    }
                    if header.archive_size > MAX_TAR_SHARD_BYTES {
                        bail!(
                            "TarShardHeader.archive_size {} exceeds local cap {} bytes",
                            header.archive_size,
                            MAX_TAR_SHARD_BYTES
                        );
                    }
                    // Modest initial reservation — chunks grow the
                    // buffer incrementally up to declared_size, so
                    // we don't have to trust archive_size for the
                    // up-front allocation.
                    let initial_capacity = (header.archive_size as usize)
                        .min(1024 * 1024)
                        .min(MAX_TAR_SHARD_BYTES as usize);
                    active_shard = Some(InProgressShard {
                        files: header.files,
                        buffer: Vec::with_capacity(initial_capacity),
                        declared_size: header.archive_size,
                    });
                }
                Some(server_pull_message::Payload::TarShardChunk(chunk)) => {
                    let shard = active_shard
                        .as_mut()
                        .ok_or_else(|| eyre!("TarShardChunk arrived without a preceding header"))?;
                    let new_total = shard.buffer.len() as u64 + chunk.content.len() as u64;
                    if new_total > shard.declared_size {
                        bail!(
                            "TarShardChunk would overflow declared archive_size: \
                             buffer={} chunk={} declared={}",
                            shard.buffer.len(),
                            chunk.content.len(),
                            shard.declared_size
                        );
                    }
                    if new_total > MAX_TAR_SHARD_BYTES {
                        bail!(
                            "tar shard buffer would exceed local cap of {} bytes",
                            MAX_TAR_SHARD_BYTES
                        );
                    }
                    if let Some(progress) = progress {
                        progress.report_payload(0, chunk.content.len() as u64);
                    }
                    shard.buffer.extend_from_slice(&chunk.content);
                }
                Some(server_pull_message::Payload::TarShardComplete(_)) => {
                    let shard = active_shard
                        .take()
                        .ok_or_else(|| eyre!("TarShardComplete with no active shard"))?;
                    if shard.buffer.len() as u64 != shard.declared_size {
                        bail!(
                            "tar shard buffer length {} does not match declared archive_size {}",
                            shard.buffer.len(),
                            shard.declared_size
                        );
                    }
                    let stats = apply_pull_tar_shard(dest_root, shard, track_paths)
                        .with_context(|| "applying tar shard")?;
                    report.files_transferred += stats.files;
                    report.bytes_transferred += stats.bytes;
                    if track_paths {
                        report.downloaded_paths.extend(stats.paths);
                    }
                }
                Some(server_pull_message::Payload::Negotiation(neg)) => {
                    if neg.tcp_fallback {
                        continue;
                    }
                    data_plane_handle = Some(self.spawn_data_plane_receiver(
                        neg,
                        dest_root,
                        track_paths,
                        progress,
                    )?);
                }
                Some(server_pull_message::Payload::Summary(summary)) => {
                    files_to_delete = summary.entries_deleted;
                    report.summary = Some(summary);
                }
                Some(server_pull_message::Payload::BlockHashRequest(req)) => {
                    // Server requests block hashes for resume mode
                    // Compute Blake3 hashes of local file blocks and send them back
                    // Routes through the shared safe_join chokepoint so
                    // empty (single-file dest) and traversal/abs/UNC
                    // attacks are handled uniformly with the rest of
                    // the receive sink sites. F1 of the 2026-05-01 review.
                    let local_path = crate::path_safety::safe_join(dest_root, &req.relative_path)
                        .map_err(|e| {
                        eyre!(
                            "server returned unsafe block-hash path {:?}: {}",
                            req.relative_path,
                            e
                        )
                    })?;
                    let hashes = compute_block_hashes(&local_path, req.block_size as usize).await?;

                    tx.send(ClientPullMessage {
                        payload: Some(client_pull_message::Payload::BlockHashes(BlockHashList {
                            relative_path: req.relative_path,
                            block_size: req.block_size,
                            hashes,
                        })),
                    })
                    .await
                    .map_err(|_| eyre!("failed to send block hashes"))?;
                }
                Some(server_pull_message::Payload::BlockTransfer(block)) => {
                    // Server sends a block for resume - write at specified offset
                    use std::io::SeekFrom;
                    use tokio::io::{AsyncSeekExt, AsyncWriteExt as _};

                    let relative_path = sanitize_relative_path(&block.relative_path)?;
                    let dest_path = resolve_pull_dest(dest_root, &relative_path);

                    // Ensure parent directory exists
                    if let Some(parent) = dest_path.parent() {
                        fs::create_dir_all(parent).await.ok();
                    }

                    // Open file for writing at offset (create if not exists)
                    let mut file = tokio::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(false)
                        .open(&dest_path)
                        .await
                        .with_context(|| {
                            format!("opening {} for block write", dest_path.display())
                        })?;

                    // Seek to offset and write
                    file.seek(SeekFrom::Start(block.offset))
                        .await
                        .with_context(|| {
                            format!(
                                "seeking to offset {} in {}",
                                block.offset,
                                dest_path.display()
                            )
                        })?;

                    file.write_all(&block.content).await.with_context(|| {
                        format!(
                            "writing block at offset {} to {}",
                            block.offset,
                            dest_path.display()
                        )
                    })?;

                    report.bytes_transferred += block.content.len() as u64;
                    if let Some(progress) = progress {
                        progress.report_payload(0, block.content.len() as u64);
                    }
                }
                Some(server_pull_message::Payload::BlockComplete(complete)) => {
                    // Server signals file resume complete - truncate to final size if needed
                    let relative_path = sanitize_relative_path(&complete.relative_path)?;
                    let dest_path = resolve_pull_dest(dest_root, &relative_path);

                    // Truncate file to the correct final size
                    let file = tokio::fs::OpenOptions::new()
                        .write(true)
                        .open(&dest_path)
                        .await
                        .with_context(|| {
                            format!("opening {} for truncation", dest_path.display())
                        })?;

                    file.set_len(complete.total_bytes).await.with_context(|| {
                        format!(
                            "truncating {} to {} bytes",
                            dest_path.display(),
                            complete.total_bytes
                        )
                    })?;

                    if track_paths {
                        report.downloaded_paths.push(relative_path);
                    }
                    report.files_transferred += 1;
                }
                None => {}
            }
        }

        finalize_active_file(&mut active_file, progress).await?;
        ensure_no_open_shard(&active_shard)?;

        // Wait for the manifest send task to complete (it should have
        // finished by now — daemon's response stream couldn't have
        // ended otherwise — but await for error propagation).
        manifest_send_task
            .await
            .map_err(|err| eyre!("manifest send task panicked: {}", err))??;
        // Drop the original tx so the daemon sees end-of-stream after
        // any final messages have flushed.
        drop(tx);

        // Wait for data plane to complete and merge results
        if let Some(handle) = data_plane_handle {
            let dp_result = handle
                .await
                .map_err(|err| eyre!("data plane task panicked: {}", err))??;
            report.files_transferred = report
                .files_transferred
                .saturating_add(dp_result.files_transferred);
            report.bytes_transferred = report
                .bytes_transferred
                .saturating_add(dp_result.bytes_transferred);
            if track_paths {
                report.downloaded_paths.extend(dp_result.downloaded_paths);
            }
        }

        // Store files_to_delete in report for mirror mode handling
        if files_to_delete > 0 {
            // The caller will handle deletion based on mirror_mode
            if let Some(ref mut summary) = report.summary {
                summary.entries_deleted = files_to_delete;
            }
        }

        Ok(report)
    }
}

use std::pin::Pin;
use std::task::Poll;
use tokio_stream::Stream;
use tonic::Streaming;

/// Compute Blake3 block hashes for a local file.
/// Returns a vector of 32-byte hashes, one per block.
/// Streams the file in chunks to avoid loading the entire file into memory.
async fn compute_block_hashes(path: &Path, block_size: usize) -> Result<Vec<Vec<u8>>> {
    use crate::copy::{DEFAULT_BLOCK_SIZE, MAX_BLOCK_SIZE};
    use tokio::io::AsyncReadExt;

    let block_size = if block_size == 0 {
        DEFAULT_BLOCK_SIZE
    } else {
        block_size
    };

    if block_size > MAX_BLOCK_SIZE {
        bail!(
            "server requested unsafe block size: {} (max: {})",
            block_size,
            MAX_BLOCK_SIZE
        );
    }

    if !path.exists() {
        // File doesn't exist locally, return empty hashes
        return Ok(Vec::new());
    }

    let metadata = tokio::fs::metadata(path)
        .await
        .with_context(|| format!("getting metadata for {}", path.display()))?;

    let file_size = metadata.len() as usize;
    let num_blocks = file_size.div_ceil(block_size);

    let mut file = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("opening {} for block hashes", path.display()))?;

    let mut hashes = Vec::with_capacity(num_blocks);
    let mut buffer = vec![0u8; block_size];

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        let hash = blake3::hash(&buffer[..bytes_read]);
        hashes.push(hash.as_bytes().to_vec());
    }

    Ok(hashes)
}

struct RemoteFileStream {
    stream: Streaming<PullChunk>,
    buffer: Vec<u8>,
    position: usize,
}

impl RemoteFileStream {
    fn new(stream: Streaming<PullChunk>) -> Self {
        Self {
            stream,
            buffer: Vec::new(),
            position: 0,
        }
    }
}

impl tokio::io::AsyncRead for RemoteFileStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.position < self.buffer.len() {
            let len = std::cmp::min(buf.remaining(), self.buffer.len() - self.position);
            buf.put_slice(&self.buffer[self.position..self.position + len]);
            self.position += len;
            return Poll::Ready(Ok(()));
        }

        match Pin::new(&mut self.stream).poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                match chunk.payload {
                    Some(pull_chunk::Payload::FileData(data)) => {
                        self.buffer = data.content;
                        self.position = 0;
                        // Recurse to copy data to buf
                        self.poll_read(cx, buf)
                    }
                    Some(pull_chunk::Payload::FileHeader(_)) => {
                        // Skip headers in data stream
                        self.poll_read(cx, buf)
                    }
                    _ => {
                        // Skip non-data messages (ManifestBatch, Summary, Negotiation, etc.)
                        self.poll_read(cx, buf)
                    }
                }
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Err(std::io::Error::other(e.to_string()))),
            Poll::Ready(None) => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending,
        }
    }
}

async fn finalize_active_file(
    active: &mut Option<(File, PathBuf)>,
    progress: Option<&RemotePullProgress>,
) -> Result<()> {
    if let Some((file, path)) = active.take() {
        file.sync_all().await?;
        if let Some(progress) = progress {
            // Bytes already counted by FileData chunks, just report file completion
            progress.report_file_complete(path.to_string_lossy().into_owned(), 0);
        }
    }
    Ok(())
}

/// Hard cap on tar-shard buffer size on the pull receive side.
/// Re-exported through `tar_safety::MAX_TAR_SHARD_BYTES` so the
/// per-entry helper and the receive-loop bounds share one source
/// of truth (R5-F3 of `docs/reviews/followup_review_2026-05-02.md`).
use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;

/// R6-F2: a stream that closes with `active_shard = Some(_)` is a
/// protocol error — the daemon sent `TarShardHeader` and possibly
/// chunks but never sent `TarShardComplete`, so files inside that
/// shard never landed. The pull receive loop calls this after the
/// response stream ends. Treated like `FileData` without a
/// preceding `FileHeader`: a wire protocol error, not a partial
/// success.
fn ensure_no_open_shard(active: &Option<InProgressShard>) -> Result<()> {
    if active.is_some() {
        bail!(
            "gRPC pull stream ended with an open tar shard \
             (TarShardHeader received, no TarShardComplete)"
        );
    }
    Ok(())
}

/// Buffer state for a tar shard arriving on the gRPC pull control
/// plane (Step 4C). `declared_size` is checked at every Chunk
/// arrival and again at Complete so a daemon that lies about the
/// shard size can't desync the buffer or grow it past the cap.
struct InProgressShard {
    files: Vec<FileHeader>,
    buffer: Vec<u8>,
    /// Total archive size promised by `TarShardHeader.archive_size`.
    /// The buffer must reach exactly this length by the time the
    /// `TarShardComplete` message arrives.
    declared_size: u64,
}

#[derive(Debug)]
struct ShardApplyStats {
    files: usize,
    bytes: u64,
    paths: Vec<PathBuf>,
}

/// Extract a tar-shard buffer into `dest_root`. Thin adapter over
/// `tar_safety::safe_extract_tar_shard` — the heavy lifting (R5-F2
/// non-regular rejection, R6-F1 per-entry size bounds, R6-F3 mtime
/// preservation, path validation, bounded allocation) lives in the
/// shared helper so this site, `FsTransferSink`, and the daemon push
/// receive can't drift.
fn apply_pull_tar_shard(
    dest_root: &Path,
    shard: InProgressShard,
    track_paths: bool,
) -> Result<ShardApplyStats> {
    use crate::remote::transfer::tar_safety::{
        safe_extract_tar_shard, write_extracted_file, TarShardExtractOptions,
    };

    let opts = TarShardExtractOptions {
        // The shard buffer is already capped at declared_size (which
        // is itself capped at MAX_TAR_SHARD_BYTES on receive), so any
        // single entry is bounded by that.
        max_entry_bytes: shard.declared_size,
        require_exact_headers: true,
    };
    let extracted = safe_extract_tar_shard(&shard.buffer, shard.files, dest_root, &opts)?;

    let mut stats = ShardApplyStats {
        files: 0,
        bytes: 0,
        paths: Vec::new(),
    };
    for file in &extracted {
        write_extracted_file(file).context("applying tar shard entry")?;
        stats.files += 1;
        stats.bytes += file.size;
        if track_paths {
            stats.paths.push(PathBuf::from(&file.rel));
        }
    }
    Ok(stats)
}

#[cfg(test)]
mod tar_shard_safety_tests {
    use super::*;
    use std::io::Cursor;
    use tar::{Builder, EntryType, Header};
    use tempfile::tempdir;

    fn header(rel: &str, size: u64) -> FileHeader {
        FileHeader {
            relative_path: rel.into(),
            size,
            mtime_seconds: 0,
            permissions: 0o644,
            checksum: vec![],
        }
    }

    fn build_archive_with_symlink(rel: &str, link_target: &str) -> Vec<u8> {
        let mut builder = Builder::new(Vec::new());
        let mut h = Header::new_gnu();
        h.set_entry_type(EntryType::Symlink);
        h.set_size(0);
        h.set_mode(0o777);
        builder.append_link(&mut h, rel, link_target).unwrap();
        builder.into_inner().unwrap()
    }

    fn build_regular_archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = Builder::new(Vec::new());
        for (rel, data) in entries {
            let mut h = Header::new_gnu();
            h.set_entry_type(EntryType::Regular);
            h.set_size(data.len() as u64);
            h.set_mode(0o644);
            builder
                .append_data(&mut h, rel, Cursor::new(*data))
                .unwrap();
        }
        builder.into_inner().unwrap()
    }

    #[test]
    fn rejects_symlink_entry() {
        // Hostile daemon claims to ship `expected.txt` but the tar
        // entry is actually a symlink. Pre-R5-F2 we would have called
        // entry.unpack and created a symlink to /etc/passwd.
        let tmp = tempdir().unwrap();
        let dest = tmp.path();
        let buffer = build_archive_with_symlink("expected.txt", "/etc/passwd");
        let declared_size = buffer.len() as u64;
        let shard = InProgressShard {
            files: vec![header("expected.txt", 0)],
            buffer,
            declared_size,
        };
        let err = apply_pull_tar_shard(dest, shard, false).unwrap_err();
        assert!(
            err.to_string().contains("non-regular entry"),
            "expected non-regular rejection, got: {err}"
        );
        assert!(!dest.join("expected.txt").exists());
    }

    #[test]
    fn rejects_traversal_path_in_archive() {
        // The tar crate's Builder rejects `..` at the sender side, so
        // we craft a malicious archive by hand: standard 512-byte
        // ustar header with the path field overwritten to `../escape.txt`.
        // A hostile non-Rust peer could trivially produce this shape;
        // we want apply_pull_tar_shard to reject it via
        // validate_wire_path, not let safe_join trip later.
        let tmp = tempdir().unwrap();
        let dest = tmp.path().join("dest");
        std::fs::create_dir_all(&dest).unwrap();

        // Build a tar archive with a benign path, then surgically
        // overwrite the path bytes in the header.
        let mut buffer = build_regular_archive(&[("aaaaaaaaa.txt", b"pwn")]);
        let bad_name = b"../escape.txt\0";
        buffer[..bad_name.len()].copy_from_slice(bad_name);
        // Recompute checksum (offset 148, 8 bytes ASCII octal). Tar
        // checksum spec: sum of all header bytes treating chksum
        // field as spaces (0x20).
        let mut sum: u32 = 0;
        for (i, b) in buffer[..512].iter().enumerate() {
            if (148..156).contains(&i) {
                sum += 0x20;
            } else {
                sum += *b as u32;
            }
        }
        let chksum = format!("{:06o}\0 ", sum);
        buffer[148..156].copy_from_slice(chksum.as_bytes());

        let declared_size = buffer.len() as u64;
        let shard = InProgressShard {
            files: vec![header("../escape.txt", 3)],
            buffer,
            declared_size,
        };
        let err = apply_pull_tar_shard(&dest, shard, false).unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("validating") || msg.contains("validate"),
            "expected validation rejection, got: {err}"
        );
        // The sibling file `escape.txt` (one dir up from `dest`) must
        // not have been created.
        assert!(!dest.parent().unwrap().join("escape.txt").exists());
    }

    #[test]
    fn rejects_size_mismatch() {
        // Daemon's FileHeader advertises a different size than the
        // tar entry. R6-F1 catches this before any allocation by
        // comparing entry.size() against header.size up front.
        let tmp = tempdir().unwrap();
        let dest = tmp.path().to_path_buf();
        let buffer = build_regular_archive(&[("ok.txt", b"hello")]);
        let declared_size = buffer.len() as u64;
        let shard = InProgressShard {
            files: vec![header("ok.txt", 99)], // lie
            buffer,
            declared_size,
        };
        let err = apply_pull_tar_shard(&dest, shard, false).unwrap_err();
        assert!(
            err.to_string().contains("does not match"),
            "expected size-mismatch rejection, got: {err}"
        );
    }

    #[test]
    fn rejects_per_entry_size_above_shard_cap() {
        // R6-F1: a FileHeader claiming u64::MAX must be rejected
        // before any allocation. The check is against
        // shard.declared_size and MAX_TAR_SHARD_BYTES.
        let tmp = tempdir().unwrap();
        let dest = tmp.path().to_path_buf();
        let buffer = build_regular_archive(&[("ok.txt", b"hi")]);
        let declared_size = buffer.len() as u64;
        let shard = InProgressShard {
            // Daemon claims this single file is bigger than the
            // entire shard. We must catch this before allocating.
            files: vec![header("ok.txt", u64::MAX)],
            buffer,
            declared_size,
        };
        let err = apply_pull_tar_shard(&dest, shard, false).unwrap_err();
        // The first check that fires is the entry/header size mismatch
        // (entry says 2, header says u64::MAX).
        assert!(err.to_string().contains("does not match"));
        assert!(!dest.join("ok.txt").exists());
    }

    #[test]
    fn ensure_no_open_shard_accepts_none() {
        // Healthy stream end: no active shard, helper returns Ok.
        assert!(ensure_no_open_shard(&None).is_ok());
    }

    #[test]
    fn ensure_no_open_shard_rejects_open_shard() {
        // R6-F2: stream ended after TarShardHeader without
        // TarShardComplete — must be a hard error.
        let shard = InProgressShard {
            files: vec![header("partial.txt", 10)],
            buffer: Vec::new(),
            declared_size: 10,
        };
        let err = ensure_no_open_shard(&Some(shard)).unwrap_err();
        assert!(
            err.to_string().contains("open tar shard"),
            "expected open-shard error, got: {err}"
        );
    }

    #[test]
    fn preserves_mtime_on_pull_tar_shard() {
        // R6-F3: the pull gRPC tar extractor must apply mtime so a
        // subsequent size+mtime sync doesn't see every extracted file
        // as "modified at now" and re-transfer it.
        let tmp = tempdir().unwrap();
        let dest = tmp.path().to_path_buf();
        let buffer = build_regular_archive(&[("dated.txt", b"hi")]);
        let declared_size = buffer.len() as u64;
        let target_mtime: i64 = 1_577_836_800; // 2020-01-01 UTC, deterministic
        let mut h = header("dated.txt", 2);
        h.mtime_seconds = target_mtime;
        let shard = InProgressShard {
            files: vec![h],
            buffer,
            declared_size,
        };
        apply_pull_tar_shard(&dest, shard, false).unwrap();
        let meta = std::fs::metadata(dest.join("dated.txt")).unwrap();
        let actual = filetime::FileTime::from_last_modification_time(&meta).unix_seconds();
        assert_eq!(
            actual, target_mtime,
            "extracted file mtime should match FileHeader.mtime_seconds"
        );
    }

    #[test]
    fn happy_path_extracts_regular_files() {
        let tmp = tempdir().unwrap();
        let dest = tmp.path().to_path_buf();
        let buffer = build_regular_archive(&[("a.txt", b"alpha"), ("nested/b.txt", b"beta")]);
        let declared_size = buffer.len() as u64;
        let shard = InProgressShard {
            files: vec![header("a.txt", 5), header("nested/b.txt", 4)],
            buffer,
            declared_size,
        };
        let stats = apply_pull_tar_shard(&dest, shard, true).unwrap();
        assert_eq!(stats.files, 2);
        assert_eq!(stats.bytes, 9);
        assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"alpha");
        assert_eq!(std::fs::read(dest.join("nested/b.txt")).unwrap(), b"beta");
    }
}

/// Owned-value version for spawning data plane receiver as background task. for spawning as background task.
/// This allows the control plane to continue processing ManifestBatch messages.
async fn receive_data_plane_streams_owned(
    host: String,
    port: u32,
    token: Vec<u8>,
    stream_count: usize,
    dest_root: PathBuf,
    track_paths: bool,
    progress: Option<RemotePullProgress>,
) -> Result<DataPlaneResult> {
    let mut result = DataPlaneResult {
        files_transferred: 0,
        bytes_transferred: 0,
        downloaded_paths: Vec::new(),
    };

    if stream_count <= 1 {
        let mut stats = PullWorkerStats::new();
        receive_data_plane_stream_inner(
            &host,
            port,
            &token,
            &dest_root,
            track_paths,
            progress.as_ref(),
            &mut stats,
        )
        .await?;
        result.files_transferred = stats.files_transferred as usize;
        result.bytes_transferred = stats.bytes_transferred;
        if track_paths {
            result.downloaded_paths = stats.downloaded_paths;
        }
        return Ok(result);
    }

    let token = Arc::new(token);

    let mut handles = Vec::with_capacity(stream_count);
    for _ in 0..stream_count {
        let host_clone = host.clone();
        let token_clone = Arc::clone(&token);
        let dest_root_clone = dest_root.clone();
        let progress_clone = progress.clone();
        handles.push(tokio::spawn(async move {
            let mut stats = PullWorkerStats::new();
            receive_data_plane_stream_inner(
                &host_clone,
                port,
                &token_clone,
                &dest_root_clone,
                track_paths,
                progress_clone.as_ref(),
                &mut stats,
            )
            .await?;
            Ok::<_, eyre::Report>(stats)
        }));
    }

    for handle in handles {
        let stats = handle
            .await
            .map_err(|err| eyre!(format!("pull data-plane worker panicked: {}", err)))??;
        result.files_transferred = result
            .files_transferred
            .saturating_add(stats.files_transferred as usize);
        result.bytes_transferred = result
            .bytes_transferred
            .saturating_add(stats.bytes_transferred);
        if track_paths {
            result.downloaded_paths.extend(stats.downloaded_paths);
        }
        let elapsed = stats.start.elapsed().as_secs_f64().max(1e-6);
        let gbps = (stats.bytes as f64 * 8.0) / elapsed / 1e9;
        eprintln!(
            "[pull-data-plane] stream {:.2} Gbps ({} bytes in {:.2}s)",
            gbps, stats.bytes, elapsed
        );
    }

    Ok(result)
}

async fn receive_data_plane_stream_inner(
    host: &str,
    port: u32,
    token: &[u8],
    dest_root: &Path,
    track_paths: bool,
    progress: Option<&RemotePullProgress>,
    stats: &mut PullWorkerStats,
) -> Result<()> {
    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect(addr.clone())
        .await
        .with_context(|| format!("connecting pull data plane {}", addr))?;
    stream
        .write_all(token)
        .await
        .context("writing pull data-plane token")?;

    // Route the inbound wire through the unified receive pipeline.
    // Builds an FsTransferSink rooted at the destination, optionally
    // tracking written paths for mirror's purge phase, and lets
    // execute_receive_pipeline parse records + dispatch to the sink.
    use crate::remote::transfer::pipeline::execute_receive_pipeline;
    use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
    use std::sync::Arc;

    let config = FsSinkConfig {
        preserve_times: true,
        dry_run: false,
        checksum: None,
        resume: false,
    };
    let mut sink = FsTransferSink::new(PathBuf::new(), dest_root.to_path_buf(), config);
    let path_tracker = if track_paths {
        let t = Arc::new(std::sync::Mutex::new(Vec::new()));
        sink = sink.with_path_tracker(Arc::clone(&t));
        Some(t)
    } else {
        None
    };
    let sink: Arc<dyn TransferSink> = Arc::new(sink);

    let outcome = execute_receive_pipeline(&mut stream, sink, progress).await?;

    // Fold the unified outcome into pull's existing stats shape.
    stats.bytes_transferred = stats
        .bytes_transferred
        .saturating_add(outcome.bytes_written);
    stats.bytes = stats.bytes.saturating_add(outcome.bytes_written);
    stats.files_transferred = stats
        .files_transferred
        .saturating_add(outcome.files_written as u64);
    if let Some(tracker) = path_tracker {
        if let Ok(mut paths) = tracker.lock() {
            stats.downloaded_paths.append(&mut paths);
        }
    }
    Ok(())
}

/// Resolve a pull destination path. An empty relative path means "write to
/// dest_root directly" (single-file pull) — `dest_root.join("")` in Rust
/// produces a trailing-slash form that `File::create` rejects as ENOTDIR.
fn resolve_pull_dest(dest_root: &Path, relative_path: &Path) -> PathBuf {
    if relative_path.as_os_str().is_empty() {
        dest_root.to_path_buf()
    } else {
        dest_root.join(relative_path)
    }
}

/// Validate a wire-supplied relative path coming from the daemon.
///
/// Thin wrapper over `crate::path_safety::validate_wire_path` that
/// preserves the historical "server returned ..." error prefix so log
/// scrapers continue to find familiar messages. All actual policy
/// (rejecting absolute paths, `..`, Windows drive prefixes, UNC, etc.)
/// lives in the shared module — this is just the call site.
fn sanitize_relative_path(raw: &str) -> Result<PathBuf> {
    crate::path_safety::validate_wire_path(raw)
        .map_err(|e| eyre::eyre!("server returned unsafe path {:?}: {}", raw, e))
}

fn normalize_for_request(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        path.iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}
