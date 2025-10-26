use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

use base64::{engine::general_purpose, Engine as _};
use eyre::{bail, eyre, Context, Result};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Status;

use crate::enumeration::{EntryKind, FileEnumerator};
use crate::fs_enum::FileFilter;
use crate::generated::blit_client::BlitClient;
use crate::generated::client_push_request::Payload as ClientPayload;
use crate::generated::server_push_response::Payload as ServerPayload;
use crate::generated::{
    ClientPushRequest, FileData, FileHeader, ManifestComplete, PushHeader, PushSummary,
    ServerPushResponse, UploadComplete,
};
use crate::remote::endpoint::{RemoteEndpoint, RemotePath};

#[derive(Debug, Clone)]
pub struct RemotePushReport {
    pub files_requested: Vec<String>,
    pub fallback_used: bool,
    pub data_port: Option<u32>,
    pub summary: PushSummary,
}

const CONTROL_PLANE_CHUNK_SIZE: usize = 1 * 1024 * 1024;

pub struct RemotePushClient {
    endpoint: RemoteEndpoint,
    client: BlitClient<tonic::transport::Channel>,
}

impl RemotePushClient {
    pub async fn connect(endpoint: RemoteEndpoint) -> Result<Self> {
        let uri = endpoint.control_plane_uri();
        let client = BlitClient::connect(uri.clone())
            .await
            .map_err(|err| eyre!("failed to connect to {}: {}", uri, err))?;

        Ok(Self { endpoint, client })
    }

    pub async fn push(
        &mut self,
        source_root: &Path,
        filter: &FileFilter,
        mirror_mode: bool,
        force_grpc: bool,
    ) -> Result<RemotePushReport> {
        if !source_root.exists() {
            bail!("source path does not exist: {}", source_root.display());
        }

        let mut manifest_lookup: HashMap<String, FileHeader> = HashMap::new();

        let (tx, rx) = mpsc::channel(32);
        let outbound = ReceiverStream::new(rx);

        let response_stream = self
            .client
            .push(outbound)
            .await
            .map_err(map_status)?
            .into_inner();
        let (response_tx, mut response_rx) =
            mpsc::channel::<Result<ServerPushResponse, eyre::Report>>(32);
        let response_task = {
            let mut stream = response_stream;
            tokio::spawn(async move {
                loop {
                    match stream.message().await {
                        Ok(Some(msg)) => {
                            if response_tx.send(Ok(msg)).await.is_err() {
                                break;
                            }
                        }
                        Ok(None) => break,
                        Err(status) => {
                            let _ = response_tx.send(Err(map_status(status))).await;
                            break;
                        }
                    }
                }
            })
        };

        let (module, rel_path) = match &self.endpoint.path {
            RemotePath::Module { module, rel_path } => (module.clone(), rel_path.clone()),
            RemotePath::Root { rel_path } => (String::new(), rel_path.clone()),
            RemotePath::Discovery => {
                bail!("remote destination missing module specification");
            }
        };

        let destination_path = if rel_path.as_os_str().is_empty() {
            String::new()
        } else {
            rel_path
                .iter()
                .map(|component| component.to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
        };

        // Send header first
        send_payload(
            &tx,
            ClientPayload::Header(PushHeader {
                module,
                mirror_mode,
                destination_path,
                force_grpc,
            }),
        )
        .await?;

        let (manifest_tx, mut manifest_rx) = mpsc::channel::<FileHeader>(64);
        let enum_root: PathBuf = source_root.to_path_buf();
        let enum_filter = filter.clone_without_cache();

        let manifest_task = task::spawn_blocking(move || -> Result<u64> {
            let enumerator = FileEnumerator::new(enum_filter);
            let start = Instant::now();
            let mut last_log = start;
            let mut enumerated: u64 = 0;
            enumerator.enumerate_local_streaming(&enum_root, |entry| {
                if let EntryKind::File { size } = entry.kind {
                    let rel = normalize_relative_path(&entry.relative_path);
                    let mtime = unix_seconds(&entry.metadata);
                    let permissions = permissions_mode(&entry.metadata);
                    let header = FileHeader {
                        relative_path: rel,
                        size,
                        mtime_seconds: mtime,
                        permissions,
                    };
                    manifest_tx
                        .blocking_send(header)
                        .map_err(|_| eyre!("failed to queue manifest entry"))?;
                    enumerated += 1;
                    if last_log.elapsed() >= Duration::from_secs(1) {
                        println!("Enumerated {} entries… (streaming manifest)", enumerated);
                        last_log = Instant::now();
                    }
                }
                Ok(())
            })?;
            println!(
                "Manifest enumeration complete in {:.2?} ({} entries)",
                start.elapsed(),
                enumerated
            );
            Ok(enumerated)
        });

        let mut files_requested: Vec<String> = Vec::new();
        let mut data_port: Option<u32> = None;
        let mut fallback_used = force_grpc;
        let mut summary: Option<PushSummary> = None;

        let mut manifest_done = false;
        loop {
            if manifest_done && summary.is_some() {
                break;
            }

            tokio::select! {
                maybe_header = manifest_rx.recv(), if !manifest_done => {
                    match maybe_header {
                        Some(header) => {
                            let rel = header.relative_path.clone();
                            send_payload(&tx, ClientPayload::FileManifest(header.clone())).await?;
                            manifest_lookup.insert(rel, header);
                        }
                        None => {
                            manifest_done = true;
                            send_payload(&tx, ClientPayload::ManifestComplete(ManifestComplete {})).await?;
                        }
                    }
                }
                maybe_message = response_rx.recv() => {
                    match maybe_message {
                        Some(Ok(message)) => {
                            match message.payload {
                                Some(ServerPayload::Ack(_)) => {}
                                Some(ServerPayload::FilesToUpload(list)) => {
                                    files_requested.extend(list.relative_paths);
                                }
                                Some(ServerPayload::Negotiation(neg)) => {
                                    fallback_used = neg.tcp_fallback;
                                    if files_requested.is_empty() {
                                        data_port = None;
                                        continue;
                                    }

                                    if neg.tcp_fallback {
                                        transfer_files_via_control_plane(
                                            source_root,
                                            files_requested
                                                .iter()
                                                .filter_map(|path| manifest_lookup.get(path).cloned())
                                                .collect(),
                                            tx.clone(),
                                        )
                                        .await?;
                                        continue;
                                    }

                                    if neg.tcp_port == 0 {
                                        bail!("server reported zero data port for negotiated transfer");
                                    }

                                    let token_bytes = general_purpose::STANDARD_NO_PAD
                                        .decode(neg.one_time_token.as_bytes())
                                        .map_err(|err| eyre!("failed to decode negotiation token: {err}"))?;

                                    let headers: Vec<FileHeader> = files_requested
                                        .iter()
                                        .filter_map(|path| manifest_lookup.get(path).cloned())
                                        .collect();

                                    transfer_files_via_data_plane(
                                        source_root,
                                        &self.endpoint.host,
                                        neg.tcp_port,
                                        &token_bytes,
                                        &headers,
                                    )
                                    .await?;

                                    data_port = Some(neg.tcp_port);
                                }
                                Some(ServerPayload::Summary(s)) => {
                                    summary = Some(s);
                                    if manifest_done {
                                        break;
                                    }
                                }
                                None => {}
                            }
                        }
                        Some(Err(err)) => {
                            response_task.abort();
                            manifest_task.abort();
                            return Err(err);
                        }
                        None => break,
                    }
                }
            }
        }

        manifest_task
            .await
            .map_err(|err| eyre!("manifest enumeration task failed: {}", err))??;
        if let Err(join_err) = response_task.await {
            return Err(eyre!("response stream task failed: {}", join_err));
        }

        let summary = summary.ok_or_else(|| eyre!("push stream ended without summary"))?;

        Ok(RemotePushReport {
            files_requested,
            fallback_used,
            data_port,
            summary,
        })
    }
}

async fn send_payload(tx: &mpsc::Sender<ClientPushRequest>, payload: ClientPayload) -> Result<()> {
    tx.send(ClientPushRequest {
        payload: Some(payload),
    })
    .await
    .map_err(|_| eyre!("failed to send push request payload"))
}

async fn transfer_files_via_data_plane(
    source_root: &Path,
    host: &str,
    port: u32,
    token: &[u8],
    files: &[FileHeader],
) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect(addr.clone())
        .await
        .with_context(|| format!("connecting to data plane {}", addr))?;

    stream
        .write_all(token)
        .await
        .context("writing negotiation token")?;

    let mut buffer = vec![0u8; 64 * 1024];

    for header in files {
        let rel = &header.relative_path;
        let path = source_root.join(rel);

        let path_bytes = rel.as_bytes();
        if path_bytes.len() > u32::MAX as usize {
            bail!("relative path too long for transfer: {}", rel);
        }

        stream
            .write_all(&(path_bytes.len() as u32).to_be_bytes())
            .await
            .context("writing path length")?;
        stream
            .write_all(path_bytes)
            .await
            .context("writing path bytes")?;
        let metadata = fs::metadata(&path)
            .await
            .with_context(|| format!("stat {}", path.display()))?;
        if metadata.len() != header.size {
            bail!(
                "source file {} changed size (expected {}, found {})",
                path.display(),
                header.size,
                metadata.len()
            );
        }

        stream
            .write_all(&metadata.len().to_be_bytes())
            .await
            .context("writing file size")?;

        let mut file = fs::File::open(&path)
            .await
            .with_context(|| format!("opening {}", path.display()))?;

        let mut remaining = metadata.len();
        while remaining > 0 {
            let chunk = file
                .read(&mut buffer)
                .await
                .with_context(|| format!("reading {}", path.display()))?;
            if chunk == 0 {
                bail!(
                    "unexpected EOF while reading {} ({} bytes remaining)",
                    path.display(),
                    remaining
                );
            }
            stream
                .write_all(&buffer[..chunk])
                .await
                .with_context(|| format!("sending {}", path.display()))?;
            remaining -= chunk as u64;
        }
    }

    stream
        .write_all(&0u32.to_be_bytes())
        .await
        .context("writing transfer terminator")?;
    stream.flush().await.context("flushing data plane stream")?;
    Ok(())
}

async fn transfer_files_via_control_plane(
    source_root: &Path,
    files: Vec<FileHeader>,
    tx: mpsc::Sender<ClientPushRequest>,
) -> Result<()> {
    let mut buffer = vec![0u8; CONTROL_PLANE_CHUNK_SIZE];

    for header in files {
        let header_clone = header.clone();
        send_payload(&tx, ClientPayload::FileManifest(header_clone)).await?;

        if header.size == 0 {
            continue;
        }

        let path = source_root.join(&header.relative_path);
        let mut file = fs::File::open(&path)
            .await
            .with_context(|| format!("opening {}", path.display()))?;

        let mut remaining = header.size;
        while remaining > 0 {
            let to_read = buffer.len().min(remaining as usize);
            let chunk = file
                .read(&mut buffer[..to_read])
                .await
                .with_context(|| format!("reading {}", path.display()))?;
            if chunk == 0 {
                bail!(
                    "unexpected EOF while reading {} ({} bytes remaining)",
                    path.display(),
                    remaining
                );
            }

            send_payload(
                &tx,
                ClientPayload::FileData(FileData {
                    content: buffer[..chunk].to_vec(),
                }),
            )
            .await?;
            remaining -= chunk as u64;
        }
    }

    send_payload(&tx, ClientPayload::UploadComplete(UploadComplete {})).await?;

    Ok(())
}

fn map_status(status: Status) -> eyre::Report {
    eyre!(status.message().to_string())
}

fn normalize_relative_path(path: &std::path::Path) -> String {
    let raw = path.to_string_lossy();
    #[cfg(windows)]
    {
        raw.replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        raw.into_owned()
    }
}

fn unix_seconds(metadata: &std::fs::Metadata) -> i64 {
    match metadata.modified() {
        Ok(time) => match time.duration_since(UNIX_EPOCH) {
            Ok(dur) => dur.as_secs() as i64,
            Err(err) => {
                let duration = err.duration();
                -(duration.as_secs() as i64)
            }
        },
        Err(_) => 0,
    }
}

fn permissions_mode(metadata: &std::fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        0
    }
}
