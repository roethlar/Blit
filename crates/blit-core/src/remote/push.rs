use std::collections::{HashMap, VecDeque};
use std::fs::File as StdFile;
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
use crate::fs_enum::{FileEntry, FileFilter};
use crate::generated::blit_client::BlitClient;
use crate::generated::client_push_request::Payload as ClientPayload;
use crate::generated::server_push_response::Payload as ServerPayload;
use crate::generated::{
    ClientPushRequest, FileData, FileHeader, ManifestComplete, PushHeader, PushSummary,
    ServerPushResponse, TarShardChunk, TarShardComplete, TarShardHeader, UploadComplete,
};
use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
use crate::transfer_plan::{self, PlanOptions, TransferTask};
use tar::{Builder, EntryType, Header};

#[derive(Debug, Clone)]
pub struct RemotePushReport {
    pub files_requested: Vec<String>,
    pub fallback_used: bool,
    pub data_port: Option<u32>,
    pub summary: PushSummary,
}

const CONTROL_PLANE_CHUNK_SIZE: usize = 1 * 1024 * 1024;

enum TransferPayload {
    File(FileHeader),
    TarShard { headers: Vec<FileHeader> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransferMode {
    Undecided,
    DataPlane,
    Fallback,
}

struct DataPlaneSession {
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl DataPlaneSession {
    async fn connect(host: &str, port: u32, token: &[u8]) -> Result<Self> {
        let addr = format!("{}:{}", host, port);
        let mut stream = TcpStream::connect(addr.clone())
            .await
            .with_context(|| format!("connecting to data plane {}", addr))?;

        stream
            .write_all(token)
            .await
            .context("writing negotiation token")?;

        Ok(Self {
            stream,
            buffer: vec![0u8; 64 * 1024],
        })
    }

    async fn send_files(&mut self, source_root: &Path, headers: &[FileHeader]) -> Result<()> {
        for header in headers {
            let rel = &header.relative_path;
            let path = source_root.join(rel);

            let path_bytes = rel.as_bytes();
            eprintln!("data plane send {}", rel);
            if path_bytes.len() > u32::MAX as usize {
                bail!("relative path too long for transfer: {}", rel);
            }

            self.stream
                .write_all(&(path_bytes.len() as u32).to_be_bytes())
                .await
                .context("writing path length")?;
            self.stream
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

            self.stream
                .write_all(&metadata.len().to_be_bytes())
                .await
                .context("writing file size")?;

            let mut file = fs::File::open(&path)
                .await
                .with_context(|| format!("opening {}", path.display()))?;

            let mut remaining = metadata.len();
            while remaining > 0 {
                let chunk = file
                    .read(&mut self.buffer)
                    .await
                    .with_context(|| format!("reading {}", path.display()))?;
                if chunk == 0 {
                    bail!(
                        "unexpected EOF while reading {} ({} bytes remaining)",
                        path.display(),
                        remaining
                    );
                }
                self.stream
                    .write_all(&self.buffer[..chunk])
                    .await
                    .with_context(|| format!("sending {}", path.display()))?;
                remaining -= chunk as u64;
            }
        }

        Ok(())
    }

    async fn finish(&mut self) -> Result<()> {
        self.stream
            .write_all(&0u32.to_be_bytes())
            .await
            .context("writing transfer terminator")?;
        self.stream
            .flush()
            .await
            .context("flushing data plane stream")
    }
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
        let mut pending_queue: VecDeque<String> = VecDeque::new();
        let mut fallback_batches: Vec<Vec<TransferPayload>> = Vec::new();
        let mut data_plane_session: Option<DataPlaneSession> = None;
        let mut data_plane_outstanding: usize = 0;
        let mut data_plane_finished = false;
        let mut data_port: Option<u32> = None;
        let mut fallback_used = force_grpc;
        let mut summary: Option<PushSummary> = None;

        let mut transfer_mode = if force_grpc {
            TransferMode::Fallback
        } else {
            TransferMode::Undecided
        };

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

                            if matches!(transfer_mode, TransferMode::DataPlane) {
                                if let Some(session) = data_plane_session.as_mut() {
                                    let headers =
                                        drain_pending_headers(&mut pending_queue, &manifest_lookup);
                                    let sent = headers.len();
                                    if sent != 0 {
                                        session.send_files(source_root, &headers).await?;
                                        data_plane_outstanding =
                                            data_plane_outstanding.saturating_sub(sent);
                                    }
                                    if manifest_done
                                        && pending_queue.is_empty()
                                        && data_plane_outstanding == 0
                                        && !data_plane_finished
                                    {
                                        session.finish().await?;
                                        data_plane_finished = true;
                                    }
                                }
                            }
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
                                    let mut rels = list.relative_paths;
                                    eprintln!(
                                        "files_to_upload: {:?}",
                                        rels
                                    );
                                    files_requested.extend(rels.iter().cloned());
                                    let newly_requested = rels.len();
                                    pending_queue.extend(rels.drain(..));

                                    if !matches!(transfer_mode, TransferMode::Fallback) {
                                        data_plane_outstanding =
                                            data_plane_outstanding.saturating_add(newly_requested);
                                    }

                                    match transfer_mode {
                                        TransferMode::Fallback => {
                                            let headers = drain_pending_headers(&mut pending_queue, &manifest_lookup);
                                            if !headers.is_empty() {
                                                let payloads =
                                                    plan_transfer_payloads(headers, source_root)?;
                                                if !payloads.is_empty() {
                                                    fallback_batches.push(payloads);
                                                }
                                            }
                                        }
                                        TransferMode::DataPlane => {
                                            if let Some(session) = data_plane_session.as_mut() {
                                                let headers =
                                                    drain_pending_headers(&mut pending_queue, &manifest_lookup);
                                                if !headers.is_empty() {
                                                    session.send_files(source_root, &headers).await?;
                                                    data_plane_outstanding =
                                                        data_plane_outstanding.saturating_sub(headers.len());
                                                }
                                                if manifest_done
                                                    && pending_queue.is_empty()
                                                    && data_plane_outstanding == 0
                                                    && !data_plane_finished
                                                {
                                                    session.finish().await?;
                                                    data_plane_finished = true;
                                                }
                                            }
                                        }
                                        TransferMode::Undecided => {}
                                    }
                                }
                                Some(ServerPayload::Negotiation(neg)) => {
                                    if neg.tcp_fallback {
                                        fallback_used = true;
                                        transfer_mode = TransferMode::Fallback;
                                        let headers = drain_pending_headers(&mut pending_queue, &manifest_lookup);
                                        if !headers.is_empty() {
                                            let payloads =
                                                plan_transfer_payloads(headers, source_root)?;
                                            if !payloads.is_empty() {
                                                fallback_batches.push(payloads);
                                            }
                                        }
                                        data_plane_outstanding = 0;
                                        if let Some(session) = data_plane_session.as_mut() {
                                            if !data_plane_finished {
                                                session.finish().await?;
                                                data_plane_finished = true;
                                            }
                                        }
                                        data_plane_session = None;
                                        continue;
                                    }

                                    if neg.tcp_port == 0 {
                                        bail!("server reported zero data port for negotiated transfer");
                                    }

                                    if data_plane_session.is_none() {
                                        let token_bytes = general_purpose::STANDARD_NO_PAD
                                            .decode(neg.one_time_token.as_bytes())
                                            .map_err(|err| eyre!("failed to decode negotiation token: {err}"))?;

                                        let mut session = DataPlaneSession::connect(
                                            &self.endpoint.host,
                                            neg.tcp_port,
                                            &token_bytes,
                                        )
                                        .await?;
                                        let headers = drain_pending_headers(&mut pending_queue, &manifest_lookup);
                                        let sent = headers.len();
                                        if sent != 0 {
                                            session.send_files(source_root, &headers).await?;
                                            data_plane_outstanding =
                                                data_plane_outstanding.saturating_sub(sent);
                                        }
                                        data_plane_session = Some(session);
                                    } else if let Some(session) = data_plane_session.as_mut() {
                                        let headers = drain_pending_headers(&mut pending_queue, &manifest_lookup);
                                        let sent = headers.len();
                                        if sent != 0 {
                                            session.send_files(source_root, &headers).await?;
                                            data_plane_outstanding =
                                                data_plane_outstanding.saturating_sub(sent);
                                        }
                                    }

                                    transfer_mode = TransferMode::DataPlane;
                                    data_port = Some(neg.tcp_port);

                                    if manifest_done
                                        && pending_queue.is_empty()
                                        && data_plane_outstanding == 0
                                        && !data_plane_finished
                                    {
                                        if let Some(session) = data_plane_session.as_mut() {
                                            session.finish().await?;
                                            data_plane_finished = true;
                                        }
                                    }
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

        match transfer_mode {
            TransferMode::DataPlane => {
                if let Some(session) = data_plane_session.as_mut() {
                    if !pending_queue.is_empty() {
                        let headers = drain_pending_headers(&mut pending_queue, &manifest_lookup);
                        let sent = headers.len();
                        if sent != 0 {
                            session.send_files(source_root, &headers).await?;
                            data_plane_outstanding = data_plane_outstanding.saturating_sub(sent);
                        }
                    }
                    if data_plane_outstanding == 0 && !data_plane_finished {
                        session.finish().await?;
                    }
                }
            }
            TransferMode::Fallback => {
                let headers = drain_pending_headers(&mut pending_queue, &manifest_lookup);
                if !headers.is_empty() {
                    let payloads = plan_transfer_payloads(headers, source_root)?;
                    if !payloads.is_empty() {
                        fallback_batches.push(payloads);
                    }
                }
            }
            TransferMode::Undecided => {}
        }

        manifest_task
            .await
            .map_err(|err| eyre!("manifest enumeration task failed: {}", err))??;

        for batch in fallback_batches {
            transfer_payloads_via_control_plane(source_root, batch, &tx).await?;
        }

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

fn plan_transfer_payloads(
    headers: Vec<FileHeader>,
    source_root: &Path,
) -> Result<Vec<TransferPayload>> {
    if headers.is_empty() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<FileEntry> = Vec::with_capacity(headers.len());
    for header in &headers {
        let rel_path = Path::new(&header.relative_path);
        let absolute = source_root.join(rel_path);
        entries.push(FileEntry {
            path: absolute,
            size: header.size,
            is_directory: false,
        });
    }

    let mut header_map: HashMap<String, FileHeader> = headers
        .into_iter()
        .map(|header| (header.relative_path.clone(), header))
        .collect();

    let plan = transfer_plan::build_plan(&entries, source_root, PlanOptions::default());
    let mut payloads: Vec<TransferPayload> = Vec::new();

    for task in plan.tasks {
        match task {
            TransferTask::TarShard(paths) => {
                let mut shard_headers: Vec<FileHeader> = Vec::with_capacity(paths.len());
                for path in paths {
                    let rel = normalize_relative_path(&path);
                    if let Some(header) = header_map.remove(&rel) {
                        shard_headers.push(header);
                    }
                }
                if !shard_headers.is_empty() {
                    payloads.push(TransferPayload::TarShard {
                        headers: shard_headers,
                    });
                }
            }
            TransferTask::RawBundle(paths) => {
                for path in paths {
                    let rel = normalize_relative_path(&path);
                    if let Some(header) = header_map.remove(&rel) {
                        payloads.push(TransferPayload::File(header));
                    }
                }
            }
            TransferTask::Large { path } => {
                let rel = normalize_relative_path(&path);
                if let Some(header) = header_map.remove(&rel) {
                    payloads.push(TransferPayload::File(header));
                }
            }
        }
    }

    for (_, header) in header_map.into_iter() {
        payloads.push(TransferPayload::File(header));
    }

    Ok(payloads)
}

fn build_tar_shard(source_root: &Path, headers: &[FileHeader]) -> Result<Vec<u8>> {
    let mut builder = Builder::new(Vec::new());

    for header in headers {
        let rel = Path::new(&header.relative_path);
        let full_path = source_root.join(rel);
        let mut file = StdFile::open(&full_path)
            .with_context(|| format!("opening {}", full_path.display()))?;

        let mut tar_header = Header::new_gnu();
        tar_header.set_entry_type(EntryType::Regular);
        let mode = if header.permissions == 0 {
            0o644
        } else {
            header.permissions
        };
        tar_header.set_mode(mode.into());
        tar_header.set_size(header.size);
        let mtime = if header.mtime_seconds >= 0 {
            header.mtime_seconds as u64
        } else {
            0
        };
        tar_header.set_mtime(mtime);
        tar_header.set_uid(0);
        tar_header.set_gid(0);
        tar_header.set_cksum();

        builder
            .append_data(&mut tar_header, rel, &mut file)
            .with_context(|| format!("adding {} to tar shard", full_path.display()))?;
    }

    builder.finish()?;
    let data = builder.into_inner()?;
    Ok(data)
}

fn drain_pending_headers(
    queue: &mut VecDeque<String>,
    lookup: &HashMap<String, FileHeader>,
) -> Vec<FileHeader> {
    let mut headers = Vec::new();
    while let Some(rel) = queue.front() {
        if let Some(header) = lookup.get(rel) {
            headers.push(header.clone());
            queue.pop_front();
        } else {
            break;
        }
    }
    headers
}

async fn send_payload(tx: &mpsc::Sender<ClientPushRequest>, payload: ClientPayload) -> Result<()> {
    tx.send(ClientPushRequest {
        payload: Some(payload),
    })
    .await
    .map_err(|_| eyre!("failed to send push request payload"))
}

async fn transfer_payloads_via_control_plane(
    source_root: &Path,
    payloads: Vec<TransferPayload>,
    tx: &mpsc::Sender<ClientPushRequest>,
) -> Result<()> {
    let mut buffer = vec![0u8; CONTROL_PLANE_CHUNK_SIZE];

    for payload in payloads {
        match payload {
            TransferPayload::File(header) => {
                send_payload(tx, ClientPayload::FileManifest(header.clone())).await?;

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
                        tx,
                        ClientPayload::FileData(FileData {
                            content: buffer[..chunk].to_vec(),
                        }),
                    )
                    .await?;
                    remaining -= chunk as u64;
                }
            }
            TransferPayload::TarShard { headers } => {
                let source_root = source_root.to_path_buf();
                let headers_clone = headers.clone();
                let data =
                    task::spawn_blocking(move || build_tar_shard(&source_root, &headers_clone))
                        .await
                        .map_err(|err| eyre!("tar shard worker failed: {err}"))??;

                send_payload(
                    tx,
                    ClientPayload::TarShardHeader(TarShardHeader {
                        files: headers.clone(),
                        archive_size: data.len() as u64,
                    }),
                )
                .await?;

                for chunk in data.chunks(CONTROL_PLANE_CHUNK_SIZE) {
                    send_payload(
                        tx,
                        ClientPayload::TarShardChunk(TarShardChunk {
                            content: chunk.to_vec(),
                        }),
                    )
                    .await?;
                }

                send_payload(tx, ClientPayload::TarShardComplete(TarShardComplete {})).await?;
            }
        }
    }

    send_payload(tx, ClientPayload::UploadComplete(UploadComplete {})).await?;

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
