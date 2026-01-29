use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use base64::{engine::general_purpose, Engine as _};
use eyre::{bail, eyre, Context, Result};
use tar::Archive;
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

use crate::generated::blit_client::BlitClient;
use crate::generated::{
    client_pull_message, pull_chunk, server_pull_message, BlockHashList, ClientPullMessage,
    DataTransferNegotiation, FileData, FileHeader, ManifestComplete, PullChunk, PullRequest,
    PullSummary, PullSyncHeader,
};
use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
use crate::remote::transfer::data_plane::{
    DATA_PLANE_RECORD_BLOCK, DATA_PLANE_RECORD_BLOCK_COMPLETE, DATA_PLANE_RECORD_END,
    DATA_PLANE_RECORD_FILE, DATA_PLANE_RECORD_TAR_SHARD,
};
use crate::remote::transfer::progress::RemoteTransferProgress;

/// Options for pull synchronization operations.
#[derive(Debug, Default, Clone)]
pub struct PullSyncOptions {
    /// Force gRPC data plane (no TCP fallback).
    pub force_grpc: bool,
    /// Mirror mode: report files to delete.
    pub mirror_mode: bool,
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
        if !dest_root.exists() {
            fs::create_dir_all(dest_root).await.with_context(|| {
                format!("creating destination directory {}", dest_root.display())
            })?;
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
                    let dest_path = dest_root.join(&relative_path);
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
    pub async fn scan_remote_files(
        &mut self,
        path: &Path,
    ) -> Result<Vec<FileHeader>> {
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

        if !dest_root.exists() {
            fs::create_dir_all(dest_root).await.with_context(|| {
                format!("creating destination directory {}", dest_root.display())
            })?;
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

        // Create channel for sending messages to server
        let (tx, rx) = tokio::sync::mpsc::channel::<ClientPullMessage>(32);

        // Send header
        tx.send(ClientPullMessage {
            payload: Some(client_pull_message::Payload::Header(PullSyncHeader {
                module,
                path: path_str,
                force_grpc,
                mirror_mode,
                size_only: options.size_only,
                ignore_times: options.ignore_times,
                ignore_existing: options.ignore_existing,
                force: options.force,
                checksum: options.checksum,
                resume: options.resume,
                block_size: options.block_size,
            })),
        })
        .await
        .map_err(|_| eyre!("failed to send pull sync header"))?;

        // Send local manifest
        for header in &local_manifest {
            tx.send(ClientPullMessage {
                payload: Some(client_pull_message::Payload::LocalFile(header.clone())),
            })
            .await
            .map_err(|_| eyre!("failed to send local file header"))?;
        }

        // Send manifest done signal
        tx.send(ClientPullMessage {
            payload: Some(client_pull_message::Payload::ManifestDone(ManifestComplete {})),
        })
        .await
        .map_err(|_| eyre!("failed to send manifest done"))?;

        // Open bidirectional stream
        let request_stream = ReceiverStream::new(rx);
        let mut response_stream = self
            .client
            .pull_sync(request_stream)
            .await
            .map_err(|status| eyre!(status.message().to_string()))?
            .into_inner();

        let mut report = RemotePullReport::default();
        let mut active_file: Option<(File, PathBuf)> = None;
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
                    // Server tells us its checksum capability
                    // TODO: Store ack.server_checksums_enabled for decision making
                    let _ = ack.server_checksums_enabled;
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
                Some(server_pull_message::Payload::FileHeader(header)) => {
                    finalize_active_file(&mut active_file, progress).await?;

                    let relative_path = sanitize_relative_path(&header.relative_path)?;
                    let dest_path = dest_root.join(&relative_path);
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
                    let local_path = dest_root.join(sanitize_relative_path(&req.relative_path)?);
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
                    use tokio::io::{AsyncSeekExt, AsyncWriteExt as _};
                    use std::io::SeekFrom;

                    let relative_path = sanitize_relative_path(&block.relative_path)?;
                    let dest_path = dest_root.join(&relative_path);

                    // Ensure parent directory exists
                    if let Some(parent) = dest_path.parent() {
                        fs::create_dir_all(parent).await.ok();
                    }

                    // Open file for writing at offset (create if not exists)
                    let mut file = tokio::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .open(&dest_path)
                        .await
                        .with_context(|| format!("opening {} for block write", dest_path.display()))?;

                    // Seek to offset and write
                    file.seek(SeekFrom::Start(block.offset))
                        .await
                        .with_context(|| format!("seeking to offset {} in {}", block.offset, dest_path.display()))?;

                    file.write_all(&block.content)
                        .await
                        .with_context(|| format!("writing block at offset {} to {}", block.offset, dest_path.display()))?;

                    report.bytes_transferred += block.content.len() as u64;
                    if let Some(progress) = progress {
                        progress.report_payload(0, block.content.len() as u64);
                    }
                }
                Some(server_pull_message::Payload::BlockComplete(complete)) => {
                    // Server signals file resume complete - truncate to final size if needed
                    let relative_path = sanitize_relative_path(&complete.relative_path)?;
                    let dest_path = dest_root.join(&relative_path);

                    // Truncate file to the correct final size
                    let file = tokio::fs::OpenOptions::new()
                        .write(true)
                        .open(&dest_path)
                        .await
                        .with_context(|| format!("opening {} for truncation", dest_path.display()))?;

                    file.set_len(complete.total_bytes)
                        .await
                        .with_context(|| format!("truncating {} to {} bytes", dest_path.display(), complete.total_bytes))?;

                    if track_paths {
                        report.downloaded_paths.push(relative_path);
                    }
                    report.files_transferred += 1;
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
    let num_blocks = (file_size + block_size - 1) / block_size;

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
                        // Ignore other messages or treat as EOF?
                        // Treat as EOF for now if we don't get FileData
                         Poll::Ready(Ok(()))
                    }
                }
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))),
            Poll::Ready(None) => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending,
        }
    }
}

async fn finalize_active_file(
    active: &mut Option<(File, PathBuf)>,
    progress: Option<&RemotePullProgress>,
) -> Result<()> {
    if let Some((file, _)) = active.take() {
        file.sync_all().await?;
        if let Some(progress) = progress {
            progress.report_payload(1, 0);
        }
    }
    Ok(())
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

    loop {
        let mut tag = [0u8; 1];
        stream
            .read_exact(&mut tag)
            .await
            .context("reading pull data-plane record tag")?;
        match tag[0] {
            DATA_PLANE_RECORD_FILE => {
                handle_file_record(&mut stream, dest_root, track_paths, progress, stats).await?;
            }
            DATA_PLANE_RECORD_TAR_SHARD => {
                handle_tar_shard_record(&mut stream, dest_root, track_paths, progress, stats)
                    .await?;
            }
            DATA_PLANE_RECORD_BLOCK => {
                handle_block_record(&mut stream, dest_root, progress, stats).await?;
            }
            DATA_PLANE_RECORD_BLOCK_COMPLETE => {
                handle_block_complete_record(&mut stream, dest_root, track_paths, stats).await?;
            }
            DATA_PLANE_RECORD_END => break,
            other => bail!("unknown pull data-plane record: {}", other),
        }
    }

    Ok(())
}

async fn handle_file_record(
    stream: &mut TcpStream,
    dest_root: &Path,
    track_paths: bool,
    progress: Option<&RemotePullProgress>,
    stats: &mut PullWorkerStats,
) -> Result<()> {
    let rel_string = read_string(stream).await?;
    let relative_path = sanitize_relative_path(&rel_string)?;
    let file_size = read_u64(stream).await?;
    let dest_path = dest_root.join(&relative_path);
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    let mut file = File::create(&dest_path)
        .await
        .with_context(|| format!("creating {}", dest_path.display()))?;
    let mut remaining = file_size;
    let mut buffer = vec![0u8; 64 * 1024];
    while remaining > 0 {
        let to_read = buffer.len().min(remaining as usize);
        let read = stream
            .read(&mut buffer[..to_read])
            .await
            .context("reading pull data-plane file chunk")?;
        if read == 0 {
            bail!(
                "unexpected EOF while receiving {} ({} bytes remaining)",
                relative_path.display(),
                remaining
            );
        }
        file.write_all(&buffer[..read])
            .await
            .with_context(|| format!("writing {}", dest_path.display()))?;
        remaining -= read as u64;
        if let Some(progress) = progress {
            progress.report_payload(0, read as u64);
        }
    }
    file.sync_all()
        .await
        .with_context(|| format!("syncing {}", dest_path.display()))?;

    stats.files_transferred = stats.files_transferred.saturating_add(1);
    stats.bytes_transferred = stats.bytes_transferred.saturating_add(file_size);
    stats.bytes = stats.bytes.saturating_add(file_size);
    if track_paths {
        stats.downloaded_paths.push(relative_path);
    }
    if let Some(progress) = progress {
        progress.report_payload(1, 0);
    }
    Ok(())
}

async fn handle_tar_shard_record(
    stream: &mut TcpStream,
    dest_root: &Path,
    track_paths: bool,
    progress: Option<&RemotePullProgress>,
    stats: &mut PullWorkerStats,
) -> Result<()> {
    let file_count = read_u32(stream).await? as usize;
    let mut files = Vec::with_capacity(file_count);
    for _ in 0..file_count {
        let rel_string = read_string(stream).await?;
        let relative_path = sanitize_relative_path(&rel_string)?;
        let size = read_u64(stream).await?;
        let _mtime = read_i64(stream).await?;
        let _permissions = read_u32(stream).await?;
        files.push((relative_path, size));
    }
    let tar_size = read_u64(stream).await?;
    let mut buffer = vec![0u8; tar_size as usize];
    stream
        .read_exact(&mut buffer)
        .await
        .context("reading pull tar shard payload")?;
    if let Some(progress) = progress {
        progress.report_payload(0, tar_size);
    }

    let dest_root_path = dest_root.to_path_buf();
    let extracted = extract_tar_shard(buffer, files.clone(), dest_root_path).await?;

    if track_paths {
        stats.downloaded_paths.extend(extracted);
    }
    stats.files_transferred = stats.files_transferred.saturating_add(files.len() as u64);
    let shard_bytes: u64 = files.iter().map(|(_, size)| *size).sum();
    stats.bytes_transferred = stats.bytes_transferred.saturating_add(shard_bytes);
    stats.bytes = stats.bytes.saturating_add(shard_bytes);
    if let Some(progress) = progress {
        progress.report_payload(files.len(), 0);
    }

    Ok(())
}

/// Handle a block record: write block content at specified offset.
/// Format: [path_len:4][path][offset:8][block_len:4][content]
async fn handle_block_record(
    stream: &mut TcpStream,
    dest_root: &Path,
    progress: Option<&RemotePullProgress>,
    stats: &mut PullWorkerStats,
) -> Result<()> {
    use std::io::SeekFrom;
    use tokio::io::AsyncSeekExt;

    let rel_string = read_string(stream).await?;
    let relative_path = sanitize_relative_path(&rel_string)?;
    let offset = read_u64(stream).await?;
    let block_len = read_u32(stream).await? as usize;

    let dest_path = dest_root.join(&relative_path);
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    // Read block content
    let mut buffer = vec![0u8; block_len];
    stream
        .read_exact(&mut buffer)
        .await
        .context("reading block content")?;

    // Open file for writing at offset (create if not exists)
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&dest_path)
        .await
        .with_context(|| format!("opening {} for block write", dest_path.display()))?;

    // Seek and write
    file.seek(SeekFrom::Start(offset))
        .await
        .with_context(|| format!("seeking to offset {} in {}", offset, dest_path.display()))?;
    file.write_all(&buffer)
        .await
        .with_context(|| format!("writing block at offset {} to {}", offset, dest_path.display()))?;

    stats.bytes_transferred = stats.bytes_transferred.saturating_add(block_len as u64);
    stats.bytes = stats.bytes.saturating_add(block_len as u64);
    if let Some(progress) = progress {
        progress.report_payload(0, block_len as u64);
    }

    Ok(())
}

/// Handle block complete record: truncate file to final size.
/// Format: [path_len:4][path][total_size:8]
async fn handle_block_complete_record(
    stream: &mut TcpStream,
    dest_root: &Path,
    track_paths: bool,
    stats: &mut PullWorkerStats,
) -> Result<()> {
    let rel_string = read_string(stream).await?;
    let relative_path = sanitize_relative_path(&rel_string)?;
    let total_size = read_u64(stream).await?;

    let dest_path = dest_root.join(&relative_path);

    // Truncate file to final size
    let file = tokio::fs::OpenOptions::new()
        .write(true)
        .open(&dest_path)
        .await
        .with_context(|| format!("opening {} for truncation", dest_path.display()))?;

    file.set_len(total_size)
        .await
        .with_context(|| format!("truncating {} to {} bytes", dest_path.display(), total_size))?;

    if track_paths {
        stats.downloaded_paths.push(relative_path);
    }
    stats.files_transferred = stats.files_transferred.saturating_add(1);

    Ok(())
}

async fn extract_tar_shard(
    data: Vec<u8>,
    files: Vec<(PathBuf, u64)>,
    dest_root: PathBuf,
) -> Result<Vec<PathBuf>> {
    tokio::task::spawn_blocking(move || -> Result<Vec<PathBuf>> {
        let cursor = Cursor::new(data);
        let mut archive = Archive::new(cursor);
        let mut extracted = Vec::new();
        for (idx, entry) in archive.entries()?.enumerate() {
            let mut tar_entry = entry.context("reading tar shard entry")?;
            let (relative_path, _) = files
                .get(idx)
                .ok_or_else(|| eyre!("tar shard entry count mismatch"))?;
            let dest_path = dest_root.join(relative_path);
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
            tar_entry
                .unpack(&dest_path)
                .with_context(|| format!("extracting {}", dest_path.display()))?;
            extracted.push(relative_path.clone());
        }
        Ok(extracted)
    })
    .await
    .map_err(|err| eyre!("tar shard extraction task failed: {}", err))?
}

async fn read_u32(stream: &mut TcpStream) -> Result<u32> {
    let mut buf = [0u8; 4];
    stream
        .read_exact(&mut buf)
        .await
        .context("reading u32 from pull data plane")?;
    Ok(u32::from_be_bytes(buf))
}

async fn read_u64(stream: &mut TcpStream) -> Result<u64> {
    let mut buf = [0u8; 8];
    stream
        .read_exact(&mut buf)
        .await
        .context("reading u64 from pull data plane")?;
    Ok(u64::from_be_bytes(buf))
}

async fn read_i64(stream: &mut TcpStream) -> Result<i64> {
    let mut buf = [0u8; 8];
    stream
        .read_exact(&mut buf)
        .await
        .context("reading i64 from pull data plane")?;
    Ok(i64::from_be_bytes(buf))
}

async fn read_string(stream: &mut TcpStream) -> Result<String> {
    let len = read_u32(stream).await? as usize;
    let mut buf = vec![0u8; len];
    stream
        .read_exact(&mut buf)
        .await
        .context("reading string from pull data plane")?;
    String::from_utf8(buf).map_err(|err| eyre!("pull data-plane path not UTF-8: {err}"))
}

fn sanitize_relative_path(raw: &str) -> Result<PathBuf> {
    if raw.is_empty() {
        bail!("server sent empty relative path");
    }

    let path = Path::new(raw);
    if path.is_absolute() {
        bail!("server returned absolute path: {}", raw);
    }

    use std::path::Component;
    if path
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)))
    {
        bail!(
            "server returned parent directory component in path: {}",
            raw
        );
    }

    let normalized: PathBuf = path
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect();

    Ok(normalized)
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
