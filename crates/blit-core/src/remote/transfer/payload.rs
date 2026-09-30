use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eyre::{bail, eyre, Context, Result};
use futures::{stream, StreamExt};
use tokio::task;

use crate::fs_enum::FileEntry;
use crate::generated::FileHeader;
use crate::transfer_plan::{self, PlanOptions, TransferTask};
use tar::{Builder, EntryType, Header};

use crate::remote::transfer::sink::FileFailure;
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

/// How a source hydrates one header's Windows metadata before the
/// payload goes out: the production hydrator is
/// [`crate::windows_metadata::hydrate_payload_header`]; a test source
/// installs a failing one to prove that a per-file hydration failure is
/// a skip, never a session fault (SOURCE_SIDE_CONTAINMENT D-E, A11).
pub type Hydrator = Arc<dyn Fn(&Path, &mut FileHeader) -> Result<()> + Send + Sync>;

/// The production hydrator.
pub fn default_hydrator() -> Hydrator {
    Arc::new(|path: &Path, header: &mut FileHeader| {
        crate::windows_metadata::hydrate_payload_header(path, header)
    })
}

/// The `source:` reason a hydration failure is reported under. A
/// named-stream that changed size while being read, or metadata that no
/// longer matches the manifest, is the same drift class as a body that
/// changed size; everything else is "cannot read metadata".
pub fn hydration_failure_reason(err: &eyre::Report) -> String {
    let text = format!("{err:#}");
    if text.contains("changed size while reading") || text.contains("Windows metadata changed") {
        format!("source: changed size during transfer (Windows metadata: {text})")
    } else {
        format!("source: cannot read metadata: {text}")
    }
}

pub async fn prepare_payload(
    payload: TransferPayload,
    source_root: PathBuf,
) -> Result<PreparedPayload> {
    prepare_payload_with(payload, source_root, default_hydrator()).await
}

/// [`prepare_payload`] with an explicit hydrator. SOURCE_SIDE_CONTAINMENT
/// D-E: preparation returns PER-FILE outcomes — a `File`/`ResumeFile`
/// whose hydration fails becomes [`PreparedPayload::Skipped`], a shard
/// member whose hydration fails joins the shard's `skipped` list, and
/// only the infrastructure failures (a blocking worker that panicked)
/// remain `Err`. Every consumer emits a `Skipped` as its carrier's skip
/// record (or records it directly on the local route).
pub async fn prepare_payload_with(
    payload: TransferPayload,
    source_root: PathBuf,
    hydrate: Hydrator,
) -> Result<PreparedPayload> {
    match payload {
        TransferPayload::File(header) => {
            let hydrate_one = move |mut header: FileHeader| {
                let source_path = source_path_for_header(&source_root, &header);
                match hydrate(&source_path, &mut header) {
                    Ok(()) => PreparedPayload::File(header),
                    Err(err) => PreparedPayload::Skipped(FileFailure {
                        relative_path: header.relative_path,
                        reason: hydration_failure_reason(&err),
                    }),
                }
            };
            if header.windows_metadata.is_none() {
                // Nothing to read from disk: the production hydrator
                // returns at once, so no blocking worker is paid for.
                return Ok(hydrate_one(header));
            }
            task::spawn_blocking(move || hydrate_one(header))
                .await
                .map_err(|err| eyre!("file payload metadata worker failed: {err}"))
        }
        TransferPayload::TarShard { headers } => task::spawn_blocking(move || {
            let mut hydrated: Vec<FileHeader> = Vec::with_capacity(headers.len());
            let mut skipped: Vec<FileFailure> = Vec::new();
            for mut header in headers {
                let source_path = source_path_for_header(&source_root, &header);
                match hydrate(&source_path, &mut header) {
                    Ok(()) => hydrated.push(header),
                    Err(err) => skipped.push(FileFailure {
                        relative_path: header.relative_path,
                        reason: hydration_failure_reason(&err),
                    }),
                }
            }
            let TarShardBuild {
                data,
                headers,
                skipped: packer_skipped,
            } = build_tar_shard(&source_root, &hydrated)?;
            skipped.extend(packer_skipped);
            Ok(PreparedPayload::TarShard {
                headers,
                data,
                skipped,
            })
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
            header,
            block_size,
            dest_hashes,
        } => {
            let hydrate_one = move |mut header: FileHeader| {
                let source_path = source_path_for_header(&source_root, &header);
                match hydrate(&source_path, &mut header) {
                    Ok(()) => PreparedPayload::ResumeFile {
                        header,
                        block_size,
                        dest_hashes,
                    },
                    Err(err) => PreparedPayload::Skipped(FileFailure {
                        relative_path: header.relative_path,
                        reason: hydration_failure_reason(&err),
                    }),
                }
            };
            if header.windows_metadata.is_none() {
                return Ok(hydrate_one(header));
            }
            task::spawn_blocking(move || hydrate_one(header))
                .await
                .map_err(|err| eyre!("resume payload metadata worker failed: {err}"))
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
    /// shard threshold). `headers` lists exactly the members packed into
    /// `data`; `skipped` names the planned members the packer could not
    /// deliver as promised (ssc-2: open/read failure, or a size that no
    /// longer matches the manifest header) — each consumer emits those
    /// as contract-v7 skips BEFORE the shard record, so the destination
    /// closes them as failures and the shard's member list is a strict
    /// subset of what was granted. A shard whose every member was
    /// skipped has empty `headers` and `data` and carries only skips.
    TarShard {
        headers: Vec<FileHeader>,
        data: Vec<u8>,
        skipped: Vec<FileFailure>,
    },
    /// SOURCE_SIDE_CONTAINMENT D-E (ssc-4): one planned file the source
    /// could not prepare — its Windows metadata hydration failed
    /// (vanished, access denied, a named stream that changed size).
    /// Every consumer emits it as the carrier's skip record (in-stream
    /// `FileSkipped`, data-plane SKIP) or records it directly (local
    /// route); the destination closes the need Granted → Failed and
    /// reports it through `record_failure`. Reasons start with
    /// `source:`.
    Skipped(FileFailure),
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

/// The packer's result: the shard bytes, the members actually packed
/// (in order), and the planned members that were skipped instead.
#[derive(Debug, Default)]
pub struct TarShardBuild {
    pub data: Vec<u8>,
    pub headers: Vec<FileHeader>,
    pub skipped: Vec<FileFailure>,
}

/// Pack `headers` into one tar shard, appending each member ONLY from a
/// buffer that is exactly `header.size` bytes long (ssc-2, plan D-B).
///
/// The pre-ssc-2 packer set the tar header's size from the manifest and
/// then streamed the file to EOF; `tar::Builder::append_data` copies the
/// reader to its end and pads on the bytes it actually copied, so a
/// member that had grown since the scan pushed its extra bytes into the
/// next header slot and the destination died with `tar shard entry:
/// numeric field was not a number …` (the 2026-09-25 field failure). A
/// member that had shrunk misaligned the archive the same way. Now a
/// member whose bytes are not exactly what the manifest promised is
/// skipped — reported through the contract-v7 skip record by the
/// caller — and its shard-mates land intact.
///
/// Order of checks per member: open (`source: cannot open`), stat from
/// the opened handle (`source: changed size …`, the cheap pre-check that
/// also yields the exact "now" size for the message), a bounded read of
/// `size` bytes (`source: read error` / shrank mid-read), then a
/// one-byte probe past `size` (grew mid-read). The stat is not trusted
/// alone: the probe is what makes the guarantee hold if the file changes
/// between the stat and the read.
pub fn build_tar_shard(source_root: &Path, headers: &[FileHeader]) -> Result<TarShardBuild> {
    build_tar_shard_with(source_root, headers, &open_member_from_fs)
}

/// One shard member as the packer sees it: the length the opened handle
/// reports and the bytes it yields. Production opens the file
/// ([`open_member_from_fs`]); a test opener can make the two disagree to
/// exercise the read-side checks deterministically (cr-ssc2-3).
pub struct OpenedMember {
    pub len: u64,
    pub reader: Box<dyn std::io::Read>,
}

/// The production member opener: `File::open` + `metadata().len()` on
/// that handle, the handle as the reader.
pub fn open_member_from_fs(path: &Path) -> std::io::Result<OpenedMember> {
    let file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    Ok(OpenedMember {
        len,
        reader: Box::new(file),
    })
}

/// [`build_tar_shard`] with an explicit member opener (test seam).
pub fn build_tar_shard_with(
    source_root: &Path,
    headers: &[FileHeader],
    open: &dyn Fn(&Path) -> std::io::Result<OpenedMember>,
) -> Result<TarShardBuild> {
    use std::io::Read;

    let mut builder = Builder::new(Vec::new());
    let mut packed: Vec<FileHeader> = Vec::with_capacity(headers.len());
    let mut skipped: Vec<FileFailure> = Vec::new();

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
        let size = header.size;
        let mut skip = |reason: String| {
            log::warn!(
                "tar shard member skipped, shard continues: {} ({reason})",
                header.relative_path
            );
            skipped.push(FileFailure {
                relative_path: header.relative_path.clone(),
                reason,
            });
        };

        let OpenedMember {
            len: now,
            reader: mut file,
        } = match open(&full_path) {
            Ok(opened) => opened,
            Err(err) => {
                skip(format!("source: cannot open: {err}"));
                continue;
            }
        };
        if now != size {
            skip(changed_size_reason(size, now));
            continue;
        }
        let mut buf: Vec<u8> = Vec::with_capacity(usize::try_from(size).unwrap_or(0));
        if let Err(err) = (&mut file).take(size).read_to_end(&mut buf) {
            skip(format!("source: read error: {err}"));
            continue;
        }
        if buf.len() as u64 != size {
            // Shrank between the stat and the read.
            skip(changed_size_reason(size, buf.len() as u64));
            continue;
        }
        let mut probe = [0u8; 1];
        match file.read(&mut probe) {
            Ok(0) => {}
            Ok(_) => {
                // Grew between the stat and the read: the exact "now"
                // size is unknowable without another racy stat, so the
                // message reports the lower bound the probe proved.
                skip(changed_size_reason(size, size.saturating_add(1)));
                continue;
            }
            Err(err) => {
                skip(format!("source: read error: {err}"));
                continue;
            }
        }

        let mut tar_header = Header::new_gnu();
        tar_header.set_entry_type(EntryType::Regular);
        let mode = if header.permissions == 0 {
            0o644
        } else {
            header.permissions
        };
        tar_header.set_mode(mode);
        tar_header.set_size(size);
        let mtime = if header.mtime_seconds >= 0 {
            header.mtime_seconds as u64
        } else {
            0
        };
        tar_header.set_mtime(mtime);
        tar_header.set_cksum();

        builder
            .append_data(&mut tar_header, rel, &buf[..])
            .with_context(|| format!("adding {} to tar shard", full_path.display()))?;
        packed.push(header.clone());
    }

    if packed.is_empty() {
        // Every member was skipped: no shard record at all, only skips.
        return Ok(TarShardBuild {
            data: Vec::new(),
            headers: packed,
            skipped,
        });
    }
    let data = builder.into_inner().context("finalizing tar shard")?;
    Ok(TarShardBuild {
        data,
        headers: packed,
        skipped,
    })
}

/// The `source:`-prefixed reason every carrier reports for a member (or
/// single file) whose size no longer matches its manifest header.
pub fn changed_size_reason(manifest: u64, now: u64) -> String {
    format!("source: changed size during transfer (manifest {manifest} bytes, now {now})")
}
