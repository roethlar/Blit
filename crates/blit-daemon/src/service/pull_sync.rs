//! Bidirectional pull with manifest comparison for selective transfers.
//!
//! This module implements the PullSync RPC which allows clients to send their
//! local manifest so the server can compare and only send files that need updating.

use super::pull::{collect_pull_entries_with_checksums, PullEntry};
use super::push::{bind_data_plane_listener, generate_token, TransferStats};
use super::util::{resolve_module, resolve_relative_path};
use super::PullSyncSender;
use crate::runtime::{ModuleConfig, RootExport};

use base64::{engine::general_purpose, Engine as _};
use blit_core::generated::{
    client_pull_message, server_pull_message, BlockHashRequest, BlockTransfer, BlockTransferComplete,
    ClientPullMessage, DataTransferNegotiation, FileData, FileHeader, FileList, ManifestBatch,
    PullSummary, PullSyncAck, PullSyncHeader, ServerPullMessage,
};
use blit_core::manifest::{compare_manifests, files_needing_transfer, CompareMode, CompareOptions, FileStatus};
use blit_core::remote::transfer::source::{FsTransferSource, TransferSource};
use blit_core::remote::transfer::plan_transfer_payloads;
use blit_core::remote::tuning::determine_remote_tuning;
use blit_core::transfer_plan::PlanOptions;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;
use tonic::{Status, Streaming};

/// Handle a bidirectional PullSync stream.
pub(crate) async fn handle_pull_sync_stream(
    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    default_root: Option<RootExport>,
    mut stream: Streaming<ClientPullMessage>,
    tx: PullSyncSender,
    force_grpc_override: bool,
    server_checksums_enabled: bool,
) -> Result<(), Status> {
    // Phase 1: Receive header
    let header = match receive_header(&mut stream).await? {
        Some(h) => h,
        None => return Err(Status::invalid_argument("expected PullSyncHeader as first message")),
    };

    // Resolve module from header
    let module = resolve_module(&modules, default_root.as_ref(), &header.module).await?;

    let force_grpc = header.force_grpc || force_grpc_override;
    let mirror_mode = header.mirror_mode;
    let client_wants_checksum = header.checksum;
    let resume_mode = header.resume;
    // Clamp block size to safe limit to prevent server OOM
    use blit_core::copy::MAX_BLOCK_SIZE;
    let block_size = header.block_size.min(MAX_BLOCK_SIZE as u32);

    // Acknowledge header with server capabilities
    send_pull_sync_ack(&tx, server_checksums_enabled).await?;

    // Resolve path
    let requested = if header.path.trim().is_empty() {
        PathBuf::from(".")
    } else {
        resolve_relative_path(&header.path)?
    };

    let root = module.path.join(&requested);
    if !root.exists() {
        return Err(Status::not_found(format!(
            "path not found in module '{}': {}",
            module.name, header.path
        )));
    }

    // Phase 2: Receive client manifest
    let client_manifest = receive_client_manifest(&mut stream).await?;

    // Phase 3: Enumerate server files and compare with client manifest
    // Compute checksums if client requests checksum mode and server has checksums enabled
    let compute_checksums = client_wants_checksum && server_checksums_enabled;
    let server_entries = collect_pull_entries_with_checksums(&module.path, &root, &requested, compute_checksums).await?;

    // Convert to FileHeader for comparison
    let server_manifest: Vec<FileHeader> = server_entries
        .iter()
        .map(|e| e.header.clone())
        .collect();

    // Send manifest batch for progress reporting
    let total_bytes: u64 = server_manifest.iter().map(|h| h.size).sum();
    send_manifest_batch(&tx, server_manifest.len() as u64, total_bytes).await?;

    // Determine comparison mode based on header flags
    let compare_mode = if header.ignore_times {
        CompareMode::IgnoreTimes
    } else if header.force {
        CompareMode::Force
    } else if header.size_only {
        CompareMode::SizeOnly
    } else if header.checksum {
        CompareMode::Checksum
    } else {
        CompareMode::Default
    };

    // Compare manifests: server files are source, client files are target
    let compare_opts = CompareOptions {
        mode: compare_mode,
        ignore_existing: header.ignore_existing,
        include_deletions: mirror_mode,
    };
    let diff = compare_manifests(&server_manifest, &client_manifest, &compare_opts);

    if diff.files_to_transfer.is_empty() && diff.files_to_delete.is_empty() {
        // Nothing to transfer
        send_summary(&tx, TransferStats::default(), false, 0).await?;
        return Ok(());
    }

    // Send list of files that will be downloaded (the NeedList)
    let files_to_send = files_needing_transfer(&diff);
    send_need_list(&tx, &files_to_send).await?;

    // Build set of resume-eligible files (Modified status means client has existing file)
    let resume_eligible: std::collections::HashSet<String> = diff
        .files_to_transfer
        .iter()
        .filter(|f| f.status == FileStatus::Modified)
        .map(|f| f.relative_path.clone())
        .collect();

    // Filter server entries to only those needing transfer
    let entries_to_send: Vec<PullEntry> = server_entries
        .into_iter()
        .filter(|e| files_to_send.contains(&e.header.relative_path))
        .collect();

    if entries_to_send.is_empty() {
        send_summary(&tx, TransferStats::default(), false, diff.files_to_delete.len() as u64).await?;
        return Ok(());
    }

    // Phase 4: Transfer files
    let bytes_to_send: u64 = entries_to_send.iter().map(|e| e.header.size).sum();

    if force_grpc {
        // gRPC fallback: stream via control plane (full files or blocks)
        if resume_mode && !resume_eligible.is_empty() {
            // Block-level resume via gRPC bidirectional stream
            let stats = stream_via_block_resume_grpc(
                &module,
                &entries_to_send,
                block_size,
                &tx,
                &mut stream,
                &resume_eligible,
            ).await?;
            send_summary(&tx, stats, true, diff.files_to_delete.len() as u64).await?;
        } else {
            stream_via_grpc(&module, &entries_to_send, &tx).await?;
            send_summary(
                &tx,
                TransferStats {
                    files_transferred: entries_to_send.len() as u64,
                    bytes_transferred: bytes_to_send,
                    bytes_zero_copy: 0,
                },
                true,
                diff.files_to_delete.len() as u64,
            ).await?;
        }
    } else if resume_mode && !resume_eligible.is_empty() {
        // Data plane with block-level resume
        // Use gRPC for block hash exchange, data plane for block transfer
        let stats = stream_via_data_plane_resume(
            &module,
            entries_to_send,
            bytes_to_send,
            block_size,
            &tx,
            &mut stream,
            &resume_eligible,
        ).await?;
        send_summary(&tx, stats, false, diff.files_to_delete.len() as u64).await?;
    } else {
        // Data plane transfer (full files)
        let stats = stream_via_data_plane(&module, entries_to_send, bytes_to_send, &tx).await?;
        send_summary(&tx, stats, false, diff.files_to_delete.len() as u64).await?;
    }

    Ok(())
}

async fn receive_header(stream: &mut Streaming<ClientPullMessage>) -> Result<Option<PullSyncHeader>, Status> {
    match stream.message().await {
        Ok(Some(msg)) => {
            if let Some(client_pull_message::Payload::Header(header)) = msg.payload {
                Ok(Some(header))
            } else {
                Ok(None)
            }
        }
        Ok(None) => Ok(None),
        Err(e) => Err(Status::internal(format!("failed to receive header: {}", e))),
    }
}

async fn receive_client_manifest(stream: &mut Streaming<ClientPullMessage>) -> Result<Vec<FileHeader>, Status> {
    let mut manifest = Vec::new();

    loop {
        match stream.message().await {
            Ok(Some(msg)) => {
                match msg.payload {
                    Some(client_pull_message::Payload::LocalFile(header)) => {
                        manifest.push(header);
                    }
                    Some(client_pull_message::Payload::ManifestDone(_)) => {
                        break;
                    }
                    _ => {
                        return Err(Status::invalid_argument(
                            "unexpected message during manifest phase"
                        ));
                    }
                }
            }
            Ok(None) => break,
            Err(e) => return Err(Status::internal(format!("failed to receive manifest: {}", e))),
        }
    }

    Ok(manifest)
}

async fn send_pull_sync_ack(tx: &PullSyncSender, server_checksums_enabled: bool) -> Result<(), Status> {
    tx.send(Ok(ServerPullMessage {
        payload: Some(server_pull_message::Payload::PullSyncAck(PullSyncAck {
            server_checksums_enabled,
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send ack"))
}

async fn send_manifest_batch(tx: &PullSyncSender, file_count: u64, total_bytes: u64) -> Result<(), Status> {
    tx.send(Ok(ServerPullMessage {
        payload: Some(server_pull_message::Payload::ManifestBatch(ManifestBatch {
            file_count,
            total_bytes,
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send manifest batch"))
}

async fn send_need_list(tx: &PullSyncSender, files: &[String]) -> Result<(), Status> {
    tx.send(Ok(ServerPullMessage {
        payload: Some(server_pull_message::Payload::FilesToDownload(FileList {
            relative_paths: files.to_vec(),
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send need list"))
}

async fn send_summary(
    tx: &PullSyncSender,
    stats: TransferStats,
    tcp_fallback: bool,
    entries_deleted: u64,
) -> Result<(), Status> {
    tx.send(Ok(ServerPullMessage {
        payload: Some(server_pull_message::Payload::Summary(PullSummary {
            files_transferred: stats.files_transferred,
            bytes_transferred: stats.bytes_transferred,
            bytes_zero_copy: stats.bytes_zero_copy,
            tcp_fallback_used: tcp_fallback,
            entries_deleted,
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send summary"))
}

async fn stream_via_grpc(
    module: &ModuleConfig,
    entries: &[PullEntry],
    tx: &PullSyncSender,
) -> Result<(), Status> {
    for entry in entries {
        let abs_path = module.path.join(&entry.relative_path);

        // Send header
        tx.send(Ok(ServerPullMessage {
            payload: Some(server_pull_message::Payload::FileHeader(entry.header.clone())),
        }))
        .await
        .map_err(|_| Status::internal("failed to send file header"))?;

        // Read and send file data
        let content = tokio::fs::read(&abs_path)
            .await
            .map_err(|e| Status::internal(format!("failed to read {}: {}", abs_path.display(), e)))?;

        tx.send(Ok(ServerPullMessage {
            payload: Some(server_pull_message::Payload::FileData(FileData { content })),
        }))
        .await
        .map_err(|_| Status::internal("failed to send file data"))?;
    }

    Ok(())
}

async fn stream_via_data_plane(
    module: &ModuleConfig,
    entries: Vec<PullEntry>,
    total_bytes: u64,
    tx: &PullSyncSender,
) -> Result<TransferStats, Status> {
    use blit_core::buffer::BufferPool;
    use blit_core::remote::transfer::data_plane::DataPlaneSession;
    use blit_core::remote::transfer::payload_file_count;

    // Determine tuning based on total bytes
    let tuning = determine_remote_tuning(total_bytes);
    let mut plan_options = PlanOptions::default();
    plan_options.chunk_bytes_override = Some(tuning.chunk_bytes);

    // Set up data plane listener
    let listener = bind_data_plane_listener()
        .await
        .map_err(|err| Status::internal(format!("failed to bind data plane: {}", err)))?;
    let port = listener
        .local_addr()
        .map_err(|err| Status::internal(format!("querying listener addr: {}", err)))?
        .port();
    let token = generate_token();
    let token_string = general_purpose::STANDARD_NO_PAD.encode(&token);

    // Single stream for now - multi-stream pull requires accepting multiple connections
    let stream_count = 1u32;

    // Send negotiation
    tx.send(Ok(ServerPullMessage {
        payload: Some(server_pull_message::Payload::Negotiation(DataTransferNegotiation {
            tcp_port: port as u32,
            one_time_token: token_string,
            tcp_fallback: false,
            stream_count,
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send negotiation"))?;

    // Plan transfer payloads
    let headers: Vec<FileHeader> = entries.iter().map(|e| e.header.clone()).collect();
    let planned = plan_transfer_payloads(headers, &module.path, plan_options)
        .map_err(|err| Status::internal(format!("failed to plan payloads: {}", err)))?;

    let file_count = payload_file_count(&planned.payloads);

    // Accept connection
    let (socket, _) = listener
        .accept()
        .await
        .map_err(|e| Status::internal(format!("failed to accept data plane connection: {}", e)))?;

    // Verify token
    let expected_token = token;
    let mut token_buf = vec![0u8; expected_token.len()];
    let mut socket = socket;
    socket
        .read_exact(&mut token_buf)
        .await
        .map_err(|e| Status::internal(format!("failed to read token: {}", e)))?;
    if token_buf != expected_token {
        return Err(Status::unauthenticated("invalid data plane token"));
    }

    // Create buffer pool
    let buffer_size = tuning.chunk_bytes.max(64 * 1024);
    let pool_size = 4;
    let memory_budget = buffer_size * pool_size * 2;
    let pool = Arc::new(BufferPool::new(buffer_size, pool_size, Some(memory_budget)));

    // Create data plane session and stream payloads
    let mut session = DataPlaneSession::from_stream(
        socket,
        false,
        tuning.chunk_bytes,
        8, // payload_prefetch
        pool,
    )
    .await;

    let source: Arc<dyn TransferSource> = Arc::new(FsTransferSource::new(module.path.clone()));

    session
        .send_payloads(source, planned.payloads)
        .await
        .map_err(|err| Status::internal(format!("sending pull data plane payloads: {}", err)))?;

    session
        .finish()
        .await
        .map_err(|err| Status::internal(format!("finishing pull data plane: {}", err)))?;

    Ok(TransferStats {
        files_transferred: file_count as u64,
        bytes_transferred: total_bytes,
        bytes_zero_copy: 0,
    })
}

/// Stream files using block-level resume via data plane (primary path).
///
/// Uses gRPC for block hash exchange, then sends blocks via TCP data plane.
/// Pipelines block hash requests to avoid per-file RTT penalty.
async fn stream_via_data_plane_resume(
    module: &ModuleConfig,
    entries: Vec<PullEntry>,
    total_bytes: u64,
    block_size_param: u32,
    tx: &PullSyncSender,
    stream: &mut Streaming<ClientPullMessage>,
    resume_eligible: &std::collections::HashSet<String>,
) -> Result<TransferStats, Status> {
    use blit_core::buffer::BufferPool;
    use blit_core::copy::DEFAULT_BLOCK_SIZE;
    use blit_core::remote::transfer::data_plane::DataPlaneSession;
    use tokio::io::AsyncReadExt;

    let block_size = if block_size_param == 0 { DEFAULT_BLOCK_SIZE } else { block_size_param as usize };

    // Determine tuning based on total bytes
    let tuning = determine_remote_tuning(total_bytes);

    // Set up data plane listener
    let listener = bind_data_plane_listener()
        .await
        .map_err(|err| Status::internal(format!("failed to bind data plane: {}", err)))?;
    let port = listener
        .local_addr()
        .map_err(|err| Status::internal(format!("querying listener addr: {}", err)))?
        .port();
    let token = generate_token();
    let token_string = general_purpose::STANDARD_NO_PAD.encode(&token);

    // Send negotiation
    tx.send(Ok(ServerPullMessage {
        payload: Some(server_pull_message::Payload::Negotiation(DataTransferNegotiation {
            tcp_port: port as u32,
            one_time_token: token_string,
            tcp_fallback: false,
            stream_count: 1, // Single stream for resume mode
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send negotiation"))?;

    // Accept connection
    let (socket, _) = listener
        .accept()
        .await
        .map_err(|e| Status::internal(format!("failed to accept data plane connection: {}", e)))?;

    // Verify token
    let expected_token = token;
    let mut token_buf = vec![0u8; expected_token.len()];
    let mut socket = socket;
    socket
        .read_exact(&mut token_buf)
        .await
        .map_err(|e| Status::internal(format!("failed to read token: {}", e)))?;
    if token_buf != expected_token {
        return Err(Status::unauthenticated("invalid data plane token"));
    }

    // Create buffer pool
    let buffer_size = tuning.chunk_bytes.max(64 * 1024);
    let pool_size = 4;
    let memory_budget = buffer_size * pool_size * 2;
    let pool = Arc::new(BufferPool::new(buffer_size, pool_size, Some(memory_budget)));

    // Create data plane session
    let mut session = DataPlaneSession::from_stream(
        socket,
        false,
        tuning.chunk_bytes,
        8,
        pool,
    )
    .await;

    let mut stats = TransferStats::default();

    // Phase 1: Send all block hash requests upfront for resume-eligible files
    // This fills the pipeline so the client can compute hashes while we transfer data.
    for entry in entries.iter() {
        if resume_eligible.contains(&entry.header.relative_path) {
            tx.send(Ok(ServerPullMessage {
                payload: Some(server_pull_message::Payload::BlockHashRequest(BlockHashRequest {
                    relative_path: entry.header.relative_path.clone(),
                    block_size: block_size as u32,
                })),
            }))
            .await
            .map_err(|_| Status::internal("failed to send block hash request"))?;
        }
    }

    // Phase 2: Process each file, consuming block hash responses Just-In-Time.
    // This avoids buffering hashes for all files in memory (O(N) memory usage).
    // Because requests were sent in order, responses are guaranteed to arrive in order.
    let mut buffer = vec![0u8; block_size];

    for entry in entries {
        let abs_path = module.path.join(&entry.relative_path);
        let relative_path = &entry.header.relative_path;
        let is_resume_eligible = resume_eligible.contains(relative_path);

        // Get client hashes if resume-eligible (JIT from stream)
        let file_client_hashes = if is_resume_eligible {
            match stream.message().await {
                Ok(Some(msg)) => {
                    if let Some(client_pull_message::Payload::BlockHashes(hash_list)) = msg.payload {
                        if hash_list.relative_path == *relative_path {
                            Some(hash_list.hashes)
                        } else {
                            return Err(Status::internal(format!(
                                "protocol mismatch: expected hashes for '{}', got '{}'",
                                relative_path, hash_list.relative_path
                            )));
                        }
                    } else {
                        return Err(Status::invalid_argument("expected BlockHashes response"));
                    }
                }
                Ok(None) => return Err(Status::internal("stream closed before receiving all hash responses")),
                Err(e) => return Err(Status::internal(format!("receiving block hashes: {}", e))),
            }
        } else {
            None
        };

        // Open file for streaming read
        let mut file = tokio::fs::File::open(&abs_path)
            .await
            .map_err(|e| Status::internal(format!("failed to open {}: {}", abs_path.display(), e)))?;

        let file_size = file.metadata().await
            .map_err(|e| Status::internal(format!("failed to get metadata for {}: {}", abs_path.display(), e)))?
            .len() as usize;

        // Process blocks by streaming
        let mut block_idx = 0usize;
        let mut offset = 0usize;

        loop {
            let bytes_read = file.read(&mut buffer).await
                .map_err(|e| Status::internal(format!("reading block from {}: {}", abs_path.display(), e)))?;

            if bytes_read == 0 {
                break;
            }

            let block_content = &buffer[..bytes_read];
            let server_hash = blake3::hash(block_content);

            // Check if this block needs transfer
            let needs_transfer = match &file_client_hashes {
                Some(hashes) if block_idx < hashes.len() => {
                    server_hash.as_bytes() != hashes[block_idx].as_slice()
                }
                _ => true,
            };

            if needs_transfer {
                session
                    .send_block(relative_path, offset as u64, block_content)
                    .await
                    .map_err(|err| Status::internal(format!("sending block: {}", err)))?;

                stats.bytes_transferred += block_content.len() as u64;
            }

            offset += bytes_read;
            block_idx += 1;
        }

        // Signal file complete via data plane
        session
            .send_block_complete(relative_path, file_size as u64)
            .await
            .map_err(|err| Status::internal(format!("sending block complete: {}", err)))?;

        stats.files_transferred += 1;
    }

    // Finish data plane session
    session
        .finish()
        .await
        .map_err(|err| Status::internal(format!("finishing data plane: {}", err)))?;

    Ok(stats)
}

/// Stream files using block-level resume via gRPC (fallback path).
///
/// For each file:
/// 1. If resume-eligible: request block hashes from client, compare, send only differing blocks
/// 2. If not resume-eligible (new file): send full file via block transfers
///
/// Note: This fallback uses simpler stop-and-wait protocol since it's for diagnostic use.
async fn stream_via_block_resume_grpc(
    module: &ModuleConfig,
    entries: &[PullEntry],
    block_size: u32,
    tx: &PullSyncSender,
    stream: &mut Streaming<ClientPullMessage>,
    resume_eligible: &std::collections::HashSet<String>,
) -> Result<TransferStats, Status> {
    use blit_core::copy::DEFAULT_BLOCK_SIZE;
    use tokio::io::AsyncReadExt;

    let block_size = if block_size == 0 { DEFAULT_BLOCK_SIZE } else { block_size as usize };
    let mut stats = TransferStats::default();

    for entry in entries {
        let abs_path = module.path.join(&entry.relative_path);
        let relative_path = &entry.header.relative_path;
        let is_resume_eligible = resume_eligible.contains(relative_path);

        // Open file for streaming
        let mut file = tokio::fs::File::open(&abs_path)
            .await
            .map_err(|e| Status::internal(format!("failed to open {}: {}", abs_path.display(), e)))?;

        let file_size = file.metadata().await
            .map_err(|e| Status::internal(format!("failed to get metadata for {}: {}", abs_path.display(), e)))?
            .len() as usize;

        // Get client block hashes if resume-eligible
        let client_hashes: Option<Vec<Vec<u8>>> = if is_resume_eligible {
            // Request block hashes from client
            tx.send(Ok(ServerPullMessage {
                payload: Some(server_pull_message::Payload::BlockHashRequest(BlockHashRequest {
                    relative_path: relative_path.clone(),
                    block_size: block_size as u32,
                })),
            }))
            .await
            .map_err(|_| Status::internal("failed to send block hash request"))?;

            // Wait for client's block hash response
            match stream.message().await {
                Ok(Some(msg)) => {
                    if let Some(client_pull_message::Payload::BlockHashes(hash_list)) = msg.payload {
                        if hash_list.relative_path == *relative_path {
                            Some(hash_list.hashes)
                        } else {
                            None // Path mismatch, send full file
                        }
                    } else {
                        None // Unexpected message, send full file
                    }
                }
                Ok(None) => None, // Stream closed, send full file
                Err(_) => None,   // Error, send full file
            }
        } else {
            None // New file, no client hashes
        };

        // Stream through file blocks
        let mut buffer = vec![0u8; block_size];
        let mut block_idx = 0usize;
        let mut offset = 0usize;

        loop {
            let bytes_read = file.read(&mut buffer).await
                .map_err(|e| Status::internal(format!("reading block from {}: {}", abs_path.display(), e)))?;

            if bytes_read == 0 {
                break;
            }

            let block_content = &buffer[..bytes_read];
            let server_hash = blake3::hash(block_content);

            // Check if this block differs from client's
            let needs_transfer = match &client_hashes {
                Some(hashes) if block_idx < hashes.len() => {
                    // Compare Blake3 hashes (32 bytes)
                    server_hash.as_bytes() != hashes[block_idx].as_slice()
                }
                _ => true, // No client hash or index out of bounds, need to transfer
            };

            if needs_transfer {
                tx.send(Ok(ServerPullMessage {
                    payload: Some(server_pull_message::Payload::BlockTransfer(BlockTransfer {
                        relative_path: relative_path.clone(),
                        offset: offset as u64,
                        content: block_content.to_vec(),
                    })),
                }))
                .await
                .map_err(|_| Status::internal("failed to send block transfer"))?;

                stats.bytes_transferred += block_content.len() as u64;
            }

            offset += bytes_read;
            block_idx += 1;
        }

        // Signal file complete
        tx.send(Ok(ServerPullMessage {
            payload: Some(server_pull_message::Payload::BlockComplete(BlockTransferComplete {
                relative_path: relative_path.clone(),
                total_bytes: file_size as u64,
            })),
        }))
        .await
        .map_err(|_| Status::internal("failed to send block complete"))?;

        stats.files_transferred += 1;
    }

    Ok(stats)
}
