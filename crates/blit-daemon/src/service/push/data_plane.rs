use crate::runtime::ModuleConfig;
use blit_core::buffer::BufferPool;
use blit_core::generated::{
    client_push_request, server_push_response, ClientPushRequest, DataTransferNegotiation,
    FileHeader,
};
use blit_core::remote::transfer::tar_safety;
use rand::{rngs::SysRng, TryRng};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex as AsyncMutex, Semaphore};
use tokio::task::JoinSet;
use tonic::{Status, Streaming};

use super::super::util::resolve_manifest_relative_path;
use super::super::PushSender;
use super::control::send_control_message;

const TOKEN_LEN: usize = 32;
const MAX_PARALLEL_TAR_TASKS: usize = 4;

/// Default buffer size for pooled tar shard buffers (4 MiB).
const TAR_BUFFER_SIZE: usize = 4 * 1024 * 1024;
/// Maximum pooled buffers per connection stream.
const TAR_BUFFER_POOL_SIZE: usize = 8;

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
    SysRng.try_fill_bytes(&mut buf).expect("system RNG failed");
    buf
}

pub(crate) async fn accept_data_connection_stream(
    listener: TcpListener,
    expected_token: Vec<u8>,
    module: ModuleConfig,
    files: mpsc::Receiver<FileHeader>,
    stream_count: u32,
) -> Result<TransferStats, Status> {
    let start = Instant::now();
    let streams = stream_count.max(1) as usize;
    let files = Arc::new(AsyncMutex::new(files));
    let cache = Arc::new(AsyncMutex::new(HashMap::new()));
    let mut handles = Vec::with_capacity(streams);

    for idx in 0..streams {
        let (accepted, addr) = listener
            .accept()
            .await
            .map_err(|err| Status::internal(format!("data plane accept failed: {}", err)))?;
        // Enable nodelay + keepalive to prevent idle stream timeouts
        // during long transfers on other streams.
        let socket = {
            let std_sock = accepted
                .into_std()
                .map_err(|err| Status::internal(format!("converting socket: {err}")))?;
            let s2 = socket2::Socket::from(std_sock);
            let _ = s2.set_tcp_nodelay(true);
            let _ = s2.set_keepalive(true);
            let std_back: std::net::TcpStream = s2.into();
            TcpStream::from_std(std_back)
                .map_err(|err| Status::internal(format!("re-wrapping socket: {err}")))?
        };
        eprintln!("[data-plane] accepted connection {} from {}", idx, addr);
        let expected_token = expected_token.clone();
        let module_clone = module.clone();
        let files_clone = Arc::clone(&files);
        let cache_clone = Arc::clone(&cache);
        handles.push(tokio::spawn(async move {
            handle_data_plane_stream(
                socket,
                expected_token,
                module_clone,
                files_clone,
                cache_clone,
            )
            .await
        }));
    }

    let mut final_stats = TransferStats::default();
    for handle in handles {
        let stats = handle
            .await
            .map_err(|_| Status::internal("data plane worker cancelled"))??;
        accumulate_transfer_stats(&mut final_stats, &stats);
    }

    let elapsed = start.elapsed().as_secs_f64().max(1e-6);
    let gbps = (final_stats.bytes_transferred as f64 * 8.0) / elapsed / 1e9;
    eprintln!(
        "[data-plane] aggregate throughput {:.2} Gbps ({} bytes in {:.2}s)",
        gbps, final_stats.bytes_transferred, elapsed
    );

    Ok(final_stats)
}

async fn handle_data_plane_stream(
    mut socket: TcpStream,
    expected_token: Vec<u8>,
    module: ModuleConfig,
    files: Arc<AsyncMutex<mpsc::Receiver<FileHeader>>>,
    cache: Arc<AsyncMutex<HashMap<String, FileHeader>>>,
) -> Result<TransferStats, Status> {
    let start = Instant::now();
    let mut token_buf = vec![0u8; expected_token.len()];
    socket
        .read_exact(&mut token_buf)
        .await
        .map_err(|err| Status::internal(format!("failed to read data plane token: {}", err)))?;
    if token_buf != expected_token {
        eprintln!("[data-plane] invalid token");
        return Err(Status::permission_denied("invalid data plane token"));
    }
    eprintln!(
        "[data-plane] token accepted (module='{}', root={})",
        module.name,
        module.path.display()
    );

    // Drain the manifest channel concurrently so the gRPC control loop
    // doesn't back-pressure when the data plane no longer consumes per-
    // record headers (we get full headers off the wire now).
    let drain_handle = {
        let files = Arc::clone(&files);
        tokio::spawn(async move {
            let mut guard = files.lock().await;
            while guard.recv().await.is_some() {}
        })
    };
    let _ = cache; // headers come off the wire; cache no longer needed

    // Route the inbound wire through the unified receive pipeline:
    //   socket → execute_receive_pipeline → FsTransferSink → disk
    // Same call shape as the client's pull-receive side. Tar shards get
    // extracted inline by FsTransferSink (parallelism across streams
    // already comes from N concurrent invocations of this function).
    use blit_core::remote::transfer::pipeline::execute_receive_pipeline;
    use blit_core::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};

    let config = FsSinkConfig {
        preserve_times: true,
        dry_run: false,
        checksum: None,
        resume: false,
    };
    let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
        PathBuf::new(),
        module.path.clone(),
        config,
    ));
    let outcome = execute_receive_pipeline(&mut socket, sink, None)
        .await
        .map_err(|err| Status::internal(format!("data plane receive: {err:#}")))?;

    drain_handle.abort();

    let stats = TransferStats {
        files_transferred: outcome.files_written as u64,
        bytes_transferred: outcome.bytes_written,
        bytes_zero_copy: 0,
    };

    let elapsed = start.elapsed().as_secs_f64().max(1e-6);
    let gbps = (stats.bytes_transferred as f64 * 8.0) / elapsed / 1e9;
    eprintln!(
        "[data-plane] stream complete: files={}, bytes={} ({:.2} Gbps)",
        stats.files_transferred, stats.bytes_transferred, gbps
    );
    Ok(stats)
}

/// Validate `TarShardHeader.archive_size` at the wire boundary so a
/// hostile or buggy push client can't grow the daemon's accumulating
/// buffer past the local cap (R8-F1). Extracted for direct unit
/// testing — the receive loop calls it inline.
fn validate_fallback_shard_archive_size(archive_size: u64) -> Result<(), Status> {
    if archive_size == 0 {
        return Err(Status::invalid_argument(
            "tar shard with files must declare a non-zero archive_size",
        ));
    }
    if archive_size > tar_safety::MAX_TAR_SHARD_BYTES {
        return Err(Status::invalid_argument(format!(
            "tar shard archive_size {} exceeds local cap {} bytes",
            archive_size,
            tar_safety::MAX_TAR_SHARD_BYTES
        )));
    }
    Ok(())
}

/// Per-chunk overflow check for the daemon push fallback. Returns
/// the new running total on success; rejects when the chunk would
/// exceed either the client-declared shard size or the local cap
/// (R8-F1).
fn check_fallback_chunk_overflow(
    received: u64,
    chunk_len: u64,
    declared: u64,
) -> Result<u64, Status> {
    let new_total = received
        .checked_add(chunk_len)
        .ok_or_else(|| Status::invalid_argument("tar shard chunk size overflows u64"))?;
    if new_total > declared {
        return Err(Status::invalid_argument(format!(
            "tar shard chunk exceeds declared size ({} > {})",
            new_total, declared
        )));
    }
    if new_total > tar_safety::MAX_TAR_SHARD_BYTES {
        return Err(Status::invalid_argument(format!(
            "tar shard buffer would exceed local cap of {} bytes",
            tar_safety::MAX_TAR_SHARD_BYTES
        )));
    }
    Ok(new_total)
}

pub(crate) async fn receive_fallback_data<S>(
    stream: &mut S,
    module: &ModuleConfig,
    files_requested: Vec<FileHeader>,
) -> Result<TransferStats, Status>
where
    S: tokio_stream::Stream<Item = Result<ClientPushRequest, Status>> + Unpin,
{
    use tokio_stream::StreamExt;
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
    // R8-F2: stream EOF without explicit UploadComplete is a wire
    // protocol error, not a graceful end. FileManifest /
    // TarShardHeader remove entries from `pending` before bytes
    // arrive, so a client that sends a header then closes the stream
    // would otherwise pass the `pending.is_empty()` check despite
    // never delivering the data.
    let mut upload_complete_seen = false;

    while let Some(req) = stream.next().await.transpose()? {
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

                let rel_path = resolve_manifest_relative_path(&expected.relative_path)?;
                // F2: containment check before any directory create
                // or file open. A push client could otherwise place
                // a symlink earlier (in a previous transfer) and
                // then write through it on a later push.
                let dest_path = super::super::util::resolve_contained_path(module, &rel_path)?;
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

                // R8-F1: bound the shard buffer at the wire boundary
                // so a client that lies in `archive_size` (zero or
                // huge) can't grow our memory uncapped.
                validate_fallback_shard_archive_size(shard.archive_size)?;

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

                // Modest initial reservation regardless of advertised
                // archive_size; chunks grow the buffer up to the
                // already-bounded declared size.
                let capacity = (shard.archive_size as usize).min(1024 * 1024);
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
                    // R8-F1: enforce client-declared size and local
                    // cap on every chunk. archive_size is already
                    // known non-zero (rejected at header).
                    let new_total = check_fallback_chunk_overflow(
                        *received,
                        chunk.content.len() as u64,
                        *expected_size,
                    )?;
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
                    // archive_size==0 is rejected at TarShardHeader so
                    // expected_size is always meaningful here.
                    if expected_size != received {
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
                upload_complete_seen = true;
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

    // R8-F2: stream EOF without explicit UploadComplete is a wire
    // protocol error. FileManifest / TarShardHeader remove entries
    // from `pending` before bytes arrive, so a client that sends a
    // header then closes the stream without the data would otherwise
    // pass the `pending.is_empty()` check.
    if active.is_some() {
        return Err(Status::invalid_argument(
            "fallback stream ended with an in-flight file or tar shard",
        ));
    }
    if !upload_complete_seen {
        return Err(Status::invalid_argument(
            "fallback stream ended without UploadComplete",
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
            stream_count: 0,
        }),
    )
    .await?;

    let stats = receive_fallback_data(stream, module, files_requested).await?;

    Ok(stats)
}

struct TarShardExecutor {
    semaphore: Arc<Semaphore>,
    tasks: JoinSet<Result<(TransferStats, Option<Vec<u8>>), Status>>,
    buffer_pool: Arc<BufferPool>,
}

impl TarShardExecutor {
    fn new(max_parallel: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_parallel)),
            tasks: JoinSet::new(),
            buffer_pool: Arc::new(BufferPool::new(TAR_BUFFER_SIZE, TAR_BUFFER_POOL_SIZE, None)),
        }
    }

    /// Acquire a buffer for receiving tar data.
    /// Uses pooled buffer if size fits, otherwise allocates on demand.
    /// Currently only used by the gRPC fallback path.
    #[allow(dead_code)]
    async fn acquire_buffer(&self, size: usize) -> Vec<u8> {
        if size <= self.buffer_pool.buffer_size() {
            // Use pooled buffer - acquire and take ownership
            let pool_buf = self.buffer_pool.acquire().await;
            let mut buf = pool_buf.take();
            buf.resize(size, 0);
            buf
        } else {
            // Too large for pool - allocate directly
            vec![0u8; size]
        }
    }

    /// Return a buffer to the pool if it fits.
    fn return_buffer(&self, buffer: Vec<u8>) {
        self.buffer_pool.record_bytes(buffer.len() as u64);
        self.buffer_pool.return_vec(buffer);
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

        let pool_buffer_size = self.buffer_pool.buffer_size();
        self.tasks.spawn(async move {
            let _permit = permit;
            let result = tokio::task::spawn_blocking(move || {
                apply_tar_shard_sync(module, headers, buffer, pool_buffer_size)
            })
            .await
            .map_err(|err| Status::internal(format!("tar shard worker panicked: {}", err)))??;
            Ok(result)
        });

        Ok(())
    }

    fn drain_ready(&mut self, stats: &mut TransferStats) -> Result<(), Status> {
        while let Some(join_result) = self.tasks.try_join_next() {
            let (shard_stats, returned_buffer) = convert_join_result(join_result)?;
            accumulate_transfer_stats(stats, &shard_stats);
            if let Some(buf) = returned_buffer {
                self.return_buffer(buf);
            }
        }
        Ok(())
    }

    async fn finish(mut self, stats: &mut TransferStats) -> Result<(), Status> {
        while !self.tasks.is_empty() {
            self.collect_next(stats).await?;
        }
        // Log pool stats at end of transfer
        let pool_stats = self.buffer_pool.stats();
        if pool_stats.total_allocated > 0 {
            eprintln!(
                "[data-plane] buffer pool: {} allocated, {} cached, {} bytes through",
                pool_stats.total_allocated, pool_stats.cached, pool_stats.bytes_through
            );
        }
        Ok(())
    }

    async fn collect_next(&mut self, stats: &mut TransferStats) -> Result<(), Status> {
        if let Some(join_result) = self.tasks.join_next().await {
            let (shard_stats, returned_buffer) = convert_join_result(join_result)?;
            accumulate_transfer_stats(stats, &shard_stats);
            if let Some(buf) = returned_buffer {
                self.return_buffer(buf);
            }
        }
        Ok(())
    }
}

fn convert_join_result(
    join_result: Result<Result<(TransferStats, Option<Vec<u8>>), Status>, tokio::task::JoinError>,
) -> Result<(TransferStats, Option<Vec<u8>>), Status> {
    match join_result {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(status)) => {
            eprintln!(
                "[data-plane] tar shard worker returned error: {}",
                status.message()
            );
            Err(status)
        }
        Err(err) => {
            eprintln!("[data-plane] tar shard worker panicked: {}", err);
            Err(Status::internal(format!(
                "tar shard worker panicked: {}",
                err
            )))
        }
    }
}

fn accumulate_transfer_stats(target: &mut TransferStats, shard: &TransferStats) {
    target.files_transferred += shard.files_transferred;
    target.bytes_transferred += shard.bytes_transferred;
    target.bytes_zero_copy += shard.bytes_zero_copy;
}

/// Process a tar shard and return stats plus the buffer for potential reuse.
/// The buffer is returned if its capacity matches the pool size.
///
/// Routes through `blit_core::remote::transfer::tar_safety` so the
/// receive policy here matches the pull-receive sites bit-for-bit.
/// Critically, this closes the latent High-severity equivalent of
/// R5-F2 on the push direction: the previous `Entry::unpack` call
/// honored tar symlink/hardlink/device entries, letting an
/// authenticated push client place a symlink at a benign-looking
/// path that subsequent writes would follow outside the module root.
/// The shared helper rejects any non-regular entry up front.
fn apply_tar_shard_sync(
    module: ModuleConfig,
    headers: Vec<FileHeader>,
    buffer: Vec<u8>,
    pool_buffer_size: usize,
) -> Result<(TransferStats, Option<Vec<u8>>), Status> {
    use blit_core::remote::transfer::tar_safety::{
        safe_extract_tar_shard, write_extracted_file, TarShardExtractOptions,
    };

    let buffer_capacity = buffer.capacity();
    let opts = TarShardExtractOptions::default();
    let extracted = safe_extract_tar_shard(&buffer, headers, &module.path, &opts)
        .map_err(|err| Status::internal(format!("tar shard validation: {err:#}")))?;

    let mut stats = TransferStats::default();
    for file in &extracted {
        // F2: containment check on every entry's destination before
        // writing. The tar_safety helper only does lexical safe_join;
        // an existing symlink at a parent component (placed by a
        // previous push) would otherwise have create_dir_all/write
        // follow it outside the module root. R5-F2 already rejects
        // tar entries whose tar header type is Symlink/Hardlink, so
        // this defends against pre-existing symlinks on disk, not
        // tar-encoded ones. Check against canonical_root, not path,
        // because path may be munged with a destination subpath.
        blit_core::path_safety::verify_contained(&module.canonical_root, &file.dest_path)
            .map_err(|err| Status::permission_denied(format!("path containment: {err:#}")))?;
        write_extracted_file(file)
            .map_err(|err| Status::internal(format!("applying tar shard entry: {err:#}")))?;
        stats.files_transferred += 1;
        stats.bytes_transferred += file.size;
    }

    // Only return buffer for pooling if it matches pool size. We
    // never moved ownership into an Archive, so the buffer is intact.
    let return_buffer = if buffer_capacity >= pool_buffer_size {
        Some(buffer)
    } else {
        None
    };

    Ok((stats, return_buffer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tar::{Builder, EntryType, Header};
    use tempfile::tempdir;

    #[test]
    fn apply_tar_shard_handles_long_paths() {
        let source_root = tempdir().expect("source tempdir");
        let dest_root = tempdir().expect("dest tempdir");

        let rel = long_relative_path();
        let content = b"tar shard payload for very long path";

        let absolute_source = source_root.path().join(Path::new(&rel));
        if let Some(parent) = absolute_source.parent() {
            std::fs::create_dir_all(parent).expect("create parent dirs");
        }
        std::fs::write(&absolute_source, content).expect("write source file");

        let header = FileHeader {
            relative_path: rel.clone(),
            size: content.len() as u64,
            mtime_seconds: 0,
            permissions: 0o644,
            checksum: vec![],
        };

        let canonical = std::fs::canonicalize(dest_root.path()).expect("canonicalize tempdir");
        let module = ModuleConfig {
            name: "test".into(),
            path: canonical.clone(),
            canonical_root: canonical,
            read_only: false,
            _comment: None,
        };

        let tar_data = build_tar_bytes(source_root.path(), &header);
        let (stats, _returned_buf) = apply_tar_shard_sync(
            module.clone(),
            vec![header.clone()],
            tar_data,
            TAR_BUFFER_SIZE,
        )
        .expect("tar shard applies");

        assert_eq!(stats.files_transferred, 1);
        assert_eq!(stats.bytes_transferred, content.len() as u64);

        let dest_path = dest_root.path().join(Path::new(&rel));
        let written = std::fs::read(dest_path).expect("read written file");
        assert_eq!(written, content);
    }

    fn build_tar_bytes(root: &Path, header: &FileHeader) -> Vec<u8> {
        let mut builder = Builder::new(Vec::new());
        let rel_path = Path::new(&header.relative_path);
        let mut file =
            std::fs::File::open(root.join(rel_path)).expect("open source file for tar shard");

        let mut tar_header = Header::new_gnu();
        tar_header.set_entry_type(EntryType::Regular);
        let mode = if header.permissions == 0 {
            0o644
        } else {
            header.permissions
        };
        tar_header.set_mode(mode);
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
            .append_data(&mut tar_header, rel_path, &mut file)
            .expect("append tar entry");
        builder.finish().expect("finish tar shard");
        builder.into_inner().expect("tar buffer")
    }

    fn long_relative_path() -> String {
        let mut segments = Vec::new();
        for idx in 0..10 {
            segments.push(format!("segment_{:02}_{}", idx, "x".repeat(24)));
        }
        format!("{}/{}", segments.join("/"), "deep_file.txt")
    }

    /// Latent High-severity bug pre-Round-7 cleanup: an authenticated
    /// push client could ship a tar shard with a Symlink entry whose
    /// path was inside the module root but whose target pointed
    /// outside (e.g. `module/config.txt -> ../../etc/passwd`).
    /// `Entry::unpack` would have created the symlink, and a later
    /// push to `config.txt` would have written through it.
    /// `apply_tar_shard_sync` now routes through
    /// `tar_safety::safe_extract_tar_shard`, which rejects every
    /// non-regular entry type up front.
    #[test]
    fn apply_tar_shard_rejects_symlink_entry() {
        let dest_root = tempdir().expect("dest tempdir");
        let canonical = std::fs::canonicalize(dest_root.path()).expect("canonicalize tempdir");
        let module = ModuleConfig {
            name: "test".into(),
            path: canonical.clone(),
            canonical_root: canonical,
            read_only: false,
            _comment: None,
        };

        // Hand-build a tar with a single symlink entry.
        let mut builder = Builder::new(Vec::new());
        let mut h = Header::new_gnu();
        h.set_entry_type(EntryType::Symlink);
        h.set_size(0);
        h.set_mode(0o777);
        builder
            .append_link(&mut h, "config.txt", "/etc/passwd")
            .expect("append link");
        let tar_data = builder.into_inner().expect("tar buffer");

        let header = FileHeader {
            relative_path: "config.txt".into(),
            size: 0,
            mtime_seconds: 0,
            permissions: 0o644,
            checksum: vec![],
        };

        let err = apply_tar_shard_sync(module, vec![header], tar_data, TAR_BUFFER_SIZE)
            .expect_err("symlink entry must be rejected");
        let msg = err.message();
        assert!(
            msg.contains("non-regular entry"),
            "expected non-regular rejection, got: {msg}"
        );
        // No symlink should have been created at the would-be target.
        assert!(!dest_root.path().join("config.txt").exists());
    }

    // R8-F1 framing tests for the daemon push gRPC fallback receive
    // loop. These exercise the validation primitives directly so the
    // rules (no zero archive_size when files present, cap on
    // archive_size, per-chunk overflow against declared + cap) are
    // pinned without spinning up a real gRPC server.

    #[test]
    fn fallback_rejects_zero_archive_size() {
        let err = validate_fallback_shard_archive_size(0).unwrap_err();
        assert!(err.message().contains("non-zero archive_size"));
    }

    #[test]
    fn fallback_rejects_archive_size_above_cap() {
        let err =
            validate_fallback_shard_archive_size(tar_safety::MAX_TAR_SHARD_BYTES + 1).unwrap_err();
        assert!(err.message().contains("exceeds local cap"));
    }

    #[test]
    fn fallback_accepts_archive_size_at_cap() {
        validate_fallback_shard_archive_size(tar_safety::MAX_TAR_SHARD_BYTES)
            .expect("at-cap archive_size is allowed");
        validate_fallback_shard_archive_size(1).expect("1-byte archive_size is allowed");
    }

    #[test]
    fn fallback_chunk_overflow_rejects_above_declared() {
        // declared=10, already received 8, chunk would push us to 12.
        let err = check_fallback_chunk_overflow(8, 4, 10).unwrap_err();
        assert!(err.message().contains("exceeds declared size"));
    }

    #[test]
    fn fallback_chunk_overflow_rejects_above_local_cap() {
        // Declared is huge (u64::MAX) so the "exceeds declared"
        // branch never fires; the cap check is the load-bearing
        // line of defense.
        let near_cap = tar_safety::MAX_TAR_SHARD_BYTES - 100;
        let err = check_fallback_chunk_overflow(near_cap, 200, u64::MAX).unwrap_err();
        assert!(
            err.message().contains("local cap"),
            "expected cap rejection, got: {}",
            err.message()
        );
    }

    #[test]
    fn fallback_chunk_overflow_rejects_u64_overflow() {
        let err = check_fallback_chunk_overflow(u64::MAX - 1, 10, u64::MAX).unwrap_err();
        assert!(err.message().contains("overflows u64"));
    }

    #[test]
    fn fallback_chunk_overflow_accepts_within_bounds() {
        // declared 1024, received 100, chunk 200 → new_total 300.
        assert_eq!(check_fallback_chunk_overflow(100, 200, 1024).unwrap(), 300);
        // exact boundary: chunk lands at declared.
        assert_eq!(check_fallback_chunk_overflow(900, 124, 1024).unwrap(), 1024);
    }

    // R9-F1 regression tests: drive `receive_fallback_data` over a
    // synthetic message stream so the EOF-without-UploadComplete
    // rejection is exercised directly, not just by code review.
    // The function is now generic over `tokio_stream::Stream` so
    // we feed it an `iter` of pre-built messages.

    fn module_for_test(path: PathBuf) -> ModuleConfig {
        let canonical = std::fs::canonicalize(&path).unwrap_or(path);
        ModuleConfig {
            name: "test".into(),
            canonical_root: canonical.clone(),
            path: canonical,
            read_only: false,
            _comment: None,
        }
    }

    #[tokio::test]
    async fn fallback_rejects_eof_after_file_manifest() {
        // Client sends a FileManifest claiming 100 bytes, then closes
        // the stream without sending FileData or UploadComplete. Pre
        // R8-F2 this would have returned success because the manifest
        // already removed `expected.txt` from `pending`.
        let dest_root = tempdir().expect("tempdir");
        let module = module_for_test(dest_root.path().to_path_buf());
        let header = FileHeader {
            relative_path: "expected.txt".into(),
            size: 100,
            mtime_seconds: 0,
            permissions: 0o644,
            checksum: vec![],
        };
        let messages: Vec<Result<ClientPushRequest, Status>> = vec![Ok(ClientPushRequest {
            payload: Some(client_push_request::Payload::FileManifest(header.clone())),
        })];
        let mut stream = tokio_stream::iter(messages);
        let err = receive_fallback_data(&mut stream, &module, vec![header])
            .await
            .expect_err("EOF after FileManifest must be rejected");
        let msg = err.message();
        assert!(
            msg.contains("UploadComplete") || msg.contains("in-flight"),
            "expected UploadComplete/in-flight error, got: {msg}"
        );
    }

    #[tokio::test]
    async fn fallback_rejects_eof_after_tar_shard_header() {
        // Client sends TarShardHeader, then closes the stream without
        // any chunks or TarShardComplete or UploadComplete.
        let dest_root = tempdir().expect("tempdir");
        let module = module_for_test(dest_root.path().to_path_buf());
        let file = FileHeader {
            relative_path: "small.txt".into(),
            size: 4,
            mtime_seconds: 0,
            permissions: 0o644,
            checksum: vec![],
        };
        let shard = blit_core::generated::TarShardHeader {
            files: vec![file.clone()],
            archive_size: 1024,
        };
        let messages: Vec<Result<ClientPushRequest, Status>> = vec![Ok(ClientPushRequest {
            payload: Some(client_push_request::Payload::TarShardHeader(shard)),
        })];
        let mut stream = tokio_stream::iter(messages);
        let err = receive_fallback_data(&mut stream, &module, vec![file])
            .await
            .expect_err("EOF after TarShardHeader must be rejected");
        let msg = err.message();
        assert!(
            msg.contains("UploadComplete") || msg.contains("in-flight"),
            "expected UploadComplete/in-flight error, got: {msg}"
        );
    }

    #[tokio::test]
    async fn fallback_rejects_eof_with_empty_pending_no_complete() {
        // Even with no expected files and a clean state, a stream
        // that never carries an UploadComplete must still be
        // rejected — we treat the explicit terminator as required.
        let dest_root = tempdir().expect("tempdir");
        let module = module_for_test(dest_root.path().to_path_buf());
        let messages: Vec<Result<ClientPushRequest, Status>> = vec![];
        let mut stream = tokio_stream::iter(messages);
        let err = receive_fallback_data(&mut stream, &module, vec![])
            .await
            .expect_err("EOF without UploadComplete must be rejected");
        assert!(err.message().contains("UploadComplete"));
    }
}
