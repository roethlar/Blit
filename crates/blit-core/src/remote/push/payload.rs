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
use crate::remote::push::client::RemotePushProgress;
use crate::transfer_plan::{self, PlanOptions, TransferTask};
use tar::{Builder, EntryType, Header};

use super::data_plane::CONTROL_PLANE_CHUNK_SIZE;

#[derive(Debug, Clone)]
pub(crate) enum TransferPayload {
    File(FileHeader),
    TarShard { headers: Vec<FileHeader> },
}

pub(crate) async fn prepare_payload(
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
pub(crate) enum PreparedPayload {
    File(FileHeader),
    TarShard {
        headers: Vec<FileHeader>,
        data: Vec<u8>,
    },
}

pub(crate) const PAYLOAD_PREFETCH: usize = 8;

pub(crate) fn plan_transfer_payloads(
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

pub(crate) fn payload_file_count(payloads: &[TransferPayload]) -> usize {
    payloads
        .iter()
        .map(|payload| match payload {
            TransferPayload::File(_) => 1,
            TransferPayload::TarShard { headers } => headers.len(),
        })
        .sum()
}

pub(crate) async fn transfer_payloads_via_control_plane(
    source_root: &Path,
    payloads: Vec<TransferPayload>,
    tx: &mpsc::Sender<ClientPushRequest>,
    finish: bool,
    progress: Option<&RemotePushProgress>,
) -> Result<()> {
    let mut buffer = vec![0u8; CONTROL_PLANE_CHUNK_SIZE];
    let root_buf = source_root.to_path_buf();

    let mut prepared_stream = stream::iter(payloads.into_iter().map(|payload| {
        let root = root_buf.clone();
        async move { prepare_payload(payload, root).await }
    }))
    .buffered(PAYLOAD_PREFETCH);

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

                for chunk in data.chunks(CONTROL_PLANE_CHUNK_SIZE) {
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

pub(crate) fn build_tar_shard(source_root: &Path, headers: &[FileHeader]) -> Result<Vec<u8>> {
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

async fn send_payload(tx: &mpsc::Sender<ClientPushRequest>, payload: ClientPayload) -> Result<()> {
    tx.send(ClientPushRequest {
        payload: Some(payload),
    })
    .await
    .map_err(|_| eyre!("failed to send push request payload"))
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
