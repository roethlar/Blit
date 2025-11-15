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

use crate::generated::blit_client::BlitClient;
use crate::generated::{pull_chunk, DataTransferNegotiation, FileData, PullRequest, PullSummary};
use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
use crate::remote::transfer::data_plane::{
    DATA_PLANE_RECORD_END, DATA_PLANE_RECORD_FILE, DATA_PLANE_RECORD_TAR_SHARD,
};
use crate::remote::transfer::progress::RemoteTransferProgress;

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
        };

        let mut stream = self
            .client
            .pull(pull_request)
            .await
            .map_err(|status| eyre!(status.message().to_string()))?
            .into_inner();

        let mut report = RemotePullReport::default();
        let mut active_file: Option<(File, PathBuf)> = None;
        let mut used_data_plane = false;

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
                        .with_context(|| format!("writing {}", path.display()))?;
                    report.bytes_transferred += content.len() as u64;
                    if let Some(progress) = progress {
                        progress.report_payload(0, content.len() as u64);
                    }
                }
                Some(pull_chunk::Payload::Negotiation(neg)) => {
                    if neg.tcp_fallback {
                        continue;
                    }
                    self.handle_data_plane_negotiation(
                        neg,
                        dest_root,
                        track_paths,
                        progress,
                        &mut report,
                    )
                    .await?;
                    used_data_plane = true;
                }
                Some(pull_chunk::Payload::Summary(summary)) => {
                    report.summary = Some(summary);
                }
                None => {}
            }
        }

        finalize_active_file(&mut active_file, progress).await?;

        if used_data_plane && report.summary.is_none() {
            eprintln!("[pull] data plane completed without summary payload");
        }

        Ok(report)
    }

    async fn handle_data_plane_negotiation(
        &self,
        negotiation: DataTransferNegotiation,
        dest_root: &Path,
        track_paths: bool,
        progress: Option<&RemotePullProgress>,
        report: &mut RemotePullReport,
    ) -> Result<()> {
        if negotiation.tcp_port == 0 {
            bail!("server provided zero data-plane port for pull");
        }
        let token = general_purpose::STANDARD_NO_PAD
            .decode(negotiation.one_time_token.as_bytes())
            .map_err(|err| eyre!("failed to decode pull data-plane token: {err}"))?;
        receive_data_plane_streams(
            &self.endpoint.host,
            negotiation.tcp_port,
            &token,
            negotiation.stream_count.max(1) as usize,
            dest_root,
            track_paths,
            progress,
            report,
        )
        .await
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

async fn receive_data_plane_streams(
    host: &str,
    port: u32,
    token: &[u8],
    stream_count: usize,
    dest_root: &Path,
    track_paths: bool,
    progress: Option<&RemotePullProgress>,
    report: &mut RemotePullReport,
) -> Result<()> {
    if stream_count <= 1 {
        let mut stats = PullWorkerStats::new();
        receive_data_plane_stream_inner(
            host,
            port,
            token,
            dest_root,
            track_paths,
            progress,
            &mut stats,
        )
        .await?;
        report.files_transferred = report
            .files_transferred
            .saturating_add(stats.files_transferred as usize);
        report.bytes_transferred = report
            .bytes_transferred
            .saturating_add(stats.bytes_transferred);
        if track_paths {
            report
                .downloaded_paths
                .extend(stats.downloaded_paths.into_iter());
        }
        return Ok(());
    }

    let host = host.to_owned();
    let token = Arc::new(token.to_vec());
    let dest_root = dest_root.to_path_buf();

    let mut handles = Vec::with_capacity(stream_count);
    for _ in 0..stream_count {
        let host_clone = host.clone();
        let token_clone = Arc::clone(&token);
        let dest_root_clone = dest_root.clone();
        let progress_clone = progress.cloned();
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
        report.files_transferred = report
            .files_transferred
            .saturating_add(stats.files_transferred as usize);
        report.bytes_transferred = report
            .bytes_transferred
            .saturating_add(stats.bytes_transferred);
        if track_paths {
            report
                .downloaded_paths
                .extend(stats.downloaded_paths.into_iter());
        }
        let elapsed = stats.start.elapsed().as_secs_f64().max(1e-6);
        let gbps = (stats.bytes as f64 * 8.0) / elapsed / 1e9;
        eprintln!(
            "[pull-data-plane] stream {:.2} Gbps ({} bytes in {:.2}s)",
            gbps, stats.bytes, elapsed
        );
    }

    Ok(())
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
                dest_path.display(),
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

    Ok(path.to_path_buf())
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
