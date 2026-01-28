//! Bidirectional pull with manifest comparison for selective transfers.
//!
//! This module implements the PullSync RPC which allows clients to send their
//! local manifest so the server can compare and only send files that need updating.

use super::pull::{collect_pull_entries, PullEntry};
use super::push::{bind_data_plane_listener, generate_token, TransferStats};
use super::util::{resolve_module, resolve_relative_path};
use super::PullSyncSender;
use crate::runtime::{ModuleConfig, RootExport};

use base64::{engine::general_purpose, Engine as _};
use blit_core::generated::{
    client_pull_message, server_pull_message, Ack, ClientPullMessage, DataTransferNegotiation,
    FileData, FileHeader, FileList, ManifestBatch, PullSummary, PullSyncHeader, ServerPullMessage,
};
use blit_core::manifest::{compare_manifests, files_needing_transfer, CompareMode, CompareOptions};
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

    // Acknowledge header
    send_ack(&tx).await?;

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
    let server_entries = collect_pull_entries(&module.path, &root, &requested).await?;

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
        // gRPC fallback: stream via control plane
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
    } else {
        // Data plane transfer
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

async fn send_ack(tx: &PullSyncSender) -> Result<(), Status> {
    tx.send(Ok(ServerPullMessage {
        payload: Some(server_pull_message::Payload::Ack(Ack {})),
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

    // Calculate stream count based on bytes
    let stream_count = pull_stream_count(total_bytes, tuning.max_streams as usize);

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

fn pull_stream_count(bytes: u64, max_streams: usize) -> u32 {
    const BYTES_PER_STREAM: u64 = 256 * 1024 * 1024; // 256 MiB per stream
    let ideal = ((bytes / BYTES_PER_STREAM) + 1) as usize;
    ideal.min(max_streams).max(1) as u32
}
