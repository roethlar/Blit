use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eyre::{bail, eyre, Context, Result};
use futures::{stream, StreamExt};
use tokio::fs;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio::task;

use crate::fs_enum::FileEntry;
use crate::generated::client_push_request::Payload as ClientPayload;
use crate::generated::{
    ClientPushRequest, FileData, FileHeader, TarShardChunk, TarShardComplete, TarShardHeader,
    UploadComplete,
};
use crate::transfer_plan::{self, PlanOptions, TransferTask};
use tar::{Builder, EntryType, Header};

use super::data_plane::CONTROL_PLANE_CHUNK_SIZE;
use super::progress::RemoteTransferProgress;
use crate::remote::transfer::source::TransferSource;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum TransferPayload {
    File(FileHeader),
    TarShard { headers: Vec<FileHeader> },
}

pub async fn prepare_payload(
    payload: TransferPayload,
    source_root: PathBuf,
) -> Result<PreparedPayload> {
    match payload {
        TransferPayload::File(header) => Ok(PreparedPayload::File(header)),
        TransferPayload::TarShard { headers } => {
            let headers_clone = headers.clone();
            let source_root_clone = source_root.clone();
            let data =
                task::spawn_blocking(move || build_tar_shard(&source_root_clone, &headers_clone))
                    .await
                    .map_err(|err| eyre!("tar shard worker failed: {err}"))??;
            Ok(PreparedPayload::TarShard { headers, data })
        }
    }
}

#[derive(Debug)]
pub enum PreparedPayload {
    File(FileHeader),
    TarShard {
        headers: Vec<FileHeader>,
        data: Vec<u8>,
    },
}

pub const DEFAULT_PAYLOAD_PREFETCH: usize = 8;

pub struct PlannedPayloads {
    pub payloads: Vec<TransferPayload>,
    pub chunk_bytes: usize,
}

pub fn plan_transfer_payloads(
    headers: Vec<FileHeader>,
    source_root: &Path,
    options: PlanOptions,
) -> Result<PlannedPayloads> {
    if headers.is_empty() {
        return Ok(PlannedPayloads {
            payloads: Vec::new(),
            chunk_bytes: 0,
        });
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

    let plan = transfer_plan::build_plan(&entries, source_root, options);
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

    Ok(PlannedPayloads {
        payloads,
        chunk_bytes: plan.chunk_bytes,
    })
}

pub fn payload_file_count(payloads: &[TransferPayload]) -> usize {
    payloads
        .iter()
        .map(|payload| match payload {
            TransferPayload::File(_) => 1,
            TransferPayload::TarShard { headers } => headers.len(),
        })
        .sum()
}

fn normalize_relative_path(path: &Path) -> String {
    let raw = path.to_string_lossy();
    #[cfg(windows)]
    {
        raw.replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        raw.to_string()
    }
}

pub fn prepared_payload_stream(
    payloads: Vec<TransferPayload>,
    source: Arc<dyn TransferSource>,
    prefetch: usize,
) -> impl futures::Stream<Item = Result<PreparedPayload>> {
    let capacity = prefetch.max(1);
    stream::iter(payloads.into_iter().map(move |payload| {
        let source = source.clone();
        async move { source.prepare_payload(payload).await }
    }))
    .buffered(capacity)
}

pub async fn transfer_payloads_via_control_plane(
    source: Arc<dyn TransferSource>,
    payloads: Vec<TransferPayload>,
    tx: &mpsc::Sender<ClientPushRequest>,
    finish: bool,
    progress: Option<&RemoteTransferProgress>,
    chunk_bytes: usize,
    payload_prefetch: usize,
) -> Result<()> {
    let chunk_size = chunk_bytes.max(CONTROL_PLANE_CHUNK_SIZE);
    let mut buffer = vec![0u8; chunk_size];
    let mut prepared_stream =
        prepared_payload_stream(payloads, source.clone(), payload_prefetch);

    while let Some(prepared) = prepared_stream.next().await {
        match prepared? {
            PreparedPayload::File(header) => {
                send_payload(tx, ClientPayload::FileManifest(header.clone())).await?;

                if header.size == 0 {
                    if let Some(progress) = progress {
                        progress.report_payload(1, 0);
                    }
                    continue;
                }

                let mut file = source
                    .open_file(&header)
                    .await
                    .with_context(|| format!("opening {}", header.relative_path))?;

                let mut remaining = header.size;
                while remaining > 0 {
                    let to_read = buffer.len().min(remaining as usize);
                    let chunk = file
                        .read(&mut buffer[..to_read])
                        .await
                        .with_context(|| format!("reading {}", header.relative_path))?;
                    if chunk == 0 {
                        bail!(
                            "unexpected EOF while reading {} ({} bytes remaining)",
                            header.relative_path,
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
                    if let Some(progress) = progress {
                        progress.report_payload(0, chunk as u64);
                    }
                    remaining -= chunk as u64;
                }
                if let Some(progress) = progress {
                    progress.report_payload(1, 0);
                }
            }
            PreparedPayload::TarShard { headers, data } => {
                send_payload(
                    tx,
                    ClientPayload::TarShardHeader(TarShardHeader {
                        files: headers.clone(),
                        archive_size: data.len() as u64,
                    }),
                )
                .await?;

                for chunk in data.chunks(chunk_size) {
                    send_payload(
                        tx,
                        ClientPayload::TarShardChunk(TarShardChunk {
                            content: chunk.to_vec(),
                        }),
                    )
                    .await?;
                    if let Some(progress) = progress {
                        progress.report_payload(0, chunk.len() as u64);
                    }
                }

                send_payload(tx, ClientPayload::TarShardComplete(TarShardComplete {})).await?;
                if let Some(progress) = progress {
                    progress.report_payload(headers.len(), 0);
                }
            }
        }
    }

    if finish {
        send_payload(tx, ClientPayload::UploadComplete(UploadComplete {})).await?;
    }

    Ok(())
}

async fn send_payload(tx: &mpsc::Sender<ClientPushRequest>, payload: ClientPayload) -> Result<()> {
    tx.send(ClientPushRequest {
        payload: Some(payload),
    })
    .await
    .map_err(|_| eyre!("failed to send push request payload"))
}

pub fn build_tar_shard(source_root: &Path, headers: &[FileHeader]) -> Result<Vec<u8>> {
    let mut builder = Builder::new(Vec::new());

    for header in headers {
        let rel = Path::new(&header.relative_path);
        let full_path = source_root.join(rel);
        let mut file = std::fs::File::open(&full_path)
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
        tar_header.set_cksum();

        builder
            .append_data(&mut tar_header, rel, &mut file)
            .with_context(|| format!("adding {} to tar shard", full_path.display()))?;
    }

    builder
        .into_inner()
        .context("finalizing tar shard")
        .map(|buf| buf)
}
