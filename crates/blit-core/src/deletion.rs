//! Shared contained filesystem deletion for mirror and explicit purge paths.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DeletionStats {
    pub files: u64,
    pub dirs: u64,
}

impl DeletionStats {
    pub fn total(self) -> u64 {
        self.files + self.dirs
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectoryMode {
    /// Remove only empty directories. A filtered mirror may tolerate a
    /// non-empty directory because it can contain deliberately out-of-scope
    /// entries.
    EmptyOnly { tolerate_nonempty: bool },
    /// Remove each listed directory and its complete subtree. This is reserved
    /// for the explicit purge API, whose caller named that directory.
    Recursive,
}

#[derive(Debug)]
pub enum DeletionError {
    Cancelled {
        operation: &'static str,
    },
    Containment {
        operation: &'static str,
        target: PathBuf,
        source: eyre::Report,
    },
    Filesystem {
        operation: &'static str,
        action: &'static str,
        target: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for DeletionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled { operation } => {
                write!(formatter, "{operation} aborted: cancellation requested")
            }
            Self::Containment {
                operation,
                target,
                source,
            } => write!(
                formatter,
                "{operation} containment {}: {source:#}",
                target.display()
            ),
            Self::Filesystem {
                operation,
                action,
                target,
                source,
            } => write!(
                formatter,
                "{operation} {action} {}: {source}",
                target.display()
            ),
        }
    }
}

impl std::error::Error for DeletionError {}

pub struct DeletionOptions<'a> {
    pub operation: &'static str,
    pub canonical_root: Option<&'a Path>,
    pub abort: Option<&'a AtomicBool>,
    pub execute: bool,
    pub directory_mode: DirectoryMode,
}

/// Resolve and classify explicit purge targets without following symlinks.
/// Containment is verified before even reading target metadata, then verified
/// again by [`execute_deletion_plan`] immediately before mutation.
pub fn classify_explicit_targets(
    module_root: &Path,
    canonical_root: &Path,
    rel_paths: impl IntoIterator<Item = PathBuf>,
    operation: &'static str,
) -> Result<(Vec<PathBuf>, Vec<PathBuf>), DeletionError> {
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    for rel in rel_paths {
        if rel.as_os_str().is_empty() || rel == Path::new(".") {
            continue;
        }
        let target = module_root.join(rel);
        crate::path_safety::verify_contained(canonical_root, &target).map_err(|source| {
            DeletionError::Containment {
                operation,
                target: target.clone(),
                source,
            }
        })?;
        let metadata = match std::fs::symlink_metadata(&target) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(DeletionError::Filesystem {
                    operation,
                    action: "stat",
                    target,
                    source,
                });
            }
        };
        if metadata.file_type().is_dir() {
            dirs.push(target);
        } else {
            files.push(target);
        }
    }
    Ok((files, dirs))
}

/// Execute one already-ordered deletion plan.
///
/// Files are processed before directories. Callers that use `EmptyOnly` must
/// provide directories deepest-first; recursive purge callers do not depend on
/// directory ordering. Every target is checked against `canonical_root` before
/// it is counted or touched. Missing entries are tolerated for idempotence.
pub fn execute_deletion_plan(
    files: &[PathBuf],
    dirs: &[PathBuf],
    options: DeletionOptions<'_>,
) -> Result<DeletionStats, DeletionError> {
    execute_deletion_plan_reporting(files, dirs, options, &mut |_, _| {})
}

/// [`execute_deletion_plan`], calling `removed(target, is_dir)` for each
/// entry it actually removed — not for one already missing, and not on a
/// dry run (`execute: false`) — so a job log can name every deletion
/// (JOB_LOGS jl-1b).
pub fn execute_deletion_plan_reporting(
    files: &[PathBuf],
    dirs: &[PathBuf],
    options: DeletionOptions<'_>,
    removed: &mut dyn FnMut(&Path, bool),
) -> Result<DeletionStats, DeletionError> {
    let prepare = |target: &Path| -> Result<(), DeletionError> {
        if options
            .abort
            .is_some_and(|abort| abort.load(Ordering::Acquire))
        {
            return Err(DeletionError::Cancelled {
                operation: options.operation,
            });
        }
        if let Some(root) = options.canonical_root {
            crate::path_safety::verify_contained(root, target).map_err(|source| {
                DeletionError::Containment {
                    operation: options.operation,
                    target: target.to_path_buf(),
                    source,
                }
            })?;
        }
        Ok(())
    };
    let filesystem_error =
        |action: &'static str, target: &Path, source: std::io::Error| DeletionError::Filesystem {
            operation: options.operation,
            action,
            target: target.to_path_buf(),
            source,
        };

    let mut stats = DeletionStats::default();
    for file in files {
        prepare(file)?;
        if !options.execute {
            stats.files += 1;
            continue;
        }
        #[cfg(windows)]
        crate::win_fs::clear_readonly_recursive(file);
        match std::fs::remove_file(file) {
            Ok(()) => {
                stats.files += 1;
                removed(file, false);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error)
                if error.kind() == std::io::ErrorKind::IsADirectory
                    && options.directory_mode == DirectoryMode::Recursive =>
            {
                #[cfg(windows)]
                crate::win_fs::clear_readonly_recursive(file);
                match std::fs::remove_dir_all(file) {
                    Ok(()) => {
                        stats.dirs += 1;
                        removed(file, true);
                    }
                    Err(inner) if inner.kind() == std::io::ErrorKind::NotFound => {}
                    Err(inner) => {
                        return Err(filesystem_error("remove directory tree", file, inner))
                    }
                }
            }
            Err(error) => return Err(filesystem_error("remove file", file, error)),
        }
    }

    for dir in dirs {
        prepare(dir)?;
        if !options.execute {
            stats.dirs += 1;
            continue;
        }
        #[cfg(windows)]
        crate::win_fs::clear_readonly_recursive(dir);
        let result = match options.directory_mode {
            DirectoryMode::EmptyOnly { .. } => std::fs::remove_dir(dir),
            DirectoryMode::Recursive => std::fs::remove_dir_all(dir),
        };
        match result {
            Ok(()) => {
                stats.dirs += 1;
                removed(dir, true);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error)
                if matches!(
                    options.directory_mode,
                    DirectoryMode::EmptyOnly {
                        tolerate_nonempty: true
                    }
                ) && (error.kind() == std::io::ErrorKind::DirectoryNotEmpty
                    || error.raw_os_error() == Some(66)) => {}
            Err(error) => {
                let action = match options.directory_mode {
                    DirectoryMode::EmptyOnly { .. } => "remove directory",
                    DirectoryMode::Recursive => "remove directory tree",
                };
                return Err(filesystem_error(action, dir, error));
            }
        }
    }

    Ok(stats)
}

/// One entry [`remove_as_rm`] could not remove, and why.
#[derive(Debug)]
pub struct RemovalFailure {
    /// The entry, below the operand; empty for the operand itself.
    pub relative_path: PathBuf,
    pub error: std::io::Error,
}

/// What became of one operand of a removal.
#[derive(Debug, Default)]
pub struct Removal {
    /// Entries removed: the operand and everything below it.
    pub removed: u64,
    /// Each entry that could not be removed, in the order met. Empty when
    /// the operand is gone (or, with `force`, was never there).
    pub failures: Vec<RemovalFailure>,
    /// More entries failed than `failures` lists.
    pub failures_truncated: bool,
}

/// How [`remove_as_rm`] treats one operand: `rm`'s own two options.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RmOptions {
    /// `-r`: a directory is removed with everything in it. Without it a
    /// directory is refused — "Is a directory" — and nothing is touched.
    pub recursive: bool,
    /// `-f`: an operand that is not there is no failure.
    pub force: bool,
}

/// The most failures one operand lists; the rest are counted as truncated.
pub const MAX_LISTED_REMOVAL_FAILURES: usize = 1000;

impl Removal {
    /// Record one entry that could not be removed.
    fn fail(&mut self, relative_path: PathBuf, error: std::io::Error) {
        if self.failures.len() < MAX_LISTED_REMOVAL_FAILURES {
            self.failures.push(RemovalFailure {
                relative_path,
                error,
            });
        } else {
            self.failures_truncated = true;
        }
    }
}

/// Remove one operand the way `rm` does (UTILITY_VERBS_LOCAL_PARITY
/// slice 2, AC-3 and AC-12):
///
/// - an operand that is not there fails with the system's own error
///   (`No such file or directory`), unless `force`;
/// - a directory fails with `Is a directory` unless `recursive`, and
///   nothing in it is touched;
/// - a symbolic link is removed as the link it is and never followed, at
///   the operand and below it;
/// - a tree is removed contents first, and an entry that cannot be removed
///   is REPORTED and passed over — the walk goes on, and every directory
///   above the failure is left standing without a second report, as
///   `rm -r` leaves it. ([`execute_deletion_plan`] stops at the first
///   failure; a mirror's delete pass wants that, a user's `rm` does not.)
///
/// `target` must lie in `canonical_root`, and that is found out before
/// anything is removed. One that does not — or whose place cannot be found
/// out, below a directory that cannot be searched — is a FAILURE OF THAT
/// OPERAND, like any other (cr-uv1-3): `rm a blocked/b c` removes `a` and
/// `c` and says why `blocked/b` stays. `Err` is for what is not about one
/// operand: a cancelled operation.
///
/// # Anchoring (cr-uv1-2)
///
/// A check made on a path says nothing about what that path names a
/// moment later: a directory inside the root can be swapped for a link
/// that leaves it. So on Unix nothing here is found by a path twice. The
/// operand's directory is reached from the root one name at a time, each
/// opened IN the directory before it and never through a link; every
/// directory of the tree is opened the same way, IN the one that listed
/// it; and a directory is listed, and emptied, through the one descriptor
/// it was opened with (`anchored::Dir`). What cannot be opened so — a
/// directory that became a link, one that cannot be read — is one
/// reported failure, and nothing is removed by its path instead.
///
/// Elsewhere the standard library's own recursive removal does the tree:
/// it is anchored to handles in the same way, but it stops at the first
/// entry it cannot remove, so a tree with one such entry is left with
/// whatever had not gone yet and one failure, reported for the operand.
pub fn remove_as_rm(
    target: &Path,
    canonical_root: Option<&Path>,
    options: RmOptions,
    abort: Option<&AtomicBool>,
) -> Result<Removal, DeletionError> {
    let mut removal = Removal::default();
    remove_operand(target, canonical_root, options, abort, &mut removal)?;
    Ok(removal)
}

const RM_OPERATION: &str = "rm";

/// Why an operand that leaves the root is not removed.
fn leaves_the_root() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "the path leaves the module",
    )
}

/// `rm`'s words for a directory without `-r`, on every platform.
fn is_a_directory() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::IsADirectory, "Is a directory")
}

#[cfg(unix)]
fn remove_operand(
    target: &Path,
    canonical_root: Option<&Path>,
    options: RmOptions,
    abort: Option<&AtomicBool>,
    removal: &mut Removal,
) -> Result<(), DeletionError> {
    use std::os::unix::ffi::OsStrExt as _;

    // With `-f`, an operand that is not there is no failure — and neither
    // is one below something that is no directory, as for `rm -f`.
    let absent = |error: &std::io::Error| {
        options.force
            && matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            )
    };
    let name = target
        .file_name()
        .and_then(|name| std::ffi::CString::new(name.as_bytes()).ok());
    let Some(name) = name else {
        removal.fail(
            PathBuf::new(),
            std::io::Error::from(std::io::ErrorKind::InvalidInput),
        );
        return Ok(());
    };
    let parent = match target.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let canonical = match std::fs::canonicalize(parent) {
        Ok(canonical) => canonical,
        Err(error) if absent(&error) => return Ok(()),
        Err(error) => {
            removal.fail(PathBuf::new(), error);
            return Ok(());
        }
    };
    // Never "not there": the directory was there a moment ago, and one
    // that cannot be reached now is reported, `-f` or not.
    let above = match anchored::Dir::open_canonical(&canonical, canonical_root) {
        Ok(above) => above,
        Err(error) => {
            removal.fail(PathBuf::new(), error);
            return Ok(());
        }
    };
    let is_dir = match above.is_directory(&name) {
        Ok(is_dir) => is_dir,
        Err(error) if absent(&error) => return Ok(()),
        Err(error) => {
            removal.fail(PathBuf::new(), error);
            return Ok(());
        }
    };
    if !is_dir {
        match above.remove(&name, false) {
            Ok(()) => removal.removed += 1,
            Err(error) => removal.fail(PathBuf::new(), error),
        }
        return Ok(());
    }
    if !options.recursive {
        removal.fail(PathBuf::new(), is_a_directory());
        return Ok(());
    }

    /// One directory being emptied.
    struct Frame {
        relative_path: PathBuf,
        /// Its name in the directory above it.
        name: std::ffi::CString,
        /// The directory itself, held: listed and emptied through this.
        dir: anchored::Dir,
        /// Something below could not be removed, so this cannot be either.
        kept: bool,
    }
    let mut frames: Vec<Frame> = Vec::new();
    match above.open_child(&name) {
        Ok(dir) => frames.push(Frame {
            relative_path: PathBuf::new(),
            name,
            dir,
            kept: false,
        }),
        Err(error) => {
            removal.fail(PathBuf::new(), error);
            return Ok(());
        }
    }
    while let Some(frame) = frames.last_mut() {
        if abort.is_some_and(|abort| abort.load(Ordering::Acquire)) {
            return Err(DeletionError::Cancelled {
                operation: RM_OPERATION,
            });
        }
        let Some(entry) = frame.dir.next_entry() else {
            // Emptied, or as empty as it will get.
            let Frame {
                relative_path,
                name,
                dir,
                mut kept,
            } = frames.pop().expect("the frame just read");
            drop(dir);
            if !kept {
                // By its name, in the directory that listed it.
                let within = frames.last().map_or(&above, |above| &above.dir);
                match within.remove(&name, true) {
                    Ok(()) => removal.removed += 1,
                    Err(error) => {
                        removal.fail(relative_path, error);
                        kept = true;
                    }
                }
            }
            if let Some(above) = frames.last_mut() {
                above.kept |= kept;
            }
            continue;
        };
        let (name, listed_as_dir) = match entry {
            Ok(entry) => entry,
            Err(error) => {
                let relative_path = frame.relative_path.clone();
                frame.kept = true;
                removal.fail(relative_path, error);
                continue;
            }
        };
        let relative_path = frame
            .relative_path
            .join(std::ffi::OsStr::from_bytes(name.to_bytes()));
        // What the listing says it is; asked of the entry itself — never
        // of what a link points to — where the listing does not say.
        let is_dir = match listed_as_dir {
            Some(is_dir) => Ok(is_dir),
            None => frame.dir.is_directory(&name),
        };
        match is_dir {
            Ok(true) => {
                between("descending", &relative_path);
                match frame.dir.open_child(&name) {
                    Ok(dir) => {
                        frames.push(Frame {
                            relative_path,
                            name,
                            dir,
                            kept: false,
                        });
                        continue;
                    }
                    // No directory stands there any more: whatever does —
                    // a link put in its place — is removed as what it is.
                    Err(error) if anchored::is_no_directory(&error) => {}
                    Err(error) => {
                        frame.kept = true;
                        removal.fail(relative_path, error);
                        continue;
                    }
                }
            }
            Ok(false) => {}
            Err(error) => {
                frame.kept = true;
                removal.fail(relative_path, error);
                continue;
            }
        }
        match frame.dir.remove(&name, false) {
            Ok(()) => removal.removed += 1,
            Err(error) => {
                frame.kept = true;
                removal.fail(relative_path, error);
            }
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn remove_operand(
    target: &Path,
    canonical_root: Option<&Path>,
    options: RmOptions,
    abort: Option<&AtomicBool>,
    removal: &mut Removal,
) -> Result<(), DeletionError> {
    // Where the operand lies is checked by its path here, and what the
    // check finds is this operand's failure (cr-uv1-3).
    if let Some(root) = canonical_root {
        if crate::path_safety::verify_contained(root, target).is_err() {
            removal.fail(PathBuf::new(), leaves_the_root());
            return Ok(());
        }
    }
    let metadata = match std::fs::symlink_metadata(target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && options.force => {
            return Ok(());
        }
        Err(error) => {
            removal.fail(PathBuf::new(), error);
            return Ok(());
        }
    };
    if !metadata.is_dir() {
        #[cfg(windows)]
        crate::win_fs::clear_readonly_recursive(target);
        match std::fs::remove_file(target) {
            Ok(()) => removal.removed += 1,
            Err(error) => removal.fail(PathBuf::new(), error),
        }
        return Ok(());
    }
    if !options.recursive {
        removal.fail(PathBuf::new(), is_a_directory());
        return Ok(());
    }
    if abort.is_some_and(|abort| abort.load(Ordering::Acquire)) {
        return Err(DeletionError::Cancelled {
            operation: RM_OPERATION,
        });
    }
    // A recursive removal clears read-only attributes through the tree
    // first, as the purge it replaces did.
    #[cfg(windows)]
    crate::win_fs::clear_readonly_recursive(target);
    // Entries at and below a path, links counted and never followed: a
    // count for the report, never a decision about what is removed.
    let entries = |path: &Path| {
        walkdir::WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .count() as u64
    };
    let before = entries(target);
    // The standard library's removal: anchored to the handles it opens,
    // never to a path looked up twice (cr-uv1-2).
    match std::fs::remove_dir_all(target) {
        Ok(()) => removal.removed += before,
        Err(error) => {
            removal.removed += before.saturating_sub(entries(target));
            removal.fail(PathBuf::new(), error);
        }
    }
    Ok(())
}

/// A moment between a check and the act it guards, for a test to change
/// the tree in: `stage` says which, `path` what is about to be opened.
#[cfg(unix)]
fn between(_stage: &'static str, _path: &Path) {
    #[cfg(test)]
    BETWEEN.with(|hook| {
        if let Some(hook) = hook.borrow_mut().as_mut() {
            hook(_stage, _path);
        }
    });
}

#[cfg(all(test, unix))]
type Between = Box<dyn FnMut(&'static str, &Path)>;

#[cfg(all(test, unix))]
thread_local! {
    static BETWEEN: std::cell::RefCell<Option<Between>> = const { std::cell::RefCell::new(None) };
}

/// Directories held open, and everything done through them by name
/// (cr-uv1-2). Unix only.
#[cfg(unix)]
mod anchored {
    use std::ffi::{CStr, CString};
    use std::io;
    use std::os::unix::ffi::OsStrExt as _;
    use std::os::unix::io::RawFd;
    use std::path::{Component, Path};
    use std::ptr::NonNull;

    #[cfg(not(target_os = "linux"))]
    use libc::{fstatat, readdir, stat};
    #[cfg(target_os = "linux")]
    use libc::{fstatat64 as fstatat, readdir64 as readdir, stat64 as stat};

    /// One open directory. Its entries are listed from it, opened in it
    /// and removed from it, each by its name there: the directory is
    /// found once, when it is opened, and never looked up again.
    pub(super) struct Dir {
        /// The listing stream; it owns the descriptor.
        stream: NonNull<libc::DIR>,
        /// The listing has ended, or failed and will not be read again.
        ended: bool,
    }

    impl Dir {
        /// The directory a canonical path names, reached without trusting
        /// what a path names twice. `canonical` — a path resolved to its
        /// canonical form, no link in it — must lie in `root`; it is
        /// opened from `root` down, one name at a time, each IN the
        /// directory before it and never through a link. A directory
        /// replaced by a link after the resolution is therefore an error
        /// here, not a way out of `root`. With no root, the path is opened.
        pub(super) fn open_canonical(canonical: &Path, root: Option<&Path>) -> io::Result<Self> {
            super::between("opening the operand's directory", canonical);
            let Some(root) = root else {
                return Self::open_path(canonical);
            };
            let below = canonical
                .strip_prefix(root)
                .map_err(|_| super::leaves_the_root())?;
            let mut dir = Self::open_path(root)?;
            for component in below.components() {
                let Component::Normal(name) = component else {
                    return Err(io::Error::from(io::ErrorKind::InvalidInput));
                };
                dir = dir.open_child(&CString::new(name.as_bytes())?)?;
            }
            Ok(dir)
        }

        /// The directory at `path`.
        fn open_path(path: &Path) -> io::Result<Self> {
            let path = CString::new(path.as_os_str().as_bytes())?;
            // SAFETY: `path` is a NUL-terminated string that outlives the
            // call; `open` reads it and keeps nothing.
            let fd = unsafe {
                libc::open(
                    path.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
                )
            };
            Self::from_descriptor(fd)
        }

        /// The directory called `name` IN this one — itself, never what a
        /// link by that name points to.
        pub(super) fn open_child(&self, name: &CStr) -> io::Result<Self> {
            // SAFETY: the descriptor is this open directory's for the
            // length of the call and `name` is a NUL-terminated string
            // that outlives it; `openat` reads both and keeps neither.
            let fd = unsafe {
                libc::openat(
                    self.descriptor(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            Self::from_descriptor(fd)
        }

        fn from_descriptor(fd: RawFd) -> io::Result<Self> {
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: `fd` is an open directory descriptor this function
            // owns; on success the stream owns it, on failure it is
            // closed here and used no more.
            match NonNull::new(unsafe { libc::fdopendir(fd) }) {
                Some(stream) => Ok(Self {
                    stream,
                    ended: false,
                }),
                None => {
                    let error = io::Error::last_os_error();
                    // SAFETY: as above — still this function's descriptor.
                    unsafe { libc::close(fd) };
                    Err(error)
                }
            }
        }

        fn descriptor(&self) -> RawFd {
            // SAFETY: the stream is open until this value is dropped.
            unsafe { libc::dirfd(self.stream.as_ptr()) }
        }

        /// The next entry of this directory: its name, and whether it is
        /// a directory where the listing says (`None` where it does not).
        /// `.` and `..` are passed over. After the end, or an error —
        /// given once — there is nothing more.
        pub(super) fn next_entry(&mut self) -> Option<io::Result<(CString, Option<bool>)>> {
            loop {
                if self.ended {
                    return None;
                }
                let told_apart = clear_errno();
                // SAFETY: the stream is open, and no other call reads it
                // while this one does (`&mut self`).
                let entry = unsafe { readdir(self.stream.as_ptr()) };
                if entry.is_null() {
                    self.ended = true;
                    let error = io::Error::last_os_error();
                    return match error.raw_os_error() {
                        Some(code) if told_apart && code != 0 => Some(Err(error)),
                        _ => None,
                    };
                }
                // SAFETY: a non-null entry points at a record valid until
                // the next read of this stream; its name is NUL-terminated
                // within it. The name is copied before that next read.
                let (name, kind) = unsafe {
                    (
                        CStr::from_ptr(std::ptr::addr_of!((*entry).d_name).cast()).to_owned(),
                        (*entry).d_type,
                    )
                };
                if matches!(name.to_bytes(), b"." | b"..") {
                    continue;
                }
                let is_dir = match kind {
                    libc::DT_DIR => Some(true),
                    libc::DT_UNKNOWN => None,
                    _ => Some(false),
                };
                return Some(Ok((name, is_dir)));
            }
        }

        /// Whether the entry called `name` in this directory is a
        /// directory — the entry itself, a link being a link.
        pub(super) fn is_directory(&self, name: &CStr) -> io::Result<bool> {
            let mut found = std::mem::MaybeUninit::<stat>::uninit();
            // SAFETY: the descriptor is this open directory's, `name` is a
            // NUL-terminated string and `found` is room for one record,
            // all for the length of the call.
            let answered = unsafe {
                fstatat(
                    self.descriptor(),
                    name.as_ptr(),
                    found.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            };
            if answered != 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: the call succeeded, so the record is filled in.
            let mode = unsafe { found.assume_init() }.st_mode;
            Ok(mode & libc::S_IFMT == libc::S_IFDIR)
        }

        /// Remove the entry called `name` in this directory: an empty
        /// directory when `directory`, anything else otherwise — a link
        /// as the link it is.
        pub(super) fn remove(&self, name: &CStr, directory: bool) -> io::Result<()> {
            let flags = if directory { libc::AT_REMOVEDIR } else { 0 };
            // SAFETY: the descriptor is this open directory's for the
            // length of the call, and `name` is a NUL-terminated string
            // that outlives it; `unlinkat` reads both and keeps neither.
            if unsafe { libc::unlinkat(self.descriptor(), name.as_ptr(), flags) } == 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            // SAFETY: the stream is open and is not used after this.
            unsafe { libc::closedir(self.stream.as_ptr()) };
        }
    }

    /// The error of opening, as a directory and never through a link, a
    /// name that is no directory: a link stands there (`ELOOP`, or
    /// `EMLINK` on some systems), or something else does (`ENOTDIR`).
    pub(super) fn is_no_directory(error: &io::Error) -> bool {
        matches!(
            error.raw_os_error(),
            Some(libc::ENOTDIR | libc::ELOOP | libc::EMLINK)
        )
    }

    /// Reset `errno`, so that nothing from `readdir` can be told apart:
    /// the end of the listing, or an error. `false` where this build does
    /// not know how — the listing then just ends, and a directory left
    /// with entries in it fails to be removed and is reported so.
    fn clear_errno() -> bool {
        #[cfg(any(target_os = "linux", target_os = "emscripten", target_os = "redox"))]
        // SAFETY: the pointer is this thread's own `errno`.
        unsafe {
            *libc::__errno_location() = 0;
            return true;
        }
        #[cfg(any(
            target_os = "macos",
            target_os = "ios",
            target_os = "tvos",
            target_os = "watchos",
            target_os = "freebsd"
        ))]
        // SAFETY: the pointer is this thread's own `errno`.
        unsafe {
            *libc::__error() = 0;
            return true;
        }
        #[cfg(any(target_os = "android", target_os = "netbsd", target_os = "openbsd"))]
        // SAFETY: the pointer is this thread's own `errno`.
        unsafe {
            *libc::__errno() = 0;
            return true;
        }
        #[allow(unreachable_code)]
        false
    }
}

#[cfg(test)]
mod rm_tests {
    //! `remove_as_rm`: one operand, as `rm` treats it
    //! (UTILITY_VERBS_LOCAL_PARITY slice 2). Where the system has the
    //! reference, the end-to-end guards compare with it.
    use super::*;
    use std::fs;

    fn tree() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = fs::canonicalize(tmp.path()).unwrap();
        for dir in ["dir/sub/deep", "empty"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        for file in ["a.txt", "dir/x.txt", "dir/sub/y.txt", "dir/sub/deep/z.txt"] {
            fs::write(root.join(file), b"x").unwrap();
        }
        (tmp, root)
    }

    fn rm(root: &Path, rel: &str, recursive: bool, force: bool) -> Removal {
        remove_as_rm(
            &root.join(rel),
            Some(root),
            RmOptions { recursive, force },
            None,
        )
        .expect("an answer about the operand")
    }

    fn reasons(removal: &Removal) -> Vec<(String, std::io::ErrorKind)> {
        removal
            .failures
            .iter()
            .map(|failure| {
                (
                    failure.relative_path.to_string_lossy().replace('\\', "/"),
                    failure.error.kind(),
                )
            })
            .collect()
    }

    /// AC-3: a file goes; a directory without `-r` is refused with "Is a
    /// directory" and nothing in it is touched — an empty one too; with
    /// `-r` the whole tree goes and every entry is counted.
    #[test]
    fn a_directory_needs_recursive_and_a_file_does_not() {
        let (_tmp, root) = tree();
        let file = rm(&root, "a.txt", false, false);
        assert_eq!((file.removed, file.failures.len()), (1, 0));
        assert!(!root.join("a.txt").exists());

        for dir in ["dir", "empty"] {
            let refused = rm(&root, dir, false, false);
            assert_eq!(refused.removed, 0);
            assert_eq!(
                reasons(&refused),
                [(String::new(), std::io::ErrorKind::IsADirectory)],
                "{dir}"
            );
        }
        assert!(root.join("dir/sub/deep/z.txt").exists() && root.join("empty").exists());

        let tree = rm(&root, "dir", true, false);
        assert!(tree.failures.is_empty(), "{:?}", tree.failures);
        // Three files and three directories.
        assert_eq!(tree.removed, 6);
        assert!(!root.join("dir").exists());
    }

    /// AC-3: an operand that is not there is an error, in the system's own
    /// words, unless `-f`; and `-f` changes nothing else — a directory
    /// still needs `-r`.
    #[test]
    fn a_missing_operand_is_an_error_unless_forced() {
        let (_tmp, root) = tree();
        let missing = rm(&root, "nothing-here", false, false);
        assert_eq!(
            reasons(&missing),
            [(String::new(), std::io::ErrorKind::NotFound)]
        );
        assert_eq!(
            crate::disk_usage::os_reason(&missing.failures[0].error),
            if cfg!(windows) {
                "The system cannot find the file specified."
            } else {
                "No such file or directory"
            }
        );
        let forced = rm(&root, "nothing-here", false, true);
        assert_eq!((forced.removed, forced.failures.len()), (0, 0));
        let dir = rm(&root, "dir", false, true);
        assert_eq!(
            reasons(&dir),
            [(String::new(), std::io::ErrorKind::IsADirectory)]
        );
    }

    /// AC-12: a symbolic link is removed as the link it is, at the operand
    /// and inside a tree, and nothing it points to is touched.
    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_is_removed_and_never_followed() {
        let (_tmp, root) = tree();
        fs::create_dir_all(root.join("outside-target")).unwrap();
        fs::write(root.join("outside-target/kept.txt"), b"x").unwrap();
        std::os::unix::fs::symlink(root.join("dir"), root.join("to-dir")).unwrap();
        std::os::unix::fs::symlink(root.join("outside-target"), root.join("dir/sub/to-outside"))
            .unwrap();

        // The link to a directory needs no `-r`: it is not a directory.
        let link = rm(&root, "to-dir", false, false);
        assert_eq!((link.removed, link.failures.len()), (1, 0));
        assert!(root.join("dir/x.txt").exists(), "its target is untouched");

        let tree = rm(&root, "dir", true, false);
        assert!(tree.failures.is_empty(), "{:?}", tree.failures);
        assert!(!root.join("dir").exists());
        assert!(
            root.join("outside-target/kept.txt").exists(),
            "a link inside the tree was removed, not followed"
        );
    }

    /// AC-12: an entry that cannot be removed is reported and the walk
    /// goes on — everything else goes, and each directory above the
    /// failure is left standing with no report of its own, as `rm -r`
    /// leaves it.
    #[cfg(unix)]
    #[test]
    fn a_removal_goes_on_past_an_entry_it_cannot_remove() {
        use std::os::unix::fs::PermissionsExt as _;
        let (_tmp, root) = tree();
        fs::create_dir_all(root.join("dir/locked")).unwrap();
        fs::write(root.join("dir/locked/held.txt"), b"x").unwrap();
        fs::set_permissions(root.join("dir/locked"), fs::Permissions::from_mode(0o555)).unwrap();
        let privileged = fs::write(root.join("dir/locked/probe"), b"x").is_ok();
        let _ = fs::remove_file(root.join("dir/locked/probe"));

        let removal = rm(&root, "dir", true, false);
        fs::set_permissions(root.join("dir/locked"), fs::Permissions::from_mode(0o755)).unwrap();
        if privileged {
            eprintln!("skipping: running privileged, mode 555 does not stop a removal");
            return;
        }
        assert_eq!(
            reasons(&removal),
            [(
                "locked/held.txt".to_string(),
                std::io::ErrorKind::PermissionDenied
            )],
            "the one entry, once — not the directories above it"
        );
        assert!(root.join("dir/locked/held.txt").exists());
        assert!(
            !root.join("dir/x.txt").exists() && !root.join("dir/sub").exists(),
            "everything else went"
        );
        // Three files and two directories (`sub`, `sub/deep`); `dir` and
        // `dir/locked` stand.
        assert_eq!(removal.removed, 5);
    }

    /// A target outside the root is that operand's failure — `-f` or not,
    /// by its own path or through a link that leaves — and nothing of it
    /// is touched (cr-uv1-3: it used to end the whole request, with what
    /// the operands before it had done unreported). A cancelled removal
    /// stops.
    #[test]
    fn a_target_outside_the_root_is_a_failure_of_that_operand() {
        let (_tmp, root) = tree();
        let outside = tempfile::tempdir().unwrap();
        let elsewhere = fs::canonicalize(outside.path()).unwrap();
        fs::write(elsewhere.join("victim.txt"), b"x").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&elsewhere, root.join("way-out")).unwrap();
        let mut targets = vec![elsewhere.join("victim.txt")];
        if cfg!(unix) {
            targets.push(root.join("way-out/victim.txt"));
        }
        for target in targets {
            for force in [false, true] {
                let removal = remove_as_rm(
                    &target,
                    Some(&root),
                    RmOptions {
                        recursive: true,
                        force,
                    },
                    None,
                )
                .expect("an answer about the operand");
                assert_eq!(removal.removed, 0, "{}", target.display());
                assert_eq!(
                    reasons(&removal),
                    [(String::new(), std::io::ErrorKind::PermissionDenied)],
                    "{}",
                    target.display()
                );
                assert_eq!(
                    crate::disk_usage::os_reason(&removal.failures[0].error),
                    "the path leaves the module"
                );
            }
        }
        assert!(elsewhere.join("victim.txt").exists());

        // Below a directory that cannot be searched: the system's own
        // words for it, as `rm` gives them, and still one operand's.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::create_dir_all(root.join("blocked")).unwrap();
            fs::write(root.join("blocked/entry"), b"x").unwrap();
            fs::set_permissions(root.join("blocked"), fs::Permissions::from_mode(0o000)).unwrap();
            let privileged = fs::read_dir(root.join("blocked")).is_ok();
            let removal = rm(&root, "blocked/entry", false, false);
            fs::set_permissions(root.join("blocked"), fs::Permissions::from_mode(0o755)).unwrap();
            if !privileged {
                assert_eq!(
                    reasons(&removal),
                    [(String::new(), std::io::ErrorKind::PermissionDenied)]
                );
                assert_eq!(
                    crate::disk_usage::os_reason(&removal.failures[0].error),
                    "Permission denied"
                );
                assert!(root.join("blocked/entry").exists());
            }
        }

        let abort = AtomicBool::new(true);
        let error = remove_as_rm(
            &root.join("dir"),
            Some(&root),
            RmOptions {
                recursive: true,
                force: false,
            },
            Some(&abort),
        )
        .unwrap_err();
        assert!(matches!(error, DeletionError::Cancelled { .. }), "{error}");
        assert!(root.join("dir/x.txt").exists());
    }

    /// What a test does between a check and the act it guards, on this
    /// thread, until the value returned is dropped.
    #[cfg(unix)]
    fn between_each(hook: impl FnMut(&'static str, &Path) + 'static) -> impl Drop {
        struct Cleared;
        impl Drop for Cleared {
            fn drop(&mut self) {
                BETWEEN.with(|between| between.borrow_mut().take());
            }
        }
        BETWEEN.with(|between| *between.borrow_mut() = Some(Box::new(hook)));
        Cleared
    }

    /// cr-uv1-2: a directory of the tree that is swapped for a link after
    /// it was listed is not gone into. The walk lists `dir`, finds `sub` a
    /// directory, and before it opens `sub` the directory is moved away
    /// and a link to another tree — with the same names in it — is put in
    /// its place. The removal that looked `dir/sub` up again by its path
    /// listed, and emptied, the other tree. `sub` is opened IN `dir` and
    /// never through a link: what stands there now is removed as the link
    /// it is, and the tree it points to is whole.
    #[cfg(unix)]
    #[test]
    fn a_directory_swapped_for_a_link_mid_walk_is_not_followed() {
        let (_tmp, root) = tree();
        let elsewhere = tempfile::tempdir().unwrap();
        let other = fs::canonicalize(elsewhere.path()).unwrap();
        fs::create_dir_all(other.join("deep")).unwrap();
        for file in ["y.txt", "deep/z.txt"] {
            fs::write(other.join(file), b"not yours").unwrap();
        }
        let swapped = std::rc::Rc::new(std::cell::Cell::new(false));
        let _hook = {
            let (root, other, swapped) = (root.clone(), other.clone(), swapped.clone());
            between_each(move |stage, path| {
                if stage == "descending" && path == Path::new("sub") && !swapped.get() {
                    fs::rename(root.join("dir/sub"), root.join("moved-away")).unwrap();
                    std::os::unix::fs::symlink(&other, root.join("dir/sub")).unwrap();
                    swapped.set(true);
                }
            })
        };
        let removal = rm(&root, "dir", true, false);
        assert!(swapped.get(), "the swap happened while the walk ran");
        assert!(removal.failures.is_empty(), "{:?}", removal.failures);
        assert!(
            other.join("y.txt").exists() && other.join("deep/z.txt").exists(),
            "the tree the link points to is untouched"
        );
        assert!(!root.join("dir").exists(), "the operand went, link and all");
        // `x.txt`, the link, and `dir`.
        assert_eq!(removal.removed, 3);
        assert!(
            root.join("moved-away/deep/z.txt").exists(),
            "what was moved out of the operand is no part of it"
        );
    }

    /// cr-uv1-2: the operand's own directory is reached from the root one
    /// name at a time, never through a link. The operand `dir/sub/y.txt`
    /// is checked to lie in the root; then `dir` is moved away and a link
    /// to another tree, holding `sub/y.txt` too, is put in its place. The
    /// removal by path deleted the other tree's file. Now the directory
    /// cannot be reached, that is the one failure, and nothing is removed.
    #[cfg(unix)]
    #[test]
    fn an_operand_is_not_reached_through_a_directory_swapped_for_a_link() {
        for force in [false, true] {
            let (_tmp, root) = tree();
            let elsewhere = tempfile::tempdir().unwrap();
            let other = fs::canonicalize(elsewhere.path()).unwrap();
            fs::create_dir_all(other.join("sub")).unwrap();
            fs::write(other.join("sub/y.txt"), b"not yours").unwrap();
            let swapped = std::rc::Rc::new(std::cell::Cell::new(false));
            let _hook = {
                let (root, other, swapped) = (root.clone(), other.clone(), swapped.clone());
                between_each(move |stage, _| {
                    if stage == "opening the operand's directory" && !swapped.get() {
                        fs::rename(root.join("dir"), root.join("moved-away")).unwrap();
                        std::os::unix::fs::symlink(&other, root.join("dir")).unwrap();
                        swapped.set(true);
                    }
                })
            };
            let removal = rm(&root, "dir/sub/y.txt", false, force);
            assert!(swapped.get(), "the swap happened after the check");
            assert_eq!(removal.removed, 0, "force {force}");
            assert_eq!(removal.failures.len(), 1, "{:?}", removal.failures);
            assert!(
                removal.failures[0].relative_path.as_os_str().is_empty()
                    && anchored::is_no_directory(&removal.failures[0].error),
                "{:?}",
                removal.failures
            );
            assert!(other.join("sub/y.txt").exists(), "the other tree's file");
            assert!(root.join("moved-away/sub/y.txt").exists());
        }
    }

    /// cr-uv1-2: a directory that cannot be opened is one reported
    /// failure, and nothing in it is removed some other way — there is no
    /// removal by path to fall back on. Everything else goes.
    #[cfg(unix)]
    #[test]
    fn a_directory_that_cannot_be_held_is_reported_and_left() {
        use std::os::unix::fs::PermissionsExt as _;
        let (_tmp, root) = tree();
        fs::create_dir_all(root.join("dir/sealed")).unwrap();
        fs::write(root.join("dir/sealed/held.txt"), b"x").unwrap();
        fs::set_permissions(root.join("dir/sealed"), fs::Permissions::from_mode(0o000)).unwrap();
        let privileged = fs::read_dir(root.join("dir/sealed")).is_ok();

        let removal = rm(&root, "dir", true, false);
        fs::set_permissions(root.join("dir/sealed"), fs::Permissions::from_mode(0o755)).unwrap();
        if privileged {
            eprintln!("skipping: running privileged, mode 000 does not stop a listing");
            return;
        }
        assert_eq!(
            reasons(&removal),
            [("sealed".to_string(), std::io::ErrorKind::PermissionDenied)]
        );
        assert!(root.join("dir/sealed/held.txt").exists());
        assert!(!root.join("dir/x.txt").exists() && !root.join("dir/sub").exists());
        // Three files and two directories; `dir` and `dir/sealed` stand.
        assert_eq!(removal.removed, 5);
    }

    /// cr-uv1-2: the listing is the held directory's own — every entry,
    /// `.` and `..` never among them, a directory told from a link to one.
    #[cfg(unix)]
    #[test]
    fn a_held_directory_lists_and_removes_its_own_entries() {
        let (_tmp, root) = tree();
        std::os::unix::fs::symlink(root.join("dir"), root.join("to-dir")).unwrap();
        let mut held = anchored::Dir::open_canonical(&root, Some(&root)).unwrap();
        let mut listed = Vec::new();
        while let Some(entry) = held.next_entry() {
            let (name, is_dir) = entry.unwrap();
            let is_dir = match is_dir {
                Some(is_dir) => is_dir,
                None => held.is_directory(&name).unwrap(),
            };
            listed.push((name.to_string_lossy().into_owned(), is_dir));
        }
        listed.sort();
        assert_eq!(
            listed,
            [
                ("a.txt".to_string(), false),
                ("dir".to_string(), true),
                ("empty".to_string(), true),
                ("to-dir".to_string(), false),
            ]
        );
        assert!(held.next_entry().is_none(), "the end stays the end");
        // A link to a directory is not opened as one, and goes as a link.
        let link = std::ffi::CString::new("to-dir").unwrap();
        let refused = held
            .open_child(&link)
            .err()
            .expect("a link, not a directory");
        assert!(anchored::is_no_directory(&refused), "{refused}");
        held.remove(&link, false).unwrap();
        assert!(root.join("dir/x.txt").exists() && !root.join("to-dir").exists());
        // Outside the root is not reachable from it.
        let elsewhere = tempfile::tempdir().unwrap();
        let outside = fs::canonicalize(elsewhere.path()).unwrap();
        let refused = anchored::Dir::open_canonical(&outside, Some(&root))
            .err()
            .expect("outside the root");
        assert_eq!(refused.kind(), std::io::ErrorKind::PermissionDenied);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_only_counts_files_and_directories() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let nested = tmp.path().join("nested");
        std::fs::create_dir(&nested).expect("nested dir");
        let file = nested.join("stale.txt");
        std::fs::write(&file, b"x").expect("fixture file");

        let stats = execute_deletion_plan(
            &[file],
            std::slice::from_ref(&nested),
            DeletionOptions {
                operation: "test delete",
                canonical_root: Some(&tmp.path().canonicalize().expect("canonical root")),
                abort: None,
                execute: true,
                directory_mode: DirectoryMode::EmptyOnly {
                    tolerate_nonempty: false,
                },
            },
        )
        .expect("delete plan");

        assert_eq!(stats, DeletionStats { files: 1, dirs: 1 });
        assert!(!nested.exists());
    }

    #[test]
    fn recursive_mode_removes_a_named_nonempty_tree() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let tree = tmp.path().join("tree");
        std::fs::create_dir_all(tree.join("nested")).expect("tree");
        std::fs::write(tree.join("nested/file.txt"), b"x").expect("fixture file");

        let stats = execute_deletion_plan(
            &[],
            std::slice::from_ref(&tree),
            DeletionOptions {
                operation: "test purge",
                canonical_root: Some(&tmp.path().canonicalize().expect("canonical root")),
                abort: None,
                execute: true,
                directory_mode: DirectoryMode::Recursive,
            },
        )
        .expect("recursive purge");

        assert_eq!(stats, DeletionStats { files: 0, dirs: 1 });
        assert!(!tree.exists());
    }

    #[cfg(windows)]
    #[test]
    fn recursive_mode_clears_a_readonly_tree_before_purge() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let tree = tmp.path().join("tree");
        let nested = tree.join("nested");
        std::fs::create_dir_all(&nested).expect("tree");
        let file = nested.join("readonly.txt");
        std::fs::write(&file, b"x").expect("fixture file");
        for path in [&file, &nested, &tree] {
            let mut permissions = std::fs::metadata(path).expect("metadata").permissions();
            permissions.set_readonly(true);
            std::fs::set_permissions(path, permissions).expect("set readonly");
        }

        let stats = execute_deletion_plan(
            &[],
            std::slice::from_ref(&tree),
            DeletionOptions {
                operation: "test purge",
                canonical_root: Some(&tmp.path().canonicalize().expect("canonical root")),
                abort: None,
                execute: true,
                directory_mode: DirectoryMode::Recursive,
            },
        )
        .expect("readonly recursive purge");

        assert_eq!(stats, DeletionStats { files: 0, dirs: 1 });
        assert!(!tree.exists());
    }

    #[test]
    fn abort_and_containment_are_checked_before_deletion() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let file = tmp.path().join("keep.txt");
        std::fs::write(&file, b"x").expect("fixture file");
        let abort = AtomicBool::new(true);
        let error = execute_deletion_plan(
            std::slice::from_ref(&file),
            &[],
            DeletionOptions {
                operation: "test delete",
                canonical_root: None,
                abort: Some(&abort),
                execute: true,
                directory_mode: DirectoryMode::EmptyOnly {
                    tolerate_nonempty: false,
                },
            },
        )
        .expect_err("abort must stop deletion");
        assert!(matches!(error, DeletionError::Cancelled { .. }));
        assert!(file.exists());

        abort.store(false, Ordering::Release);
        let elsewhere = tempfile::tempdir().expect("elsewhere");
        let error = execute_deletion_plan(
            std::slice::from_ref(&file),
            &[],
            DeletionOptions {
                operation: "test delete",
                canonical_root: Some(
                    &elsewhere
                        .path()
                        .canonicalize()
                        .expect("canonical elsewhere"),
                ),
                abort: Some(&abort),
                execute: true,
                directory_mode: DirectoryMode::EmptyOnly {
                    tolerate_nonempty: false,
                },
            },
        )
        .expect_err("containment must stop deletion");
        assert!(matches!(error, DeletionError::Containment { .. }));
        assert!(file.exists());
    }
}
