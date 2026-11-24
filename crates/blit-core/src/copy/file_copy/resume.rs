//! Resumable file copy with block-level hash comparison.
//!
//! Compares existing destination content block-by-block with source using Blake3 hashes,
//! only transferring blocks that differ. This allows resuming interrupted transfers
//! and efficiently updating files with partial changes.
//!
//! Using hash comparison (vs raw bytes) enables the same logic for remote transfers
//! where only hashes are exchanged over the network.

use eyre::{Context, Result};
use std::fs::File;
#[cfg(not(unix))]
use std::fs::OpenOptions;
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
    // cr-ecm33-6: without waiting, and only a regular file.
    let mut src_file = crate::copy::open_regular(src)
        .with_context(|| format!("opening source: {}", src.display()))?;
    let src_len = src_file
        .metadata()
        .with_context(|| format!("reading source metadata: {}", src.display()))?
        .len();
    resume_copy_from(&mut src_file, src_len, dst, block_size)
}

/// Test seam (cr-ecm26-3): a read of a resumed destination's block fails, by
/// its path and offset — an `EIO` no test can make for real. Production
/// never sets it.
fn dst_read_fault(_dst: &Path, _offset: u64) -> std::io::Result<()> {
    #[cfg(test)]
    if DST_READ_FAULTS.lock().is_ok_and(|faults| {
        faults
            .iter()
            .any(|(at, offset)| at == _dst && *offset == _offset)
    }) {
        return Err(std::io::Error::other("an input/output error (a test's)"));
    }
    Ok(())
}

/// The blocks a test has [`dst_read_fault`] fail.
#[cfg(test)]
pub(crate) static DST_READ_FAULTS: std::sync::Mutex<Vec<(std::path::PathBuf, u64)>> =
    std::sync::Mutex::new(Vec::new());

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
///
/// cr-ecm21-3: and one that grants others what its source (`source_mode`)
/// does not is not reused either — a reader who has it open would read
/// the source's bytes as they land: it is removed, and a fresh partial
/// made at the empty name ([`fresh_partial`]; in place, D-2026-11-23-1).
#[cfg(unix)]
pub fn open_resume_partial(
    dst: &Path,
    source_mode: Option<u32>,
    src_len: u64,
    within: Option<&super::Within>,
) -> Result<(File, u64)> {
    open_resume_partial_with(dst, source_mode, src_len, within, false)
        .map(|(file, _, len)| (file, len))
}

/// cr-ecm24-2: [`open_resume_partial`], with a second handle to the partial
/// for the caller's cleanup — had before anything of the destination is
/// changed — and a fresh partial's while it is made ready, before the
/// caller has it: refused, the partial is removed. Taken after, a handle
/// the system refused (`EMFILE`) left a fresh partial in the destination's
/// place, and nothing held to undo it.
#[cfg(unix)]
pub fn open_resume_partial_held(
    dst: &Path,
    source_mode: Option<u32>,
    src_len: u64,
    within: Option<&super::Within>,
) -> Result<(File, File, u64)> {
    let (file, cleanup, len) = open_resume_partial_with(dst, source_mode, src_len, within, true)?;
    let cleanup = cleanup.ok_or_else(|| eyre::eyre!("no second handle to {}", dst.display()))?;
    Ok((file, cleanup, len))
}

#[cfg(unix)]
fn open_resume_partial_with(
    dst: &Path,
    source_mode: Option<u32>,
    src_len: u64,
    within: Option<&super::Within>,
    cleanup: bool,
) -> Result<(File, Option<File>, u64)> {
    use std::os::unix::fs::MetadataExt as _;
    // cr-ecm24-1: had where the chain of links at the name ends — checked
    // against the destination, in the directory held — never by the name:
    // opened by it to be made, a link to a name outside the destination
    // was followed out and the file made there.
    let resolved = resolve_destination(dst, within)?;
    let keep = |file: &File| cleanup.then(|| keep_handle(file, dst)).transpose();
    // cr-ecm29-2: and what a fresh partial may replace is what was judged:
    // what the look found, or — once it is opened — what was opened. Looked
    // at again when the partial was made, a file come there since was taken
    // for it and replaced.
    let fresh = |resolved: &Resolved, seen: Entry| -> Result<(File, Option<File>, u64)> {
        let (file, kept) = resolved.fresh(dst, src_len.saturating_add(1), seen, |file| {
            keep(file).map_err(eyre::Report::new)
        })?;
        Ok((file, kept, 0))
    };
    // cr-ecm22-4, cr-ecm23-4: a file is reused only when this run's user
    // owns it and it grants others nothing its source does not.
    let reusable = |mode: u32, owner: u32| {
        let wider = source_mode.is_some_and(|source| mode & 0o077 & !(source & 0o077) != 0);
        !wider && owner == effective_user(dst)
    };
    // cr-ecm25-5: asked of what is seen at the name BEFORE it is opened —
    // a file that may not be reused is replaced whole unopened — and one
    // this run can neither read nor write is replaced whole too, as the
    // wire does (cr-ecm24-4). Opened first, a destination this run could
    // not read failed the copy.
    let seen = resolved
        .look()
        .with_context(|| format!("looking at {}", dst.display()))?;
    after_a_destination_is_looked_at(dst);
    let seen = match seen {
        // cr-ecm26-1: nothing there: the partial is made exclusively from
        // this look on. An open after it could adopt a file made in between.
        None => {
            let (file, kept) =
                resolved.fresh_exclusive(dst, src_len.saturating_add(1), |file| {
                    keep(file).map_err(eyre::Report::new)
                })?;
            while_a_partial_is_opened(dst);
            return Ok((file, kept, 0));
        }
        // cr-ecm26-5: and what is not a regular file — a FIFO, a socket, a
        // device — is not opened: it is replaced whole (a directory there
        // refuses the rename). Opened, a FIFO this run could only read
        // waited for a writer.
        // cr-ecm27-3: a link at the name — come there since the chain was
        // followed — is left as it is, and the file fails: replaced, the
        // link was overwritten.
        Some(seen) if is_link(seen.mode) => {
            eyre::bail!(
                "{} became a link while it was being had; it is left as it is",
                dst.display()
            );
        }
        Some(seen) if !is_regular(seen.mode) || !reusable(seen.mode, seen.owner) => {
            return fresh(&resolved, seen);
        }
        Some(seen) => seen,
    };
    // cr-ecm25-4: where nothing stands, the partial is made only while
    // nothing has the name — private, at its length, its cleanup's handle
    // had — and removed if any of that is refused. Opened by the name to be
    // made, an empty file stood there before its cleanup's handle was had,
    // and stayed when that was refused.
    // cr-ecm27-3: no open waits — a FIFO put in the file's place after the
    // look cannot hold it; what was opened is checked below.
    let first = match resolved.open(libc::O_RDWR | libc::O_NONBLOCK) {
        Err(absent) if absent.kind() == std::io::ErrorKind::NotFound => {
            let (file, kept) =
                resolved.fresh_exclusive(dst, src_len.saturating_add(1), |file| {
                    keep(file).map_err(eyre::Report::new)
                })?;
            while_a_partial_is_opened(dst);
            return Ok((file, kept, 0));
        }
        opened => opened,
    };
    // cr-ecm20-3: a partial there already that this run may read but not
    // write — left read-only by a copy of a read-only source that failed
    // after its mode was given, or by an older blit — is its owner's to
    // make writable again: it is had to be read, made private and
    // writable through that, and opened again to be written below.
    let (file, writable) = match first {
        Ok(file) => (file, true),
        Err(refused) if refused.kind() == std::io::ErrorKind::PermissionDenied => {
            match resolved.open(libc::O_RDONLY | libc::O_NONBLOCK) {
                Ok(file) => (file, false),
                Err(unread) if unread.kind() == std::io::ErrorKind::PermissionDenied => {
                    return fresh(&resolved, seen);
                }
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
    let found = file.metadata()?;
    // cr-ecm26-5: and what was opened is a regular file, or it is replaced.
    if !found.is_file() {
        drop(file);
        return fresh(&resolved, Entry::of(&found));
    }
    // cr-ecm22-4: a source of mode 0000 is a mode known, and the most
    // private of all — it was taken for no mode, and the destination's
    // readers kept it. cr-ecm23-4: and only a file this run's user owns is
    // reused: one another owns stays theirs, readable by them whatever its
    // mode. Asked again of the file opened, which is what is patched.
    // cr-ecm28-5: and only while it has one name: another — a hard link, or
    // the staged name a failed publication left — would read what lands in
    // it. Patched in place on a retry, both names held the new bytes.
    if found.nlink() == 1 && reusable(found.mode(), found.uid()) {
        // cr-ecm25-4: the cleanup's handle is had before the file is changed:
        // had after it was made private, a refusal left a file others could
        // read made 600.
        let kept_early = if writable {
            keep(&file).with_context(|| format!("holding the partial {}", dst.display()))?
        } else {
            None
        };
        if make_partial_private(&file)
            .with_context(|| format!("making the partial {} private", dst.display()))?
        {
            // cr-ecm26-4, cr-ecm27-4: made private (and writable) to be had,
            // and until it is had — to its last look — any failure puts its
            // mode back through the handle that changed it. A writable reopen
            // refused left a 0444 file 0600, and so did a last look that
            // failed.
            let undo = |error: eyre::Report| {
                use std::os::unix::fs::PermissionsExt as _;
                let _ =
                    file.set_permissions(std::fs::Permissions::from_mode(found.mode() & 0o7777));
                error
            };
            let (again, kept) = if writable {
                (None, kept_early)
            } else {
                // The same file, now writable, or nothing is had.
                let reopened = reopen_fault(dst)
                    .and_then(|()| resolved.open(libc::O_RDWR | libc::O_NONBLOCK))
                    .with_context(|| format!("opening destination: {}", dst.display()));
                let again = reopened.map_err(undo)?;
                let same = file.metadata().and_then(|was| {
                    again
                        .metadata()
                        .map(|now| (was.dev(), was.ino()) == (now.dev(), now.ino()))
                });
                if !matches!(same, Ok(true)) {
                    return Err(undo(eyre::eyre!(
                        "the partial {} changed while it was made writable",
                        dst.display()
                    )));
                }
                let kept = keep(&again)
                    .with_context(|| format!("holding the partial {}", dst.display()))
                    .map_err(undo)?;
                (Some(again), kept)
            };
            let len = len_fault(dst)
                .and_then(|()| again.as_ref().unwrap_or(&file).metadata())
                .map(|meta| meta.len())
                .with_context(|| format!("looking at the partial {}", dst.display()))
                .map_err(undo)?;
            return Ok((again.unwrap_or(file), kept, len));
        }
    }
    drop(file);
    fresh(&resolved, Entry::of(&found))
}

/// Whether `mode` (type bits included) is a regular file's.
#[cfg(unix)]
#[allow(clippy::unnecessary_cast)]
fn is_regular(mode: u32) -> bool {
    mode & libc::S_IFMT as u32 == libc::S_IFREG as u32
}

/// Whether `mode` (type bits included) is a directory's.
#[cfg(unix)]
#[allow(clippy::unnecessary_cast)]
fn is_directory(mode: u32) -> bool {
    mode & libc::S_IFMT as u32 == libc::S_IFDIR as u32
}

/// Whether `mode` (type bits included) is a link's.
#[cfg(unix)]
#[allow(clippy::unnecessary_cast)]
pub(crate) fn is_link(mode: u32) -> bool {
    mode & libc::S_IFMT as u32 == libc::S_IFLNK as u32
}

/// A second handle to `file`, the partial for `dst`. Test seam
/// (cr-ecm24-2): a test may have it refused, by path — a process out of
/// descriptors (`EMFILE`), which no test makes for real. Production never
/// sets it.
#[cfg(unix)]
fn keep_handle(file: &File, _dst: &Path) -> std::io::Result<File> {
    #[cfg(test)]
    if KEEP_FAULTS
        .lock()
        .is_ok_and(|faults| faults.iter().any(|at| at == _dst))
    {
        return Err(std::io::Error::from_raw_os_error(libc::EMFILE));
    }
    file.try_clone()
}

/// Test seam (cr-ecm26-1): runs once what stands at a resume's name has been
/// looked at and before anything is opened or made — for a test to put a
/// file there in between. Keyed by path. Production never sets it.
#[cfg(all(test, unix))]
pub(crate) static AFTER_A_LOOK_HOOKS: std::sync::Mutex<
    Vec<(std::path::PathBuf, PartialOpenedHook)>,
> = std::sync::Mutex::new(Vec::new());

#[cfg(unix)]
fn after_a_destination_is_looked_at(_dst: &Path) {
    #[cfg(test)]
    {
        let hooks: Vec<PartialOpenedHook> = AFTER_A_LOOK_HOOKS
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

/// Test seam (cr-ecm31-3): a fresh partial's first look at itself refused
/// (`EIO`), by path. Production never sets it.
#[cfg(all(test, unix))]
pub(crate) static FRESH_LOOK_FAULTS: std::sync::Mutex<Vec<std::path::PathBuf>> =
    std::sync::Mutex::new(Vec::new());

#[cfg(unix)]
fn fresh_look_fault(_dst: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    if FRESH_LOOK_FAULTS
        .lock()
        .is_ok_and(|faults| faults.iter().any(|at| at == _dst))
    {
        return Err(std::io::Error::from_raw_os_error(libc::EIO));
    }
    Ok(())
}

/// Test seam (cr-ecm26-4): a read-only partial's writable reopen refused,
/// by path — a process out of descriptors (`EMFILE`). Production never
/// sets it.
#[cfg(unix)]
fn reopen_fault(_dst: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    if REOPEN_FAULTS
        .lock()
        .is_ok_and(|faults| faults.iter().any(|at| at == _dst))
    {
        return Err(std::io::Error::from_raw_os_error(libc::EMFILE));
    }
    Ok(())
}

/// Test seam (cr-ecm27-4): the last look at a partial being had fails, by
/// path — an `ESTALE`. Production never sets it.
#[cfg(unix)]
fn len_fault(_dst: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    if LEN_FAULTS
        .lock()
        .is_ok_and(|faults| faults.iter().any(|at| at == _dst))
    {
        return Err(std::io::Error::from_raw_os_error(libc::ESTALE));
    }
    Ok(())
}

/// The paths a test has [`len_fault`] fail.
#[cfg(all(test, unix))]
pub(crate) static LEN_FAULTS: std::sync::Mutex<Vec<std::path::PathBuf>> =
    std::sync::Mutex::new(Vec::new());

/// The paths a test has [`reopen_fault`] refuse.
#[cfg(all(test, unix))]
pub(crate) static REOPEN_FAULTS: std::sync::Mutex<Vec<std::path::PathBuf>> =
    std::sync::Mutex::new(Vec::new());

/// The paths a test has [`keep_handle`] refused for.
#[cfg(all(test, unix))]
pub(crate) static KEEP_FAULTS: std::sync::Mutex<Vec<std::path::PathBuf>> =
    std::sync::Mutex::new(Vec::new());

/// The user this run is to the system — its effective user id. Test seam
/// (cr-ecm23-4): a test may make the run another user for one path — root,
/// which no test runs as. Production never sets it.
#[cfg(unix)]
pub(crate) fn effective_user(_at: &Path) -> u32 {
    #[cfg(test)]
    if let Some(user) = RUN_AS.lock().ok().and_then(|run_as| {
        run_as
            .iter()
            .find(|(at, _)| at == _at)
            .map(|(_, user)| *user)
    }) {
        return user;
    }
    // SAFETY: `geteuid` has no preconditions and cannot fail.
    unsafe { libc::geteuid() }
}

/// The paths a test runs as another user for ([`effective_user`]).
#[cfg(all(test, unix))]
pub(crate) static RUN_AS: std::sync::Mutex<Vec<(std::path::PathBuf, u32)>> =
    std::sync::Mutex::new(Vec::new());

/// cr-ecm19-4: make the partial `file` private and writable by its owner
/// (600) through that handle. `Ok(false)`: this run may not set its mode —
/// it is not the file's owner — and it is not to be reused.
#[cfg(unix)]
pub fn make_partial_private(file: &File) -> std::io::Result<bool> {
    use std::os::unix::fs::PermissionsExt as _;
    match file.set_permissions(std::fs::Permissions::from_mode(0o600)) {
        Ok(()) => Ok(true),
        Err(not_its_own) if not_its_own.kind() == std::io::ErrorKind::PermissionDenied => Ok(false),
        Err(error) => Err(error),
    }
}

/// cr-ecm21-3, D-2026-11-23-1: a FRESH private partial for `dst`, made in
/// place, as every write to a destination is (D-2026-09-29-2: no staging):
/// the file the name holds — or, for a link standing at the name, the file
/// it names, the link kept — is removed, and the partial made at the empty
/// name, private and writable by its owner (600) from the moment it
/// exists. Cut to nothing through the link, the old file's bytes were gone
/// before the new one's mode was set.
///
/// cr-ecm22-1: and it is made READY — its mode, and its length
/// (`unfinished`: one byte longer than its source) — or it is removed:
/// nothing at the name looks finished, and the file fails for its retry.
///
/// cr-ecm22-2: what a CHAIN of links ends at is the file replaced — the
/// file opening the name reaches, which a resume in place patches — and
/// every link of it kept. One link was followed, and the next link of the
/// chain replaced by the partial, the file at its end left as it was.
///
/// cr-ecm23-3: and it is made in the directory the chain ends in, HELD,
/// and only when that lies `within` the destination's root (canonical),
/// when one is given: a link there to a name outside it made the file
/// outside. See [`fresh_partial_ready`].
#[cfg(unix)]
pub fn fresh_partial(dst: &Path, unfinished: u64, within: Option<&super::Within>) -> Result<File> {
    fresh_partial_ready(dst, unfinished, within, |_| Ok(())).map(|(file, ())| file)
}

/// cr-ecm23-1: [`fresh_partial`], with `ready` run on the fresh file —
/// private and at its length — before it is handed back: what a caller must
/// know of it (which file it is) is known before the caller has it, and an
/// error there removes it. Learned after, a look that failed left a partial
/// nothing held.
#[cfg(unix)]
pub fn fresh_partial_ready<T>(
    dst: &Path,
    unfinished: u64,
    within: Option<&super::Within>,
    ready: impl FnOnce(&File) -> Result<T>,
) -> Result<(File, T)> {
    let resolved = resolve_destination(dst, within)?;
    match resolved
        .look()
        .with_context(|| format!("looking at {}", dst.display()))?
    {
        None => resolved.fresh_exclusive(dst, unfinished, ready),
        Some(seen) => resolved.fresh(dst, unfinished, seen, ready),
    }
}

/// cr-ecm32-1: where a name leads, as resolution finds it: the directory the
/// chain of links ends in (which it is: device and number) and the name in
/// it. A whole-file resume is written only where its comparison led.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    directory: (u64, u64),
    name: Vec<u8>,
}

/// cr-ecm24-1: where a resume's partial is had — ONE resolution for every
/// way one is had, opened where it stands or made fresh, locally and by
/// the wire: the chain of links at the name followed to its end
/// (cr-ecm22-2), a name written as a directory's refused (cr-ecm24-6), the
/// end's directory within the destination when a root is given (cr-ecm23-3,
/// cr-ecm24-3) and HELD to be searched (cr-ecm24-7), and the partial opened
/// or made by its name in that directory, never through a link. Opened by
/// its name to be made, a link to a name outside the destination was
/// followed out and the file made there.
#[cfg(unix)]
pub(crate) struct Resolved {
    directory: std::os::unix::io::OwnedFd,
    name: std::ffi::CString,
}

/// cr-ecm36-8: how many links a path's resolution follows before the system
/// refuses it (`ELOOP`): Linux's 40; elsewhere — macOS and the BSDs,
/// `MAXSYMLINKS` — 32.
#[cfg(unix)]
pub(crate) const MAX_FOLLOWED_LINKS: usize = if cfg!(target_os = "linux") { 40 } else { 32 };

/// [`Resolved`]: the place `dst` names, `within` the destination's root
/// when one is given.
#[cfg(unix)]
pub(crate) fn resolve_destination(dst: &Path, within: Option<&super::Within>) -> Result<Resolved> {
    resolve_destination_following(dst, within, true)
}

/// [`resolve_destination`], the chain of links at the name followed only
/// when `follow`: a name to be made is never followed.
///
/// cr-ecm38-2: on Linux, with the root held, from the directory the transfer
/// holds ([`resolve_beneath`]) — never by its path, which a directory put in
/// the root's place since names instead.
#[cfg(unix)]
pub(crate) fn resolve_destination_following(
    dst: &Path,
    within: Option<&super::Within>,
    follow: bool,
) -> Result<Resolved> {
    // cr-ecm24-6: a name typed as a directory — with a `/` or `/.` after it,
    // as typed or as a link of the chain names it — is no place for a file:
    // its last name taken, `absent/` was made a regular file `absent`.
    let not_a_file = || {
        Err(std::io::Error::from_raw_os_error(libc::ENOTDIR))
            .with_context(|| format!("putting a fresh partial in place of {}", dst.display()))
    };
    if typed_as_directory(dst) {
        return not_a_file();
    }
    #[cfg(target_os = "linux")]
    if let Some(resolved) = within.and_then(|within| resolve_beneath(within, dst, follow)) {
        let resolved = resolved.map_err(|error| {
            if error.raw_os_error() == Some(libc::ENOTDIR) && follow {
                eyre::Report::new(error).wrap_err(format!(
                    "putting a fresh partial in place of {}",
                    dst.display()
                ))
            } else {
                eyre::Report::new(error).wrap_err(format!("resolving {}", dst.display()))
            }
        })?;
        after_a_destination_is_resolved(dst);
        return Ok(resolved);
    }
    resolve_by_path(dst, within.map(super::Within::path), follow)
}

/// [`resolve_destination_following`] by the path: the chain followed by its
/// names, the directory it ends in canonicalized and checked within `within`
/// (the root's canonical path), and held — where no root's directory is held
/// to walk from, and off Linux.
#[cfg(unix)]
pub(crate) fn resolve_by_path(dst: &Path, within: Option<&Path>, follow: bool) -> Result<Resolved> {
    let not_a_file = || {
        Err(std::io::Error::from_raw_os_error(libc::ENOTDIR))
            .with_context(|| format!("putting a fresh partial in place of {}", dst.display()))
    };
    if typed_as_directory(dst) {
        return not_a_file();
    }
    let mut target = dst.to_path_buf();
    let mut links = 0;
    while follow
        && std::fs::symlink_metadata(&target).is_ok_and(|there| there.file_type().is_symlink())
    {
        // As many as opening the name would follow — the system's own limit
        // (cr-ecm36-8).
        links += 1;
        if links > MAX_FOLLOWED_LINKS {
            eyre::bail!("{} is a chain of too many links", dst.display());
        }
        let named = std::fs::read_link(&target)
            .with_context(|| format!("reading the link {}", target.display()))?;
        if typed_as_directory(&named) {
            return not_a_file();
        }
        target = match target.parent() {
            Some(parent) if named.is_relative() => parent.join(named),
            _ => named,
        };
    }
    let Some(name) = target.file_name().map(std::ffi::OsStr::to_os_string) else {
        eyre::bail!("{} names no file", dst.display());
    };
    // cr-ecm23-3: the partial is had in the directory the chain ends in,
    // HELD: found by its path with no link in it, that directory lies
    // within the destination — a link there to a name outside it made the
    // file outside — and it is opened from `/` one name at a time, never
    // through a link, so a folder on its way replaced since cannot move
    // where the file is made or named.
    let parent = match target.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => std::path::PathBuf::from("."),
    };
    let directory = std::fs::canonicalize(&parent)
        .with_context(|| format!("finding the directory {} is in", target.display()))?;
    // cr-ecm24-3: within it — or, when the destination is one file, that
    // file itself, whose directory lies above it. Judged by its directory
    // alone, a destination that is one file was refused its own place.
    if let Some(root) = within {
        if !directory.starts_with(root) && directory.join(&name) != root {
            eyre::bail!(
                "{} leads out of the destination, to {}",
                dst.display(),
                target.display()
            );
        }
    }
    let held = crate::held::directory_unlinked(&directory)
        .with_context(|| format!("holding the directory {}", directory.display()))?;
    let resolved = Resolved {
        directory: held,
        name: c_string(&name)?,
    };
    after_a_destination_is_resolved(dst);
    Ok(resolved)
}

/// cr-ecm38-2 (Linux): `dst` resolved from the root `within` holds — one name
/// at a time from the held directory, never through a link it does not read
/// itself. A link is read and its target walked in its place: a relative one
/// from where it stands, an absolute one from the root when it names a place
/// beneath the root (by either of its names), else it leads out; `..` above
/// the root leads out too. The last name, when it is a link, is followed
/// when `follow`. `None` where the root holds no directory, or `dst` lies
/// beneath neither of its names by its path: resolved by its path, then.
/// By its path, a fallback opened what had since taken the root's name.
#[cfg(target_os = "linux")]
pub(crate) fn resolve_beneath(
    within: &super::Within,
    dst: &Path,
    follow: bool,
) -> Option<std::io::Result<Resolved>> {
    use std::os::unix::io::AsRawFd as _;
    let held = within.held.as_ref()?;
    let place = within.place(dst)?;
    Some((|| {
        let mut names = Walk::names(&place);
        let Some(mut last) = names.pop_back() else {
            return Err(std::io::Error::other(format!(
                "{} names no file",
                dst.display()
            )));
        };
        // cr-ecm39-6: an ordinary path's directory is found by the kernel's
        // own walk beneath the root, in one call; the walk below serves what
        // that cannot — a link at the name to follow, an absolute link on
        // the way, no `openat2`. Taken for every resume, the walk ran out of
        // descriptors on a deep path.
        let direct = if names.is_empty() {
            open_directory_at(held.as_raw_fd(), c".")
        } else {
            let parent: std::path::PathBuf = names.iter().collect();
            super::openat2_beneath(
                held.as_raw_fd(),
                &parent,
                libc::O_PATH | libc::O_DIRECTORY,
                0,
            )
        };
        if let Ok(directory) = direct {
            let name = name_of(&last)?;
            let a_link = match stat_at(directory.as_raw_fd(), &name) {
                Ok(found) => found.st_mode & libc::S_IFMT == libc::S_IFLNK,
                Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => false,
                Err(error) => return Err(error),
            };
            if !follow || !a_link {
                return Ok(Resolved { directory, name });
            }
        }
        let mut walk = Walk::new(within, held.as_raw_fd(), dst);
        walk.enter(names, false)?;
        loop {
            let name = name_of(&last)?;
            if !follow || !walk.is_link(&name)? {
                break;
            }
            walk.count_link()?;
            let target = read_link_at(walk.here(), &name)?;
            if typed_as_directory(&target) {
                return Err(std::io::Error::from_raw_os_error(libc::ENOTDIR));
            }
            let mut more = walk.splice(&target)?;
            let Some(next) = more.pop_back() else {
                return Err(std::io::Error::other(format!(
                    "{} names no file",
                    dst.display()
                )));
            };
            walk.enter(more, false)?;
            last = next;
        }
        Ok(Resolved {
            directory: walk.into_directory()?,
            name: name_of(&last)?,
        })
    })())
}

/// cr-ecm38-2 (Linux): every directory of `place` (beneath the root `within`
/// holds) that is missing, made — each in the one found above it, from the
/// held directory, links walked as [`resolve_beneath`] walks them.
#[cfg(target_os = "linux")]
pub(crate) fn make_dirs_beneath(
    within: &super::Within,
    place: &Path,
    shown: &Path,
) -> std::io::Result<()> {
    use std::os::unix::io::AsRawFd as _;
    let Some(held) = within.held.as_ref() else {
        return Err(std::io::Error::other("no directory is held"));
    };
    Walk::new(within, held.as_raw_fd(), shown).enter(Walk::names(place), true)
}

/// A walk from a held root: where it stands, as the names of the directories
/// entered beneath the root, and the one directory it stands in, held — one
/// descriptor, however deep (cr-ecm39-6): a `..` is walked back by finding
/// the parent again from the root. Holding every directory on the way, a
/// path 72 directories deep ran out of descriptors.
#[cfg(target_os = "linux")]
struct Walk<'a> {
    within: &'a super::Within,
    root: std::os::unix::io::RawFd,
    here: Option<HeldDir>,
    names: Vec<std::ffi::OsString>,
    links: usize,
    shown: &'a Path,
}

/// A directory a walk holds (counted, under test).
#[cfg(target_os = "linux")]
struct HeldDir(Option<std::os::unix::io::OwnedFd>);

#[cfg(target_os = "linux")]
impl HeldDir {
    fn new(fd: std::os::unix::io::OwnedFd) -> Self {
        #[cfg(test)]
        WALK_HELD.with(|(now, most)| {
            now.set(now.get() + 1);
            most.set(most.get().max(now.get()));
        });
        Self(Some(fd))
    }

    fn raw(&self) -> std::os::unix::io::RawFd {
        use std::os::unix::io::AsRawFd as _;
        self.0.as_ref().map_or(-1, |fd| fd.as_raw_fd())
    }

    fn into_fd(mut self) -> std::os::unix::io::OwnedFd {
        self.0.take().expect("a held directory holds one")
    }
}

#[cfg(target_os = "linux")]
impl Drop for HeldDir {
    fn drop(&mut self) {
        #[cfg(test)]
        WALK_HELD.with(|(now, _)| now.set(now.get().saturating_sub(1)));
    }
}

#[cfg(all(test, target_os = "linux"))]
thread_local! {
    /// Test seam (cr-ecm39-6): the directories this thread's walks hold now,
    /// and the most held at once.
    pub(crate) static WALK_HELD: (std::cell::Cell<usize>, std::cell::Cell<usize>) =
        const { (std::cell::Cell::new(0), std::cell::Cell::new(0)) };
}

#[cfg(target_os = "linux")]
impl<'a> Walk<'a> {
    fn new(within: &'a super::Within, root: std::os::unix::io::RawFd, shown: &'a Path) -> Self {
        Self {
            within,
            root,
            here: None,
            names: Vec::new(),
            links: 0,
            shown,
        }
    }

    fn here(&self) -> std::os::unix::io::RawFd {
        self.here.as_ref().map_or(self.root, HeldDir::raw)
    }

    fn leads_out(&self) -> std::io::Error {
        std::io::Error::other(format!(
            "{} leads out of the destination",
            self.shown.display()
        ))
    }

    /// A relative path's names, `..` kept as a name.
    fn names(path: &Path) -> std::collections::VecDeque<std::ffi::OsString> {
        path.components()
            .filter_map(|component| match component {
                std::path::Component::Normal(name) => Some(name.to_os_string()),
                std::path::Component::ParentDir => Some("..".into()),
                _ => None,
            })
            .collect()
    }

    fn count_link(&mut self) -> std::io::Result<()> {
        self.links += 1;
        if self.links > MAX_FOLLOWED_LINKS {
            return Err(std::io::Error::from_raw_os_error(libc::ELOOP));
        }
        Ok(())
    }

    /// Back up to the directory above, found again from the root: by the
    /// kernel's walk beneath it, or a name at a time — one held at once.
    fn up(&mut self) -> std::io::Result<()> {
        if self.names.pop().is_none() {
            return Err(self.leads_out());
        }
        self.here = None;
        if self.names.is_empty() {
            return Ok(());
        }
        let place: std::path::PathBuf = self.names.iter().collect();
        if let Ok(found) =
            super::openat2_beneath(self.root, &place, libc::O_PATH | libc::O_DIRECTORY, 0)
        {
            self.here = Some(HeldDir::new(found));
            return Ok(());
        }
        let mut found: Option<HeldDir> = None;
        for name in &self.names {
            let at = found.as_ref().map_or(self.root, HeldDir::raw);
            let next = open_directory_at(at, &name_of(name)?)?;
            found = Some(HeldDir::new(next));
        }
        self.here = found;
        Ok(())
    }

    /// What a link's `target` puts in its place: an absolute one is walked
    /// from the root, when it names a place beneath it.
    fn splice(
        &mut self,
        target: &Path,
    ) -> std::io::Result<std::collections::VecDeque<std::ffi::OsString>> {
        if target.is_absolute() {
            let place = self.within.place(target).ok_or_else(|| self.leads_out())?;
            self.here = None;
            self.names.clear();
            Ok(Self::names(&place))
        } else {
            Ok(Self::names(target))
        }
    }

    fn is_link(&self, name: &std::ffi::CStr) -> std::io::Result<bool> {
        match stat_at(self.here(), name) {
            Ok(found) => Ok(found.st_mode & libc::S_IFMT == libc::S_IFLNK),
            Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Enter each of `names` in turn, as a directory; missing ones made when
    /// `make`.
    fn enter(
        &mut self,
        mut names: std::collections::VecDeque<std::ffi::OsString>,
        make: bool,
    ) -> std::io::Result<()> {
        while let Some(name) = names.pop_front() {
            if name == ".." {
                self.up()?;
                continue;
            }
            if name.is_empty() || name == "." {
                continue;
            }
            let c_name = name_of(&name)?;
            let found = match stat_at(self.here(), &c_name) {
                Ok(found) => found,
                Err(missing) if missing.kind() == std::io::ErrorKind::NotFound && make => {
                    // SAFETY: `here` is open for the call; `c_name` is one
                    // NUL-terminated name that outlives it.
                    if unsafe { libc::mkdirat(self.here(), c_name.as_ptr(), 0o777) } < 0 {
                        let error = std::io::Error::last_os_error();
                        if error.kind() != std::io::ErrorKind::AlreadyExists {
                            return Err(error);
                        }
                    }
                    stat_at(self.here(), &c_name)?
                }
                Err(error) => return Err(error),
            };
            match found.st_mode & libc::S_IFMT {
                libc::S_IFLNK => {
                    self.count_link()?;
                    let target = read_link_at(self.here(), &c_name)?;
                    let mut more = self.splice(&target)?;
                    while let Some(spliced) = more.pop_back() {
                        names.push_front(spliced);
                    }
                }
                libc::S_IFDIR => {
                    let entered = open_directory_at(self.here(), &c_name)?;
                    self.here = Some(HeldDir::new(entered));
                    self.names.push(name);
                }
                _ => return Err(std::io::Error::from_raw_os_error(libc::ENOTDIR)),
            }
        }
        Ok(())
    }

    /// The directory reached, held.
    fn into_directory(mut self) -> std::io::Result<std::os::unix::io::OwnedFd> {
        match self.here.take() {
            Some(directory) => Ok(directory.into_fd()),
            None => open_directory_at(self.root, c"."),
        }
    }
}

#[cfg(target_os = "linux")]
fn name_of(name: &std::ffi::OsStr) -> std::io::Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt as _;
    std::ffi::CString::new(name.as_bytes())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))
}

#[cfg(target_os = "linux")]
fn stat_at(
    directory: std::os::unix::io::RawFd,
    name: &std::ffi::CStr,
) -> std::io::Result<libc::stat> {
    // SAFETY: `stat` is plain integers, filled by the call.
    let mut found: libc::stat = unsafe { std::mem::zeroed() };
    // SAFETY: `directory` is open for the call; `name` is NUL-terminated
    // and `found` a properly sized `stat`, both outliving it.
    if unsafe {
        libc::fstatat(
            directory,
            name.as_ptr(),
            &mut found,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } < 0
    {
        return Err(std::io::Error::last_os_error());
    }
    Ok(found)
}

#[cfg(target_os = "linux")]
fn read_link_at(
    directory: std::os::unix::io::RawFd,
    name: &std::ffi::CStr,
) -> std::io::Result<std::path::PathBuf> {
    use std::os::unix::ffi::OsStringExt as _;
    let mut buffer = vec![0u8; libc::PATH_MAX as usize];
    // SAFETY: `directory` is open for the call; `name` is NUL-terminated and
    // `buffer` is writable for its length, all outliving it.
    let read = unsafe {
        libc::readlinkat(
            directory,
            name.as_ptr(),
            buffer.as_mut_ptr().cast::<libc::c_char>(),
            buffer.len(),
        )
    };
    if read < 0 {
        return Err(std::io::Error::last_os_error());
    }
    buffer.truncate(read as usize);
    Ok(std::path::PathBuf::from(std::ffi::OsString::from_vec(
        buffer,
    )))
}

#[cfg(target_os = "linux")]
fn open_directory_at(
    directory: std::os::unix::io::RawFd,
    name: &std::ffi::CStr,
) -> std::io::Result<std::os::unix::io::OwnedFd> {
    use std::os::unix::io::{FromRawFd as _, OwnedFd};
    // SAFETY: `directory` is open for the call; `name` is NUL-terminated.
    let opened = unsafe {
        libc::openat(
            directory,
            name.as_ptr(),
            libc::O_PATH | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if opened < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: `opened` is an open descriptor nothing else owns.
    Ok(unsafe { OwnedFd::from_raw_fd(opened) })
}

/// Test seam (cr-ecm27-3): runs once a resume's destination is resolved and
/// its directory held, before anything at the name is looked at — for a
/// test to put something else there. Keyed by path. Production never sets
/// it.
#[cfg(all(test, unix))]
pub(crate) static AFTER_A_RESOLVE_HOOKS: std::sync::Mutex<
    Vec<(std::path::PathBuf, PartialOpenedHook)>,
> = std::sync::Mutex::new(Vec::new());

#[cfg(unix)]
fn after_a_destination_is_resolved(_dst: &Path) {
    #[cfg(test)]
    {
        let hooks: Vec<PartialOpenedHook> = AFTER_A_RESOLVE_HOOKS
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

#[cfg(unix)]
impl Resolved {
    /// The file at the name, opened with `flags` in the held directory and
    /// never through a link; one made is private (600).
    pub(crate) fn open(&self, flags: libc::c_int) -> std::io::Result<File> {
        self.open_as(flags, 0o600)
    }

    /// [`Self::open`], a file made taking `mode` (less the umask): the
    /// opener's own creations are made as any other file is (cr-ecm37-4).
    pub(crate) fn open_as(&self, flags: libc::c_int, mode: libc::c_uint) -> std::io::Result<File> {
        use std::os::unix::io::AsRawFd as _;
        open_in_as(self.directory.as_raw_fd(), &self.name, flags, mode)
    }

    /// cr-ecm21-3, D-2026-11-23-1: a FRESH private partial at the name in
    /// place of `seen` — what the caller judged there (cr-ecm29-2) — made IN
    /// PLACE, as every write to a destination is (D-2026-09-29-2: no
    /// staging). What stands at the name is removed, only while it is still
    /// what was seen, and the partial is made at the empty name by a
    /// creation that fails if the name is taken: anything come there
    /// meanwhile is left as it is, and the file fails. See
    /// [`Self::fresh_as`].
    pub(crate) fn fresh<T>(
        &self,
        dst: &Path,
        unfinished: u64,
        seen: Entry,
        ready: impl FnOnce(&File) -> Result<T>,
    ) -> Result<(File, T)> {
        self.fresh_as(dst, unfinished, ready, Some(seen))
    }

    /// cr-ecm25-2: [`Self::fresh`] where nothing stood at the name: the
    /// partial is made only while nothing has it — what comes to stand there
    /// first is left as it is, and the file fails.
    pub(crate) fn fresh_exclusive<T>(
        &self,
        dst: &Path,
        unfinished: u64,
        ready: impl FnOnce(&File) -> Result<T>,
    ) -> Result<(File, T)> {
        self.fresh_as(dst, unfinished, ready, None)
    }

    /// cr-ecm32-1: where this resolution leads — the held directory (which
    /// it is) and the name in it.
    pub(crate) fn place(&self) -> std::io::Result<Place> {
        use std::os::unix::fs::MetadataExt as _;
        let directory = File::from(self.directory.try_clone()?).metadata()?;
        Ok(Place {
            directory: (directory.dev(), directory.ino()),
            name: self.name.as_bytes().to_vec(),
        })
    }

    /// cr-ecm35-1: the file at the name removed only while it is `mine`, the
    /// file made there; anything else there now is somebody's. Not known
    /// which file was made, nothing is removed.
    pub(crate) fn remove_if(&self, mine: Option<(u64, u64)>) -> std::io::Result<()> {
        use std::os::unix::io::AsRawFd as _;
        let Some(mine) = mine else {
            return Err(std::io::Error::other(
                "which file was made could not be told, so what is at the name is not removed",
            ));
        };
        match self.look()? {
            Some(entry) if entry.identity == mine => {}
            _ => return Ok(()),
        }
        // SAFETY: the held directory is open for the call, and the name is a
        // NUL-terminated string that outlives it.
        let removed = unsafe { libc::unlinkat(self.directory.as_raw_fd(), self.name.as_ptr(), 0) };
        if removed == 0 {
            return Ok(());
        }
        match std::io::Error::last_os_error() {
            error if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            error => Err(error),
        }
    }

    /// What stands at the name in the held directory, itself — its mode
    /// (type bits included), its owner, and which it is — or nothing.
    pub(crate) fn look(&self) -> std::io::Result<Option<Entry>> {
        use std::os::unix::io::AsRawFd as _;
        entry_at(self.directory.as_raw_fd(), &self.name)
    }

    /// The fresh partial, made in place (D-2026-11-23-1). `occupant` is what
    /// the caller judged at the name, or nothing.
    ///
    /// - A link there — come since the chain was followed — or a directory
    ///   is left as it is, and the file fails.
    /// - What stands at the name is removed only while it is still
    ///   `occupant` (cr-ecm29-2): the caller's look, or the handle it opened
    ///   and judged. Anything else there now is left as it is, and the file
    ///   fails. Nothing there any more: the name is empty already.
    /// - The partial is made at the empty name by a creation that fails if
    ///   the name is taken (cr-ecm25-2), in the directory held (cr-ecm24-1):
    ///   private (600) from the moment it exists, so a mask cannot narrow it
    ///   past its owner, and in the directory itself, so it takes that
    ///   directory's group where the directory gives one (setgid).
    /// - It is made READY — 600, its unfinished length, and what `ready`
    ///   learns of it — or it is removed, only while it is still the one
    ///   made: nothing at the name looks finished, and the file fails for
    ///   its retry to land.
    ///
    /// The name stands empty from the removal until the partial is made;
    /// what comes to it between the look and the removal is removed with
    /// it. The owner accepted both (D-2026-11-23-1). Staged beside the name
    /// and swapped in, the partial needed an exchange no network filesystem
    /// and no macOS build here had, and its staging directory lost the
    /// destination directory's group.
    fn fresh_as<T>(
        &self,
        dst: &Path,
        unfinished: u64,
        ready: impl FnOnce(&File) -> Result<T>,
        occupant: Option<Entry>,
    ) -> Result<(File, T)> {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        use std::os::unix::io::AsRawFd as _;
        before_a_fresh_partial_is_made(dst);
        let held = self.directory.as_raw_fd();
        let left = || {
            eyre::eyre!(
                "something else came to stand at {} while it was being had; it is left as it is",
                dst.display()
            )
        };
        if let Some(seen) = occupant {
            if is_link(seen.mode) {
                eyre::bail!(
                    "{} became a link while it was being had; it is left as it is",
                    dst.display()
                );
            }
            if is_directory(seen.mode) {
                return Err(std::io::Error::from_raw_os_error(libc::EISDIR)).with_context(|| {
                    format!("putting a fresh partial in place of {}", dst.display())
                });
            }
            let now = entry_at(held, &self.name)
                .with_context(|| format!("looking at {}", dst.display()))?;
            match now {
                // cr-ecm30-1: the file as it was seen — not only which
                // number it had.
                Some(now) if now.version == seen.version => {
                    // SAFETY: `held` is open for the length of the call, and
                    // the name is NUL-terminated and outlives it.
                    if unsafe { libc::unlinkat(held, self.name.as_ptr(), 0) } != 0 {
                        let error = std::io::Error::last_os_error();
                        if error.kind() != std::io::ErrorKind::NotFound {
                            return Err(error).with_context(|| {
                                format!(
                                    "removing {} to make a fresh partial in its place",
                                    dst.display()
                                )
                            });
                        }
                    }
                }
                None => {}
                Some(_) => return Err(left()),
            }
        }
        before_a_fresh_partial_takes_the_name(dst);
        let file = match open_in(
            held,
            &self.name,
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
        ) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Err(left()),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("making a fresh partial for {}", dst.display()))
            }
        };
        // cr-ecm31-3: a partial that cannot be removed is left marked
        // unfinished — its length past its source's, set through what holds
        // it — so that nothing there looks finished, and the error says where
        // and why. Left at no length, or its removal's failure dropped, it
        // stood there as an empty file, or as the copy's own, unreported.
        // cr-ecm32-5: said in the words every record of a failure is read for
        // "this run left its own copy here" — the summary's left-in-place
        // set, a job's record, a retry. Said otherwise, the partial was
        // named in the error and missing from that set.
        let left = |why: String| -> String {
            use crate::remote::transfer::sink::INCOMPLETE_LEFT_IN_PLACE;
            match file.set_len(unfinished) {
                Ok(()) => format!(
                    "{INCOMPLETE_LEFT_IN_PLACE}: the fresh partial is left at {}, marked unfinished: {why}",
                    dst.display()
                ),
                Err(unmarked) => format!(
                    "{INCOMPLETE_LEFT_IN_PLACE}: the fresh partial is left at {}, and could not be marked unfinished ({unmarked}): {why}",
                    dst.display()
                ),
            }
        };
        // Which file it is, so that a failure removes it and nothing else.
        let mine = match fresh_look_fault(dst).and_then(|()| file.metadata()) {
            Ok(found) => (found.dev(), found.ino()),
            Err(error) => {
                let said = left(format!("looking at it failed: {error}"));
                return Err(error).context(crate::remote::transfer::sink::LeftInPlace(said));
            }
        };
        let unmade = || -> Option<String> {
            match entry_at(held, &self.name) {
                Ok(Some(there)) if there.identity == mine => {
                    // SAFETY: as for the removal above.
                    if unsafe { libc::unlinkat(held, self.name.as_ptr(), 0) } == 0 {
                        return None;
                    }
                    let error = std::io::Error::last_os_error();
                    Some(left(format!("removing it failed: {error}")))
                }
                // Something else has the name now: nothing of this run's is
                // there to remove.
                Ok(_) => None,
                Err(error) => Some(left(format!("looking at the name failed: {error}"))),
            }
        };
        let prepared = file
            .set_permissions(std::fs::Permissions::from_mode(0o600))
            .and_then(|()| file.set_len(unfinished))
            .map_err(eyre::Report::new)
            .and_then(|()| ready(&file));
        match prepared {
            Ok(readied) => Ok((file, readied)),
            Err(error) => {
                let error = error.wrap_err(format!(
                    "making a fresh partial ready for {}",
                    dst.display()
                ));
                Err(match unmade() {
                    None => error,
                    Some(left) => error.wrap_err(crate::remote::transfer::sink::LeftInPlace(left)),
                })
            }
        }
    }
}

/// What stands at a name, itself: its mode (type bits included), its owner,
/// and which it is (device, inode).
#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) mode: u32,
    pub(crate) owner: u32,
    pub(crate) identity: (u64, u64),
    /// cr-ecm30-1: the entry as it stood — its size, its times and which it
    /// is: what a removal must find again, unchanged, to take it. A device
    /// and number alone may be another file's, given the number once the
    /// first is gone.
    pub(crate) version: crate::moved::Version,
}

#[cfg(unix)]
impl Entry {
    /// What `found` says of the file it was had of.
    fn of(found: &std::fs::Metadata) -> Entry {
        use std::os::unix::fs::MetadataExt as _;
        Entry {
            mode: found.mode(),
            owner: found.uid(),
            identity: (found.dev(), found.ino()),
            version: crate::moved::Version::of(found),
        }
    }
}

/// What stands at `name` in the directory `directory` — looked at itself,
/// never through a link — or nothing.
#[cfg(unix)]
fn entry_at(
    directory: std::os::unix::io::RawFd,
    name: &std::ffi::CStr,
) -> std::io::Result<Option<Entry>> {
    #[cfg(not(target_os = "linux"))]
    use libc::{fstatat, stat};
    #[cfg(target_os = "linux")]
    use libc::{fstatat64 as fstatat, stat64 as stat};
    let mut found = std::mem::MaybeUninit::<stat>::uninit();
    // SAFETY: `directory` is open for the length of the call, `name` is
    // NUL-terminated and outlives it, and `found` is a writable record the
    // call fills on success.
    let looked = unsafe {
        fstatat(
            directory,
            name.as_ptr(),
            found.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if looked == 0 {
        // SAFETY: the call succeeded, so the record is filled in.
        let found = unsafe { found.assume_init() };
        #[allow(clippy::unnecessary_cast)]
        return Ok(Some(Entry {
            mode: found.st_mode as u32,
            owner: found.st_uid as u32,
            identity: (found.st_dev as u64, found.st_ino as u64),
            // As `crate::moved::Version::of` reads the same times.
            version: crate::moved::Version {
                size: found.st_size as u64,
                mtime_nanos: i128::from(found.st_mtime) * 1_000_000_000
                    + i128::from(found.st_mtime_nsec),
                ctime_nanos: i128::from(found.st_ctime) * 1_000_000_000
                    + i128::from(found.st_ctime_nsec),
                device: found.st_dev as u64,
                inode: found.st_ino as u64,
            },
        }));
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::NotFound {
        return Ok(None);
    }
    Err(error)
}

/// Test seam (D-2026-11-23-1): runs once what stood at a fresh partial's
/// name is removed and before the partial is made there — for a test to put
/// something at the empty name. Keyed by the destination's path. Production
/// never sets it.
#[cfg(all(test, unix))]
pub(crate) static BEFORE_A_FRESH_PARTIAL_TAKES_THE_NAME_HOOKS: std::sync::Mutex<
    Vec<(std::path::PathBuf, PartialOpenedHook)>,
> = std::sync::Mutex::new(Vec::new());

#[cfg(unix)]
fn before_a_fresh_partial_takes_the_name(_dst: &Path) {
    #[cfg(test)]
    {
        let hooks: Vec<PartialOpenedHook> = BEFORE_A_FRESH_PARTIAL_TAKES_THE_NAME_HOOKS
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

/// The file called `name` in the directory `directory`, opened with `flags`
/// and never through a link; one made is private (600).
#[cfg(unix)]
fn open_in(
    directory: std::os::unix::io::RawFd,
    name: &std::ffi::CStr,
    flags: libc::c_int,
) -> std::io::Result<File> {
    open_in_as(directory, name, flags, 0o600)
}

/// [`open_in`], a file made taking `mode`.
#[cfg(unix)]
fn open_in_as(
    directory: std::os::unix::io::RawFd,
    name: &std::ffi::CStr,
    flags: libc::c_int,
    mode: libc::c_uint,
) -> std::io::Result<File> {
    use std::os::unix::io::{FromRawFd as _, OwnedFd};
    // SAFETY: `directory` is open for the length of the call, and `name` is
    // a NUL-terminated string that outlives it.
    let opened = unsafe {
        libc::openat(
            directory,
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            mode,
        )
    };
    if opened < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: `opened` is an open descriptor nothing else owns.
    Ok(File::from(unsafe { OwnedFd::from_raw_fd(opened) }))
}

/// Whether `path` is written as a directory's: a `/` or `/.` after its last
/// name, which the system then resolves as a directory.
#[cfg(unix)]
fn typed_as_directory(path: &Path) -> bool {
    let bytes = path.as_os_str().as_encoded_bytes();
    bytes.ends_with(b"/") || bytes.ends_with(b"/.")
}

#[cfg(unix)]
fn c_string(name: &std::ffi::OsStr) -> Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt as _;
    std::ffi::CString::new(name.as_bytes())
        .map_err(|error| eyre::eyre!("a name with a NUL in it: {error}"))
}

/// Test seam (cr-ecm23-3): runs once a fresh partial's directory is held
/// and before the partial is made in it — for a test to replace that
/// directory's path. Keyed by the destination's path. Production never
/// sets it.
#[cfg(all(test, unix))]
pub(crate) static BEFORE_A_FRESH_PARTIAL_IS_MADE_HOOKS: std::sync::Mutex<
    Vec<(std::path::PathBuf, PartialOpenedHook)>,
> = std::sync::Mutex::new(Vec::new());

#[cfg(unix)]
fn before_a_fresh_partial_is_made(_dst: &Path) {
    #[cfg(test)]
    {
        let hooks: Vec<PartialOpenedHook> = BEFORE_A_FRESH_PARTIAL_IS_MADE_HOOKS
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

/// Test seam (cr-ecm19-4): runs once a resume's partial is open at its name —
/// one there opened, before its mode is set; one made now, once it takes
/// the name (cr-ecm25-4) — when a reader could open it. Keyed by path.
/// Production never sets it.
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

/// [`open_resume_partial`], with a second handle to the partial for the
/// caller's cleanup (cr-ecm24-2).
#[cfg(not(unix))]
pub fn open_resume_partial_held(
    dst: &Path,
    source_mode: Option<u32>,
    src_len: u64,
    within: Option<&super::Within>,
) -> Result<(File, File, u64)> {
    let (file, len) = open_resume_partial(dst, source_mode, src_len, within)?;
    let cleanup = file
        .try_clone()
        .with_context(|| format!("holding the partial {}", dst.display()))?;
    Ok((file, cleanup, len))
}

/// The partial a resume patches, and how much of it there already is.
#[cfg(not(unix))]
pub fn open_resume_partial(
    dst: &Path,
    _source_mode: Option<u32>,
    _src_len: u64,
    _within: Option<&super::Within>,
) -> Result<(File, u64)> {
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

    #[cfg(unix)]
    let source_mode = {
        use std::os::unix::fs::PermissionsExt as _;
        Some(src_file.metadata()?.permissions().mode())
    };
    #[cfg(not(unix))]
    let source_mode = None;
    let (dst_file, dst_len) = open_resume_partial(dst, source_mode, src_len, None)?;
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
    resume_copy_through(src_file, src_len, &mut dst_file, dst_len, dst, block_size)
}

/// cr-ecm25-1: [`resume_copy_into`] through a partial the caller keeps — to
/// give it its time and mode through the same handle afterwards, with no
/// other taken once it is had. Duplicated after a fresh partial had replaced
/// the destination, a handle the system refused left the destination's bytes
/// gone.
pub fn resume_copy_through(
    src_file: &mut File,
    src_len: u64,
    dst_file: &mut File,
    dst_len: u64,
    dst: &Path,
    block_size: usize,
) -> Result<ResumeCopyOutcome> {
    src_file.seek(SeekFrom::Start(0))?;
    // cr-ecm26-2: the kept handle's cursor stands wherever its caller left
    // it; the comparison below starts from the file's start. Taken from the
    // cursor, `xxABCD` at offset 2 was taken for `ABCD` and left `xxAB`.
    dst_file.seek(SeekFrom::Start(0))?;

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
    // cr-ecm26-3: once a destination block cannot be read, the rest of the
    // file is written: it cannot be compared. Read with `?`, such a block
    // failed the copy, a readable source and a writable file notwithstanding.
    let mut unreadable = false;

    while offset < src_len {
        let remaining = src_len - offset;
        let this_block = remaining.min(block_size as u64) as usize;

        // Read source block and compute hash. No seek: src is sequential.
        src_file.read_exact(&mut src_buf[..this_block])?;
        let src_hash = hash_block(&src_buf[..this_block]);

        // Check if we can compare with destination
        let should_write = if offset < dst_len {
            let dst_available = (dst_len - offset).min(this_block as u64) as usize;

            if dst_available == this_block && !unreadable {
                // Full block available, read and hash (seek only if the
                // dst cursor isn't already aligned to this offset).
                let read = dst_read_fault(dst, offset).and_then(|()| {
                    if dst_cursor_pos != offset {
                        dst_file.seek(SeekFrom::Start(offset))?;
                    }
                    dst_file.read_exact(&mut dst_buf[..this_block])
                });
                match read {
                    Ok(()) => {
                        dst_cursor_pos = offset + this_block as u64;
                        src_hash != hash_block(&dst_buf[..this_block])
                    }
                    Err(_) => {
                        unreadable = true;
                        dst_cursor_pos = u64::MAX;
                        true
                    }
                }
            } else if unreadable {
                true
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

    /// cr-ecm21-3: a destination that grants others what its source does
    /// not is not resumed in place — a reader who has it open would read
    /// the source's bytes as they land — but into a fresh private partial
    /// made in its place; and one that cannot be removed, its directory
    /// barred, is left exactly as it stands.
    #[cfg(unix)]
    #[test]
    fn a_resume_more_private_than_its_destination_takes_a_fresh_partial() -> Result<()> {
        use std::io::Read as _;
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let data: Vec<u8> = (0..8192u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&src, &data)?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600))?;
        let dst = tmp.path().join("dst");
        std::fs::write(&dst, &data[..4096])?;
        std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o644))?;
        let before = std::fs::metadata(&dst)?.ino();
        let mut reader = File::open(&dst)?;
        resume_copy_from(&mut File::open(&src)?, data.len() as u64, &dst, 1024)?;
        assert_ne!(std::fs::metadata(&dst)?.ino(), before, "a fresh partial");
        assert_eq!(std::fs::read(&dst)?, data);
        let mut held = Vec::new();
        reader.read_to_end(&mut held)?;
        assert_eq!(held, &data[..4096], "the reader keeps only what it held");

        let closed = tmp.path().join("closed");
        std::fs::create_dir(&closed)?;
        let kept = closed.join("dst");
        std::fs::write(&kept, &data[..4096])?;
        std::fs::set_permissions(&kept, std::fs::Permissions::from_mode(0o644))?;
        std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o555))?;
        let barred = std::fs::write(closed.join("probe"), b"").is_err();
        let copied = resume_copy_from(&mut File::open(&src)?, data.len() as u64, &kept, 1024);
        std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o755))?;
        if barred {
            assert!(copied.is_err());
            assert_eq!(std::fs::read(&kept)?, &data[..4096], "left as it stood");
            assert_eq!(
                std::fs::metadata(&kept)?.permissions().mode() & 0o777,
                0o644
            );
        }
        Ok(())
    }

    /// cr-ecm23-3: a fresh partial is made in the directory it was found in,
    /// held: that directory's path replaced by a link to another, before the
    /// partial is made, moves nothing — the partial is made and named in the
    /// directory held, and nothing lands in the other.
    #[cfg(unix)]
    #[test]
    fn a_fresh_partial_is_made_in_the_directory_it_holds() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let data = vec![7u8; 4096];
        std::fs::write(&src, &data)?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600))?;
        let folder = tmp.path().join("folder");
        let moved = tmp.path().join("moved");
        let elsewhere = tmp.path().join("elsewhere");
        std::fs::create_dir(&folder)?;
        std::fs::create_dir(&elsewhere)?;
        let dst = folder.join("dst");
        std::fs::write(&dst, &data[..1000])?;
        std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o644))?;
        {
            let (folder, moved, elsewhere) = (folder.clone(), moved.clone(), elsewhere.clone());
            BEFORE_A_FRESH_PARTIAL_IS_MADE_HOOKS.lock().unwrap().push((
                dst.clone(),
                std::sync::Arc::new(move |_: &Path| {
                    std::fs::rename(&folder, &moved).unwrap();
                    std::os::unix::fs::symlink(&elsewhere, &folder).unwrap();
                }),
            ));
        }
        let copied = resume_copy_from(&mut File::open(&src)?, data.len() as u64, &dst, 1024);
        BEFORE_A_FRESH_PARTIAL_IS_MADE_HOOKS
            .lock()
            .unwrap()
            .retain(|(at, _)| at != &dst);
        copied?;
        assert_eq!(
            std::fs::read(moved.join("dst"))?,
            data,
            "in the directory held"
        );
        assert_eq!(
            std::fs::read_dir(&elsewhere)?.count(),
            0,
            "nothing elsewhere"
        );
        Ok(())
    }

    /// cr-ecm26-1, D-2026-11-23-1: where nothing stood when it was looked
    /// at, a resume makes its partial by a creation that fails if the name
    /// is taken: a file put at the name since is left as it is, and the
    /// resume fails. Opened by the name after a look, a file made between
    /// was adopted.
    #[cfg(unix)]
    #[test]
    fn a_partial_where_nothing_stood_is_made_exclusively() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempdir()?;
        let dst = tmp.path().join("absent");
        {
            let theirs = dst.clone();
            AFTER_A_LOOK_HOOKS.lock().unwrap().push((
                dst.clone(),
                std::sync::Arc::new(move |_: &Path| {
                    std::fs::write(&theirs, b"theirs").unwrap();
                    std::fs::set_permissions(&theirs, std::fs::Permissions::from_mode(0o644))
                        .unwrap();
                }),
            ));
        }
        let had = open_resume_partial_held(&dst, Some(0o100600), 8, None);
        AFTER_A_LOOK_HOOKS
            .lock()
            .unwrap()
            .retain(|(at, _)| at != &dst);
        assert!(had.is_err(), "the resume fails");
        assert_eq!(std::fs::read(&dst)?, b"theirs", "left as it is");
        Ok(())
    }

    /// A 0600 source of 4,096 bytes and, in a folder of its own, a 0644
    /// destination holding its first 1,000 — wider than its source, so not
    /// reused: a fresh partial is made to replace it.
    #[cfg(unix)]
    fn wider_destination(tmp: &Path) -> Result<(std::path::PathBuf, std::path::PathBuf, Vec<u8>)> {
        use std::os::unix::fs::PermissionsExt as _;
        let src = tmp.join("src");
        let data: Vec<u8> = (0..4096u32).map(|at| (at % 251) as u8).collect();
        std::fs::write(&src, &data)?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600))?;
        let folder = tmp.join("folder");
        std::fs::create_dir(&folder)?;
        let dst = folder.join("dst");
        std::fs::write(&dst, &data[..1000])?;
        std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o644))?;
        Ok((src, dst, data))
    }

    /// cr-ecm29-2: what a fresh partial replaces is what the first look
    /// found: a file come to the name after it — before the partial is even
    /// made — is left as it is, and the resume fails. Looked at again when
    /// the partial was made, the newcomer became what was replaced.
    #[cfg(unix)]
    #[test]
    fn a_file_come_since_the_first_look_is_not_replaced() -> Result<()> {
        let tmp = tempdir()?;
        let (src, dst, data) = wider_destination(tmp.path())?;
        {
            let (at, aside) = (dst.clone(), tmp.path().join("theirs"));
            BEFORE_A_FRESH_PARTIAL_IS_MADE_HOOKS.lock().unwrap().push((
                dst.clone(),
                std::sync::Arc::new(move |_: &Path| {
                    std::fs::write(&aside, b"theirs").unwrap();
                    std::fs::rename(&aside, &at).unwrap();
                }),
            ));
        }
        let copied = resume_copy_from(&mut File::open(&src)?, data.len() as u64, &dst, 1024);
        BEFORE_A_FRESH_PARTIAL_IS_MADE_HOOKS
            .lock()
            .unwrap()
            .retain(|(at, _)| at != &dst);
        assert!(copied.is_err(), "the resume fails");
        assert_eq!(std::fs::read(&dst)?, b"theirs", "left as it is");
        let left: Vec<_> =
            std::fs::read_dir(dst.parent().unwrap())?.collect::<std::io::Result<_>>()?;
        assert_eq!(left.len(), 1, "no stage left");
        Ok(())
    }

    /// cr-ecm30-1: what a fresh partial replaces is the file as it was seen,
    /// not only the number it had: one written since — the same number,
    /// grown by a byte — is left as it is, and the resume fails. Asked only
    /// its device and number, a file given the number of one gone matched.
    #[cfg(unix)]
    #[test]
    fn a_file_changed_since_it_was_seen_is_not_removed() -> Result<()> {
        use std::io::Write as _;
        let tmp = tempdir()?;
        let (src, dst, data) = wider_destination(tmp.path())?;
        {
            let at = dst.clone();
            BEFORE_A_FRESH_PARTIAL_IS_MADE_HOOKS.lock().unwrap().push((
                dst.clone(),
                std::sync::Arc::new(move |_: &Path| {
                    let mut grown = std::fs::OpenOptions::new().append(true).open(&at).unwrap();
                    grown.write_all(b"!").unwrap();
                }),
            ));
        }
        let copied = resume_copy_from(&mut File::open(&src)?, data.len() as u64, &dst, 1024);
        BEFORE_A_FRESH_PARTIAL_IS_MADE_HOOKS
            .lock()
            .unwrap()
            .retain(|(at, _)| at != &dst);
        assert!(copied.is_err(), "the resume fails");
        let mut kept = data[..1000].to_vec();
        kept.push(b'!');
        assert_eq!(std::fs::read(&dst)?, kept, "left as it is");
        Ok(())
    }

    /// cr-ecm31-3: a fresh partial that cannot be made ready, and then cannot
    /// be removed — its directory barred meanwhile — is left marked
    /// unfinished, and the error says where and why; and one whose first
    /// look at itself fails is left marked unfinished too. Left with the
    /// error naming only its preparation, its cleanup's failure was dropped;
    /// left at no length, it stood there as an empty file.
    #[cfg(unix)]
    #[test]
    fn a_fresh_partial_its_cleanup_cannot_remove_is_left_marked_and_said() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;
        struct Reopened(std::path::PathBuf);
        impl Drop for Reopened {
            fn drop(&mut self) {
                let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
            }
        }
        let tmp = tempdir()?;
        let (_, dst, _) = wider_destination(tmp.path())?;
        let folder = dst.parent().unwrap().to_path_buf();
        let _reopened = Reopened(folder.clone());
        let made = fresh_partial_ready(&dst, 9, None, |_| -> Result<()> {
            std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o555))?;
            eyre::bail!("refused")
        });
        let barred = std::fs::write(folder.join("probe"), b"").is_err();
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755))?;
        let error = made.expect_err("not made ready");
        if barred {
            let said = format!("{error:#}");
            assert!(
                said.contains(&format!("left at {}", dst.display())),
                "{said}"
            );
            assert_eq!(std::fs::metadata(&dst)?.len(), 9, "marked unfinished");
        } else {
            eprintln!("SKIPPED the barred half: this process is not barred by a directory's mode");
        }

        let looked = tmp.path().join("looked");
        FRESH_LOOK_FAULTS.lock().unwrap().push(looked.clone());
        let made = fresh_partial_ready(&looked, 9, None, |_| Ok(()));
        FRESH_LOOK_FAULTS.lock().unwrap().retain(|at| at != &looked);
        let error = made.expect_err("its look refused");
        assert!(
            format!("{error:#}").contains("marked unfinished"),
            "{error:#}"
        );
        assert_eq!(std::fs::metadata(&looked)?.len(), 9, "marked unfinished");
        Ok(())
    }

    /// D-2026-11-23-1: a destination gone before its fresh partial is made
    /// leaves the name to the partial, made only while nothing has it, and
    /// the resume completes.
    #[cfg(unix)]
    #[test]
    fn a_destination_gone_before_its_fresh_partial_takes_the_partial() -> Result<()> {
        let tmp = tempdir()?;
        let (src, dst, data) = wider_destination(tmp.path())?;
        {
            let at = dst.clone();
            BEFORE_A_FRESH_PARTIAL_IS_MADE_HOOKS.lock().unwrap().push((
                dst.clone(),
                std::sync::Arc::new(move |_: &Path| std::fs::remove_file(&at).unwrap()),
            ));
        }
        let copied = resume_copy_from(&mut File::open(&src)?, data.len() as u64, &dst, 1024);
        BEFORE_A_FRESH_PARTIAL_IS_MADE_HOOKS
            .lock()
            .unwrap()
            .retain(|(at, _)| at != &dst);
        copied?;
        assert_eq!(std::fs::read(&dst)?, data);
        Ok(())
    }

    /// cr-ecm28-2, D-2026-11-23-1: a link that takes the name while a fresh
    /// partial is had is left as it is, and the resume fails — one put in
    /// place of what was seen, before that is removed, and one put at the
    /// emptied name, before the partial is made there. Removed, the link
    /// went; made through, what it names was written.
    #[cfg(unix)]
    #[test]
    fn a_link_that_takes_the_name_while_a_partial_is_had_is_left() -> Result<()> {
        for (phase, before_the_removal) in
            [("before the removal", true), ("at the emptied name", false)]
        {
            let tmp = tempdir()?;
            let (src, dst, data) = wider_destination(tmp.path())?;
            let elsewhere = tmp.path().join("elsewhere");
            std::fs::write(&elsewhere, b"elsewhere")?;
            let hooks = if before_the_removal {
                &BEFORE_A_FRESH_PARTIAL_IS_MADE_HOOKS
            } else {
                &BEFORE_A_FRESH_PARTIAL_TAKES_THE_NAME_HOOKS
            };
            {
                let (at, to) = (dst.clone(), elsewhere.clone());
                hooks.lock().unwrap().push((
                    dst.clone(),
                    std::sync::Arc::new(move |_: &Path| {
                        let _ = std::fs::remove_file(&at);
                        std::os::unix::fs::symlink(&to, &at).unwrap();
                    }),
                ));
            }
            let copied = resume_copy_from(&mut File::open(&src)?, data.len() as u64, &dst, 1024);
            hooks.lock().unwrap().retain(|(at, _)| at != &dst);
            assert!(copied.is_err(), "{phase}: the resume fails");
            assert!(
                std::fs::symlink_metadata(&dst)?.file_type().is_symlink(),
                "{phase}: left a link"
            );
            assert_eq!(std::fs::read(&elsewhere)?, b"elsewhere", "{phase}");
        }
        Ok(())
    }

    /// D-2026-11-23-1: a fresh partial is made at the destination's own
    /// name: while it is made ready nothing else stands in its directory —
    /// no staged file, no directory of its own — and it is the file at the
    /// name, private and unfinished.
    #[cfg(unix)]
    #[test]
    fn a_fresh_partial_is_made_at_the_name_itself() -> Result<()> {
        use std::os::unix::fs::MetadataExt as _;
        let tmp = tempdir()?;
        let (_, dst, _) = wider_destination(tmp.path())?;
        let folder = dst.parent().unwrap().to_path_buf();
        let before = std::fs::metadata(&dst)?.ino();
        // cr-ecm31-6: held open, so that its number cannot be given to the
        // fresh partial — a number freed may be given again at once (ext4).
        let _held = File::open(&dst)?;
        let (file, names) = fresh_partial_ready(&dst, 9, None, |_| {
            let mut names = std::fs::read_dir(&folder)?
                .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
                .collect::<std::io::Result<Vec<_>>>()?;
            names.sort();
            Ok(names)
        })?;
        assert_eq!(
            names,
            ["dst"],
            "only the name itself, while it is made ready"
        );
        let made = file.metadata()?;
        assert_eq!(
            made.ino(),
            std::fs::metadata(&dst)?.ino(),
            "the file at the name"
        );
        assert_ne!(made.ino(), before, "a fresh partial");
        assert_eq!(made.len(), 9, "unfinished");
        assert_eq!(made.mode() & 0o777, 0o600, "private");
        Ok(())
    }

    /// D-2026-11-23-1 (cr-ecm30-2): a fresh partial made in a setgid
    /// directory takes the directory's group, as any file made there does.
    /// Staged in a directory of its own made 700, it took this run's group.
    #[cfg(unix)]
    #[test]
    fn a_fresh_partial_takes_its_setgid_directorys_group() -> Result<()> {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let mut groups = vec![0 as libc::gid_t; 256];
        // SAFETY: `groups` has room for as many as the count given.
        let count = unsafe { libc::getgroups(groups.len() as libc::c_int, groups.as_mut_ptr()) };
        // SAFETY: `getegid` has no preconditions and cannot fail.
        let own = unsafe { libc::getegid() };
        let Some(other) = groups[..count.max(0) as usize]
            .iter()
            .copied()
            .find(|group| *group != own)
        else {
            eprintln!("SKIPPED: this run belongs to no group but its own");
            return Ok(());
        };
        let tmp = tempdir()?;
        let (src, dst, data) = wider_destination(tmp.path())?;
        let folder = dst.parent().unwrap();
        std::os::unix::fs::chown(folder, None, Some(other))?;
        std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o2775))?;
        resume_copy_from(&mut File::open(&src)?, data.len() as u64, &dst, 1024)?;
        assert_eq!(
            std::fs::metadata(&dst)?.gid(),
            other,
            "the directory's group"
        );
        assert_eq!(std::fs::read(&dst)?, data);
        Ok(())
    }

    /// cr-ecm28-5: a destination with a second name is not patched in place:
    /// a fresh partial takes its name, and the other name keeps what it had.
    /// Patched in place on a retry, both names held the new bytes.
    #[cfg(unix)]
    #[test]
    fn a_destination_with_a_second_name_is_not_patched_in_place() -> Result<()> {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let data = vec![7u8; 4096];
        std::fs::write(&src, &data)?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600))?;
        let dst = tmp.path().join("dst");
        std::fs::write(&dst, &data[..1000])?;
        std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o600))?;
        let alias = tmp.path().join("alias");
        std::fs::hard_link(&dst, &alias)?;
        let before = std::fs::metadata(&dst)?.ino();
        resume_copy_from(&mut File::open(&src)?, data.len() as u64, &dst, 1024)?;
        assert_ne!(std::fs::metadata(&dst)?.ino(), before, "a fresh partial");
        assert_eq!(std::fs::read(&dst)?, data);
        assert_eq!(
            std::fs::read(&alias)?,
            &data[..1000],
            "the other name as it was"
        );
        Ok(())
    }

    /// cr-ecm27-3: what takes the name during acquisition is met safely — a
    /// FIFO put in a reusable file's place after the look does not hold the
    /// open (the file is replaced, the resume ends), and a link put there
    /// before the look is left as it is and the resume fails. Opened to be
    /// read, the FIFO waited for a writer; replaced, the link was
    /// overwritten.
    #[cfg(unix)]
    #[test]
    fn what_takes_the_name_during_acquisition_is_met_safely() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let data = vec![7u8; 4096];
        std::fs::write(&src, &data)?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o644))?;

        let fifo = tmp.path().join("fifo");
        std::fs::write(&fifo, &data[..1000])?;
        std::fs::set_permissions(&fifo, std::fs::Permissions::from_mode(0o644))?;
        {
            let at = fifo.clone();
            AFTER_A_LOOK_HOOKS.lock().unwrap().push((
                fifo.clone(),
                std::sync::Arc::new(move |_: &Path| {
                    std::fs::remove_file(&at).unwrap();
                    let name = std::ffi::CString::new(at.as_os_str().as_encoded_bytes()).unwrap();
                    // SAFETY: `name` is a NUL-terminated path that outlives the call.
                    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o444) }, 0);
                }),
            ));
        }
        let (done, waited) = std::sync::mpsc::channel();
        {
            let (src, fifo, data) = (src.clone(), fifo.clone(), data.clone());
            std::thread::spawn(move || {
                let copied = File::open(&src)
                    .map_err(eyre::Report::new)
                    .and_then(|mut opened| {
                        resume_copy_from(&mut opened, data.len() as u64, &fifo, 1024)
                    });
                let _ = done.send(copied.map(|_| ()));
            });
        }
        let ended = waited.recv_timeout(std::time::Duration::from_secs(20));
        AFTER_A_LOOK_HOOKS
            .lock()
            .unwrap()
            .retain(|(at, _)| at != &fifo);
        ended.map_err(|_| eyre::eyre!("the resume waited on the FIFO"))??;
        assert!(std::fs::symlink_metadata(&fifo)?.file_type().is_file());
        assert_eq!(std::fs::read(&fifo)?, data);

        let linked = tmp.path().join("linked");
        let elsewhere = tmp.path().join("elsewhere");
        std::fs::write(&linked, &data[..1000])?;
        std::fs::write(&elsewhere, b"elsewhere")?;
        {
            let (at, to) = (linked.clone(), elsewhere.clone());
            AFTER_A_RESOLVE_HOOKS.lock().unwrap().push((
                linked.clone(),
                std::sync::Arc::new(move |_: &Path| {
                    std::fs::remove_file(&at).unwrap();
                    std::os::unix::fs::symlink(&to, &at).unwrap();
                }),
            ));
        }
        let copied = resume_copy_from(&mut File::open(&src)?, data.len() as u64, &linked, 1024);
        AFTER_A_RESOLVE_HOOKS
            .lock()
            .unwrap()
            .retain(|(at, _)| at != &linked);
        assert!(copied.is_err(), "a link at the name fails the resume");
        assert!(
            std::fs::symlink_metadata(&linked)?.file_type().is_symlink(),
            "left a link"
        );
        assert_eq!(std::fs::read(&elsewhere)?, b"elsewhere");
        Ok(())
    }

    /// cr-ecm27-4: a partial made private to be had gets its mode back when
    /// its last look fails — one reused where it stood (0644) and one made
    /// writable first (0444). Left so, each became 0600.
    #[cfg(unix)]
    #[test]
    fn a_partial_whose_last_look_fails_gets_its_mode_back() -> Result<()> {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let tmp = tempdir()?;
        for (name, mode) in [("reused", 0o644), ("readonly", 0o444)] {
            let dst = tmp.path().join(name);
            std::fs::write(&dst, b"8 bytes!")?;
            std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(mode))?;
            if std::fs::metadata(&dst)?.uid() == 0 {
                // Root writes any file: nothing to show.
                return Ok(());
            }
            LEN_FAULTS.lock().unwrap().push(dst.clone());
            let had = open_resume_partial_held(&dst, Some(0o100000 | mode), 8, None);
            LEN_FAULTS.lock().unwrap().retain(|at| at != &dst);
            assert!(had.is_err(), "{name}");
            assert_eq!(
                std::fs::metadata(&dst)?.permissions().mode() & 0o777,
                mode,
                "{name}: its mode as it was"
            );
        }
        Ok(())
    }

    /// cr-ecm26-4: a read-only partial made writable to be had gets its mode
    /// back when its writable reopen is refused: the file stands as it did.
    /// Left made writable, a 0444 file became 0600.
    #[cfg(unix)]
    #[test]
    fn a_partial_made_writable_and_not_had_gets_its_mode_back() -> Result<()> {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let tmp = tempdir()?;
        let dst = tmp.path().join("dst");
        std::fs::write(&dst, b"8 bytes!")?;
        std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o444))?;
        if std::fs::metadata(&dst)?.uid() == 0 {
            // Root writes any file: nothing to show.
            return Ok(());
        }
        REOPEN_FAULTS.lock().unwrap().push(dst.clone());
        let had = open_resume_partial_held(&dst, Some(0o100444), 8, None);
        REOPEN_FAULTS.lock().unwrap().retain(|at| at != &dst);
        assert!(had.is_err());
        assert_eq!(
            std::fs::metadata(&dst)?.permissions().mode() & 0o777,
            0o444,
            "its mode as it was"
        );
        assert_eq!(std::fs::read(&dst)?, b"8 bytes!");
        Ok(())
    }

    /// cr-ecm26-5: a FIFO at the name — one this run may only read — is
    /// replaced whole, not opened: the resume ends with the source's bytes
    /// in a regular file. Opened to be read, the FIFO waited for a writer,
    /// and the resume with it.
    #[cfg(unix)]
    #[test]
    fn a_fifo_at_the_name_is_replaced_not_opened() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let data = vec![7u8; 4096];
        std::fs::write(&src, &data)?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600))?;
        let dst = tmp.path().join("fifo");
        let name = std::ffi::CString::new(dst.as_os_str().as_encoded_bytes())?;
        // SAFETY: `name` is a NUL-terminated path that outlives the call.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o444) }, 0);
        let (done, waited) = std::sync::mpsc::channel();
        {
            let (src, dst, data) = (src.clone(), dst.clone(), data.clone());
            std::thread::spawn(move || {
                let copied = File::open(&src)
                    .map_err(eyre::Report::new)
                    .and_then(|mut opened| {
                        resume_copy_from(&mut opened, data.len() as u64, &dst, 1024)
                    });
                let _ = done.send(copied.map(|_| ()));
            });
        }
        waited
            .recv_timeout(std::time::Duration::from_secs(20))
            .map_err(|_| eyre::eyre!("the resume waited on the FIFO"))??;
        assert!(std::fs::symlink_metadata(&dst)?.file_type().is_file());
        assert_eq!(std::fs::read(&dst)?, data);
        Ok(())
    }

    /// cr-ecm25-5: a local resume into a destination this run cannot read is
    /// copied whole — one whose mode says it is not reused (0004, for a
    /// 0600 source), and one that would be (its own, 0200) but cannot be
    /// opened — and one that may not be reused (0644, for a 0600 source) is
    /// not opened at all. Opened first, the first two failed the copy where
    /// the wire copies them whole.
    #[cfg(unix)]
    #[test]
    fn a_local_resume_into_a_destination_it_cannot_read_copies_it_whole() -> Result<()> {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let data = vec![7u8; 4096];
        std::fs::write(&src, &data)?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600))?;
        if std::fs::metadata(&src)?.uid() == 0 {
            // Root reads any file: nothing to show.
            return Ok(());
        }
        for (name, mode) in [
            ("others.bin", 0o004),
            ("own.bin", 0o200),
            ("wider.bin", 0o644),
        ] {
            let dst = tmp.path().join(name);
            std::fs::write(&dst, &data[..1000])?;
            std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(mode))?;
            let opened = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            {
                let opened = std::sync::Arc::clone(&opened);
                WHILE_A_PARTIAL_IS_OPENED_HOOKS.lock().unwrap().push((
                    dst.clone(),
                    std::sync::Arc::new(move |_: &Path| {
                        opened.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }),
                ));
            }
            let copied = resume_copy_from(&mut File::open(&src)?, data.len() as u64, &dst, 1024);
            WHILE_A_PARTIAL_IS_OPENED_HOOKS
                .lock()
                .unwrap()
                .retain(|(at, _)| at != &dst);
            copied.map_err(|error| eyre::eyre!("{name}: {error:#}"))?;
            assert_eq!(std::fs::read(&dst)?, data, "{name}");
            if name == "wider.bin" {
                assert_eq!(
                    opened.load(std::sync::atomic::Ordering::SeqCst),
                    0,
                    "not opened: its mode said it is not reused"
                );
            }
        }
        Ok(())
    }

    /// cr-ecm25-4: a resume refused its cleanup's handle leaves the
    /// destination as it stood — a file it would reuse keeps its mode, and
    /// where nothing stood nothing is made. Had after the file was made
    /// private, a refusal left a 644 file 600; made by the name, it left an
    /// empty file.
    #[cfg(unix)]
    #[test]
    fn a_resume_refused_its_cleanup_handle_changes_nothing() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempdir()?;
        let reused = tmp.path().join("reused");
        std::fs::write(&reused, b"8 bytes!")?;
        std::fs::set_permissions(&reused, std::fs::Permissions::from_mode(0o644))?;
        let absent = tmp.path().join("absent");
        for at in [&reused, &absent] {
            KEEP_FAULTS.lock().unwrap().push(at.clone());
            let had = open_resume_partial_held(at, Some(0o100644), 8, None);
            KEEP_FAULTS.lock().unwrap().retain(|fault| fault != at);
            assert!(had.is_err(), "{}", at.display());
        }
        assert_eq!(
            std::fs::metadata(&reused)?.permissions().mode() & 0o777,
            0o644,
            "its mode as it was"
        );
        assert_eq!(std::fs::read(&reused)?, b"8 bytes!");
        assert!(!absent.exists(), "nothing made where nothing stood");
        assert_eq!(std::fs::read_dir(tmp.path())?.count(), 1);
        Ok(())
    }

    /// cr-ecm24-2, D-2026-11-23-1: a resume refused the second handle its
    /// caller's cleanup needs fails, and leaves nothing at the name that
    /// looks finished: the destination it could not reuse was removed to
    /// make the partial in place, and the partial is removed in turn, for
    /// the retry to land. Taken after the caller had the partial, a refusal
    /// (`EMFILE`) left nine bytes of nothing there, and nothing held to undo
    /// it.
    #[cfg(unix)]
    #[test]
    fn a_resume_refused_its_cleanup_handle_leaves_nothing_that_looks_finished() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempdir()?;
        let dst = tmp.path().join("dst");
        std::fs::write(&dst, b"8 bytes!")?;
        std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o644))?;
        KEEP_FAULTS.lock().unwrap().push(dst.clone());
        let had = open_resume_partial_held(&dst, Some(0o100600), 8, None);
        KEEP_FAULTS.lock().unwrap().retain(|at| at != &dst);
        assert!(had.is_err());
        assert!(!dst.exists(), "nothing at the name");
        assert_eq!(
            std::fs::read_dir(tmp.path())?.count(),
            0,
            "nothing beside it"
        );
        Ok(())
    }

    /// cr-ecm24-1: a local resume through a link to a name outside the
    /// destination, not there yet, makes nothing there: the partial is had
    /// where the chain ends, checked against the destination first. Opened
    /// by its name to be made, the link was followed out and the file made
    /// outside.
    #[cfg(unix)]
    #[test]
    fn a_local_resume_makes_nothing_outside_the_destination() -> Result<()> {
        let tmp = tempdir()?;
        let root = tmp.path().join("dst");
        let outside = tmp.path().join("outside");
        std::fs::create_dir(&root)?;
        std::fs::create_dir(&outside)?;
        let at = root.join("r.bin");
        std::os::unix::fs::symlink("../outside/new", &at)?;
        let within = std::fs::canonicalize(&root)?;
        assert!(open_resume_partial(
            &at,
            Some(0o100600),
            10,
            Some(&crate::copy::Within::hold(&within))
        )
        .is_err());
        assert_eq!(
            std::fs::read_dir(&outside)?.count(),
            0,
            "nothing made outside"
        );
        Ok(())
    }

    /// cr-ecm24-6: a name typed as a directory — `absent/`, or a link whose
    /// target ends in `/` or `/.` — takes no fresh partial, and nothing is
    /// made. Its last name taken, `absent/` was made a regular file.
    #[cfg(unix)]
    #[test]
    fn a_name_typed_as_a_directory_takes_no_fresh_partial() -> Result<()> {
        let tmp = tempdir()?;
        let absent = tmp.path().join("absent/");
        assert!(fresh_partial(&absent, 11, None).is_err());
        for (link, named) in [("slashed", "gone/"), ("dotted", "gone/.")] {
            std::os::unix::fs::symlink(named, tmp.path().join(link))?;
            assert!(
                fresh_partial(&tmp.path().join(link), 11, None).is_err(),
                "{named}"
            );
        }
        let mut left: Vec<_> = std::fs::read_dir(tmp.path())?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<std::io::Result<_>>()?;
        left.sort();
        assert_eq!(left, ["dotted", "slashed"], "nothing made");
        Ok(())
    }

    /// cr-ecm24-3: a destination that is one file takes a fresh partial in
    /// its own place — the file is the destination, its directory above it
    /// — and a name beside it is still outside. Judged by its directory
    /// alone, the file itself was refused.
    #[cfg(unix)]
    #[test]
    fn a_destination_that_is_one_file_takes_a_fresh_partial() -> Result<()> {
        let tmp = tempdir()?;
        let dst = tmp.path().join("one.bin");
        std::fs::write(&dst, b"old")?;
        let root = std::fs::canonicalize(&dst)?;
        let file = fresh_partial(&dst, 11, Some(&crate::copy::Within::hold(&root)))?;
        assert_eq!(file.metadata()?.len(), 11);
        assert_eq!(std::fs::metadata(&dst)?.len(), 11, "in its own place");
        let beside = tmp.path().join("beside.bin");
        std::fs::write(&beside, b"x")?;
        assert!(
            fresh_partial(&beside, 11, Some(&crate::copy::Within::hold(&root))).is_err(),
            "outside"
        );
        assert_eq!(std::fs::read(&beside)?, b"x");
        Ok(())
    }

    /// cr-ecm24-7: a fresh partial is made in a directory this run may write
    /// and search but not list (0300): its directory is held to be searched.
    /// Held to be read, it was refused.
    #[cfg(unix)]
    #[test]
    fn a_fresh_partial_is_made_in_a_directory_that_cannot_be_listed() -> Result<()> {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let data = vec![7u8; 4096];
        std::fs::write(&src, &data)?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600))?;
        let folder = tmp.path().join("folder");
        std::fs::create_dir(&folder)?;
        let dst = folder.join("dst");
        std::fs::write(&dst, &data[..1000])?;
        std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o644))?;
        if std::fs::metadata(&dst)?.uid() == 0 {
            // Root lists any directory: nothing to show.
            return Ok(());
        }
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o300))?;
        let copied = resume_copy_from(&mut File::open(&src)?, data.len() as u64, &dst, 1024);
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o700))?;
        copied?;
        assert_eq!(std::fs::read(&dst)?, data);
        Ok(())
    }

    /// cr-ecm23-4: a destination another user owns is not reused, even by a
    /// run that may set its mode (root): its owner can read what lands in
    /// it. Made private and patched in place, it was.
    #[cfg(unix)]
    #[test]
    fn a_destination_another_owns_takes_a_fresh_partial() -> Result<()> {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let data = vec![7u8; 4096];
        std::fs::write(&src, &data)?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600))?;
        let dst = tmp.path().join("dst");
        std::fs::write(&dst, &data[..1000])?;
        std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o600))?;
        let found = std::fs::metadata(&dst)?;
        if found.uid() == 0 {
            // Run as root, the file is root's own: nothing to model.
            return Ok(());
        }
        // The run as root, and the file this test's user's — another's.
        RUN_AS.lock().unwrap().push((dst.clone(), 0));
        let copied = resume_copy_from(&mut File::open(&src)?, data.len() as u64, &dst, 1024);
        RUN_AS.lock().unwrap().retain(|(at, _)| at != &dst);
        copied?;
        assert_ne!(
            std::fs::metadata(&dst)?.ino(),
            found.ino(),
            "a fresh partial"
        );
        assert_eq!(std::fs::read(&dst)?, data);
        Ok(())
    }

    /// cr-ecm22-2: a chain of links at the destination is followed to its
    /// end — the file there is the one replaced — and every link of it kept.
    /// One link was followed and the next link replaced, the file at the
    /// chain's end left as it was.
    #[cfg(unix)]
    #[test]
    fn a_fresh_partial_through_a_chain_of_links_replaces_its_end() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let data = vec![7u8; 4096];
        std::fs::write(&src, &data)?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600))?;
        let actual = tmp.path().join("actual");
        std::fs::write(&actual, &data[..1000])?;
        std::fs::set_permissions(&actual, std::fs::Permissions::from_mode(0o644))?;
        std::fs::create_dir(tmp.path().join("sub"))?;
        let middle = tmp.path().join("sub").join("middle");
        std::os::unix::fs::symlink("../actual", &middle)?;
        let entry = tmp.path().join("entry");
        std::os::unix::fs::symlink("sub/middle", &entry)?;
        resume_copy_from(&mut File::open(&src)?, data.len() as u64, &entry, 1024)?;
        for link in [&entry, &middle] {
            assert!(
                std::fs::symlink_metadata(link)?.file_type().is_symlink(),
                "{} kept",
                link.display()
            );
        }
        assert_eq!(std::fs::read(&actual)?, data, "the chain's end replaced");
        Ok(())
    }

    /// cr-ecm22-4: a source of mode 0000 is a known mode — the most private
    /// — and a destination others can read takes a fresh partial for it.
    /// It was taken for no mode, and the destination's inode reused.
    #[cfg(unix)]
    #[test]
    fn a_source_of_no_mode_bits_is_more_private_than_its_destination() -> Result<()> {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let data = vec![7u8; 4096];
        std::fs::write(&src, &data)?;
        let mut opened = File::open(&src)?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o000))?;
        let dst = tmp.path().join("dst");
        std::fs::write(&dst, &data[..1000])?;
        std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o644))?;
        let before = std::fs::metadata(&dst)?.ino();
        // cr-ecm31-6: held open, so that its number cannot be given to the
        // fresh partial — a number freed may be given again at once (ext4).
        let _held = File::open(&dst)?;
        resume_copy_from(&mut opened, data.len() as u64, &dst, 1024)?;
        assert_ne!(std::fs::metadata(&dst)?.ino(), before, "a fresh partial");
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600))?;
        Ok(())
    }

    /// cr-ecm22-6: a destination whose name is the longest a name can be
    /// takes a fresh partial too: the name made beside it is blit's own and
    /// short. It held the destination's name, and was too long to make.
    #[cfg(unix)]
    #[test]
    fn a_destination_of_the_longest_name_takes_a_fresh_partial() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let data = vec![7u8; 4096];
        std::fs::write(&src, &data)?;
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600))?;
        let dst = tmp.path().join("n".repeat(255));
        std::fs::write(&dst, &data[..1000])?;
        std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o644))?;
        resume_copy_from(&mut File::open(&src)?, data.len() as u64, &dst, 1024)?;
        assert_eq!(std::fs::read(&dst)?, data);
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

    /// cr-ecm26-3: a local resume whose destination block cannot be read
    /// writes the rest of the file and succeeds: it cannot be compared. Read
    /// with `?`, the block failed the copy.
    #[test]
    fn a_block_that_cannot_be_read_is_written_with_the_rest() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let data: Vec<u8> = (0..4096).map(|i| (i % 251) as u8).collect();
        std::fs::write(&src, &data)?;
        let dst = tmp.path().join("dst");
        std::fs::write(&dst, &data)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600))?;
            std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o600))?;
        }
        DST_READ_FAULTS.lock().unwrap().push((dst.clone(), 1024));
        let copied = resume_copy_from(&mut File::open(&src)?, 4096, &dst, 1024);
        DST_READ_FAULTS.lock().unwrap().retain(|(at, _)| at != &dst);
        let outcome = copied?;
        assert_eq!(outcome.bytes_transferred, 3072, "the rest written");
        assert_eq!(std::fs::read(&dst)?, data);
        Ok(())
    }

    /// cr-ecm26-2: a copy through a kept handle compares from the start of
    /// the file, wherever the handle's cursor stands. From the cursor,
    /// `xxABCD` at offset 2 was taken for `ABCD`, nothing was written, and
    /// `xxAB` was left.
    #[test]
    fn a_copy_through_a_kept_handle_compares_from_the_start() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        std::fs::write(&src, b"ABCD")?;
        let dst = tmp.path().join("dst");
        std::fs::write(&dst, b"xxABCD")?;
        let mut kept = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&dst)?;
        kept.seek(SeekFrom::Start(2))?;
        let outcome = resume_copy_through(&mut File::open(&src)?, 4, &mut kept, 6, &dst, 4)?;
        drop(kept);
        assert_eq!(std::fs::read(&dst)?, b"ABCD");
        assert_eq!(outcome.bytes_transferred, 4);
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
