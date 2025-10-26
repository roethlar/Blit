use std::collections::HashMap;
use std::path::Path;
use std::time::UNIX_EPOCH;

use base64::{engine::general_purpose, Engine as _};
use eyre::{bail, eyre, Context, Result};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Status;

use crate::enumeration::{EntryKind, FileEnumerator};
use crate::fs_enum::FileFilter;
use crate::generated::blit_client::BlitClient;
use crate::generated::client_push_request::Payload as ClientPayload;
use crate::generated::server_push_response::Payload as ServerPayload;
use crate::generated::{
    ClientPushRequest, FileData, FileHeader, ManifestComplete, PushHeader, PushSummary,
    UploadComplete,
};
use crate::remote::endpoint::{RemoteEndpoint, RemotePath};

#[derive(Debug, Clone)]
pub struct RemotePushReport {
    pub files_requested: Vec<String>,
    pub fallback_used: bool,
    pub data_port: Option<u32>,
    pub summary: PushSummary,
}

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

        let manifest = enumerate_manifest(source_root, filter)
            .with_context(|| format!("enumerating {}", source_root.display()))?;
        let manifest_lookup: HashMap<String, FileHeader> = manifest
            .iter()
            .map(|header| (header.relative_path.clone(), header.clone()))
            .collect();

        let (tx, rx) = mpsc::channel(32);
        let outbound = ReceiverStream::new(rx);

        let mut response_stream = self
            .client
            .push(outbound)
            .await
            .map_err(map_status)?
            .into_inner();

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

        for header in &manifest {
            send_payload(&tx, ClientPayload::FileManifest(header.clone())).await?;
        }

        send_payload(&tx, ClientPayload::ManifestComplete(ManifestComplete {})).await?;

        let mut files_requested: Vec<String> = Vec::new();
        let mut data_port: Option<u32> = None;
        let mut fallback_used = force_grpc;
        let mut summary: Option<PushSummary> = None;

        while let Some(message) = response_stream.message().await.map_err(map_status)? {
            match message.payload {
                Some(ServerPayload::Ack(_)) => {}
                Some(ServerPayload::FilesToUpload(list)) => {
                    files_requested = list.relative_paths;
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
                    break;
                }
                None => {}
            }
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

fn enumerate_manifest(source_root: &Path, filter: &FileFilter) -> Result<Vec<FileHeader>> {
    let enumerator = FileEnumerator::new(filter.clone_without_cache());
    let entries = enumerator.enumerate_local(source_root)?;

    let mut headers = Vec::new();
    for entry in entries {
        if let EntryKind::File { size } = entry.kind {
            let rel = normalize_relative_path(&entry.relative_path);
            let mtime = unix_seconds(&entry.metadata);
            let permissions = permissions_mode(&entry.metadata);

            headers.push(FileHeader {
                relative_path: rel,
                size,
                mtime_seconds: mtime,
                permissions,
            });
        }
    }

    Ok(headers)
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
    for header in files {
        let header_clone = header.clone();
        send_payload(&tx, ClientPayload::FileManifest(header_clone)).await?;

        let data = read_file_bytes(source_root, &header).await?;
        send_payload(&tx, ClientPayload::FileData(FileData { content: data })).await?;
    }

    send_payload(&tx, ClientPayload::UploadComplete(UploadComplete {})).await?;

    Ok(())
}

async fn read_file_bytes(source_root: &Path, header: &FileHeader) -> Result<Vec<u8>> {
    let path = source_root.join(&header.relative_path);
    let mut file = fs::File::open(&path)
        .await
        .with_context(|| format!("opening {}", path.display()))?;
    let mut content = Vec::new();
    file.read_to_end(&mut content)
        .await
        .with_context(|| format!("reading {}", path.display()))?;
    Ok(content)
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
