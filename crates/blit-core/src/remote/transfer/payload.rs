use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eyre::{bail, eyre, Context, Result};
use futures::{stream, StreamExt};
use tokio::task;

use crate::fs_enum::FileEntry;
use crate::generated::FileHeader;
use crate::transfer_plan::{self, PlanOptions, TransferTask};
use tar::{Builder, EntryType, Header};

use crate::remote::transfer::source::TransferSource;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum TransferPayload {
    File(FileHeader),
    TarShard {
        headers: Vec<FileHeader>,
    },
    /// Resume protocol: overwrite a block of an existing file.
    FileBlock {
        relative_path: String,
        offset: u64,
        size: u64,
    },
    /// Resume protocol: finalize a resumed file (truncate to total_size).
    FileBlockComplete {
        relative_path: String,
        total_size: u64,
    },
    /// otp-7b: one resume-flagged file's WHOLE block phase as a single
    /// work item — the manifest header plus the destination's block
    /// hashes. Choreography-originated only (the session's send half
    /// queues it once the file's `BlockHashList` has arrived); the
    /// outbound planner never emits it. One work item ⇒ one pipeline
    /// worker ⇒ one socket, which is what keeps the record strictly
    /// serialized (every `BLOCK` before its `BLOCK_COMPLETE`, no
    /// cross-socket reorder hazard against the truncate+stamp).
    ResumeFile {
        header: FileHeader,
        block_size: u32,
        dest_hashes: Vec<Vec<u8>>,
    },
}

pub async fn prepare_payload(
    payload: TransferPayload,
    source_root: PathBuf,
) -> Result<PreparedPayload> {
    match payload {
        TransferPayload::File(mut header) => {
            if header.windows_metadata.is_none() {
                return Ok(PreparedPayload::File(header));
            }
            task::spawn_blocking(move || {
                let source_path = source_path_for_header(&source_root, &header);
                crate::windows_metadata::hydrate_payload_header(&source_path, &mut header)?;
                Ok(PreparedPayload::File(header))
            })
            .await
            .map_err(|err| eyre!("file payload metadata worker failed: {err}"))?
        }
        TransferPayload::TarShard { headers } => task::spawn_blocking(move || {
            let mut headers = headers;
            for header in &mut headers {
                let source_path = source_path_for_header(&source_root, header);
                crate::windows_metadata::hydrate_payload_header(&source_path, header)?;
            }
            let data = build_tar_shard(&source_root, &headers)?;
            Ok(PreparedPayload::TarShard { headers, data })
        })
        .await
        .map_err(|err| eyre!("tar shard worker failed: {err}"))?,
        // Resume payloads can only originate on the receive side (parsed
        // off the wire by DataPlaneSource); the file-system source never
        // produces them.
        TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => {
            bail!("FileBlock payloads cannot be prepared from a filesystem source")
        }
        // otp-7b: nothing to prepare — the block-diff streams the source
        // file inside the sink write (DataPlaneSink), where the record's
        // strict serialization lives. Pass through.
        TransferPayload::ResumeFile {
            mut header,
            block_size,
            dest_hashes,
        } => {
            if header.windows_metadata.is_none() {
                return Ok(PreparedPayload::ResumeFile {
                    header,
                    block_size,
                    dest_hashes,
                });
            }
            task::spawn_blocking(move || {
                let source_path = source_path_for_header(&source_root, &header);
                crate::windows_metadata::hydrate_payload_header(&source_path, &mut header)?;
                Ok(PreparedPayload::ResumeFile {
                    header,
                    block_size,
                    dest_hashes,
                })
            })
            .await
            .map_err(|err| eyre!("resume payload metadata worker failed: {err}"))?
        }
    }
}

fn source_path_for_header(source_root: &Path, header: &FileHeader) -> PathBuf {
    if header.relative_path.is_empty() {
        source_root.to_path_buf()
    } else {
        source_root.join(&header.relative_path)
    }
}

/// A payload ready for a sink to consume.
///
/// `File` and `TarShard` are used by both outbound and inbound paths
/// (they carry self-contained data). The receive pipeline additionally
/// uses `FileBlock` / `FileBlockComplete` for the resume protocol.
///
/// Streaming file bytes (4 GiB pulls, no point buffering) are NOT a
/// payload variant — they go through `TransferSink::write_file_stream`
/// directly so the receiver can hand the sink a borrowed reader without
/// fighting `'static` trait-object lifetimes.
#[derive(Debug)]
pub enum PreparedPayload {
    /// Whole file, source has it accessible by `src_root.join(relative_path)`.
    /// The sink performs a (zero-copy when possible) local copy.
    File(FileHeader),
    /// In-memory tar shard. Already buffered (bounded by the planner's
    /// shard threshold).
    TarShard {
        headers: Vec<FileHeader>,
        data: Vec<u8>,
    },
    /// Resume: write `bytes` at `offset` into the existing file at
    /// `dst_root.join(relative_path)`.
    FileBlock {
        relative_path: String,
        offset: u64,
        bytes: Vec<u8>,
    },
    /// Resume: finalize the file at `dst_root.join(relative_path)` by
    /// truncating to `total_size` and stamping mtime + perms.
    /// Metadata is carried inline so a "mtime touched, content
    /// identical" mirror correctly updates the destination's mtime
    /// even when zero blocks needed to be transferred.
    FileBlockComplete {
        relative_path: String,
        total_size: u64,
        mtime_seconds: i64,
        permissions: u32,
        windows_metadata: Option<crate::generated::WindowsFileMetadata>,
    },
    /// otp-7b: a resume-flagged file's whole block phase, send-side only
    /// (see [`TransferPayload::ResumeFile`]). Consumed by `DataPlaneSink`,
    /// which runs the block-diff against `dest_hashes` and emits the
    /// `BLOCK*`/`BLOCK_COMPLETE` wire records; every receive-side sink
    /// rejects it (the wire never carries this composite shape — the
    /// receive pipeline decodes per-block `FileBlock`/`FileBlockComplete`).
    ResumeFile {
        header: FileHeader,
        block_size: u32,
        dest_hashes: Vec<Vec<u8>>,
    },
}

pub const DEFAULT_PAYLOAD_PREFETCH: usize = 8;

pub fn plan_transfer_payloads(
    headers: Vec<FileHeader>,
    source_root: &Path,
    options: PlanOptions,
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
            // Tar payload cost includes named-stream content. Planning only on
            // the unnamed stream could multiply a valid 2 MiB metadata payload
            // by thousands of members before the receiver sees the tar body.
            size: header
                .size
                .saturating_add(crate::windows_metadata::payload_bytes(header)),
            is_directory: false,
        });
    }

    let mut header_map: HashMap<String, FileHeader> = headers
        .into_iter()
        .map(|header| (header.relative_path.clone(), header))
        .collect();

    let tasks = transfer_plan::build_plan(&entries, source_root, options);
    let mut payloads: Vec<TransferPayload> = Vec::new();

    for task in tasks {
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

    // Sort payloads: tar shards first (small, distribute well across streams),
    // then files ascending by size. This ensures all streams stay busy with
    // small work before a single large file monopolizes one stream's tail.
    // Resume variants (FileBlock / FileBlockComplete) are receive-only and
    // never appear here — plan_transfer_payloads is the outbound planner.
    payloads.sort_by_key(|p| match p {
        TransferPayload::TarShard { .. } => (0, 0),
        TransferPayload::File(h) => (1, h.size),
        TransferPayload::ResumeFile { header, .. } => (1, header.size),
        TransferPayload::FileBlock { size, .. } => (2, *size),
        TransferPayload::FileBlockComplete { .. } => (3, 0),
    });

    Ok(payloads)
}

pub fn payload_file_count(payloads: &[TransferPayload]) -> usize {
    payloads
        .iter()
        .map(|payload| match payload {
            TransferPayload::File(_) => 1,
            TransferPayload::TarShard { headers } => headers.len(),
            // Resume payloads patch existing files in-place — they
            // don't add to the "files transferred" count.
            TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => 0,
            // One composite resume item completes exactly one file.
            TransferPayload::ResumeFile { .. } => 1,
        })
        .sum()
}

fn normalize_relative_path(path: &Path) -> String {
    // Canonical POSIX form — see `crate::path_posix` for why a
    // component-walk is correct on every platform and the historical
    // string `replace('\\', "/")` was destructive on POSIX.
    crate::path_posix::relative_path_to_posix(path)
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

pub fn build_tar_shard(source_root: &Path, headers: &[FileHeader]) -> Result<Vec<u8>> {
    let mut builder = Builder::new(Vec::new());

    for header in headers {
        let rel = Path::new(&header.relative_path);
        // Empty relative_path = "root is itself the file" (single-file
        // source). See FsTransferSource::open_file for context — join("")
        // can preserve a trailing separator that File::open rejects.
        let full_path = if header.relative_path.is_empty() {
            source_root.to_path_buf()
        } else {
            source_root.join(rel)
        };
        let mut file = std::fs::File::open(&full_path)
            .with_context(|| format!("opening {}", full_path.display()))?;

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
        tar_header.set_cksum();

        builder
            .append_data(&mut tar_header, rel, &mut file)
            .with_context(|| format!("adding {} to tar shard", full_path.display()))?;
    }

    builder.into_inner().context("finalizing tar shard")
}
