use super::PullPayload;
use crate::runtime::ModuleConfig;
use crate::service::PullSender;
use base64::{engine::general_purpose, Engine as _};
use blit_core::buffer::BufferPool;
use blit_core::generated::{
    DataTransferNegotiation, FileData, FileHeader, ManifestBatch, PullChunk, PullSummary,
};
use blit_core::remote::transfer::pipeline::{
    execute_sink_pipeline, execute_sink_pipeline_streaming,
};
use blit_core::remote::transfer::sink::{DataPlaneSink, TransferSink};
use blit_core::remote::transfer::source::FsTransferSource;
use blit_core::remote::transfer::{plan_transfer_payloads, TransferPayload};
use blit_core::remote::tuning::determine_remote_tuning;
use blit_core::transfer_plan::PlanOptions;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tonic::Status;

use super::push::{bind_data_plane_listener, generate_token, TransferStats};
use super::util::{
    metadata_mtime_seconds, normalize_relative_path, permissions_mode, resolve_relative_path,
};

/// Batch size for streaming enumeration (number of files per batch).
const ENUM_BATCH_SIZE: usize = 500;

/// Minimum bytes before starting data plane (allows better tuning estimate).
const MIN_BYTES_FOR_TUNING: u64 = 16 * 1024 * 1024;

pub(crate) async fn stream_pull(
    module: ModuleConfig,
    requested_path: String,
    force_grpc: bool,
    metadata_only: bool,
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

    // For single files, use non-streaming path
    if root.is_file() {
        let entries = collect_pull_entries(&module.path, &root, &requested).await?;
        if entries.is_empty() {
            send_summary(&tx, TransferStats::default(), force_grpc).await?;
            return Ok(());
        }
        let total_bytes: u64 = entries.iter().map(|e| e.header.size).sum();
        send_manifest_batch(&tx, entries.len() as u64, total_bytes).await?;
        return stream_pull_non_streaming(
            module,
            root,
            entries,
            total_bytes,
            force_grpc,
            metadata_only,
            tx,
        )
        .await;
    }

    // For gRPC fallback or metadata-only, use non-streaming path
    if force_grpc || metadata_only {
        let entries = collect_pull_entries(&module.path, &root, &requested).await?;
        if entries.is_empty() {
            send_summary(&tx, TransferStats::default(), true).await?;
            return Ok(());
        }
        let total_bytes: u64 = entries.iter().map(|e| e.header.size).sum();
        send_manifest_batch(&tx, entries.len() as u64, total_bytes).await?;
        stream_via_grpc(&module, &entries, &tx, metadata_only).await?;
        send_summary(
            &tx,
            TransferStats {
                files_transferred: entries.len() as u64,
                bytes_transferred: total_bytes,
                bytes_zero_copy: 0,
            },
            true,
        )
        .await?;
        return Ok(());
    }

    // Streaming enumeration with parallel data plane transfer
    stream_pull_streaming(module, root, requested, tx).await
}

/// Non-streaming pull for single files or when data plane is not used.
///
/// `source_root` is the absolute path being enumerated (module.path joined
/// with the requested subpath) — this is what header.relative_path is
/// relative to, and what the daemon's FsTransferSource uses to locate
/// source files for reading.
async fn stream_pull_non_streaming(
    module: ModuleConfig,
    source_root: PathBuf,
    entries: Vec<PullEntry>,
    total_bytes: u64,
    force_grpc: bool,
    metadata_only: bool,
    tx: PullSender,
) -> Result<(), Status> {
    if force_grpc || metadata_only {
        stream_via_grpc(&module, &entries, &tx, metadata_only).await?;
        send_summary(
            &tx,
            TransferStats {
                files_transferred: entries.len() as u64,
                bytes_transferred: total_bytes,
                bytes_zero_copy: 0,
            },
            true,
        )
        .await?;
        return Ok(());
    }

    let tuning = determine_remote_tuning(total_bytes);
    let plan_options = PlanOptions {
        chunk_bytes_override: Some(tuning.chunk_bytes),
        ..Default::default()
    };

    let headers: Vec<FileHeader> = entries.iter().map(|e| e.header.clone()).collect();
    // Plan against the enumeration root so header.relative_path (which is
    // relative to the enumeration root, not the module root) resolves correctly.
    let planned = plan_transfer_payloads(headers, &source_root, plan_options)
        .map_err(|err| Status::internal(format!("failed to plan pull payloads: {}", err)))?;

    if planned.payloads.is_empty() {
        send_summary(&tx, TransferStats::default(), false).await?;
        return Ok(());
    }

    let listener = bind_data_plane_listener()
        .await
        .map_err(|err| Status::internal(format!("failed to bind data plane: {}", err)))?;
    let port = listener
        .local_addr()
        .map_err(|err| Status::internal(format!("querying listener addr: {}", err)))?
        .port();
    let token = generate_token();
    let token_string = general_purpose::STANDARD_NO_PAD.encode(&token);
    let stream_target = pull_stream_count(total_bytes, tuning.max_streams as usize);

    tx.send(Ok(PullChunk {
        payload: Some(PullPayload::Negotiation(DataTransferNegotiation {
            tcp_port: port as u32,
            one_time_token: token_string,
            tcp_fallback: false,
            stream_count: stream_target,
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send pull negotiation"))?;

    let transfer_task = tokio::spawn(accept_pull_data_connection(
        listener,
        token,
        source_root,
        planned.payloads,
        tuning.chunk_bytes,
        tuning.max_streams,
        stream_target,
    ));

    transfer_task
        .await
        .map_err(|err| Status::internal(format!("pull data-plane task failed: {}", err)))??;

    send_summary(
        &tx,
        TransferStats {
            files_transferred: entries.len() as u64,
            bytes_transferred: total_bytes,
            bytes_zero_copy: 0,
        },
        false,
    )
    .await?;
    Ok(())
}

/// Streaming pull with parallel enumeration and data plane transfer.
async fn stream_pull_streaming(
    module: ModuleConfig,
    root: PathBuf,
    requested: PathBuf,
    tx: PullSender,
) -> Result<(), Status> {
    // Channel for streaming enumeration batches
    let (entry_tx, mut entry_rx) = mpsc::channel::<Vec<PullEntry>>(4);

    // Start enumeration in background
    let module_path = module.path.clone();
    let root_clone = root.clone();
    let requested_clone = requested.clone();
    let enum_handle = tokio::task::spawn_blocking(move || {
        enumerate_to_channel(
            module_path,
            root_clone,
            requested_clone,
            entry_tx,
            ENUM_BATCH_SIZE,
        )
    });

    // Collect first batch(es) to estimate size for tuning
    let mut pending_entries: Vec<PullEntry> = Vec::new();
    let mut pending_bytes = 0u64;
    let mut enumeration_done = false;

    while pending_bytes < MIN_BYTES_FOR_TUNING {
        match entry_rx.recv().await {
            Some(batch) => {
                let batch_bytes: u64 = batch.iter().map(|e| e.header.size).sum();
                let batch_count = batch.len() as u64;
                pending_bytes += batch_bytes;
                pending_entries.extend(batch);
                // Send ManifestBatch for this batch
                send_manifest_batch(&tx, batch_count, batch_bytes).await?;
            }
            None => {
                enumeration_done = true;
                break;
            }
        }
    }

    if pending_entries.is_empty() {
        // Wait for enumeration to complete and check for errors
        let _ = enum_handle.await;
        send_summary(&tx, TransferStats::default(), false).await?;
        return Ok(());
    }

    // Determine tuning based on accumulated bytes
    let tuning = determine_remote_tuning(pending_bytes);
    let plan_options = PlanOptions {
        chunk_bytes_override: Some(tuning.chunk_bytes),
        ..Default::default()
    };

    // Set up data plane
    let listener = bind_data_plane_listener()
        .await
        .map_err(|err| Status::internal(format!("failed to bind data plane: {}", err)))?;
    let port = listener
        .local_addr()
        .map_err(|err| Status::internal(format!("querying listener addr: {}", err)))?
        .port();
    let token = generate_token();
    let token_string = general_purpose::STANDARD_NO_PAD.encode(&token);
    let stream_target = pull_stream_count(pending_bytes, tuning.max_streams as usize);

    // Send negotiation
    tx.send(Ok(PullChunk {
        payload: Some(PullPayload::Negotiation(DataTransferNegotiation {
            tcp_port: port as u32,
            one_time_token: token_string,
            tcp_fallback: false,
            stream_count: stream_target,
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send pull negotiation"))?;

    // Channel for payloads to data plane
    let (payload_tx, payload_rx) = mpsc::channel::<Vec<TransferPayload>>(4);

    // Start streaming data plane. Source root = enumeration root (`root`),
    // NOT module.path — header.relative_path is relative to `root`.
    let source_root = root.clone();
    let data_plane_handle = tokio::spawn(accept_pull_data_connection_streaming(
        listener,
        token,
        source_root.clone(),
        payload_rx,
        tuning.chunk_bytes,
        tuning.max_streams,
        stream_target,
    ));

    // Plan and queue pending entries
    let headers: Vec<FileHeader> = pending_entries.iter().map(|e| e.header.clone()).collect();
    let planned = plan_transfer_payloads(headers, &source_root, plan_options)
        .map_err(|err| Status::internal(format!("failed to plan pull payloads: {}", err)))?;
    if !planned.payloads.is_empty() {
        payload_tx
            .send(planned.payloads)
            .await
            .map_err(|_| Status::internal("data plane died"))?;
    }

    // Continue receiving batches if enumeration not done
    if !enumeration_done {
        while let Some(batch) = entry_rx.recv().await {
            let batch_bytes: u64 = batch.iter().map(|e| e.header.size).sum();
            let batch_count = batch.len() as u64;

            // Send ManifestBatch for this batch
            send_manifest_batch(&tx, batch_count, batch_bytes).await?;

            // Plan and queue
            let headers: Vec<FileHeader> = batch.iter().map(|e| e.header.clone()).collect();
            let planned =
                plan_transfer_payloads(headers, &source_root, plan_options).map_err(|err| {
                    Status::internal(format!("failed to plan pull payloads: {}", err))
                })?;
            if !planned.payloads.is_empty() {
                payload_tx
                    .send(planned.payloads)
                    .await
                    .map_err(|_| Status::internal("data plane died"))?;
            }
        }
    }

    // Close payload channel to signal completion
    drop(payload_tx);

    // Wait for data plane to complete
    let stats = data_plane_handle
        .await
        .map_err(|err| Status::internal(format!("data plane task panicked: {}", err)))??;

    // Wait for enumeration to complete (should already be done)
    let _ = enum_handle.await;

    send_summary(&tx, stats, false).await?;
    Ok(())
}

pub(crate) struct PullEntry {
    pub(crate) header: FileHeader,
    pub(crate) relative_path: PathBuf,
}

impl PullEntry {
    fn absolute_path(&self, module_root: &Path) -> PathBuf {
        module_root.join(&self.relative_path)
    }
}

pub(crate) async fn collect_pull_entries(
    module_root: &Path,
    root: &Path,
    requested: &Path,
) -> Result<Vec<PullEntry>, Status> {
    collect_pull_entries_with_checksums(module_root, root, requested, false).await
}

pub(crate) async fn collect_pull_entries_with_checksums(
    module_root: &Path,
    root: &Path,
    requested: &Path,
    compute_checksums: bool,
) -> Result<Vec<PullEntry>, Status> {
    if root.is_file() {
        // Single-file root: physical path (for reads) is the requested path
        // from the module; wire path (in the header, for the client's
        // dest_root.join) must be empty so the client writes to its
        // already-resolved dest target — not nested under a basename it
        // already appended.
        let physical = if requested == Path::new(".") {
            root.file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
        } else {
            requested.to_path_buf()
        };
        let mut header = build_file_header(module_root, &physical, compute_checksums)?;
        header.relative_path = String::new();
        return Ok(vec![PullEntry {
            header,
            relative_path: physical,
        }]);
    }

    if !root.is_dir() {
        return Err(Status::invalid_argument("unsupported path type for pull"));
    }

    let root_clone = root.to_path_buf();
    let requested_clone = requested.to_path_buf();
    let module_root = module_root.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<Vec<PullEntry>, Status> {
        use rayon::prelude::*;

        let enumerator =
            blit_core::enumeration::FileEnumerator::new(blit_core::fs_enum::FileFilter::default());
        let entries = enumerator
            .enumerate_local(&root_clone)
            .map_err(|err| Status::internal(format!("enumeration error: {}", err)))?;

        // Filter to files only
        let file_entries: Vec<_> = entries
            .into_iter()
            .filter(|e| matches!(e.kind, blit_core::enumeration::EntryKind::File { .. }))
            .collect();

        // Physical path (relative to module_root, used for reading) = requested + entry
        // Wire path (in header.relative_path, used by client for dest_root.join) = entry only
        // Previously both were set to the joined form, causing the client to double-nest
        // when the CLI resolver had already appended the basename to dest_root.
        let files: Result<Vec<PullEntry>, Status> = if compute_checksums {
            file_entries
                .into_par_iter()
                .map(|entry| {
                    let physical = requested_clone.join(&entry.relative_path);
                    let wire = entry.relative_path.clone();
                    let mut header = build_file_header(&module_root, &physical, true)?;
                    header.relative_path = wire.to_string_lossy().replace('\\', "/");
                    Ok(PullEntry {
                        header,
                        relative_path: physical,
                    })
                })
                .collect()
        } else {
            file_entries
                .into_iter()
                .map(|entry| {
                    let physical = requested_clone.join(&entry.relative_path);
                    let wire = entry.relative_path.clone();
                    let mut header = build_file_header(&module_root, &physical, false)?;
                    header.relative_path = wire.to_string_lossy().replace('\\', "/");
                    Ok(PullEntry {
                        header,
                        relative_path: physical,
                    })
                })
                .collect()
        };

        files
    })
    .await
    .map_err(|err| Status::internal(format!("enumeration task failed: {}", err)))?
}

fn build_file_header(
    base: &Path,
    relative: &Path,
    compute_checksum: bool,
) -> Result<FileHeader, Status> {
    use std::io::Read;

    let abs_path = base.join(relative);

    if compute_checksum {
        // Open file once for both metadata and hashing
        let file = std::fs::File::open(&abs_path)
            .map_err(|err| Status::internal(format!("open {}: {}", abs_path.display(), err)))?;
        let metadata = file
            .metadata()
            .map_err(|err| Status::internal(format!("stat {}: {}", abs_path.display(), err)))?;

        // Compute Blake3 hash using the already-open file
        let mut hasher = blake3::Hasher::new();
        let mut reader = std::io::BufReader::new(file);
        let mut buf = [0u8; 256 * 1024];
        loop {
            let n = reader
                .read(&mut buf)
                .map_err(|err| Status::internal(format!("read {}: {}", abs_path.display(), err)))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let checksum = hasher.finalize().as_bytes().to_vec();

        Ok(FileHeader {
            relative_path: normalize_relative_path(relative),
            size: metadata.len(),
            mtime_seconds: metadata_mtime_seconds(&metadata).unwrap_or(0),
            permissions: permissions_mode(&metadata),
            checksum,
        })
    } else {
        // Just get metadata, no checksum needed
        let metadata = std::fs::metadata(&abs_path)
            .map_err(|err| Status::internal(format!("stat {}: {}", abs_path.display(), err)))?;

        Ok(FileHeader {
            relative_path: normalize_relative_path(relative),
            size: metadata.len(),
            mtime_seconds: metadata_mtime_seconds(&metadata).unwrap_or(0),
            permissions: permissions_mode(&metadata),
            checksum: vec![],
        })
    }
}

async fn stream_via_grpc(
    module: &ModuleConfig,
    entries: &[PullEntry],
    tx: &PullSender,
    metadata_only: bool,
) -> Result<(), Status> {
    for entry in entries {
        let abs_path = entry.absolute_path(&module.path);
        stream_single_file(tx, &entry.relative_path, &abs_path, metadata_only).await?;
    }
    Ok(())
}

async fn stream_single_file(
    tx: &PullSender,
    relative: &Path,
    abs_path: &Path,
    metadata_only: bool,
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
            checksum: vec![],
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send pull header"))?;

    if metadata_only {
        return Ok(());
    }

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

async fn accept_pull_data_connection(
    listener: TcpListener,
    expected_token: Vec<u8>,
    source_root: PathBuf,
    payloads: Vec<TransferPayload>,
    chunk_bytes: usize,
    payload_prefetch: usize,
    stream_count: u32,
) -> Result<(), Status> {
    let start = Instant::now();
    let streams = stream_count.max(1) as usize;
    let total_bytes: u64 = payloads.iter().map(payload_bytes).sum();

    // Accept N TCP connections and wrap each as a DataPlaneSink.
    let sinks = accept_and_wrap_sinks(
        &listener,
        &expected_token,
        streams,
        chunk_bytes,
        payload_prefetch,
        &source_root,
    )
    .await?;

    // `source_root` is the enumeration root (module.path.join(requested)) —
    // header.relative_path is relative to this, so source_root.join(header.rel)
    // locates the physical file on disk.
    let source: Arc<dyn blit_core::remote::transfer::source::TransferSource> =
        Arc::new(FsTransferSource::new(source_root));
    execute_sink_pipeline(source, sinks, payloads, payload_prefetch, None)
        .await
        .map_err(|err| Status::internal(format!("pull data plane pipeline: {err}")))?;

    let elapsed = start.elapsed().as_secs_f64().max(1e-6);
    if total_bytes > 0 {
        let gbps = (total_bytes as f64 * 8.0) / elapsed / 1e9;
        eprintln!(
            "[pull-data-plane] aggregate throughput {:.2} Gbps ({} bytes in {:.2}s)",
            gbps, total_bytes, elapsed
        );
    }

    Ok(())
}

/// Accept N TCP connections, validate each token, wrap each in a
/// `DataPlaneSink`. `source_root` is the enumeration root (module.path +
/// requested subpath) — files are read relative to this via header.relative_path.
async fn accept_and_wrap_sinks(
    listener: &TcpListener,
    expected_token: &[u8],
    streams: usize,
    chunk_bytes: usize,
    payload_prefetch: usize,
    source_root: &Path,
) -> Result<Vec<Arc<dyn TransferSink>>, Status> {
    let source: Arc<dyn blit_core::remote::transfer::source::TransferSource> =
        Arc::new(FsTransferSource::new(source_root.to_path_buf()));

    // Shared buffer pool across all streams.
    let buffer_size = chunk_bytes.max(64 * 1024);
    let pool_size = streams * 2 + 4;
    let memory_budget = buffer_size * pool_size * 2;
    let pool = Arc::new(BufferPool::new(buffer_size, pool_size, Some(memory_budget)));

    let dst_root = source_root.to_path_buf();
    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(streams);
    for idx in 0..streams {
        let (mut socket, addr) = listener
            .accept()
            .await
            .map_err(|err| Status::internal(format!("data plane accept failed: {err}")))?;
        eprintln!(
            "[pull-data-plane] accepted connection {} from {}",
            idx, addr
        );

        // Validate token before handing the socket to a sink.
        let mut token_buf = vec![0u8; expected_token.len()];
        socket
            .read_exact(&mut token_buf)
            .await
            .map_err(|err| Status::internal(format!("failed to read pull token: {err}")))?;
        if token_buf != expected_token {
            eprintln!("[pull-data-plane] invalid token");
            return Err(Status::permission_denied("invalid pull data plane token"));
        }

        let session = blit_core::remote::transfer::data_plane::DataPlaneSession::from_stream(
            socket,
            false,
            chunk_bytes,
            payload_prefetch,
            Arc::clone(&pool),
        )
        .await;

        sinks.push(Arc::new(DataPlaneSink::new(
            session,
            source.clone(),
            dst_root.clone(),
        )));
    }

    Ok(sinks)
}

/// Streaming enumeration that sends entries through a channel as they're discovered.
/// Returns total file count and bytes when enumeration completes.
fn enumerate_to_channel(
    module_root: PathBuf,
    root: PathBuf,
    requested: PathBuf,
    tx: mpsc::Sender<Vec<PullEntry>>,
    batch_size: usize,
) -> Result<(u64, u64), Status> {
    use blit_core::enumeration::{EntryKind, FileEnumerator};
    use blit_core::fs_enum::FileFilter;

    if root.is_file() {
        // Single-file root: empty wire path so client writes to dest_root directly.
        let physical = if requested == Path::new(".") {
            root.file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
        } else {
            requested.clone()
        };
        let mut header = build_file_header(&module_root, &physical, false)?;
        header.relative_path = String::new();
        let size = header.size;
        let entry = PullEntry {
            header,
            relative_path: physical,
        };
        let _ = tx.blocking_send(vec![entry]);
        return Ok((1, size));
    }

    if !root.is_dir() {
        return Err(Status::invalid_argument("unsupported path type for pull"));
    }

    let enumerator = FileEnumerator::new(FileFilter::default());
    let mut batch = Vec::with_capacity(batch_size);
    let mut total_files = 0u64;
    let mut total_bytes = 0u64;

    enumerator
        .enumerate_local_streaming(&root, |entry| {
            if matches!(entry.kind, EntryKind::File { .. }) {
                let physical = requested.join(&entry.relative_path);
                let wire = entry.relative_path.clone();
                let mut header = build_file_header(&module_root, &physical, false)
                    .map_err(|e| eyre::eyre!("{}", e.message()))?;
                header.relative_path = wire.to_string_lossy().replace('\\', "/");
                let size = header.size;
                total_files += 1;
                total_bytes += size;
                batch.push(PullEntry {
                    header,
                    relative_path: physical,
                });

                if batch.len() >= batch_size {
                    let to_send = std::mem::take(&mut batch);
                    if tx.blocking_send(to_send).is_err() {
                        // Receiver dropped, stop enumeration
                        return Err(eyre::eyre!("enumeration cancelled"));
                    }
                }
            }
            Ok(())
        })
        .map_err(|err| Status::internal(format!("enumeration error: {}", err)))?;

    // Send remaining entries
    if !batch.is_empty() {
        let _ = tx.blocking_send(batch);
    }

    Ok((total_files, total_bytes))
}

/// Streaming data plane that routes payloads through the unified pipeline
/// as they arrive from enumeration.
async fn accept_pull_data_connection_streaming(
    listener: TcpListener,
    expected_token: Vec<u8>,
    source_root: PathBuf,
    mut batch_rx: mpsc::Receiver<Vec<TransferPayload>>,
    chunk_bytes: usize,
    payload_prefetch: usize,
    stream_count: u32,
) -> Result<TransferStats, Status> {
    let start = Instant::now();
    let streams = stream_count.max(1) as usize;

    // Accept N TCP connections, validate tokens, wrap as DataPlaneSinks.
    let sinks = accept_and_wrap_sinks(
        &listener,
        &expected_token,
        streams,
        chunk_bytes,
        payload_prefetch,
        &source_root,
    )
    .await?;

    // Bridge channel: flatten Vec<TransferPayload> → individual payloads
    // with byte/file counting along the way.
    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(payload_prefetch.max(1));

    let flatten = tokio::spawn(async move {
        let mut total_bytes = 0u64;
        let mut total_files = 0u64;
        while let Some(batch) = batch_rx.recv().await {
            for payload in batch {
                total_bytes += payload_bytes(&payload);
                total_files += match &payload {
                    TransferPayload::File(_) => 1,
                    TransferPayload::TarShard { headers } => headers.len() as u64,
                    // Daemon's pull source (filesystem) never produces resume payloads.
                    TransferPayload::FileBlock { .. }
                    | TransferPayload::FileBlockComplete { .. } => 0,
                };
                if payload_tx.send(payload).await.is_err() {
                    return (total_files, total_bytes);
                }
            }
        }
        (total_files, total_bytes)
    });

    let source: Arc<dyn blit_core::remote::transfer::source::TransferSource> =
        Arc::new(FsTransferSource::new(source_root));
    execute_sink_pipeline_streaming(source, sinks, payload_rx, payload_prefetch, None)
        .await
        .map_err(|err| Status::internal(format!("pull streaming pipeline: {err}")))?;

    let (total_files, total_bytes) = flatten
        .await
        .map_err(|err| Status::internal(format!("pull flatten task panicked: {err}")))?;

    let elapsed = start.elapsed().as_secs_f64().max(1e-6);
    if total_bytes > 0 {
        let gbps = (total_bytes as f64 * 8.0) / elapsed / 1e9;
        eprintln!(
            "[pull-data-plane] aggregate throughput {:.2} Gbps ({} bytes in {:.2}s)",
            gbps, total_bytes, elapsed
        );
    }

    Ok(TransferStats {
        files_transferred: total_files,
        bytes_transferred: total_bytes,
        bytes_zero_copy: 0,
    })
}

fn pull_stream_count(total_bytes: u64, tuning_max: usize) -> u32 {
    let mut streams = if total_bytes >= 32 * 1024 * 1024 * 1024 {
        16
    } else if total_bytes >= 8 * 1024 * 1024 * 1024 {
        12
    } else if total_bytes >= 2 * 1024 * 1024 * 1024 {
        10
    } else if total_bytes >= 512 * 1024 * 1024 {
        8
    } else if total_bytes >= 128 * 1024 * 1024 {
        4
    } else if total_bytes >= 32 * 1024 * 1024 {
        2
    } else {
        1
    } as usize;
    streams = streams.min(tuning_max.max(1));
    streams as u32
}

async fn send_summary(
    tx: &PullSender,
    stats: TransferStats,
    fallback_used: bool,
) -> Result<(), Status> {
    tx.send(Ok(PullChunk {
        payload: Some(PullPayload::Summary(PullSummary {
            files_transferred: stats.files_transferred,
            bytes_transferred: stats.bytes_transferred,
            bytes_zero_copy: stats.bytes_zero_copy,
            tcp_fallback_used: fallback_used,
            entries_deleted: 0,
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send pull summary"))
}

async fn send_manifest_batch(
    tx: &PullSender,
    file_count: u64,
    total_bytes: u64,
) -> Result<(), Status> {
    tx.send(Ok(PullChunk {
        payload: Some(PullPayload::ManifestBatch(ManifestBatch {
            file_count,
            total_bytes,
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send manifest batch"))
}

fn payload_bytes(payload: &TransferPayload) -> u64 {
    match payload {
        TransferPayload::File(header) => header.size,
        TransferPayload::TarShard { headers } => headers.iter().map(|h| h.size).sum(),
        TransferPayload::FileBlock { size, .. } => *size,
        TransferPayload::FileBlockComplete { .. } => 0,
    }
}
