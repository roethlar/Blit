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
///
/// cr-ecm21-3: and one that grants others what its source (`source_mode`)
/// does not is not reused either — a reader who has it open would read
/// the source's bytes as they land: a fresh partial is made in its place
/// ([`fresh_partial`]), and nothing of it is touched until that exists.
#[cfg(unix)]
pub fn open_resume_partial(
    dst: &Path,
    source_mode: Option<u32>,
    src_len: u64,
    within: Option<&Path>,
) -> Result<(File, u64)> {
    open_resume_partial_with(dst, source_mode, src_len, within, false)
        .map(|(file, _, len)| (file, len))
}

/// cr-ecm24-2: [`open_resume_partial`], with a second handle to the partial
/// for the caller's cleanup — had before anything of the destination is
/// changed or replaced: a fresh partial's while it is staged. Taken after,
/// a handle the system refused (`EMFILE`) left a fresh partial in the
/// destination's place, the destination's bytes gone, and nothing held to
/// undo it.
#[cfg(unix)]
pub fn open_resume_partial_held(
    dst: &Path,
    source_mode: Option<u32>,
    src_len: u64,
    within: Option<&Path>,
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
    within: Option<&Path>,
    cleanup: bool,
) -> Result<(File, Option<File>, u64)> {
    use std::os::unix::fs::MetadataExt as _;
    // cr-ecm24-1: had where the chain of links at the name ends — checked
    // against the destination, in the directory held — never by the name:
    // opened by it to be made, a link to a name outside the destination
    // was followed out and the file made there.
    let resolved = resolve_destination(dst, within)?;
    let keep = |file: &File| cleanup.then(|| keep_handle(file, dst)).transpose();
    // cr-ecm25-4: where nothing stands, the partial is made staged —
    // private, at its length, its cleanup's handle had — and takes the name
    // only while nothing does. Opened by the name to be made, an empty file
    // stood there before its cleanup's handle was had, and stayed when that
    // was refused.
    let first = match resolved.open(libc::O_RDWR) {
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
            match resolved.open(libc::O_RDONLY) {
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
    let found = file.metadata()?;
    // cr-ecm22-4: a source of mode 0000 is a mode known, and the most
    // private of all — it was taken for no mode, and the destination's
    // readers kept it.
    let wider = source_mode.is_some_and(|source| found.mode() & 0o077 & !(source & 0o077) != 0);
    // cr-ecm23-4: and only a file this run's user owns is reused: one
    // another owns stays theirs, readable by them whatever its mode, and
    // what lands in it would be theirs to read. A run as root may set any
    // file's mode, so that was no test of it.
    let foreign = found.uid() != effective_user(dst);
    if !wider && !foreign {
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
            let (file, kept) = if writable {
                (file, kept_early)
            } else {
                // The same file, now writable, or nothing is had.
                let again = resolved
                    .open(libc::O_RDWR)
                    .with_context(|| format!("opening destination: {}", dst.display()))?;
                let (was, now) = (file.metadata()?, again.metadata()?);
                if (was.dev(), was.ino()) != (now.dev(), now.ino()) {
                    eyre::bail!(
                        "the partial {} changed while it was made writable",
                        dst.display()
                    );
                }
                // Made writable to be had, and not had: its mode goes back.
                let kept = match keep(&again) {
                    Ok(kept) => kept,
                    Err(error) => {
                        use std::os::unix::fs::PermissionsExt as _;
                        let _ = again.set_permissions(std::fs::Permissions::from_mode(
                            found.mode() & 0o7777,
                        ));
                        return Err(error)
                            .with_context(|| format!("holding the partial {}", dst.display()));
                    }
                };
                (again, kept)
            };
            let len = file.metadata()?.len();
            return Ok((file, kept, len));
        }
    }
    drop(file);
    let (file, kept) = resolved.fresh(dst, src_len.saturating_add(1), |file| {
        keep(file).map_err(eyre::Report::new)
    })?;
    Ok((file, kept, 0))
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

/// cr-ecm21-3: a FRESH private partial for `dst`, made without destroying
/// what stands there first: made beside the file the name holds — or, for
/// a link standing at the name, beside the file it names, which is
/// replaced and the link kept — private and writable by its owner (600)
/// from the moment it exists, and only then renamed over it. If it cannot
/// be made, what stands there is as it was: it was removed, or cut to
/// nothing through the link, before the new one could be had.
///
/// cr-ecm22-1: and it is READY first — its mode, and its length
/// (`unfinished`: one byte longer than its source) — before it takes the
/// destination's place: renamed first, a length the system then refused left
/// the destination an empty file.
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
pub fn fresh_partial(dst: &Path, unfinished: u64, within: Option<&Path>) -> Result<File> {
    fresh_partial_ready(dst, unfinished, within, |_| Ok(())).map(|(file, ())| file)
}

/// cr-ecm23-1: [`fresh_partial`], with `ready` run on the staged file —
/// private and at its length — before it is renamed over the destination:
/// what a caller must know of it (which file it is) is known before it
/// takes the name, and an error there leaves the destination as it stood.
/// Learned after the rename, a look that failed left the destination
/// replaced by a partial nothing held.
#[cfg(unix)]
pub fn fresh_partial_ready<T>(
    dst: &Path,
    unfinished: u64,
    within: Option<&Path>,
    ready: impl FnOnce(&File) -> Result<T>,
) -> Result<(File, T)> {
    resolve_destination(dst, within)?.fresh(dst, unfinished, ready)
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

/// [`Resolved`]: the place `dst` names, `within` the destination's root
/// (canonical) when one is given.
#[cfg(unix)]
pub(crate) fn resolve_destination(dst: &Path, within: Option<&Path>) -> Result<Resolved> {
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
    let mut target = dst.to_path_buf();
    let mut links = 0;
    while std::fs::symlink_metadata(&target).is_ok_and(|there| there.file_type().is_symlink()) {
        // As many as opening the name would follow (Linux's own limit).
        links += 1;
        if links > 40 {
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
    Ok(Resolved {
        directory: held,
        name: c_string(&name)?,
    })
}

#[cfg(unix)]
impl Resolved {
    /// The file at the name, opened with `flags` in the held directory and
    /// never through a link; one made is private (600).
    pub(crate) fn open(&self, flags: libc::c_int) -> std::io::Result<File> {
        use std::os::unix::io::AsRawFd as _;
        open_in(self.directory.as_raw_fd(), &self.name, flags)
    }

    /// cr-ecm21-3: a FRESH private partial at the name, made without
    /// destroying what stands there first — staged beside it in the held
    /// directory, private and writable by its owner (600) from the moment it
    /// exists, made READY (cr-ecm22-1: its length; cr-ecm23-1: what `ready`
    /// learns of it), and only then renamed over it. If it cannot be, what
    /// stands there is as it was.
    pub(crate) fn fresh<T>(
        &self,
        dst: &Path,
        unfinished: u64,
        ready: impl FnOnce(&File) -> Result<T>,
    ) -> Result<(File, T)> {
        self.fresh_as(dst, unfinished, ready, false)
    }

    /// cr-ecm25-2: [`Self::fresh`] where nothing stands at the name: it takes
    /// the name only while nothing does — what comes to stand there first is
    /// left as it is and the partial goes, as a creation that fails if the
    /// name is taken would.
    pub(crate) fn fresh_exclusive<T>(
        &self,
        dst: &Path,
        unfinished: u64,
        ready: impl FnOnce(&File) -> Result<T>,
    ) -> Result<(File, T)> {
        self.fresh_as(dst, unfinished, ready, true)
    }

    /// Whether anything stands at the name in the held directory — itself,
    /// a link being a link.
    pub(crate) fn stands(&self) -> std::io::Result<bool> {
        use std::os::unix::io::AsRawFd as _;
        let mut found = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: the held directory is open for the length of the call, the
        // name is NUL-terminated and outlives it, and `found` is a writable
        // record the call fills on success.
        let looked = unsafe {
            libc::fstatat(
                self.directory.as_raw_fd(),
                self.name.as_ptr(),
                found.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if looked == 0 {
            return Ok(true);
        }
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::NotFound {
            return Ok(false);
        }
        Err(error)
    }

    fn fresh_as<T>(
        &self,
        dst: &Path,
        unfinished: u64,
        ready: impl FnOnce(&File) -> Result<T>,
        exclusive: bool,
    ) -> Result<(File, T)> {
        use std::os::unix::fs::PermissionsExt as _;
        use std::os::unix::io::AsRawFd as _;
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        before_a_fresh_partial_is_made(dst);
        // cr-ecm22-6: by a short name of blit's own beside it, whatever the
        // destination is called — the destination's name in it made a name too
        // long for a destination whose own name was the longest a name can be
        // — and another is tried when that one is taken.
        let mut tries = 0;
        let (staged, file) = loop {
            let staged = c_string(std::ffi::OsStr::new(&format!(
                ".blit-partial-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )))?;
            let error = match open_in(
                self.directory.as_raw_fd(),
                &staged,
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            ) {
                Ok(made) => break (staged, made),
                Err(error) => error,
            };
            if error.kind() == std::io::ErrorKind::AlreadyExists && tries < 16 {
                tries += 1;
                continue;
            }
            return Err(error)
                .with_context(|| format!("making a fresh partial for {}", dst.display()));
        };
        let unmade = || {
            // SAFETY: the held directory is open for the length of the call,
            // and `staged` is a NUL-terminated name that outlives it.
            unsafe { libc::unlinkat(self.directory.as_raw_fd(), staged.as_ptr(), 0) };
        };
        let prepared = file
            .set_permissions(std::fs::Permissions::from_mode(0o600))
            .and_then(|()| file.set_len(unfinished))
            .map_err(eyre::Report::new)
            .and_then(|()| ready(&file));
        let readied = match prepared {
            Ok(readied) => readied,
            Err(error) => {
                unmade();
                return Err(error.wrap_err(format!(
                    "making a fresh partial ready for {}",
                    dst.display()
                )));
            }
        };
        if exclusive {
            // A second name for the staged file, made only while nothing has
            // it, and the staged name let go.
            // SAFETY: the held directory is open for the length of the call,
            // and both names are NUL-terminated and outlive it.
            let linked = unsafe {
                libc::linkat(
                    self.directory.as_raw_fd(),
                    staged.as_ptr(),
                    self.directory.as_raw_fd(),
                    self.name.as_ptr(),
                    0,
                )
            };
            if linked == 0 {
                unmade();
                return Ok((file, readied));
            }
            let error = std::io::Error::last_os_error();
            // A filesystem with no second names (FAT) refuses the link: there
            // the staged file is renamed in only while nothing stands at the
            // name, looked at just before — the one window it leaves.
            let no_links = matches!(error.raw_os_error(), Some(libc::EPERM | libc::EOPNOTSUPP));
            if !no_links || self.stands().unwrap_or(true) {
                unmade();
                let error = if no_links {
                    std::io::Error::from(std::io::ErrorKind::AlreadyExists)
                } else {
                    error
                };
                return Err(error).with_context(|| {
                    format!(
                        "putting a fresh partial where nothing stood, {}",
                        dst.display()
                    )
                });
            }
        }
        // SAFETY: the held directory is open for the length of the call, and
        // both names are NUL-terminated and outlive it.
        if unsafe {
            libc::renameat(
                self.directory.as_raw_fd(),
                staged.as_ptr(),
                self.directory.as_raw_fd(),
                self.name.as_ptr(),
            )
        } != 0
        {
            let error = std::io::Error::last_os_error();
            unmade();
            return Err(error)
                .with_context(|| format!("putting a fresh partial in place of {}", dst.display()));
        }
        Ok((file, readied))
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
    use std::os::unix::io::{FromRawFd as _, OwnedFd};
    // SAFETY: `directory` is open for the length of the call, and `name` is
    // a NUL-terminated string that outlives it.
    let opened = unsafe {
        libc::openat(
            directory,
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600 as libc::c_uint,
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
    within: Option<&Path>,
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
    _within: Option<&Path>,
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

    /// cr-ecm21-3: a destination that grants others what its source does
    /// not is not resumed in place — a reader who has it open would read
    /// the source's bytes as they land — but into a fresh private partial
    /// put in its place; and one no fresh partial can be put beside is left
    /// exactly as it stands.
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

    /// cr-ecm24-2: a resume refused the second handle its caller's cleanup
    /// needs leaves the destination as it stood: the handle is had while the
    /// fresh partial is staged. Taken after, a refusal (`EMFILE`) left nine
    /// bytes of nothing where an eight-byte file had been.
    #[cfg(unix)]
    #[test]
    fn a_resume_refused_its_cleanup_handle_leaves_the_destination() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempdir()?;
        let dst = tmp.path().join("dst");
        std::fs::write(&dst, b"8 bytes!")?;
        std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o644))?;
        KEEP_FAULTS.lock().unwrap().push(dst.clone());
        let had = open_resume_partial_held(&dst, Some(0o100600), 8, None);
        KEEP_FAULTS.lock().unwrap().retain(|at| at != &dst);
        assert!(had.is_err());
        assert_eq!(std::fs::read(&dst)?, b"8 bytes!", "as it stood");
        assert_eq!(
            std::fs::read_dir(tmp.path())?.count(),
            1,
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
        assert!(open_resume_partial(&at, Some(0o100600), 10, Some(&within)).is_err());
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
        let file = fresh_partial(&dst, 11, Some(&root))?;
        assert_eq!(file.metadata()?.len(), 11);
        assert_eq!(std::fs::metadata(&dst)?.len(), 11, "in its own place");
        let beside = tmp.path().join("beside.bin");
        std::fs::write(&beside, b"x")?;
        assert!(fresh_partial(&beside, 11, Some(&root)).is_err(), "outside");
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
