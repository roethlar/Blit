use base64::{engine::general_purpose, Engine as _};
use blit_core::enumeration::{EntryKind, FileEnumerator};
use blit_core::fs_enum::FileFilter;
use blit_core::generated::blit_server::{Blit, BlitServer};
use blit_core::generated::{
    client_push_request, pull_chunk::Payload as PullPayload, server_push_response, Ack,
    ClientPushRequest, CompletionRequest, CompletionResponse, DataTransferNegotiation, FileData,
    FileHeader, FileList, ListModulesRequest, ListModulesResponse, ListRequest, ListResponse,
    PullChunk, PullRequest, PurgeRequest, PurgeResponse, PushSummary, ServerPushResponse,
};
use clap::Parser;
use rand::{rngs::OsRng, RngCore};
use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status, Streaming};

use eyre::Result;

type PushSender = mpsc::Sender<Result<ServerPushResponse, Status>>;
type PullSender = mpsc::Sender<Result<PullChunk, Status>>;

const TOKEN_LEN: usize = 32;

#[derive(Debug, Clone)]
struct ModuleConfig {
    name: String,
    path: PathBuf,
    read_only: bool,
}

#[derive(Debug, Default)]
struct TransferStats {
    files_transferred: u64,
    bytes_transferred: u64,
    bytes_zero_copy: u64,
}

#[derive(Parser, Debug)]
#[command(name = "blit-daemon", about = "Remote transfer daemon for blit v2")]
struct DaemonArgs {
    /// Bind address for the gRPC control plane (host:port)
    #[arg(long, default_value = "127.0.0.1:50051")]
    bind: String,
    /// Force the daemon to use the gRPC data plane instead of TCP
    #[arg(long)]
    force_grpc_data: bool,
}

pub struct BlitService {
    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    force_grpc_data: bool,
}

impl Default for BlitService {
    fn default() -> Self {
        Self::new(false)
    }
}

impl BlitService {
    pub fn new(force_grpc_data: bool) -> Self {
        let mut modules = HashMap::new();
        if let Ok(cwd) = std::env::current_dir() {
            modules.insert(
                "default".to_string(),
                ModuleConfig {
                    name: "default".to_string(),
                    path: cwd,
                    read_only: false,
                },
            );
        }

        Self {
            modules: Arc::new(Mutex::new(modules)),
            force_grpc_data,
        }
    }
}

#[tonic::async_trait]
impl Blit for BlitService {
    type PushStream = ReceiverStream<Result<ServerPushResponse, Status>>;
    type PullStream = tokio_stream::wrappers::ReceiverStream<Result<PullChunk, Status>>;

    async fn push(
        &self,
        request: Request<Streaming<ClientPushRequest>>,
    ) -> Result<Response<Self::PushStream>, Status> {
        let modules = Arc::clone(&self.modules);
        let (tx, rx) = mpsc::channel(32);
        let stream = request.into_inner();
        let force_grpc_data = self.force_grpc_data;

        tokio::spawn(async move {
            if let Err(status) =
                handle_push_stream(modules, stream, tx.clone(), force_grpc_data).await
            {
                let _ = tx.send(Err(status)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn pull(
        &self,
        request: Request<PullRequest>,
    ) -> Result<Response<Self::PullStream>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, &req.module).await?;

        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(async move {
            if let Err(status) = stream_pull(module, req.path, tx.clone()).await {
                let _ = tx.send(Err(status)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn list(&self, _request: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        Err(Status::unimplemented("List is not yet implemented"))
    }

    async fn purge(
        &self,
        _request: Request<PurgeRequest>,
    ) -> Result<Response<PurgeResponse>, Status> {
        Err(Status::unimplemented("Purge is not yet implemented"))
    }

    async fn complete_path(
        &self,
        _request: Request<CompletionRequest>,
    ) -> Result<Response<CompletionResponse>, Status> {
        Err(Status::unimplemented("CompletePath is not yet implemented"))
    }

    async fn list_modules(
        &self,
        _request: Request<ListModulesRequest>,
    ) -> Result<Response<ListModulesResponse>, Status> {
        Err(Status::unimplemented("ListModules is not yet implemented"))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = DaemonArgs::parse();
    let addr: SocketAddr = args.bind.parse()?;
    let service = BlitService::new(args.force_grpc_data);

    println!("blitd v2 listening on {}", addr);

    Server::builder()
        .add_service(BlitServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}

async fn handle_push_stream(
    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    mut stream: Streaming<ClientPushRequest>,
    tx: PushSender,
    force_grpc_data: bool,
) -> Result<(), Status> {
    let mut module: Option<ModuleConfig> = None;
    let mut manifest: Vec<FileHeader> = Vec::new();
    let mut manifest_complete = false;

    while let Some(request) = stream.message().await? {
        match request.payload {
            Some(client_push_request::Payload::Header(header)) => {
                if module.is_some() {
                    return Err(Status::invalid_argument("duplicate push header received"));
                }
                let config = resolve_module(&modules, &header.module).await?;
                if config.read_only {
                    return Err(Status::permission_denied(format!(
                        "module '{}' is read-only",
                        config.name
                    )));
                }
                module = Some(config);
                send_control_message(&tx, server_push_response::Payload::Ack(Ack {})).await?;
            }
            Some(client_push_request::Payload::FileManifest(file)) => {
                manifest.push(file);
            }
            Some(client_push_request::Payload::ManifestComplete(_)) => {
                manifest_complete = true;
                break;
            }
            Some(client_push_request::Payload::FileData(_)) => {
                return Err(Status::failed_precondition(
                    "data payload received before negotiation",
                ));
            }
            Some(client_push_request::Payload::UploadComplete(_)) => {
                // Ignore; summary is driven once data plane completes.
            }
            None => {}
        }
    }

    let module = module.ok_or_else(|| Status::invalid_argument("push stream missing header"))?;
    if !manifest_complete {
        return Err(Status::invalid_argument(
            "push stream ended before manifest completion",
        ));
    }

    let files_requested = compute_need_list(&module, &manifest)?;
    let relative_paths: Vec<String> = files_requested
        .iter()
        .map(|header| header.relative_path.clone())
        .collect();

    send_control_message(
        &tx,
        server_push_response::Payload::FilesToUpload(FileList { relative_paths }),
    )
    .await?;

    if files_requested.is_empty() {
        execute_grpc_fallback(&tx, &mut stream, &module, files_requested).await?;
        return Ok(());
    }

    if force_grpc_data {
        execute_grpc_fallback(&tx, &mut stream, &module, files_requested).await?;
        return Ok(());
    }

    let listener = match bind_data_plane_listener().await {
        Ok(listener) => listener,
        Err(_) => {
            execute_grpc_fallback(&tx, &mut stream, &module, files_requested).await?;
            return Ok(());
        }
    };
    let port = listener
        .local_addr()
        .map_err(|err| Status::internal(format!("querying listener addr: {}", err)))?
        .port();

    let token = generate_token();
    let token_string = general_purpose::STANDARD_NO_PAD.encode(&token);

    let (summary_tx, summary_rx) = oneshot::channel();
    let module_for_transfer = module.clone();
    let files_for_transfer = files_requested.clone();

    tokio::spawn(async move {
        let result =
            accept_data_connection(listener, token, module_for_transfer, files_for_transfer).await;
        let _ = summary_tx.send(result);
    });

    send_control_message(
        &tx,
        server_push_response::Payload::Negotiation(DataTransferNegotiation {
            tcp_port: port as u32,
            one_time_token: token_string,
            tcp_fallback: false,
        }),
    )
    .await?;

    let summary_stats = summary_rx
        .await
        .map_err(|_| Status::internal("data plane task cancelled"))??;

    send_control_message(
        &tx,
        server_push_response::Payload::Summary(PushSummary {
            files_transferred: summary_stats.files_transferred,
            bytes_transferred: summary_stats.bytes_transferred,
            bytes_zero_copy: summary_stats.bytes_zero_copy,
            tcp_fallback_used: false,
        }),
    )
    .await?;

    Ok(())
}

async fn resolve_module(
    modules: &Arc<Mutex<HashMap<String, ModuleConfig>>>,
    name: &str,
) -> Result<ModuleConfig, Status> {
    let guard = modules.lock().await;
    guard
        .get(name)
        .cloned()
        .ok_or_else(|| Status::not_found(format!("module '{}' not found", name)))
}

async fn send_control_message(
    tx: &PushSender,
    payload: server_push_response::Payload,
) -> Result<(), Status> {
    tx.send(Ok(ServerPushResponse {
        payload: Some(payload),
    }))
    .await
    .map_err(|_| Status::internal("failed to send push response"))
}

fn compute_need_list(
    module: &ModuleConfig,
    manifest: &[FileHeader],
) -> Result<Vec<FileHeader>, Status> {
    let mut needs = Vec::new();
    for file in manifest {
        let rel = resolve_relative_path(&file.relative_path)?;
        let sanitized = rel.to_string_lossy().to_string();
        let full_path = module.path.join(&rel);

        let requires_upload = match fs::metadata(&full_path) {
            Ok(meta) => {
                if !meta.is_file() {
                    true
                } else {
                    let same_size = meta.len() == file.size;
                    let same_mtime = metadata_mtime_seconds(&meta)
                        .map(|seconds| seconds == file.mtime_seconds)
                        .unwrap_or(false);
                    !(same_size && same_mtime)
                }
            }
            Err(_) => true,
        };

        if requires_upload {
            let mut header = file.clone();
            header.relative_path = sanitized;
            needs.push(header);
        }
    }

    Ok(needs)
}

fn resolve_relative_path(rel: &str) -> Result<PathBuf, Status> {
    #[cfg(windows)]
    {
        if rel.starts_with('/') || rel.starts_with('\\') {
            return Err(Status::invalid_argument(format!(
                "absolute-style path not allowed in manifest: {}",
                rel
            )));
        }
    }

    let path = Path::new(rel);
    if path.is_absolute() {
        return Err(Status::invalid_argument(format!(
            "absolute paths not allowed in manifest: {}",
            rel
        )));
    }

    use std::path::Component;
    if path
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(Status::invalid_argument(format!(
            "parent directory segments not allowed: {}",
            rel
        )));
    }

    Ok(path.to_path_buf())
}

fn metadata_mtime_seconds(meta: &fs::Metadata) -> Option<i64> {
    use std::time::UNIX_EPOCH;

    let modified = meta.modified().ok()?;
    match modified.duration_since(UNIX_EPOCH) {
        Ok(duration) => Some(duration.as_secs() as i64),
        Err(err) => {
            let dur = err.duration();
            Some(-(dur.as_secs() as i64))
        }
    }
}

fn permissions_mode(meta: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        0
    }
}

async fn bind_data_plane_listener() -> Result<TcpListener, Status> {
    TcpListener::bind("0.0.0.0:0")
        .await
        .map_err(|err| Status::internal(format!("failed to bind data plane socket: {}", err)))
}

fn generate_token() -> Vec<u8> {
    let mut buf = vec![0u8; TOKEN_LEN];
    OsRng.fill_bytes(&mut buf);
    buf
}

async fn accept_data_connection(
    listener: TcpListener,
    expected_token: Vec<u8>,
    module: ModuleConfig,
    files: Vec<FileHeader>,
) -> Result<TransferStats, Status> {
    let (mut socket, _) = listener
        .accept()
        .await
        .map_err(|err| Status::internal(format!("data plane accept failed: {}", err)))?;

    let mut token_buf = vec![0u8; expected_token.len()];
    socket
        .read_exact(&mut token_buf)
        .await
        .map_err(|err| Status::internal(format!("failed to read data plane token: {}", err)))?;
    if token_buf != expected_token {
        return Err(Status::permission_denied("invalid data plane token"));
    }

    let mut pending: HashMap<String, FileHeader> = files
        .into_iter()
        .map(|header| (header.relative_path.clone(), header))
        .collect();

    let mut stats = TransferStats::default();

    loop {
        let path_len = read_u32(&mut socket).await?;
        if path_len == 0 {
            break;
        }

        let mut path_bytes = vec![0u8; path_len as usize];
        socket
            .read_exact(&mut path_bytes)
            .await
            .map_err(|err| Status::internal(format!("failed to read path bytes: {}", err)))?;
        let rel_string = String::from_utf8(path_bytes)
            .map_err(|_| Status::invalid_argument("data plane path not valid UTF-8"))?;

        let header = pending
            .remove(&rel_string)
            .ok_or_else(|| Status::invalid_argument(format!("unexpected file '{}'", rel_string)))?;

        let file_size = read_u64(&mut socket).await?;
        if file_size != header.size {
            return Err(Status::invalid_argument(format!(
                "size mismatch for {} (declared {}, expected {})",
                rel_string, file_size, header.size
            )));
        }
        let rel_path = resolve_relative_path(&rel_string)?;
        let dest_path = module.path.join(&rel_path);

        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|err| {
                Status::internal(format!("create dir {}: {}", parent.display(), err))
            })?;
        }

        let mut file = tokio::fs::File::create(&dest_path).await.map_err(|err| {
            Status::internal(format!("create file {}: {}", dest_path.display(), err))
        })?;

        let mut limited = (&mut socket).take(file_size);
        let bytes_copied = tokio::io::copy(&mut limited, &mut file)
            .await
            .map_err(|err| Status::internal(format!("writing {}: {}", dest_path.display(), err)))?;
        if bytes_copied != file_size {
            return Err(Status::internal(format!(
                "short transfer for {} (expected {} bytes, received {})",
                rel_string, file_size, bytes_copied
            )));
        }

        stats.files_transferred += 1;
        stats.bytes_transferred += bytes_copied;
    }

    if !pending.is_empty() {
        let missing: Vec<String> = pending.into_keys().collect();
        return Err(Status::internal(format!(
            "transfer incomplete; missing files: {:?}",
            missing
        )));
    }

    Ok(stats)
}

async fn receive_fallback_data(
    stream: &mut Streaming<ClientPushRequest>,
    module: &ModuleConfig,
    files: Vec<FileHeader>,
) -> Result<TransferStats, Status> {
    let mut pending: HashMap<String, FileHeader> = files
        .into_iter()
        .map(|header| (header.relative_path.clone(), header))
        .collect();

    let mut current: Option<FileHeader> = None;
    let mut stats = TransferStats::default();

    while let Some(req) = stream.message().await? {
        match req.payload {
            Some(client_push_request::Payload::FileManifest(header)) => {
                if !pending.contains_key(&header.relative_path) {
                    return Err(Status::invalid_argument(format!(
                        "unexpected fallback file manifest '{}'",
                        header.relative_path
                    )));
                }
                current = Some(header);
            }
            Some(client_push_request::Payload::FileData(data)) => {
                let header = current.take().ok_or_else(|| {
                    Status::invalid_argument("file data received before file manifest")
                })?;

                let rel_path = resolve_relative_path(&header.relative_path)?;
                let dest_path = module.path.join(&rel_path);
                if let Some(parent) = dest_path.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|err| {
                        Status::internal(format!("create dir {}: {}", parent.display(), err))
                    })?;
                }

                let mut file = tokio::fs::File::create(&dest_path).await.map_err(|err| {
                    Status::internal(format!("create file {}: {}", dest_path.display(), err))
                })?;
                file.write_all(&data.content).await.map_err(|err| {
                    Status::internal(format!("write {}: {}", dest_path.display(), err))
                })?;

                stats.files_transferred += 1;
                stats.bytes_transferred += data.content.len() as u64;

                pending.remove(&header.relative_path);
            }
            Some(client_push_request::Payload::UploadComplete(_)) => break,
            Some(_) => {
                return Err(Status::invalid_argument(
                    "unexpected message during fallback transfer",
                ));
            }
            None => break,
        }
    }

    if current.is_some() {
        return Err(Status::invalid_argument(
            "fallback transfer ended mid-file (missing data block)",
        ));
    }

    if !pending.is_empty() {
        let missing: Vec<String> = pending.into_keys().collect();
        return Err(Status::internal(format!(
            "fallback transfer incomplete; missing files: {:?}",
            missing
        )));
    }

    Ok(stats)
}

async fn execute_grpc_fallback(
    tx: &PushSender,
    stream: &mut Streaming<ClientPushRequest>,
    module: &ModuleConfig,
    files_requested: Vec<FileHeader>,
) -> Result<(), Status> {
    send_control_message(
        tx,
        server_push_response::Payload::Negotiation(DataTransferNegotiation {
            tcp_port: 0,
            one_time_token: String::new(),
            tcp_fallback: true,
        }),
    )
    .await?;

    let stats = receive_fallback_data(stream, module, files_requested).await?;

    send_control_message(
        tx,
        server_push_response::Payload::Summary(PushSummary {
            files_transferred: stats.files_transferred,
            bytes_transferred: stats.bytes_transferred,
            bytes_zero_copy: stats.bytes_zero_copy,
            tcp_fallback_used: true,
        }),
    )
    .await?;

    Ok(())
}

async fn stream_pull(
    module: ModuleConfig,
    requested_path: String,
    tx: PullSender,
) -> Result<(), Status> {
    let requested = if requested_path.trim().is_empty() {
        PathBuf::from(".")
    } else {
        resolve_relative_path(&requested_path)?
    };

    let root = module.path.join(&requested);

    if !root.exists() {
        return Err(Status::not_found(format!(
            "path not found in module '{}': {}",
            module.name, requested_path
        )));
    }

    if root.is_file() {
        let relative_name = if requested == PathBuf::from(".") {
            root.file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
        } else {
            requested.clone()
        };
        stream_single_file(&tx, &relative_name, &root).await?;
    } else if root.is_dir() {
        let root_clone = root.clone();
        let entries = tokio::task::spawn_blocking(move || {
            let enumerator = FileEnumerator::new(FileFilter::default());
            enumerator.enumerate_local(&root_clone)
        })
        .await
        .map_err(|e| Status::internal(format!("enumeration task failed: {}", e)))?
        .map_err(|e| Status::internal(format!("enumeration error: {}", e)))?;

        for entry in entries {
            if matches!(entry.kind, EntryKind::File { .. }) {
                stream_single_file(&tx, &entry.relative_path, &entry.absolute_path).await?;
            }
        }
    } else {
        return Err(Status::invalid_argument(format!(
            "unsupported path type for pull: {}",
            requested_path
        )));
    }

    Ok(())
}

async fn stream_single_file(
    tx: &PullSender,
    relative: &Path,
    abs_path: &Path,
) -> Result<(), Status> {
    let metadata = tokio::fs::metadata(abs_path)
        .await
        .map_err(|err| Status::internal(format!("stat {}: {}", abs_path.display(), err)))?;

    let normalized = normalize_relative_path(relative);

    tx.send(Ok(PullChunk {
        payload: Some(PullPayload::FileHeader(FileHeader {
            relative_path: normalized,
            size: metadata.len(),
            mtime_seconds: metadata_mtime_seconds(&metadata).unwrap_or(0),
            permissions: permissions_mode(&metadata),
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send pull header"))?;

    let mut file = tokio::fs::File::open(abs_path)
        .await
        .map_err(|err| Status::internal(format!("open {}: {}", abs_path.display(), err)))?;
    let mut buffer = vec![0u8; 64 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .await
            .map_err(|err| Status::internal(format!("read {}: {}", abs_path.display(), err)))?;
        if read == 0 {
            break;
        }

        tx.send(Ok(PullChunk {
            payload: Some(PullPayload::FileData(FileData {
                content: buffer[..read].to_vec(),
            })),
        }))
        .await
        .map_err(|_| Status::internal("failed to send pull chunk"))?;
    }

    Ok(())
}

fn normalize_relative_path(path: &Path) -> String {
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
async fn read_u32(stream: &mut TcpStream) -> Result<u32, Status> {
    let mut buf = [0u8; 4];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|err| Status::internal(format!("failed to read u32: {}", err)))?;
    Ok(u32::from_be_bytes(buf))
}

async fn read_u64(stream: &mut TcpStream) -> Result<u64, Status> {
    let mut buf = [0u8; 8];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|err| Status::internal(format!("failed to read u64: {}", err)))?;
    Ok(u64::from_be_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_core::remote::{RemoteEndpoint, RemotePullClient};
    use eyre::Result;
    use tempfile::tempdir;
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;
    use tokio::task::JoinHandle;
    use tokio_stream::wrappers::TcpListenerStream;

    #[test]
    fn resolve_relative_path_rejects_parent_segments() {
        assert!(resolve_relative_path("../evil").is_err());
        assert!(resolve_relative_path("sub/../../evil").is_err());
        #[cfg(unix)]
        {
            assert!(resolve_relative_path("/abs/path").is_err());
        }
        #[cfg(windows)]
        {
            assert!(resolve_relative_path("/abs/path").is_err());
            assert!(resolve_relative_path("\\abs\\path").is_err());
            assert!(resolve_relative_path("C:\\abs\\path").is_err());
        }
    }

    async fn spawn_test_daemon(
        root: PathBuf,
        force_grpc_data: bool,
    ) -> (
        SocketAddr,
        oneshot::Sender<()>,
        JoinHandle<Result<(), tonic::transport::Error>>,
    ) {
        let service = BlitService::new(force_grpc_data);
        {
            let mut modules = service.modules.lock().await;
            modules.insert(
                "default".to_string(),
                ModuleConfig {
                    name: "default".to_string(),
                    path: root,
                    read_only: false,
                },
            );
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let server = tokio::spawn(async move {
            Server::builder()
                .add_service(BlitServer::new(service))
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async move {
                    let _ = shutdown_rx.await;
                })
                .await
        });

        (addr, shutdown_tx, server)
    }

    fn default_endpoint(addr: SocketAddr) -> Result<RemoteEndpoint> {
        RemoteEndpoint::parse(&format!("blit://{}:{}/default", addr.ip(), addr.port()))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_pull_transfers_directory_tree() -> Result<()> {
        let src = tempdir()?;
        let nested = src.path().join("nested");
        fs::create_dir_all(&nested)?;
        fs::write(src.path().join("alpha.txt"), b"alpha")?;
        fs::write(nested.join("beta.txt"), b"beta")?;

        let dest = tempdir()?;

        let (addr, shutdown, server) = spawn_test_daemon(src.path().to_path_buf(), false).await;

        let mut endpoint = default_endpoint(addr)?;
        let remote_path = endpoint.resource.clone().unwrap_or_else(|| ".".to_string());
        endpoint.resource = None;

        let mut client = RemotePullClient::connect(endpoint).await?;
        let pull_result = client.pull(&remote_path, dest.path()).await;
        drop(client);
        let _ = shutdown.send(());
        server.await.unwrap().unwrap();
        let report = pull_result?;

        assert_eq!(report.files_transferred, 2);
        assert_eq!(
            std::fs::read_to_string(dest.path().join("alpha.txt"))?,
            "alpha"
        );
        assert_eq!(
            std::fs::read_to_string(dest.path().join("nested").join("beta.txt"))?,
            "beta"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_pull_transfers_directory_tree_with_forced_grpc() -> Result<()> {
        let src = tempdir()?;
        let nested = src.path().join("nested");
        fs::create_dir_all(&nested)?;
        fs::write(src.path().join("alpha.txt"), b"alpha")?;
        fs::write(nested.join("beta.txt"), b"beta")?;

        let dest = tempdir()?;

        let (addr, shutdown, server) = spawn_test_daemon(src.path().to_path_buf(), true).await;

        let mut endpoint = default_endpoint(addr)?;
        let remote_path = endpoint.resource.clone().unwrap_or_else(|| ".".to_string());
        endpoint.resource = None;

        let mut client = RemotePullClient::connect(endpoint).await?;
        let pull_result = client.pull(&remote_path, dest.path()).await;
        drop(client);
        let _ = shutdown.send(());
        server.await.unwrap().unwrap();
        let report = pull_result?;

        assert_eq!(report.files_transferred, 2);
        assert_eq!(
            std::fs::read_to_string(dest.path().join("alpha.txt"))?,
            "alpha"
        );
        assert_eq!(
            std::fs::read_to_string(dest.path().join("nested").join("beta.txt"))?,
            "beta"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_pull_transfers_single_file() -> Result<()> {
        let src = tempdir()?;
        let nested = src.path().join("nested");
        fs::create_dir_all(&nested)?;
        fs::write(nested.join("beta.txt"), b"beta")?;

        let dest = tempdir()?;

        let (addr, shutdown, server) = spawn_test_daemon(src.path().to_path_buf(), false).await;

        let mut endpoint = default_endpoint(addr)?;
        endpoint.resource = None;

        let mut client = RemotePullClient::connect(endpoint).await?;
        let pull_result = client.pull("nested/beta.txt", dest.path()).await;
        drop(client);
        let _ = shutdown.send(());
        server.await.unwrap().unwrap();
        let report = pull_result?;

        assert_eq!(report.files_transferred, 1);
        assert_eq!(
            std::fs::read_to_string(dest.path().join("nested").join("beta.txt"))?,
            "beta"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_pull_rejects_parent_segments_request() -> Result<()> {
        let src = tempdir()?;
        fs::write(src.path().join("file.txt"), b"content")?;
        let dest = tempdir()?;

        let (addr, shutdown, server) = spawn_test_daemon(src.path().to_path_buf(), false).await;

        let mut endpoint = default_endpoint(addr)?;
        endpoint.resource = None;

        let mut client = RemotePullClient::connect(endpoint).await?;
        let pull_result = client.pull("../secret", dest.path()).await;
        drop(client);
        let _ = shutdown.send(());
        server.await.unwrap().unwrap();

        assert!(pull_result.is_err());
        let err = pull_result.unwrap_err().to_string();
        assert!(
            err.contains("parent directory"),
            "unexpected error message: {err}"
        );
        assert!(
            dest.path().read_dir()?.next().is_none(),
            "destination should remain empty"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_pull_reports_missing_paths() -> Result<()> {
        let src = tempdir()?;
        fs::write(src.path().join("file.txt"), b"content")?;
        let dest = tempdir()?;

        let (addr, shutdown, server) = spawn_test_daemon(src.path().to_path_buf(), false).await;

        let mut endpoint = default_endpoint(addr)?;
        endpoint.resource = None;

        let mut client = RemotePullClient::connect(endpoint).await?;
        let pull_result = client.pull("missing.txt", dest.path()).await;
        drop(client);
        let _ = shutdown.send(());
        server.await.unwrap().unwrap();

        assert!(pull_result.is_err());
        let err = pull_result.unwrap_err().to_string();
        assert!(
            err.contains("path not found"),
            "unexpected error message: {err}"
        );
        assert!(
            dest.path().read_dir()?.next().is_none(),
            "destination should remain empty"
        );

        Ok(())
    }

    #[test]
    fn compute_need_list_detects_missing_and_outdated_files() {
        let dir = tempdir().unwrap();
        let module = ModuleConfig {
            name: "default".to_string(),
            path: dir.path().to_path_buf(),
            read_only: false,
        };

        let match_path = dir.path().join("match.txt");
        fs::write(&match_path, b"hello").unwrap();
        let match_meta = fs::metadata(&match_path).unwrap();
        let match_header = FileHeader {
            relative_path: "match.txt".into(),
            size: match_meta.len(),
            mtime_seconds: metadata_mtime_seconds(&match_meta).unwrap(),
            permissions: 0,
        };

        let missing_header = FileHeader {
            relative_path: "missing.txt".into(),
            size: 42,
            mtime_seconds: 0,
            permissions: 0,
        };

        let stale_path = dir.path().join("stale.txt");
        fs::write(&stale_path, b"old").unwrap();
        let stale_meta = fs::metadata(&stale_path).unwrap();
        let stale_header = FileHeader {
            relative_path: "stale.txt".into(),
            size: stale_meta.len() + 10,
            mtime_seconds: metadata_mtime_seconds(&stale_meta).unwrap(),
            permissions: 0,
        };

        let manifest = vec![match_header, missing_header.clone(), stale_header.clone()];
        let needs = compute_need_list(&module, &manifest).unwrap();
        let requested: Vec<String> = needs.into_iter().map(|h| h.relative_path).collect();

        assert!(
            requested.iter().any(|rel| rel == "missing.txt"),
            "missing file should be requested"
        );
        assert!(
            requested.iter().any(|rel| rel == "stale.txt"),
            "stale file should be requested"
        );
        assert!(
            !requested.iter().any(|rel| rel == "match.txt"),
            "identical file should not be requested"
        );
    }
}
