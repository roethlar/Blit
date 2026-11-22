//! Resumable file copy with block-level hash comparison.
//!
//! Compares existing destination content block-by-block with source using Blake3 hashes,
//! only transferring blocks that differ. This allows resuming interrupted transfers
//! and efficiently updating files with partial changes.
//!
//! Using hash comparison (vs raw bytes) enables the same logic for remote transfers
//! where only hashes are exchanged over the network.

use eyre::{Context, Result};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Default block size for comparison (1 MiB).
pub const DEFAULT_BLOCK_SIZE: usize = 1024 * 1024;

/// Maximum block size for comparison (64 MiB) to prevent excessive memory usage.
pub const MAX_BLOCK_SIZE: usize = 64 * 1024 * 1024;

/// Outcome of a resume copy operation.
#[derive(Debug)]
pub struct ResumeCopyOutcome {
    /// Total bytes in the final file.
    pub total_bytes: u64,
    /// Bytes that were actually transferred (not skipped).
    pub bytes_transferred: u64,
    /// Number of blocks that matched and were skipped.
    pub blocks_skipped: u64,
    /// Number of blocks that were transferred.
    pub blocks_transferred: u64,
}

/// Compute Blake3 hash of a data block.
#[inline]
fn hash_block(data: &[u8]) -> blake3::Hash {
    blake3::hash(data)
}

/// Copy a file with resume capability using block-level hash comparison.
///
/// Compares source and destination block-by-block using Blake3 hashes:
/// - Matching blocks (same hash) are skipped
/// - Mismatched blocks are overwritten from source
/// - If source is longer, remaining bytes are appended
/// - If source is shorter, destination is truncated
///
/// This is efficient for:
/// - Resuming interrupted transfers (partial files)
/// - Updating files with localized changes
/// - Verifying and fixing corrupted copies
pub fn resume_copy_file(src: &Path, dst: &Path, block_size: usize) -> Result<ResumeCopyOutcome> {
    let mut src_file =
        File::open(src).with_context(|| format!("opening source: {}", src.display()))?;
    let src_len = src_file
        .metadata()
        .with_context(|| format!("reading source metadata: {}", src.display()))?
        .len();
    resume_copy_from(&mut src_file, src_len, dst, block_size)
}

/// The partial a resume patches, and how much of it there already is.
///
/// cr-ecm17-3, cr-ecm18-2, cr-ecm19-4 (Unix): a partial is PRIVATE and
/// writable by its owner (600) while it is worked on — whatever mode it
/// had, and whatever the mask it was made under — set through the handle
/// it is patched through before a byte of it is read or written
/// ([`make_partial_private`]). One made now is made so, for a reader who
/// opens a file keeps what it could open it for whatever its mode becomes
/// later. A destination there already — the file a resume exists to patch
/// in place — is patched in place (cr-ecm18-2 replaced every one others
/// could read, which is every ordinary file); only one whose mode this run
/// may not set is replaced by a fresh one. The source's own mode is given
/// when the copy completes (cr-ecm16-1). A link standing at the name is
/// followed, as it always was: a link to nothing makes what it names.
///
/// cr-ecm19-1: this is the acquisition. Until it returns, nothing is
/// this resume's: a caller arms its cleanup only once it has the file.
#[cfg(unix)]
pub fn open_resume_partial(dst: &Path) -> Result<(File, u64)> {
    use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _};
    let open = |options: &mut OpenOptions| options.read(true).write(true).mode(0o600).open(dst);
    // cr-ecm20-3: a partial there already that this run may read but not
    // write — left read-only by a copy of a read-only source that failed
    // after its mode was given, or by an older blit — is its owner's to
    // make writable again: it is had to be read, made private and
    // writable through that, and opened again to be written below.
    let (file, writable) = match open(OpenOptions::new().create(true).truncate(false)) {
        Ok(file) => (file, true),
        Err(refused) if refused.kind() == std::io::ErrorKind::PermissionDenied => {
            match File::open(dst) {
                Ok(file) => (file, false),
                Err(_) => {
                    return Err(refused)
                        .with_context(|| format!("opening destination: {}", dst.display()))
                }
            }
        }
        Err(error) => {
            return Err(error).with_context(|| format!("opening destination: {}", dst.display()))
        }
    };
    while_a_partial_is_opened(dst);
    if make_partial_private(&file, dst).with_context(|| {
        format!(
            "making the partial {} private, or replacing it",
            dst.display()
        )
    })? {
        let file = if writable {
            file
        } else {
            // The same file, now writable, or nothing is had.
            let again = open(&mut OpenOptions::new())
                .with_context(|| format!("opening destination: {}", dst.display()))?;
            let (was, now) = (file.metadata()?, again.metadata()?);
            if (was.dev(), was.ino()) != (now.dev(), now.ino()) {
                eyre::bail!(
                    "the partial {} changed while it was made writable",
                    dst.display()
                );
            }
            again
        };
        let len = file.metadata()?.len();
        return Ok((file, len));
    }
    let fresh = open(OpenOptions::new().create_new(true))
        .with_context(|| format!("making the partial {}", dst.display()))?;
    make_partial_private(&fresh, dst)
        .with_context(|| format!("making the partial {} private", dst.display()))?;
    Ok((fresh, 0))
}

/// cr-ecm19-4: make the partial `file`, at `dst`, private and writable by
/// its owner (600) through that handle. `Ok(false)`: this run may not set
/// its mode — it is not the file's owner — and it is not reused: the name
/// is removed, for a fresh partial to be made there. An error: it could be
/// neither made private nor removed.
#[cfg(unix)]
pub fn make_partial_private(file: &File, dst: &Path) -> std::io::Result<bool> {
    use std::os::unix::fs::PermissionsExt as _;
    match file.set_permissions(std::fs::Permissions::from_mode(0o600)) {
        Ok(()) => Ok(true),
        Err(not_its_own) if not_its_own.kind() == std::io::ErrorKind::PermissionDenied => {
            std::fs::remove_file(dst)?;
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

/// Test seam (cr-ecm19-4): runs once a resume's partial is open and before
/// its mode is set — when a reader could open a file made with the usual
/// mask. Keyed by path. Production never sets it.
#[cfg(all(test, unix))]
pub(crate) type PartialOpenedHook = std::sync::Arc<dyn Fn(&Path) + Send + Sync>;

#[cfg(all(test, unix))]
pub(crate) static WHILE_A_PARTIAL_IS_OPENED_HOOKS: std::sync::Mutex<
    Vec<(std::path::PathBuf, PartialOpenedHook)>,
> = std::sync::Mutex::new(Vec::new());

#[cfg(unix)]
fn while_a_partial_is_opened(_dst: &Path) {
    #[cfg(test)]
    {
        let hooks: Vec<PartialOpenedHook> = WHILE_A_PARTIAL_IS_OPENED_HOOKS
            .lock()
            .map(|hooks| {
                hooks
                    .iter()
                    .filter(|(at, _)| at == _dst)
                    .map(|(_, hook)| std::sync::Arc::clone(hook))
                    .collect()
            })
            .unwrap_or_default();
        for hook in hooks {
            hook(_dst);
        }
    }
}

/// The partial a resume patches, and how much of it there already is.
#[cfg(not(unix))]
pub fn open_resume_partial(dst: &Path) -> Result<(File, u64)> {
    let dst_len = std::fs::metadata(dst).map(|m| m.len()).unwrap_or(0);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dst)
        .with_context(|| format!("opening destination: {}", dst.display()))?;
    Ok((file, dst_len))
}

/// [`resume_copy_file`] on an already-OPENED source handle (ssc-4,
/// SOURCE_SIDE_CONTAINMENT D-C): the caller validated `src_len` against
/// the manifest on this same handle and re-stats it afterwards; the
/// diff reads exactly `src_len` bytes from it and never re-opens a path.
/// The destination is patched in place (D-2026-07-09-1 Q2).
pub fn resume_copy_from(
    src_file: &mut File,
    src_len: u64,
    dst: &Path,
    block_size: usize,
) -> Result<ResumeCopyOutcome> {
    // Ensure parent directory exists
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating parent directory: {}", parent.display()))?;
    }

    let (dst_file, dst_len) = open_resume_partial(dst)?;
    resume_copy_into(src_file, src_len, dst_file, dst_len, dst, block_size)
}

/// [`resume_copy_from`] into a partial already had (cr-ecm19-1):
/// `dst_file`, `dst_len` bytes long, from [`open_resume_partial`]. `dst`
/// names it in what is said.
pub fn resume_copy_into(
    src_file: &mut File,
    src_len: u64,
    mut dst_file: File,
    dst_len: u64,
    dst: &Path,
    block_size: usize,
) -> Result<ResumeCopyOutcome> {
    src_file.seek(SeekFrom::Start(0))?;

    // cr-win-1: while blocks are patched in place the destination must
    // never be the source's size — an interruption would otherwise leave a
    // half-patched file every compare takes for a finished copy (a
    // same-size file patched in place keeps its size, and its mtime is the
    // write time). Hold it one byte longer until the patch completes; the
    // comparison below still reads the original `dst_len` bytes.
    if dst_len <= src_len {
        dst_file
            .set_len(src_len + 1)
            .with_context(|| format!("marking {} unfinished", dst.display()))?;
    }

    let block_size = if block_size == 0 {
        DEFAULT_BLOCK_SIZE
    } else {
        block_size.min(MAX_BLOCK_SIZE)
    };
    let mut src_buf = vec![0u8; block_size];
    let mut dst_buf = vec![0u8; block_size];

    let mut offset = 0u64;
    let mut bytes_transferred = 0u64;
    let mut blocks_skipped = 0u64;
    let mut blocks_transferred = 0u64;
    // audit-14: track the destination cursor so we only `seek` when it
    // isn't already at the target offset. The source is read purely
    // sequentially (offset advances by exactly this_block each iteration),
    // so it never needs a seek at all. Both files are opened with their
    // cursor at 0, matching `offset == 0` / `dst_cursor_pos == 0` on the
    // first iteration.
    let mut dst_cursor_pos = 0u64;

    while offset < src_len {
        let remaining = src_len - offset;
        let this_block = remaining.min(block_size as u64) as usize;

        // Read source block and compute hash. No seek: src is sequential.
        src_file.read_exact(&mut src_buf[..this_block])?;
        let src_hash = hash_block(&src_buf[..this_block]);

        // Check if we can compare with destination
        let should_write = if offset < dst_len {
            let dst_available = (dst_len - offset).min(this_block as u64) as usize;

            if dst_available == this_block {
                // Full block available, read and hash (seek only if the
                // dst cursor isn't already aligned to this offset).
                if dst_cursor_pos != offset {
                    dst_file.seek(SeekFrom::Start(offset))?;
                }
                dst_file.read_exact(&mut dst_buf[..this_block])?;
                dst_cursor_pos = offset + this_block as u64;
                let dst_hash = hash_block(&dst_buf[..this_block]);
                src_hash != dst_hash
            } else {
                // Partial block at end of dest, need to write
                true
            }
        } else {
            // Beyond destination, definitely need to write
            true
        };

        if should_write {
            // The preceding read_exact (or a jump past dst_len) may have
            // left the cursor off `offset`; seek back only if so.
            if dst_cursor_pos != offset {
                dst_file.seek(SeekFrom::Start(offset))?;
            }
            dst_file.write_all(&src_buf[..this_block])?;
            dst_cursor_pos = offset + this_block as u64;
            bytes_transferred += this_block as u64;
            blocks_transferred += 1;
        } else {
            blocks_skipped += 1;
        }

        offset += this_block as u64;
    }

    // Truncate to the source's length: an older destination that was
    // longer, or the one-byte unfinished mark set above.
    dst_file.set_len(src_len)?;

    // cr-ecm16-1: the source's mode, through this writer's own handle,
    // as every other copy on this machine gives it (local-copy-mode). A
    // resumed copy kept the mode its destination was created with — 644
    // under the usual mask — or the one a partial left before had: a
    // private file became readable by all.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = src_file.metadata()?.permissions().mode();
        if dst_file
            .set_permissions(std::fs::Permissions::from_mode(mode))
            .is_err()
        {
            log::debug!("Could not preserve permissions for {}", dst.display());
        }
    }

    // Sync to disk
    dst_file.sync_all()?;

    Ok(ResumeCopyOutcome {
        total_bytes: src_len,
        bytes_transferred,
        blocks_skipped,
        blocks_transferred,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// cr-win-1: blocks patch the destination in place, so an interrupted
    /// resume of a same-size file kept the source's size with a write-time
    /// mtime and passed for finished on every later compare. It stays one
    /// byte longer than the source until the patch completes.
    #[test]
    fn an_interrupted_resume_never_leaves_the_source_size() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");
        std::fs::write(&src, vec![1u8; 10_000])?;
        std::fs::write(&dst, vec![2u8; 12_000])?;
        // The source claims 12,000 bytes but holds 10,000: the read fails
        // mid-patch, after the blocks before it already landed.
        let mut src_file = File::open(&src)?;
        assert!(resume_copy_from(&mut src_file, 12_000, &dst, 1024).is_err());
        assert_eq!(std::fs::metadata(&dst)?.len(), 12_001);
        Ok(())
    }

    /// cr-ecm17-3: a resume that fails part-way leaves nothing of its
    /// source readable beyond what its source allows — not in a partial it
    /// made, not in one left before with a wider mode. It left 1 KiB of a
    /// private source in a partial readable by all.
    #[cfg(unix)]
    #[test]
    fn an_interrupted_resume_leaves_nothing_readable_beyond_its_source() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempdir()?;
        let src = tmp.path().join("private");
        std::fs::write(&src, vec![9u8; 10_000])?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600))?;
        let mode_of = |path: &Path| -> Result<u32> {
            Ok(std::fs::metadata(path)?.permissions().mode() & 0o7777)
        };
        let fresh = tmp.path().join("fresh");
        let left = tmp.path().join("left");
        std::fs::write(&left, vec![0u8; 512])?;
        std::fs::set_permissions(&left, std::fs::Permissions::from_mode(0o644))?;
        for dst in [&fresh, &left] {
            // The source claims more than it holds: the read fails after
            // blocks of it have landed.
            assert!(resume_copy_from(&mut File::open(&src)?, 12_000, dst, 1024).is_err());
            assert!(
                std::fs::read(dst)?.contains(&9),
                "some of the source landed in {}",
                dst.display()
            );
            assert_eq!(
                mode_of(dst)? & 0o077,
                0,
                "{} is readable beyond its owner: {:o}",
                dst.display(),
                mode_of(dst)?
            );
        }
        Ok(())
    }

    /// cr-ecm18-2: a partial is private and writable by its owner while it
    /// is worked on, whatever its source's mode — a read-only source's
    /// partial was made read-only, and no retry could write to it — and has
    /// its source's mode once the copy completes.
    #[cfg(unix)]
    #[test]
    fn a_partial_is_private_and_writable_while_it_is_worked_on() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempdir()?;
        let mode_of = |path: &Path| -> Result<u32> {
            Ok(std::fs::metadata(path)?.permissions().mode() & 0o777)
        };
        for mode in [0o644, 0o444] {
            let src = tmp.path().join(format!("src-{mode:o}"));
            std::fs::write(&src, vec![3u8; 10_000])?;
            std::fs::set_permissions(&src, std::fs::Permissions::from_mode(mode))?;
            let dst = tmp.path().join(format!("dst-{mode:o}"));
            // Interrupted: the source claims more than it holds.
            assert!(resume_copy_from(&mut File::open(&src)?, 12_000, &dst, 1024).is_err());
            assert_eq!(mode_of(&dst)?, 0o600, "{mode:o}: while it is worked on");
            // The retry writes to it, and gives the source's mode at the end.
            resume_copy_from(&mut File::open(&src)?, 10_000, &dst, 1024)?;
            assert_eq!(std::fs::read(&dst)?, vec![3u8; 10_000]);
            assert_eq!(mode_of(&dst)?, mode, "{mode:o}: once it is done");
        }
        Ok(())
    }

    /// cr-ecm19-4: a destination there already — the file a resume
    /// exists to patch in place — is patched in place: the same file, the
    /// blocks it shares with the source skipped. It was replaced, and
    /// copied whole, whenever others could read it, which is every
    /// ordinary file. While it is worked on it is private (600); done, it
    /// has its source's mode.
    #[cfg(unix)]
    #[test]
    fn an_existing_destination_is_patched_in_place_and_private_meanwhile() -> Result<()> {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let data: Vec<u8> = (0..8192u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&src, &data)?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o644))?;
        let dst = tmp.path().join("dst");
        std::fs::write(&dst, &data[..4096])?;
        std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o644))?;
        let before = std::fs::metadata(&dst)?.ino();
        // Interrupted: the source claims more than it holds.
        assert!(resume_copy_from(&mut File::open(&src)?, 9000, &dst, 1024).is_err());
        let meta = std::fs::metadata(&dst)?;
        assert_eq!(meta.ino(), before, "patched in place");
        assert_eq!(
            meta.permissions().mode() & 0o777,
            0o600,
            "private meanwhile"
        );
        let outcome = resume_copy_from(&mut File::open(&src)?, data.len() as u64, &dst, 1024)?;
        assert!(outcome.blocks_skipped > 0, "{outcome:?}");
        let meta = std::fs::metadata(&dst)?;
        assert_eq!(meta.ino(), before, "patched in place");
        assert_eq!(std::fs::read(&dst)?, data);
        assert_eq!(
            meta.permissions().mode() & 0o777,
            0o644,
            "its source's mode once done"
        );
        Ok(())
    }

    /// cr-ecm20-3: a partial left read-only — by a copy of a read-only
    /// source that failed once its mode was given, or by an older blit —
    /// is its owner's: it is made writable again, through what holds it,
    /// and resumed in place. Its retry failed with `Permission denied`.
    #[cfg(unix)]
    #[test]
    fn a_partial_left_read_only_is_made_writable_and_resumed() -> Result<()> {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let data: Vec<u8> = (0..8192u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&src, &data)?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o644))?;
        let dst = tmp.path().join("dst");
        std::fs::write(&dst, &data[..4096])?;
        std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o444))?;
        let before = std::fs::metadata(&dst)?.ino();
        let outcome = resume_copy_from(&mut File::open(&src)?, data.len() as u64, &dst, 1024)?;
        assert!(outcome.blocks_skipped > 0, "{outcome:?}");
        let meta = std::fs::metadata(&dst)?;
        assert_eq!(meta.ino(), before, "resumed in place");
        assert_eq!(std::fs::read(&dst)?, data);
        assert_eq!(meta.permissions().mode() & 0o777, 0o644);
        Ok(())
    }

    /// cr-ecm19-4: a link standing at the name is followed, as it always
    /// was — a link to nothing makes what it names (it failed: the name
    /// was no file) — and what a link names is private while it is worked
    /// on, like any partial. The link stays.
    #[cfg(unix)]
    #[test]
    fn a_resume_through_a_link_works_on_what_it_names() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        std::fs::write(&src, vec![5u8; 3000])?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o644))?;
        let to_nothing = tmp.path().join("to-nothing");
        std::os::unix::fs::symlink("made", &to_nothing)?;
        resume_copy_from(&mut File::open(&src)?, 3000, &to_nothing, 1024)?;
        assert!(std::fs::symlink_metadata(&to_nothing)?
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read(tmp.path().join("made"))?, vec![5u8; 3000]);

        let named = tmp.path().join("named");
        std::fs::write(&named, vec![1u8; 1000])?;
        std::fs::set_permissions(&named, std::fs::Permissions::from_mode(0o644))?;
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink("named", &link)?;
        assert!(resume_copy_from(&mut File::open(&src)?, 4000, &link, 1024).is_err());
        assert!(std::fs::symlink_metadata(&link)?.file_type().is_symlink());
        assert_eq!(
            std::fs::metadata(&named)?.permissions().mode() & 0o777,
            0o600,
            "what it names, while it is worked on"
        );
        Ok(())
    }

    /// cr-ecm19-4: a partial made now is private from the moment it
    /// exists — made so, before its mode is set through the handle — for a
    /// reader who opens it then keeps what it could open it for.
    #[cfg(unix)]
    #[test]
    fn a_partial_made_now_is_never_open_to_others() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;
        use std::sync::{Arc, Mutex};
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        std::fs::write(&src, vec![5u8; 100])?;
        let dst = tmp.path().join("fresh");
        let seen: Arc<Mutex<Option<u32>>> = Arc::default();
        {
            let seen = Arc::clone(&seen);
            WHILE_A_PARTIAL_IS_OPENED_HOOKS.lock().unwrap().push((
                dst.clone(),
                Arc::new(move |at: &Path| {
                    *seen.lock().unwrap() =
                        Some(std::fs::metadata(at).unwrap().permissions().mode() & 0o777);
                }),
            ));
        }
        let result = resume_copy_from(&mut File::open(&src)?, 100, &dst, 1024);
        WHILE_A_PARTIAL_IS_OPENED_HOOKS
            .lock()
            .unwrap()
            .retain(|(at, _)| at != &dst);
        result?;
        let made_as = seen.lock().unwrap().expect("the seam ran");
        assert_eq!(made_as & 0o077, 0, "made as {made_as:o}");
        Ok(())
    }

    /// cr-ecm16-1: a resumed copy has its source's mode — into a file it
    /// makes, and into a partial left before with another mode.
    #[cfg(unix)]
    #[test]
    fn a_resumed_copy_has_its_sources_mode() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempdir()?;
        let data = vec![7u8; 3 * 1024 + 5];
        let mode_of = |path: &Path| -> Result<u32> {
            Ok(std::fs::metadata(path)?.permissions().mode() & 0o7777)
        };
        for mode in [0o600, 0o640, 0o750] {
            let src = tmp.path().join(format!("src-{mode:o}"));
            std::fs::write(&src, &data)?;
            std::fs::set_permissions(&src, std::fs::Permissions::from_mode(mode))?;

            let fresh = tmp.path().join(format!("fresh-{mode:o}"));
            resume_copy_from(&mut File::open(&src)?, data.len() as u64, &fresh, 1024)?;
            assert_eq!(mode_of(&fresh)?, mode, "into a new file, {mode:o}");
            assert_eq!(std::fs::read(&fresh)?, data);

            let partial = tmp.path().join(format!("partial-{mode:o}"));
            std::fs::write(&partial, &data[..1024])?;
            std::fs::set_permissions(&partial, std::fs::Permissions::from_mode(0o644))?;
            resume_copy_from(&mut File::open(&src)?, data.len() as u64, &partial, 1024)?;
            assert_eq!(mode_of(&partial)?, mode, "into a partial, {mode:o}");
            assert_eq!(std::fs::read(&partial)?, data);
        }
        Ok(())
    }

    #[test]
    fn test_resume_new_file() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");

        // Create source file
        let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&src, &data)?;

        // Copy to new destination
        let outcome = resume_copy_file(&src, &dst, 1024)?;

        assert_eq!(outcome.total_bytes, 10000);
        assert_eq!(outcome.bytes_transferred, 10000);
        assert_eq!(outcome.blocks_skipped, 0);
        assert_eq!(std::fs::read(&dst)?, data);

        Ok(())
    }

    #[test]
    fn test_resume_partial_file() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");

        // Create source file (10KB)
        let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&src, &data)?;

        // Create partial destination (first 5KB)
        std::fs::write(&dst, &data[..5000])?;

        // Resume copy
        let outcome = resume_copy_file(&src, &dst, 1024)?;

        assert_eq!(outcome.total_bytes, 10000);
        // First 4 blocks (4KB) should be skipped, rest transferred
        assert!(outcome.blocks_skipped >= 4);
        assert_eq!(std::fs::read(&dst)?, data);

        Ok(())
    }

    #[test]
    fn test_resume_identical_file() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");

        // Create identical files
        let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&src, &data)?;
        std::fs::write(&dst, &data)?;

        // Resume should skip all blocks
        let outcome = resume_copy_file(&src, &dst, 1024)?;

        assert_eq!(outcome.total_bytes, 10000);
        assert_eq!(outcome.bytes_transferred, 0);
        assert_eq!(outcome.blocks_transferred, 0);

        Ok(())
    }

    #[test]
    fn test_resume_truncates_longer_dest() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");

        // Create source (5KB)
        let src_data: Vec<u8> = (0..5000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&src, &src_data)?;

        // Create longer destination (10KB)
        let dst_data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&dst, &dst_data)?;

        // Resume should truncate
        let outcome = resume_copy_file(&src, &dst, 1024)?;

        assert_eq!(outcome.total_bytes, 5000);
        let result = std::fs::read(&dst)?;
        assert_eq!(result.len(), 5000);
        assert_eq!(result, src_data);

        Ok(())
    }

    #[test]
    fn test_resume_corrupted_block() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");

        // Create source (10KB)
        let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&src, &data)?;

        // Create destination with corrupted middle block
        let mut corrupted = data.clone();
        for byte in &mut corrupted[3000..4000] {
            *byte = 0xFF;
        }
        std::fs::write(&dst, &corrupted)?;

        // Resume should fix the corrupted block
        let outcome = resume_copy_file(&src, &dst, 1024)?;

        assert_eq!(outcome.total_bytes, 10000);
        assert!(outcome.blocks_transferred >= 1); // At least the corrupted block
        assert_eq!(std::fs::read(&dst)?, data);

        Ok(())
    }
}
