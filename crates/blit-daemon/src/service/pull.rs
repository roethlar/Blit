use super::{PullPayload, PullSender};
use crate::runtime::ModuleConfig;
use base64::{engine::general_purpose, Engine as _};
use blit_core::generated::{DataTransferNegotiation, FileData, FileHeader, PullChunk, PullSummary};
use blit_core::remote::transfer::{plan_transfer_payloads, TransferPayload};
use blit_core::remote::tuning::determine_remote_tuning;
use blit_core::transfer_plan::PlanOptions;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tonic::Status;

use super::push::{bind_data_plane_listener, generate_token, TransferStats};
use super::util::{
    metadata_mtime_seconds, normalize_relative_path, permissions_mode, resolve_relative_path,
};

pub(crate) async fn stream_pull(
    module: ModuleConfig,
    requested_path: String,
    force_grpc: bool,
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

    let entries = collect_pull_entries(&module.path, &root, &requested).await?;

    if entries.is_empty() {
        send_summary(&tx, TransferStats::default(), force_grpc).await?;
        return Ok(());
    }

    let total_bytes: u64 = entries.iter().map(|entry| entry.header.size).sum();

    if force_grpc {
        stream_via_grpc(&module, &entries, &tx).await?;
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
    let mut plan_options = PlanOptions::default();
    plan_options.chunk_bytes_override = Some(tuning.chunk_bytes);

    let headers: Vec<FileHeader> = entries.iter().map(|entry| entry.header.clone()).collect();
    let planned = plan_transfer_payloads(headers.clone(), &module.path, plan_options)
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

    let module_path = module.path.clone();
    let transfer_task = tokio::spawn(accept_pull_data_connection(
        listener,
        token,
        module_path,
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

struct PullEntry {
    header: FileHeader,
    relative_path: PathBuf,
}

impl PullEntry {
    fn absolute_path(&self, module_root: &Path) -> PathBuf {
        module_root.join(&self.relative_path)
    }
}

async fn collect_pull_entries(
    module_root: &Path,
    root: &Path,
    requested: &Path,
) -> Result<Vec<PullEntry>, Status> {
    if root.is_file() {
        let relative_name = if requested == Path::new(".") {
            root.file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
        } else {
            requested.to_path_buf()
        };
        let header = build_file_header(module_root, &relative_name)?;
        return Ok(vec![PullEntry {
            header,
            relative_path: relative_name,
        }]);
    }

    if !root.is_dir() {
        return Err(Status::invalid_argument("unsupported path type for pull"));
    }

    let root_clone = root.to_path_buf();
    let requested_clone = requested.to_path_buf();
    let module_root = module_root.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<Vec<PullEntry>, Status> {
        let enumerator =
            blit_core::enumeration::FileEnumerator::new(blit_core::fs_enum::FileFilter::default());
        let entries = enumerator
            .enumerate_local(&root_clone)
            .map_err(|err| Status::internal(format!("enumeration error: {}", err)))?;
        let mut files = Vec::new();
        for entry in entries {
            if matches!(entry.kind, blit_core::enumeration::EntryKind::File { .. }) {
                let relative_path = requested_clone.join(&entry.relative_path);
                let header = build_file_header(&module_root, &relative_path)?;
                files.push(PullEntry {
                    header,
                    relative_path,
                });
            }
        }
        Ok(files)
    })
    .await
    .map_err(|err| Status::internal(format!("enumeration task failed: {}", err)))?
}

fn build_file_header(base: &Path, relative: &Path) -> Result<FileHeader, Status> {
    let abs_path = base.join(relative);
    let metadata = std::fs::metadata(&abs_path)
        .map_err(|err| Status::internal(format!("stat {}: {}", abs_path.display(), err)))?;
    Ok(FileHeader {
        relative_path: normalize_relative_path(relative),
        size: metadata.len(),
        mtime_seconds: metadata_mtime_seconds(&metadata).unwrap_or(0),
        permissions: permissions_mode(&metadata),
    })
}

async fn stream_via_grpc(
    module: &ModuleConfig,
    entries: &[PullEntry],
    tx: &PullSender,
) -> Result<(), Status> {
    for entry in entries {
        let abs_path = entry.absolute_path(&module.path);
        stream_single_file(tx, &entry.relative_path, &abs_path).await?;
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

async fn accept_pull_data_connection(
    listener: TcpListener,
    expected_token: Vec<u8>,
    module_root: PathBuf,
    payloads: Vec<TransferPayload>,
    chunk_bytes: usize,
    payload_prefetch: usize,
    stream_count: u32,
) -> Result<(), Status> {
    let start = Instant::now();
    let streams = stream_count.max(1) as usize;
    let total_bytes: u64 = payloads.iter().map(|p| payload_bytes(p)).sum();
    let mut handles = Vec::with_capacity(streams);
    let chunked = chunk_transfer_payloads(payloads, streams);

    for (idx, payload_chunk) in chunked.into_iter().enumerate() {
        let (socket, addr) = listener
            .accept()
            .await
            .map_err(|err| Status::internal(format!("data plane accept failed: {}", err)))?;
        eprintln!(
            "[pull-data-plane] accepted connection {} from {}",
            idx, addr
        );
        let expected_token = expected_token.clone();
        let module_root = module_root.clone();
        handles.push(tokio::spawn(async move {
            handle_pull_stream(
                socket,
                expected_token,
                module_root,
                payload_chunk,
                chunk_bytes,
                payload_prefetch,
            )
            .await
        }));
    }

    for handle in handles {
        handle.await.map_err(|err| {
            Status::internal(format!("pull data plane worker cancelled: {}", err))
        })??;
    }

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

async fn handle_pull_stream(
    mut socket: TcpStream,
    expected_token: Vec<u8>,
    module_root: PathBuf,
    payloads: Vec<TransferPayload>,
    chunk_bytes: usize,
    payload_prefetch: usize,
) -> Result<(), Status> {
    let mut token_buf = vec![0u8; expected_token.len()];
    socket
        .read_exact(&mut token_buf)
        .await
        .map_err(|err| Status::internal(format!("failed to read pull token: {}", err)))?;
    if token_buf != expected_token {
        eprintln!("[pull-data-plane] invalid token");
        return Err(Status::permission_denied("invalid pull data plane token"));
    }

    let mut session = blit_core::remote::transfer::data_plane::DataPlaneSession::from_stream(
        socket,
        false,
        chunk_bytes,
        payload_prefetch,
    );

    for payload in payloads {
        session
            .send_payloads(&module_root, vec![payload])
            .await
            .map_err(|err| {
                Status::internal(format!("sending pull data plane payloads: {}", err))
            })?;
    }

    session
        .finish()
        .await
        .map_err(|err| Status::internal(format!("finishing pull data plane: {}", err)))
}

fn pull_stream_count(total_bytes: u64, tuning_max: usize) -> u32 {
    let mut streams = if total_bytes >= 4 * 1024 * 1024 * 1024 {
        8
    } else if total_bytes >= 512 * 1024 * 1024 {
        6
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
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send pull summary"))
}

fn chunk_transfer_payloads(
    payloads: Vec<TransferPayload>,
    streams: usize,
) -> Vec<Vec<TransferPayload>> {
    if streams <= 1 || payloads.is_empty() {
        return vec![payloads];
    }
    let buckets = streams.min(payloads.len());
    let mut chunks = vec![Vec::new(); buckets];
    for (idx, payload) in payloads.into_iter().enumerate() {
        chunks[idx % buckets].push(payload);
    }
    chunks
}

fn payload_bytes(payload: &TransferPayload) -> u64 {
    match payload {
        TransferPayload::File(header) => header.size,
        TransferPayload::TarShard { headers } => headers.iter().map(|h| h.size).sum(),
    }
}
