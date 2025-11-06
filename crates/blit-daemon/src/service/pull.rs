use super::{PullPayload, PullSender};
use crate::runtime::ModuleConfig;
use blit_core::generated::{FileData, FileHeader, PullChunk};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;
use tonic::Status;

use super::util::{
    metadata_mtime_seconds, normalize_relative_path, permissions_mode, resolve_relative_path,
};

pub(crate) async fn stream_pull(
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
            let enumerator = blit_core::enumeration::FileEnumerator::new(
                blit_core::fs_enum::FileFilter::default(),
            );
            enumerator.enumerate_local(&root_clone)
        })
        .await
        .map_err(|e| Status::internal(format!("enumeration task failed: {}", e)))?
        .map_err(|e| Status::internal(format!("enumeration error: {}", e)))?;

        for entry in entries {
            if matches!(entry.kind, blit_core::enumeration::EntryKind::File { .. }) {
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
