//! Shared safe tar-shard extraction primitive.
//!
//! Three sites in the codebase consume tar shards from a remote peer:
//!
//!   - `crates/blit-core/src/remote/pull.rs::apply_pull_tar_shard`
//!     (gRPC fallback receive on the pull-client side)
//!   - `crates/blit-core/src/remote/transfer/sink.rs::write_tar_shard_payload`
//!     (TCP data plane on the pull-client side and local-local sink)
//!   - `crates/blit-daemon/src/service/push/data_plane.rs::apply_tar_shard_sync`
//!     (daemon receiving an authenticated push)
//!
//! All three need the same safety policy:
//!
//!   1. Reject non-regular entries (no symlinks, hardlinks, or device
//!      nodes — a hostile tar can otherwise materialize a symlink at
//!      a benign path that escapes the destination root on later
//!      writes; this is the R5-F2 class of bug).
//!   2. Verify each entry corresponds to an expected `FileHeader` and
//!      that the tar header's declared size matches the manifest's
//!      declared size and is within the local cap, *before*
//!      allocating (R6-F1).
//!   3. Validate the path through `path_safety::validate_wire_path`
//!      and `safe_join`.
//!   4. Allocate via `try_reserve_exact` and read bytes manually
//!      (never `Entry::unpack`).
//!   5. Surface mtime and Unix permissions from the `FileHeader` so
//!      callers can apply them and avoid size+mtime resync churn
//!      (R6-F3).
//!
//! Each caller has different surrounding concerns (eyre vs `Status`
//! errors, parallel vs sequential writes, buffer-pool reuse) so the
//! helper returns a `Vec<ExtractedFile>` and lets the caller adapt.
//! `write_extracted_file` is provided as a convenience for the
//! sequential-write case.

use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use eyre::{bail, eyre, Context, Result};
use filetime::{set_file_mtime, FileTime};
use tar::{Archive, EntryType};

use crate::generated::FileHeader;
use crate::path_safety;

/// Default per-entry / per-shard byte cap. Tar shards target 4–64 MiB;
/// 256 MiB is comfortable headroom while bounding pathological
/// allocations from a hostile or buggy peer.
pub const MAX_TAR_SHARD_BYTES: u64 = 256 * 1024 * 1024;

/// Tunable knobs for `safe_extract_tar_shard`.
#[derive(Debug, Clone)]
pub struct TarShardExtractOptions {
    /// Reject any entry whose `FileHeader.size` exceeds this cap.
    /// Also bounds the per-entry allocation.
    pub max_entry_bytes: u64,
    /// When true (default), every header in `expected_headers` must
    /// be matched by an entry in the tar — leftover headers produce
    /// `Err`. Required for the strict "manifest is the wire contract"
    /// receivers (push receive, pull gRPC fallback). Set to false
    /// only when the caller knows extra headers may legitimately be
    /// produced by a separate code path.
    pub require_exact_headers: bool,
}

impl Default for TarShardExtractOptions {
    fn default() -> Self {
        Self {
            max_entry_bytes: MAX_TAR_SHARD_BYTES,
            require_exact_headers: true,
        }
    }
}

/// One file successfully extracted from a tar shard, validated and
/// ready to write. The contents have already been read into memory;
/// the path is already joined under the caller-supplied root via
/// `safe_join`.
#[derive(Debug)]
pub struct ExtractedFile {
    /// Wire-supplied relative path (validated, slash-normalized).
    pub rel: String,
    /// Absolute filesystem path. Already inside `dst_root` per
    /// `safe_join`.
    pub dest_path: PathBuf,
    /// File contents from the tar entry. Length matches
    /// `FileHeader.size` exactly.
    pub contents: Vec<u8>,
    /// mtime to apply, derived from `FileHeader.mtime_seconds`.
    /// `None` when the header carried no mtime (`mtime_seconds == 0`).
    pub mtime: Option<FileTime>,
    /// Unix permissions from `FileHeader.permissions`. `None` when
    /// the header carried no perms (`permissions == 0`).
    pub permissions: Option<u32>,
    /// Original size from `FileHeader.size`. Equals `contents.len()`.
    pub size: u64,
}

/// Walk a tar-shard buffer and return validated `ExtractedFile`s
/// ready to write. Does not touch the filesystem itself — callers
/// invoke `write_extracted_file` (or roll their own write loop, e.g.
/// in parallel via rayon) on the returned vec.
pub fn safe_extract_tar_shard(
    buffer: &[u8],
    expected_headers: Vec<FileHeader>,
    dst_root: &Path,
    options: &TarShardExtractOptions,
) -> Result<Vec<ExtractedFile>> {
    let mut expected: HashMap<String, FileHeader> = expected_headers
        .into_iter()
        .map(|h| (h.relative_path.clone(), h))
        .collect();

    let mut out: Vec<ExtractedFile> = Vec::with_capacity(expected.len());

    let mut archive = Archive::new(Cursor::new(buffer));
    let entries = archive.entries().context("reading tar shard entries")?;

    for entry_result in entries {
        let mut entry = entry_result.context("tar shard entry")?;
        let entry_type = entry.header().entry_type();
        if entry_type == EntryType::Directory {
            continue;
        }
        // Reject Symlink/Link/Block/Char/Fifo/GNU-* etc. so a hostile
        // peer can't substitute a special inode for a regular file.
        if entry_type != EntryType::Regular && entry_type != EntryType::Continuous {
            bail!("tar shard contained non-regular entry type {entry_type:?}; only files allowed");
        }

        let raw_path = entry.path().context("tar shard path")?;
        let rel_string = raw_path.to_string_lossy().replace('\\', "/");

        let header = expected.remove(&rel_string).ok_or_else(|| {
            eyre!("tar shard produced unexpected entry '{rel_string}' (not in manifest)")
        })?;

        // Size validation BEFORE any allocation (R6-F1). The tar
        // header's size and the manifest's FileHeader.size must
        // agree, and the size must be within the configured cap.
        let entry_size = entry.size();
        if entry_size != header.size {
            bail!(
                "tar shard entry '{rel_string}' tar-header size {} does not match \
                 FileHeader size {}",
                entry_size,
                header.size
            );
        }
        if header.size > options.max_entry_bytes {
            bail!(
                "tar shard entry '{rel_string}' size {} exceeds local cap {}",
                header.size,
                options.max_entry_bytes
            );
        }

        // Path validation through the shared chokepoint, then
        // safe_join for the actual filesystem path.
        path_safety::validate_wire_path(&rel_string)
            .with_context(|| format!("validating tar shard entry {rel_string:?}"))?;
        let dest_path = path_safety::safe_join(dst_root, &rel_string)
            .with_context(|| format!("resolving tar shard dest {rel_string:?}"))?;

        // Bounded allocation; pathological size returns AllocError
        // instead of aborting.
        let mut contents: Vec<u8> = Vec::new();
        contents
            .try_reserve_exact(header.size as usize)
            .with_context(|| {
                format!(
                    "allocating buffer for tar entry '{rel_string}' (size {})",
                    header.size
                )
            })?;
        std::io::copy(&mut entry, &mut contents)
            .with_context(|| format!("buffering tar entry {rel_string}"))?;
        if contents.len() as u64 != header.size {
            bail!(
                "tar shard entry '{rel_string}' produced {} bytes; expected {}",
                contents.len(),
                header.size
            );
        }

        let mtime = if header.mtime_seconds > 0 {
            Some(FileTime::from_unix_time(header.mtime_seconds, 0))
        } else {
            None
        };
        let permissions = if header.permissions != 0 {
            Some(header.permissions)
        } else {
            None
        };
        let size = header.size;

        out.push(ExtractedFile {
            rel: rel_string,
            dest_path,
            contents,
            mtime,
            permissions,
            size,
        });
    }

    if options.require_exact_headers && !expected.is_empty() {
        let missing: Vec<String> = expected.into_keys().collect();
        bail!("tar shard missing expected entries: {missing:?}");
    }

    Ok(out)
}

/// Write one `ExtractedFile` to disk, applying mtime and Unix
/// permissions best-effort. Convenience for the sequential-write
/// case; callers that want parallel writes can inline this in a
/// rayon loop.
pub fn write_extracted_file(file: &ExtractedFile) -> Result<()> {
    if let Some(parent) = file.dest_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    std::fs::write(&file.dest_path, &file.contents)
        .with_context(|| format!("writing {}", file.dest_path.display()))?;
    if let Some(ft) = file.mtime {
        let _ = set_file_mtime(&file.dest_path, ft);
    }
    #[cfg(unix)]
    if let Some(perms) = file.permissions {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&file.dest_path, std::fs::Permissions::from_mode(perms));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor as StdCursor;
    use tar::{Builder, EntryType, Header};
    use tempfile::tempdir;

    fn fh(rel: &str, size: u64) -> FileHeader {
        FileHeader {
            relative_path: rel.into(),
            size,
            mtime_seconds: 0,
            permissions: 0o644,
            checksum: vec![],
        }
    }

    fn build_regular_archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = Builder::new(Vec::new());
        for (rel, data) in entries {
            let mut h = Header::new_gnu();
            h.set_entry_type(EntryType::Regular);
            h.set_size(data.len() as u64);
            h.set_mode(0o644);
            builder
                .append_data(&mut h, rel, StdCursor::new(*data))
                .unwrap();
        }
        builder.into_inner().unwrap()
    }

    fn build_archive_with_symlink(rel: &str, link_target: &str) -> Vec<u8> {
        let mut builder = Builder::new(Vec::new());
        let mut h = Header::new_gnu();
        h.set_entry_type(EntryType::Symlink);
        h.set_size(0);
        h.set_mode(0o777);
        builder.append_link(&mut h, rel, link_target).unwrap();
        builder.into_inner().unwrap()
    }

    #[test]
    fn rejects_symlink_entry() {
        let buffer = build_archive_with_symlink("expected.txt", "/etc/passwd");
        let tmp = tempdir().unwrap();
        let err = safe_extract_tar_shard(
            &buffer,
            vec![fh("expected.txt", 0)],
            tmp.path(),
            &TarShardExtractOptions::default(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("non-regular entry"));
    }

    #[test]
    fn rejects_size_mismatch_between_tar_and_manifest() {
        let buffer = build_regular_archive(&[("ok.txt", b"hello")]);
        let tmp = tempdir().unwrap();
        let err = safe_extract_tar_shard(
            &buffer,
            vec![fh("ok.txt", 99)], // manifest lies
            tmp.path(),
            &TarShardExtractOptions::default(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("does not match"));
    }

    #[test]
    fn rejects_size_above_cap() {
        let buffer = build_regular_archive(&[("big.txt", b"hi")]);
        let tmp = tempdir().unwrap();
        let opts = TarShardExtractOptions {
            max_entry_bytes: 1, // way smaller than tar entry
            require_exact_headers: true,
        };
        let err =
            safe_extract_tar_shard(&buffer, vec![fh("big.txt", 2)], tmp.path(), &opts).unwrap_err();
        assert!(err.to_string().contains("exceeds local cap"));
    }

    #[test]
    fn rejects_unexpected_entry_when_strict() {
        let buffer = build_regular_archive(&[("rogue.txt", b"hi")]);
        let tmp = tempdir().unwrap();
        let err = safe_extract_tar_shard(
            &buffer,
            vec![], // no headers — anything in the tar is unexpected
            tmp.path(),
            &TarShardExtractOptions::default(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("not in manifest"));
    }

    #[test]
    fn rejects_missing_expected_when_strict() {
        let buffer = build_regular_archive(&[("a.txt", b"hi")]);
        let tmp = tempdir().unwrap();
        let err = safe_extract_tar_shard(
            &buffer,
            vec![fh("a.txt", 2), fh("b.txt", 5)],
            tmp.path(),
            &TarShardExtractOptions::default(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("missing expected"));
    }

    #[test]
    fn happy_path_returns_extracted_files_with_metadata() {
        let buffer = build_regular_archive(&[("a.txt", b"alpha"), ("b.txt", b"beta")]);
        let tmp = tempdir().unwrap();
        let mut h_a = fh("a.txt", 5);
        h_a.mtime_seconds = 1_577_836_800;
        h_a.permissions = 0o755;
        let h_b = fh("b.txt", 4);
        let extracted = safe_extract_tar_shard(
            &buffer,
            vec![h_a, h_b],
            tmp.path(),
            &TarShardExtractOptions::default(),
        )
        .unwrap();
        assert_eq!(extracted.len(), 2);
        let a = extracted.iter().find(|e| e.rel == "a.txt").unwrap();
        assert_eq!(a.contents, b"alpha");
        assert_eq!(a.size, 5);
        assert!(a.mtime.is_some());
        assert_eq!(a.permissions, Some(0o755));
        let b = extracted.iter().find(|e| e.rel == "b.txt").unwrap();
        assert_eq!(b.contents, b"beta");
        assert!(b.mtime.is_none());
    }

    #[test]
    fn write_extracted_file_applies_mtime_and_perms() {
        let tmp = tempdir().unwrap();
        let dest = tmp.path().join("written.txt");
        let f = ExtractedFile {
            rel: "written.txt".into(),
            dest_path: dest.clone(),
            contents: b"payload".to_vec(),
            mtime: Some(FileTime::from_unix_time(1_577_836_800, 0)),
            permissions: Some(0o600),
            size: 7,
        };
        write_extracted_file(&f).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"payload");
        let meta = std::fs::metadata(&dest).unwrap();
        assert_eq!(
            FileTime::from_last_modification_time(&meta).unix_seconds(),
            1_577_836_800
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(meta.permissions().mode() & 0o777, 0o600);
        }
    }

    #[test]
    fn rejects_traversal_path_via_tar_header_bytes() {
        // Hand-craft a malicious tar: regular entry with path
        // overwritten to ../escape.txt and checksum recomputed.
        let mut buffer = build_regular_archive(&[("aaaaaaaaa.txt", b"pwn")]);
        let bad_name = b"../escape.txt\0";
        buffer[..bad_name.len()].copy_from_slice(bad_name);
        let mut sum: u32 = 0;
        for (i, b) in buffer[..512].iter().enumerate() {
            if (148..156).contains(&i) {
                sum += 0x20;
            } else {
                sum += *b as u32;
            }
        }
        let chksum = format!("{:06o}\0 ", sum);
        buffer[148..156].copy_from_slice(chksum.as_bytes());

        let tmp = tempdir().unwrap();
        let dest = tmp.path().join("dest");
        std::fs::create_dir_all(&dest).unwrap();
        let err = safe_extract_tar_shard(
            &buffer,
            vec![fh("../escape.txt", 3)],
            &dest,
            &TarShardExtractOptions::default(),
        )
        .unwrap_err();
        assert!(err.to_string().to_lowercase().contains("validating"));
        assert!(!dest.parent().unwrap().join("escape.txt").exists());
    }
}
