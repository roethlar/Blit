use crate::runtime::ModuleConfig;
use blit_core::generated::{
    client_push_request, server_push_response, ClientPushRequest, DataTransferNegotiation,
    FileHeader,
};
use rand::{rngs::OsRng, RngCore};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;
use tar::Archive;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinSet;
use tonic::{Status, Streaming};

use super::super::util::resolve_relative_path;
use super::super::PushSender;
use super::control::send_control_message;

const TOKEN_LEN: usize = 32;
const DATA_PLANE_RECORD_FILE: u8 = 0;
const DATA_PLANE_RECORD_TAR_SHARD: u8 = 1;
const DATA_PLANE_RECORD_END: u8 = 0xFF;
const MAX_PARALLEL_TAR_TASKS: usize = 4;

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct TransferStats {
    pub files_transferred: u64,
    pub bytes_transferred: u64,
    pub bytes_zero_copy: u64,
}

pub(crate) async fn bind_data_plane_listener() -> Result<TcpListener, Status> {
    TcpListener::bind("0.0.0.0:0")
        .await
        .map_err(|err| Status::internal(format!("failed to bind data plane socket: {}", err)))
}

pub(crate) fn generate_token() -> Vec<u8> {
    let mut buf = vec![0u8; TOKEN_LEN];
    OsRng.fill_bytes(&mut buf);
    buf
}

pub(crate) async fn accept_data_connection_stream(
    listener: TcpListener,
    expected_token: Vec<u8>,
    module: ModuleConfig,
    mut files: mpsc::Receiver<FileHeader>,
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

    let mut cache: HashMap<String, FileHeader> = HashMap::new();
    let mut stats = TransferStats::default();
    let mut tar_executor = TarShardExecutor::new(MAX_PARALLEL_TAR_TASKS);

    loop {
        tar_executor.drain_ready(&mut stats)?;

        let mut kind_buf = [0u8; 1];
        if let Err(err) = socket.read_exact(&mut kind_buf).await {
            return Err(Status::internal(format!(
                "failed to read data plane record tag: {}",
                err
            )));
        }

        match kind_buf[0] {
            DATA_PLANE_RECORD_FILE => {
                let path_len = read_u32(&mut socket).await?;
                let mut path_bytes = vec![0u8; path_len as usize];
                socket.read_exact(&mut path_bytes).await.map_err(|err| {
                    Status::internal(format!("failed to read path bytes: {}", err))
                })?;
                let rel_string = String::from_utf8(path_bytes)
                    .map_err(|_| Status::invalid_argument("data plane path not valid UTF-8"))?;

                let header = next_data_plane_header(&mut files, &mut cache, &rel_string).await?;

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
                let bytes_copied =
                    tokio::io::copy(&mut limited, &mut file)
                        .await
                        .map_err(|err| {
                            Status::internal(format!("writing {}: {}", dest_path.display(), err))
                        })?;
                if bytes_copied != file_size {
                    return Err(Status::internal(format!(
                        "short transfer for {} (expected {} bytes, received {})",
                        rel_string, file_size, bytes_copied
                    )));
                }

                stats.files_transferred += 1;
                stats.bytes_transferred += bytes_copied;
            }
            DATA_PLANE_RECORD_TAR_SHARD => {
                let file_count = read_u32(&mut socket).await? as usize;
                let mut headers = Vec::with_capacity(file_count);

                for _ in 0..file_count {
                    let path_len = read_u32(&mut socket).await?;
                    let mut path_bytes = vec![0u8; path_len as usize];
                    socket.read_exact(&mut path_bytes).await.map_err(|err| {
                        Status::internal(format!("failed to read shard path bytes: {}", err))
                    })?;
                    let rel_string = String::from_utf8(path_bytes)
                        .map_err(|_| Status::invalid_argument("tar shard path not valid UTF-8"))?;

                    let expected_size = read_u64(&mut socket).await?;
                    let expected_mtime = read_i64(&mut socket).await?;
                    let expected_permissions = read_u32(&mut socket).await?;

                    let header =
                        next_data_plane_header(&mut files, &mut cache, &rel_string).await?;

                    if header.size != expected_size
                        || header.mtime_seconds != expected_mtime
                        || header.permissions != expected_permissions
                    {
                        return Err(Status::invalid_argument(format!(
                            "tar shard metadata mismatch for '{}'",
                            rel_string
                        )));
                    }

                    headers.push(header);
                }

                let tar_len = read_u64(&mut socket).await?;
                let mut buffer = vec![0u8; tar_len as usize];
                socket.read_exact(&mut buffer).await.map_err(|err| {
                    Status::internal(format!("failed to read tar shard bytes: {}", err))
                })?;

                tar_executor.spawn(module.clone(), headers, buffer).await?;
                tar_executor.drain_ready(&mut stats)?;
            }
            DATA_PLANE_RECORD_END => break,
            other => {
                return Err(Status::invalid_argument(format!(
                    "unknown data plane record type: {}",
                    other
                )))
            }
        }
    }

    tar_executor.finish(&mut stats).await?;

    if !cache.is_empty() {
        let missing: Vec<String> = cache.into_keys().collect();
        return Err(Status::internal(format!(
            "transfer incomplete; missing files: {:?}",
            missing
        )));
    }

    Ok(stats)
}

pub(crate) async fn next_data_plane_header(
    files: &mut mpsc::Receiver<FileHeader>,
    cache: &mut HashMap<String, FileHeader>,
    rel_string: &str,
) -> Result<FileHeader, Status> {
    if let Some(header) = cache.remove(rel_string) {
        return Ok(header);
    }

    while let Some(header) = files.recv().await {
        if header.relative_path == rel_string {
            return Ok(header);
        }
        cache.insert(header.relative_path.clone(), header);
    }

    Err(Status::internal(
        "data plane received unexpected file entry",
    ))
}

pub(crate) async fn receive_fallback_data(
    stream: &mut Streaming<ClientPushRequest>,
    module: &ModuleConfig,
    files_requested: Vec<FileHeader>,
) -> Result<TransferStats, Status> {
    #[derive(Debug)]
    struct ActiveFile {
        header: FileHeader,
        file: tokio::fs::File,
        remaining: u64,
        dest_path: PathBuf,
    }

    enum ActiveTransfer {
        File(ActiveFile),
        Tar {
            headers: Vec<FileHeader>,
            expected_size: u64,
            received: u64,
            buffer: Vec<u8>,
        },
    }

    let mut pending: HashMap<String, FileHeader> = files_requested
        .into_iter()
        .map(|header| (header.relative_path.clone(), header))
        .collect();

    let mut active: Option<ActiveTransfer> = None;
    let mut stats = TransferStats::default();
    let mut tar_executor = TarShardExecutor::new(MAX_PARALLEL_TAR_TASKS);

    while let Some(req) = stream.message().await? {
        tar_executor.drain_ready(&mut stats)?;
        match req.payload {
            Some(client_push_request::Payload::FileManifest(header)) => {
                if active.is_some() {
                    return Err(Status::failed_precondition(
                        "received new file manifest while another transfer is active",
                    ));
                }

                let expected = pending.remove(&header.relative_path).ok_or_else(|| {
                    Status::invalid_argument(format!(
                        "unexpected fallback file manifest '{}'",
                        header.relative_path
                    ))
                })?;

                if expected.size != header.size {
                    return Err(Status::invalid_argument(format!(
                        "size mismatch for '{}' (declared {}, expected {})",
                        header.relative_path, header.size, expected.size
                    )));
                }

                let rel_path = resolve_relative_path(&expected.relative_path)?;
                let dest_path = module.path.join(&rel_path);
                if let Some(parent) = dest_path.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|err| {
                        Status::internal(format!("create dir {}: {}", parent.display(), err))
                    })?;
                }

                let file = tokio::fs::File::create(&dest_path).await.map_err(|err| {
                    Status::internal(format!("create file {}: {}", dest_path.display(), err))
                })?;

                if expected.size == 0 {
                    stats.files_transferred += 1;
                    continue;
                }

                let remaining = expected.size;
                active = Some(ActiveTransfer::File(ActiveFile {
                    header: expected,
                    file,
                    remaining,
                    dest_path,
                }));
            }
            Some(client_push_request::Payload::FileData(data)) => match active.as_mut() {
                Some(ActiveTransfer::File(active_file)) => {
                    let chunk_len = data.content.len() as u64;
                    if chunk_len > active_file.remaining {
                        return Err(Status::invalid_argument(format!(
                            "received {} bytes for '{}' but only {} bytes remain",
                            chunk_len, active_file.header.relative_path, active_file.remaining
                        )));
                    }

                    active_file
                        .file
                        .write_all(&data.content)
                        .await
                        .map_err(|err| {
                            Status::internal(format!(
                                "write {}: {}",
                                active_file.dest_path.display(),
                                err
                            ))
                        })?;

                    active_file.remaining -= chunk_len;
                    stats.bytes_transferred += chunk_len;

                    if active_file.remaining == 0 {
                        stats.files_transferred += 1;
                        active = None;
                    }
                }
                Some(ActiveTransfer::Tar { .. }) => {
                    return Err(Status::invalid_argument(
                        "file data received while a tar shard is active",
                    ));
                }
                None => {
                    return Err(Status::invalid_argument(
                        "file data received before file manifest",
                    ));
                }
            },
            Some(client_push_request::Payload::TarShardHeader(shard)) => {
                if active.is_some() {
                    return Err(Status::failed_precondition(
                        "received tar shard header while another transfer is active",
                    ));
                }
                if shard.files.is_empty() {
                    return Err(Status::invalid_argument(
                        "tar shard header contained no files",
                    ));
                }

                let mut headers: Vec<FileHeader> = Vec::with_capacity(shard.files.len());
                for file_header in shard.files {
                    let expected = pending.remove(&file_header.relative_path).ok_or_else(|| {
                        Status::invalid_argument(format!(
                            "tar shard referenced unexpected file '{}'",
                            file_header.relative_path
                        ))
                    })?;
                    if expected.size != file_header.size {
                        return Err(Status::invalid_argument(format!(
                            "tar shard size mismatch for '{}' (declared {}, expected {})",
                            file_header.relative_path, file_header.size, expected.size
                        )));
                    }
                    headers.push(expected);
                }

                let capacity = usize::try_from(shard.archive_size)
                    .unwrap_or(usize::MAX)
                    .min(8 * 1024 * 1024);
                active = Some(ActiveTransfer::Tar {
                    headers,
                    expected_size: shard.archive_size,
                    received: 0,
                    buffer: Vec::with_capacity(capacity),
                });
            }
            Some(client_push_request::Payload::TarShardChunk(chunk)) => match active.as_mut() {
                Some(ActiveTransfer::Tar {
                    buffer,
                    received,
                    expected_size,
                    ..
                }) => {
                    let chunk_len = chunk.content.len() as u64;
                    let new_total = received.saturating_add(chunk_len);
                    if *expected_size != 0 && new_total > *expected_size {
                        return Err(Status::invalid_argument(format!(
                            "tar shard chunk exceeds declared size ({} > {})",
                            new_total, expected_size
                        )));
                    }
                    buffer.extend_from_slice(&chunk.content);
                    *received = new_total;
                }
                Some(ActiveTransfer::File(_)) => {
                    return Err(Status::invalid_argument(
                        "tar shard chunk received during file transfer",
                    ));
                }
                None => {
                    return Err(Status::invalid_argument(
                        "tar shard chunk received with no active shard",
                    ));
                }
            },
            Some(client_push_request::Payload::TarShardComplete(_)) => match active.take() {
                Some(ActiveTransfer::Tar {
                    headers,
                    expected_size,
                    received,
                    buffer,
                }) => {
                    if expected_size != 0 && expected_size != received {
                        return Err(Status::invalid_argument(format!(
                            "tar shard ended with {} bytes received (expected {})",
                            received, expected_size
                        )));
                    }
                    tar_executor.spawn(module.clone(), headers, buffer).await?;
                    tar_executor.drain_ready(&mut stats)?;
                }
                Some(ActiveTransfer::File(_)) => {
                    return Err(Status::invalid_argument(
                        "tar shard complete received during file transfer",
                    ));
                }
                None => {
                    return Err(Status::invalid_argument(
                        "tar shard complete received with no active shard",
                    ));
                }
            },
            Some(client_push_request::Payload::UploadComplete(_)) => {
                if active.is_some() {
                    return Err(Status::invalid_argument(
                        "upload complete received while a transfer is still active",
                    ));
                }
                break;
            }
            Some(_) => {
                return Err(Status::invalid_argument(
                    "unexpected message during fallback transfer",
                ));
            }
            None => break,
        }
    }

    tar_executor.finish(&mut stats).await?;

    if !pending.is_empty() {
        let missing: Vec<String> = pending.into_keys().collect();
        return Err(Status::internal(format!(
            "fallback transfer incomplete; missing files: {:?}",
            missing
        )));
    }

    Ok(stats)
}

pub(crate) async fn execute_grpc_fallback(
    tx: &PushSender,
    stream: &mut Streaming<ClientPushRequest>,
    module: &ModuleConfig,
    files_requested: Vec<FileHeader>,
) -> Result<TransferStats, Status> {
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

    Ok(stats)
}

struct TarShardExecutor {
    semaphore: Arc<Semaphore>,
    tasks: JoinSet<Result<TransferStats, Status>>,
}

impl TarShardExecutor {
    fn new(max_parallel: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_parallel)),
            tasks: JoinSet::new(),
        }
    }

    async fn spawn(
        &mut self,
        module: ModuleConfig,
        headers: Vec<FileHeader>,
        buffer: Vec<u8>,
    ) -> Result<(), Status> {
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|err| {
                Status::internal(format!("tar shard semaphore closed unexpectedly: {}", err))
            })?;

        self.tasks.spawn(async move {
            let _permit = permit;
            tokio::task::spawn_blocking(move || apply_tar_shard_sync(module, headers, buffer))
                .await
                .map_err(|err| Status::internal(format!("tar shard worker panicked: {}", err)))?
        });

        Ok(())
    }

    fn drain_ready(&mut self, stats: &mut TransferStats) -> Result<(), Status> {
        while let Some(join_result) = self.tasks.try_join_next() {
            let shard_stats = convert_join_result(join_result)?;
            accumulate_transfer_stats(stats, &shard_stats);
        }
        Ok(())
    }

    async fn finish(mut self, stats: &mut TransferStats) -> Result<(), Status> {
        while self.tasks.len() > 0 {
            self.collect_next(stats).await?;
        }
        Ok(())
    }

    async fn collect_next(&mut self, stats: &mut TransferStats) -> Result<(), Status> {
        if let Some(join_result) = self.tasks.join_next().await {
            let shard_stats = convert_join_result(join_result)?;
            accumulate_transfer_stats(stats, &shard_stats);
        }
        Ok(())
    }
}

fn convert_join_result(
    join_result: Result<Result<TransferStats, Status>, tokio::task::JoinError>,
) -> Result<TransferStats, Status> {
    match join_result {
        Ok(Ok(stats)) => Ok(stats),
        Ok(Err(status)) => Err(status),
        Err(err) => Err(Status::internal(format!(
            "tar shard worker panicked: {}",
            err
        ))),
    }
}

fn accumulate_transfer_stats(target: &mut TransferStats, shard: &TransferStats) {
    target.files_transferred += shard.files_transferred;
    target.bytes_transferred += shard.bytes_transferred;
    target.bytes_zero_copy += shard.bytes_zero_copy;
}

pub(crate) async fn read_u32(stream: &mut TcpStream) -> Result<u32, Status> {
    let mut buf = [0u8; 4];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|err| Status::internal(format!("failed to read u32: {}", err)))?;
    Ok(u32::from_be_bytes(buf))
}

pub(crate) async fn read_u64(stream: &mut TcpStream) -> Result<u64, Status> {
    let mut buf = [0u8; 8];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|err| Status::internal(format!("failed to read u64: {}", err)))?;
    Ok(u64::from_be_bytes(buf))
}

pub(crate) async fn read_i64(stream: &mut TcpStream) -> Result<i64, Status> {
    let mut buf = [0u8; 8];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|err| Status::internal(format!("failed to read i64: {}", err)))?;
    Ok(i64::from_be_bytes(buf))
}

fn apply_tar_shard_sync(
    module: ModuleConfig,
    headers: Vec<FileHeader>,
    buffer: Vec<u8>,
) -> Result<TransferStats, Status> {
    let mut expected: HashMap<String, FileHeader> = headers
        .into_iter()
        .map(|header| (header.relative_path.clone(), header))
        .collect();

    let mut archive = Archive::new(Cursor::new(buffer));
    let mut stats = TransferStats::default();

    let entries = archive
        .entries()
        .map_err(|err| Status::internal(format!("tar shard entries: {}", err)))?;
    for entry_result in entries {
        let mut entry = entry_result
            .map_err(|err| Status::internal(format!("tar shard entry error: {}", err)))?;
        if entry.header().entry_type().is_dir() {
            continue;
        }

        let rel_path = entry
            .path()
            .map_err(|err| Status::internal(format!("tar shard path error: {}", err)))?;
        let rel_string = rel_path.to_string_lossy().replace('\\', "/");

        let header = expected.remove(&rel_string).ok_or_else(|| {
            Status::invalid_argument(format!(
                "tar shard produced unexpected entry '{}'",
                rel_string
            ))
        })?;

        let resolved = resolve_relative_path(&rel_string)?;
        let dest_path = module.path.join(&resolved);
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                Status::internal(format!("create dir {}: {}", parent.display(), err))
            })?;
        }

        entry
            .unpack(&dest_path)
            .map_err(|err| Status::internal(format!("unpack {}: {}", dest_path.display(), err)))?;

        stats.files_transferred += 1;
        stats.bytes_transferred += header.size;
    }

    if !expected.is_empty() {
        let missing: Vec<String> = expected.into_keys().collect();
        return Err(Status::internal(format!(
            "tar shard missing expected entries: {:?}",
            missing
        )));
    }

    Ok(stats)
}
