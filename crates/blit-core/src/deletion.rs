//! Shared contained filesystem deletion for mirror and explicit purge paths.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::disk_usage::Stopped;

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

/// What became of one operand of a removal, held whole
/// ([`remove_as_rm`]).
#[derive(Debug, Default)]
pub struct Removal {
    /// Entries removed: the operand and everything below it.
    pub removed: u64,
    /// EVERY entry that could not be removed, in the order met. Empty
    /// when the operand is gone (or, with `force`, was never there).
    pub failures: Vec<RemovalFailure>,
}

/// What an entry that is no directory is, as a removal for a move is told
/// of it before it removes it (cr-uv3-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Met {
    pub kind: MetKind,
    /// The entry as it stands, whole (cr-uv4-1): what a move's account
    /// must have held for it to go.
    pub version: crate::moved::Version,
}

/// What kind of entry [`Met`] is: the entry itself, a link being a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetKind {
    File,
    Link,
    Other,
}

/// Asked of each entry that is no directory before it is removed, with
/// its path below the operand (empty for the operand itself): `Some` is
/// why it STAYS — it is left where it is and reported with that reason —
/// and `None` lets it go (cr-uv3-1). Directories are not asked about:
/// one goes when what was removed below it leaves it empty.
pub type Keep<'a> = &'a dyn Fn(&Path, &Met) -> Option<std::io::Error>;

/// How [`remove_as_rm`] treats one operand: `rm`'s own two options.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RmOptions {
    /// `-r`: a directory is removed with everything in it. Without it a
    /// directory is refused — "Is a directory" — and nothing is touched.
    pub recursive: bool,
    /// `-f`: an operand that is not there is no failure.
    pub force: bool,
}

/// Entries removed ONE AT A TIME, each by its name in the directory that
/// holds it (`find -delete`, slice 4): an empty directory, or anything
/// else — a link as the link it is. Nothing below a directory is ever
/// removed with it; one that still holds something stays, and says so.
///
/// On Unix the directory is reached from the root one name at a time,
/// never through a link, and kept while entries of it are removed — a
/// walk gives a directory's entries together. A directory swapped for a
/// link after the walk listed it is an error for what was in it, not a
/// way out of the root. Elsewhere an entry is removed by its path, which
/// must lie in the root.
pub struct Remover {
    canonical_root: PathBuf,
    #[cfg(unix)]
    held: Option<(PathBuf, anchored::Passage)>,
}

impl Remover {
    /// A remover for entries below `canonical_root` — a path with no link
    /// in it.
    pub fn below(canonical_root: &Path) -> Self {
        Self {
            canonical_root: canonical_root.to_path_buf(),
            #[cfg(unix)]
            held: None,
        }
    }

    /// Remove the entry at `target`: an empty directory when `directory`,
    /// anything else otherwise. `target`'s directory is a path with no
    /// link in it, at or below the root; the root itself is never removed.
    pub fn remove(&mut self, target: &Path, directory: bool) -> std::io::Result<()> {
        let (Some(parent), Some(name)) = (target.parent(), target.file_name()) else {
            return Err(leaves_the_root());
        };
        if !parent.starts_with(&self.canonical_root) {
            return Err(leaves_the_root());
        }
        #[cfg(unix)]
        {
            use anchored::ByName as _;
            use std::os::unix::ffi::OsStrExt as _;
            let name = std::ffi::CString::new(name.as_bytes())?;
            if self.held.as_ref().is_none_or(|(held, _)| held != parent) {
                self.held = None;
                let passage =
                    anchored::Passage::open_canonical(parent, Some(&self.canonical_root))?;
                self.held = Some((parent.to_path_buf(), passage));
            }
            let (_, passage) = self.held.as_ref().expect("held just above");
            passage.remove(&name, directory)
        }
        #[cfg(not(unix))]
        {
            let _ = name;
            crate::path_safety::verify_contained(&self.canonical_root, target)
                .map_err(|_| leaves_the_root())?;
            if directory {
                // By itself: a directory that is not empty stays, and
                // nothing in it is touched — its entries' read-only
                // attributes included (AC-12).
                std::fs::remove_dir(target)
            } else {
                remove_what_is_no_directory(target)
            }
        }
    }
}

/// Elsewhere than Unix: remove a file or a link by its path. A file's
/// read-only attribute is cleared first — its own, and nothing else's. A
/// link to a directory, which Windows removes the way it removes a
/// directory, goes as the link, and what it points to stays. A directory
/// standing where the walk met something else stays, untouched.
///
/// NOT RUN where this was written (Linux): it compiles for Windows, and
/// no more is known of it.
#[cfg(not(unix))]
#[allow(clippy::permissions_set_readonly_false)]
fn remove_what_is_no_directory(target: &Path) -> std::io::Result<()> {
    let metadata = std::fs::symlink_metadata(target)?;
    if metadata.file_type().is_symlink() {
        return std::fs::remove_file(target).or_else(|_| std::fs::remove_dir(target));
    }
    if metadata.is_dir() {
        return Err(std::io::Error::from(std::io::ErrorKind::IsADirectory));
    }
    let mut permissions = metadata.permissions();
    if permissions.readonly() {
        permissions.set_readonly(false);
        let _ = std::fs::set_permissions(target, permissions);
    }
    std::fs::remove_file(target)
}

/// What a removal says as it goes: the count of what went, and each entry
/// that did not, handed on at once (cr-uv1-5). Nothing is held back and
/// nothing left out — a tree with a million entries that cannot be
/// removed reports a million, as `rm` does, and holds none of them here.
struct Report<'a> {
    removed: u64,
    failed: &'a mut dyn FnMut(RemovalFailure) -> Result<(), Stopped>,
    /// Whoever was being told has gone: the removal stops.
    unheard: bool,
}

impl Report<'_> {
    /// One entry that could not be removed.
    fn fail(&mut self, relative_path: PathBuf, error: std::io::Error) {
        if !self.unheard
            && (self.failed)(RemovalFailure {
                relative_path,
                error,
            })
            .is_err()
        {
            self.unheard = true;
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
/// `as_directory` is how the operand was TYPED (cr-uv1-1): with a
/// separator after it. `rm` then removes it only if it is a directory —
/// `rm doc/` of a file is `Not a directory`, which `-f` passes over as it
/// does a missing operand, the file left where it is. A link typed so is
/// not removed and NOT GONE THROUGH: `Is a directory` without `-r` when it
/// points to one, as `rm` says, and `Not a directory` with it. That last
/// is where this is not `rm`, which empties the directory the link points
/// to before saying the same words; nothing here is ever removed through
/// a link.
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
/// it was opened with (`anchored::Dir`). The directories on the way to
/// the operand are opened to be searched and no more (`anchored::Passage`,
/// cr-uv2-4): `rm` asks nothing else of them. What cannot be opened so — a
/// directory that became a link, one that cannot be read — is one
/// reported failure, and nothing is removed by its path instead.
///
/// Elsewhere the standard library's own recursive removal does the tree:
/// it is anchored to handles in the same way, but it stops at the first
/// entry it cannot remove, so a tree with one such entry is left with
/// whatever had not gone yet and one failure, reported for the operand.
pub fn remove_as_rm(
    target: &Path,
    as_directory: bool,
    canonical_root: Option<&Path>,
    options: RmOptions,
    abort: Option<&AtomicBool>,
) -> Result<Removal, DeletionError> {
    let mut failures = Vec::new();
    let called_off = || abort.is_some_and(|abort| abort.load(Ordering::Acquire));
    let removed = remove_as_rm_reporting(
        target,
        as_directory,
        canonical_root,
        options,
        Some(&called_off),
        &mut |failure| {
            failures.push(failure);
            Ok(())
        },
    )?;
    Ok(Removal { removed, failures })
}

/// [`remove_as_rm`], telling `failed` of each entry that could not be
/// removed AS IT IS MET (cr-uv1-5) and returning the count of what went.
/// Every failure is told; none is counted in place of being named.
/// `failed` answering `Err(Stopped)` — whoever was listening has gone —
/// ends the removal where it is, as a cancelled one.
///
/// `called_off` is asked before the operand is touched and before each
/// entry below it (cr-uv2-3), and a `true` ends the removal there. A
/// removal that fails nowhere tells `failed` nothing, so a listener that
/// had gone was found out only at the end, with everything removed for
/// nobody.
pub fn remove_as_rm_reporting(
    target: &Path,
    as_directory: bool,
    canonical_root: Option<&Path>,
    options: RmOptions,
    called_off: Option<&dyn Fn() -> bool>,
    failed: &mut dyn FnMut(RemovalFailure) -> Result<(), Stopped>,
) -> Result<u64, DeletionError> {
    remove_what_may_go(
        target,
        as_directory,
        canonical_root,
        options,
        called_off,
        None,
        failed,
    )
}

/// [`remove_as_rm_reporting`], asking `keep` of each entry that is no
/// directory before it goes (cr-uv3-1). What `keep` holds back is told to
/// `failed` like any entry that could not be removed, and the directories
/// above it stay with it.
pub fn remove_what_may_go(
    target: &Path,
    as_directory: bool,
    canonical_root: Option<&Path>,
    options: RmOptions,
    called_off: Option<&dyn Fn() -> bool>,
    keep: Option<Keep<'_>>,
    failed: &mut dyn FnMut(RemovalFailure) -> Result<(), Stopped>,
) -> Result<u64, DeletionError> {
    let mut report = Report {
        removed: 0,
        failed,
        unheard: false,
    };
    remove_operand(
        target,
        as_directory,
        canonical_root,
        options,
        called_off,
        keep,
        &mut report,
    )?;
    if report.unheard {
        return Err(DeletionError::Cancelled {
            operation: RM_OPERATION,
        });
    }
    Ok(report.removed)
}

/// The system's words for a name typed as a directory that is none.
fn not_a_directory() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::NotADirectory, "Not a directory")
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
    as_directory: bool,
    canonical_root: Option<&Path>,
    options: RmOptions,
    called_off: Option<&dyn Fn() -> bool>,
    keep: Option<Keep<'_>>,
    removal: &mut Report<'_>,
) -> Result<(), DeletionError> {
    use anchored::ByName as _;
    use std::os::unix::ffi::OsStrExt as _;

    let called_off = || called_off.is_some_and(|called_off| called_off());
    if called_off() {
        return Err(DeletionError::Cancelled {
            operation: RM_OPERATION,
        });
    }
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
    let above = match anchored::Passage::open_canonical(&canonical, canonical_root) {
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
        if as_directory {
            // Typed as a directory, and no directory stands there. A link
            // to one is a directory to `rm` without `-r`; anything else
            // is none, and nothing is removed either way.
            let error = if !options.recursive && above.points_to_directory(&name) {
                is_a_directory()
            } else {
                not_a_directory()
            };
            if !absent(&error) {
                removal.fail(PathBuf::new(), error);
            }
            return Ok(());
        }
        if let Some(error) = stays(keep, &above, &name, Path::new("")) {
            removal.fail(PathBuf::new(), error);
            return Ok(());
        }
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
        // It cannot be read, so it cannot be emptied — and it may be
        // empty: `rm` finds out by removing it (cr-uv2-4).
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            match above.remove(&name, true) {
                Ok(()) => removal.removed += 1,
                Err(_) => removal.fail(PathBuf::new(), error),
            }
            return Ok(());
        }
        Err(error) => {
            removal.fail(PathBuf::new(), error);
            return Ok(());
        }
    }
    while let Some(frame) = frames.last_mut() {
        if removal.unheard || called_off() {
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
                let within: &dyn anchored::ByName = match frames.last() {
                    Some(frame) => &frame.dir,
                    None => &above,
                };
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
                    // Not to be read: removed if it is empty, as `rm`
                    // removes it, and reported if it is not (cr-uv2-4).
                    Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                        match frame.dir.remove(&name, true) {
                            Ok(()) => removal.removed += 1,
                            Err(_) => {
                                frame.kept = true;
                                removal.fail(relative_path, error);
                            }
                        }
                        continue;
                    }
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
        if let Some(error) = stays(keep, &frame.dir, &name, &relative_path) {
            frame.kept = true;
            removal.fail(relative_path, error);
            continue;
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

/// Why the entry called `name` in `within` stays, if `keep` holds it back
/// (cr-uv3-1): asked of the entry itself, by its name in the directory
/// that is held. An entry that cannot be looked at stays too.
#[cfg(unix)]
fn stays(
    keep: Option<Keep<'_>>,
    within: &dyn anchored::ByName,
    name: &std::ffi::CStr,
    relative_path: &Path,
) -> Option<std::io::Error> {
    let keep = keep?;
    match within.met(name) {
        Ok(met) => keep(relative_path, &met),
        Err(error) => Some(error),
    }
}

#[cfg(not(unix))]
fn remove_operand(
    target: &Path,
    as_directory: bool,
    canonical_root: Option<&Path>,
    options: RmOptions,
    called_off: Option<&dyn Fn() -> bool>,
    keep: Option<Keep<'_>>,
    removal: &mut Report<'_>,
) -> Result<(), DeletionError> {
    if called_off.is_some_and(|called_off| called_off()) {
        return Err(DeletionError::Cancelled {
            operation: RM_OPERATION,
        });
    }
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
        if as_directory {
            // Typed as a directory, and no directory stands there
            // (cr-uv1-1): nothing is removed.
            let through = std::fs::metadata(target).is_ok_and(|found| found.is_dir());
            if !options.recursive && through {
                removal.fail(PathBuf::new(), is_a_directory());
            } else if !options.force {
                removal.fail(PathBuf::new(), not_a_directory());
            }
            return Ok(());
        }
        if let Some(error) = keep.and_then(|keep| keep(Path::new(""), &met_by_path(&metadata))) {
            removal.fail(PathBuf::new(), error);
            return Ok(());
        }
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
    if let Some(keep) = keep {
        return remove_tree_asking(target, keep, called_off, removal);
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

/// [`Met`] from what a path's own metadata says (the entry itself).
#[cfg(not(unix))]
fn met_by_path(metadata: &std::fs::Metadata) -> Met {
    let kind = if metadata.file_type().is_file() {
        MetKind::File
    } else if metadata.file_type().is_symlink() {
        MetKind::Link
    } else {
        MetKind::Other
    };
    Met {
        kind,
        version: crate::moved::Version::of(metadata),
    }
}

/// A tree removed entry by entry, each one that is no directory asked
/// about first (cr-uv3-1), where there is no removal held to a directory's
/// handle. Entries are reached by their paths; what lets one go is that an
/// account holds this path at this size and time, and a path led
/// elsewhere finds no such file.
#[cfg(not(unix))]
fn remove_tree_asking(
    target: &Path,
    keep: Keep<'_>,
    called_off: Option<&dyn Fn() -> bool>,
    removal: &mut Report<'_>,
) -> Result<(), DeletionError> {
    // The directories that hold something that stayed: they stay too,
    // and are not reported again.
    let mut holding: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    for entry in walkdir::WalkDir::new(target)
        .follow_links(false)
        .contents_first(true)
    {
        if removal.unheard || called_off.is_some_and(|called_off| called_off()) {
            return Err(DeletionError::Cancelled {
                operation: RM_OPERATION,
            });
        }
        let stayed = |holding: &mut std::collections::HashSet<PathBuf>, path: &Path| {
            for above in path.ancestors().skip(1) {
                if !above.starts_with(target) || !holding.insert(above.to_path_buf()) {
                    break;
                }
            }
        };
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                let path = error.path().unwrap_or(target).to_path_buf();
                stayed(&mut holding, &path);
                let relative_path = path
                    .strip_prefix(target)
                    .unwrap_or(Path::new(""))
                    .to_path_buf();
                removal.fail(relative_path, std::io::Error::other(error));
                continue;
            }
        };
        let path = entry.path();
        let relative_path = path
            .strip_prefix(target)
            .unwrap_or(Path::new(""))
            .to_path_buf();
        if entry.file_type().is_dir() {
            if holding.contains(path) {
                continue;
            }
            match std::fs::remove_dir(path) {
                Ok(()) => removal.removed += 1,
                Err(error) => {
                    stayed(&mut holding, path);
                    removal.fail(relative_path, error);
                }
            }
            continue;
        }
        let stays = match std::fs::symlink_metadata(path) {
            Ok(metadata) => keep(&relative_path, &met_by_path(&metadata)),
            Err(error) => Some(error),
        };
        if let Some(error) = stays {
            stayed(&mut holding, path);
            removal.fail(relative_path, error);
            continue;
        }
        #[cfg(windows)]
        crate::win_fs::clear_readonly_recursive(path);
        match std::fs::remove_file(path) {
            Ok(()) => removal.removed += 1,
            Err(error) => {
                stayed(&mut holding, path);
                removal.fail(relative_path, error);
            }
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
    use std::os::unix::io::{AsRawFd as _, FromRawFd as _, OwnedFd, RawFd};
    use std::path::{Component, Path};
    use std::ptr::NonNull;

    #[cfg(not(target_os = "linux"))]
    use libc::{fstatat, readdir, stat};
    #[cfg(target_os = "linux")]
    use libc::{fstatat64 as fstatat, readdir64 as readdir, stat64 as stat};

    /// How a directory is opened to be SEARCHED and no more (cr-uv2-4):
    /// entries in it can then be opened, looked at and removed by name,
    /// and it need not be readable. That is all `rm` asks of the
    /// directories on the way to its operand; each of them was opened to
    /// be read, so a file in a directory of mode 0300 — which `rm`
    /// removes — was `Permission denied`. Where the system has no such
    /// way to open a directory, it is opened to be read, as before.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    const TO_SEARCH: libc::c_int = libc::O_PATH;
    #[cfg(any(target_vendor = "apple", target_os = "freebsd"))]
    const TO_SEARCH: libc::c_int = libc::O_SEARCH;
    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_vendor = "apple",
        target_os = "freebsd"
    )))]
    const TO_SEARCH: libc::c_int = libc::O_RDONLY;

    /// What is done IN an open directory, by a name there: the directory
    /// is found once, when it is opened, and never looked up again.
    pub(super) trait ByName {
        /// The open directory. It stays open while `self` lives.
        fn descriptor(&self) -> RawFd;

        /// The directory called `name` here, opened to be listed and
        /// emptied — itself, never what a link by that name points to.
        fn open_child(&self, name: &CStr) -> io::Result<Dir> {
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
            Dir::from_descriptor(fd)
        }

        /// Whether the entry called `name` in this directory is a
        /// directory — the entry itself, a link being a link.
        fn is_directory(&self, name: &CStr) -> io::Result<bool> {
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

        /// What the entry called `name` here is — the entry itself, a
        /// link being a link — with its size and its modification time
        /// (cr-uv3-1): what a move's account knows a file by.
        fn met(&self, name: &CStr) -> io::Result<super::Met> {
            let mut found = std::mem::MaybeUninit::<stat>::uninit();
            // SAFETY: as in `is_directory`.
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
            let found = unsafe { found.assume_init() };
            let kind = match found.st_mode & libc::S_IFMT {
                libc::S_IFREG => super::MetKind::File,
                libc::S_IFLNK => super::MetKind::Link,
                _ => super::MetKind::Other,
            };
            // The entry whole, as `moved::Version::of` reads one from
            // the standard library's record of the same call (cr-uv4-1).
            #[allow(clippy::useless_conversion, clippy::unnecessary_cast)]
            let version = crate::moved::Version {
                size: u64::try_from(found.st_size).unwrap_or(0),
                mtime_nanos: i128::from(found.st_mtime) * 1_000_000_000
                    + i128::from(found.st_mtime_nsec),
                ctime_nanos: i128::from(found.st_ctime) * 1_000_000_000
                    + i128::from(found.st_ctime_nsec),
                device: found.st_dev as u64,
                inode: found.st_ino as u64,
            };
            Ok(super::Met { kind, version })
        }

        /// Whether the entry called `name` here is a directory once a link
        /// is resolved. Asked only to choose the words of a refusal.
        fn points_to_directory(&self, name: &CStr) -> bool {
            let mut found = std::mem::MaybeUninit::<stat>::uninit();
            // SAFETY: as in `is_directory`.
            let answered =
                unsafe { fstatat(self.descriptor(), name.as_ptr(), found.as_mut_ptr(), 0) };
            // SAFETY: read only when the call succeeded and filled it in.
            answered == 0 && unsafe { found.assume_init() }.st_mode & libc::S_IFMT == libc::S_IFDIR
        }

        /// Remove the entry called `name` in this directory: an empty
        /// directory when `directory`, anything else otherwise — a link
        /// as the link it is.
        fn remove(&self, name: &CStr, directory: bool) -> io::Result<()> {
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

    /// A directory on the way to an operand: searched, and never listed
    /// (cr-uv2-4). It needs no permission to be read.
    pub(super) struct Passage {
        fd: OwnedFd,
    }

    impl Passage {
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
                dir = dir.through(&CString::new(name.as_bytes())?)?;
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
                    TO_SEARCH | libc::O_DIRECTORY | libc::O_CLOEXEC,
                )
            };
            Self::from_descriptor(fd)
        }

        /// The directory called `name` IN this one — itself, never what a
        /// link by that name points to — to be passed through in turn.
        fn through(&self, name: &CStr) -> io::Result<Self> {
            // SAFETY: the descriptor is this open directory's for the
            // length of the call and `name` is a NUL-terminated string
            // that outlives it; `openat` reads both and keeps neither.
            let fd = unsafe {
                libc::openat(
                    self.descriptor(),
                    name.as_ptr(),
                    TO_SEARCH | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            Self::from_descriptor(fd)
        }

        fn from_descriptor(fd: RawFd) -> io::Result<Self> {
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: `fd` is an open descriptor that nothing else owns.
            Ok(Self {
                fd: unsafe { OwnedFd::from_raw_fd(fd) },
            })
        }
    }

    impl ByName for Passage {
        fn descriptor(&self) -> RawFd {
            self.fd.as_raw_fd()
        }
    }

    /// One directory being listed and emptied. Its entries are listed
    /// from it, opened in it and removed from it, each by its name there.
    pub(super) struct Dir {
        /// The listing stream; it owns the descriptor.
        stream: NonNull<libc::DIR>,
        /// The listing has ended, or failed and will not be read again.
        ended: bool,
    }

    impl ByName for Dir {
        fn descriptor(&self) -> RawFd {
            // SAFETY: the stream is open until this value is dropped.
            unsafe { libc::dirfd(self.stream.as_ptr()) }
        }
    }

    impl Dir {
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
        typed(root, rel, false, recursive, force)
    }

    /// The same, for an operand typed with a separator after it or not.
    fn typed(root: &Path, rel: &str, as_directory: bool, recursive: bool, force: bool) -> Removal {
        remove_as_rm(
            &root.join(rel),
            as_directory,
            Some(root),
            RmOptions { recursive, force },
            None,
        )
        .expect("an answer about the operand")
    }

    /// cr-uv1-1: an operand typed as a directory — `doc/` — is removed
    /// only if it is one. A file typed so is `Not a directory` and stays
    /// (`blit rm HOST:/m/doc/` unlinked it); `-f` passes over that as it
    /// does a missing operand, and the file still stays. A directory typed
    /// so is treated as any directory. Each row is what `rm` 9.12 does.
    #[test]
    fn an_operand_typed_as_a_directory_is_removed_only_if_it_is_one() {
        use std::io::ErrorKind::{IsADirectory, NotADirectory, NotFound};
        for (rel, recursive, force, kind) in [
            ("a.txt", false, false, Some(NotADirectory)),
            ("a.txt", true, false, Some(NotADirectory)),
            ("a.txt", false, true, None),
            ("a.txt", true, true, None),
            ("empty", false, false, Some(IsADirectory)),
            ("nothing-here", true, false, Some(NotFound)),
            ("nothing-here", true, true, None),
        ] {
            let (_tmp, root) = tree();
            let removal = typed(&root, rel, true, recursive, force);
            let found: Vec<_> = removal.failures.iter().map(|f| f.error.kind()).collect();
            let expected: Vec<_> = kind.into_iter().collect();
            assert_eq!(
                found, expected,
                "{rel}/ recursive {recursive} force {force}"
            );
            assert_eq!(removal.removed, 0, "{rel}/");
            assert!(root.join("a.txt").exists() && root.join("empty").exists());
        }
        let (_tmp, root) = tree();
        let words = typed(&root, "a.txt", true, false, false);
        assert_eq!(
            crate::disk_usage::os_reason(&words.failures[0].error),
            "Not a directory"
        );
        // A directory typed as one goes, with `-r`, like any other.
        let gone = typed(&root, "dir", true, true, false);
        assert!(gone.failures.is_empty() && !root.join("dir").exists());
        assert_eq!(gone.removed, 6);
    }

    /// cr-uv1-1: a link typed as a directory — `link/` — is not removed,
    /// and what it points to is not touched: `Is a directory` without
    /// `-r`, as `rm` says of a link to one; `Not a directory` with it, as
    /// `rm` says too — after emptying the directory the link points to,
    /// which this never does. A link to a file, or to nothing, is
    /// `Not a directory` either way.
    #[cfg(unix)]
    #[test]
    fn a_link_typed_as_a_directory_is_never_gone_through() {
        use std::io::ErrorKind::{IsADirectory, NotADirectory};
        for (link, recursive, force, kind) in [
            ("to-dir", false, false, Some(IsADirectory)),
            ("to-dir", false, true, Some(IsADirectory)),
            ("to-dir", true, false, Some(NotADirectory)),
            ("to-dir", true, true, None),
            ("to-file", false, false, Some(NotADirectory)),
            ("to-file", true, false, Some(NotADirectory)),
            ("to-nothing", true, false, Some(NotADirectory)),
            ("to-nothing", true, true, None),
        ] {
            let (_tmp, root) = tree();
            std::os::unix::fs::symlink("dir", root.join("to-dir")).unwrap();
            std::os::unix::fs::symlink("a.txt", root.join("to-file")).unwrap();
            std::os::unix::fs::symlink("nowhere", root.join("to-nothing")).unwrap();
            let removal = typed(&root, link, true, recursive, force);
            let found: Vec<_> = removal.failures.iter().map(|f| f.error.kind()).collect();
            let expected: Vec<_> = kind.into_iter().collect();
            assert_eq!(
                found, expected,
                "{link}/ recursive {recursive} force {force}"
            );
            assert_eq!(removal.removed, 0, "{link}/");
            assert!(
                root.join(link).symlink_metadata().is_ok(),
                "{link}: the link stands"
            );
            assert!(
                root.join("dir/x.txt").exists()
                    && root.join("dir/sub/deep/z.txt").exists()
                    && root.join("a.txt").exists(),
                "{link}/: nothing it points to is touched"
            );
        }
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
                    false,
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
            false,
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

    /// cr-uv1-5: every entry that cannot be removed is told, however
    /// many — 1,001 here, where the list stopped at 1,000 and a flag said
    /// "more" — each as it is met, and the count of what went is exact.
    /// And a listener that has gone ends the removal where it is.
    #[cfg(unix)]
    #[test]
    fn every_entry_that_cannot_be_removed_is_told() {
        use std::os::unix::fs::PermissionsExt as _;
        let (_tmp, root) = tree();
        fs::create_dir_all(root.join("dir/many")).unwrap();
        for at in 0..1001 {
            fs::write(root.join(format!("dir/many/f{at:04}")), b"").unwrap();
        }
        fs::set_permissions(root.join("dir/many"), fs::Permissions::from_mode(0o555)).unwrap();
        let privileged = fs::write(root.join("dir/many/probe"), b"x").is_ok();
        let _ = fs::remove_file(root.join("dir/many/probe"));
        let unlock = || {
            fs::set_permissions(root.join("dir/many"), fs::Permissions::from_mode(0o755)).unwrap()
        };
        if privileged {
            unlock();
            eprintln!("skipping: running privileged, mode 555 does not stop a removal");
            return;
        }
        let options = RmOptions {
            recursive: true,
            force: false,
        };

        // Told as it goes: the first failure is heard before the last
        // entry is met, and none is held back.
        let mut told = Vec::new();
        let removed = remove_as_rm_reporting(
            &root.join("dir"),
            false,
            Some(&root),
            options,
            None,
            &mut |failure| {
                told.push(failure.relative_path);
                Ok(())
            },
        )
        .expect("an answer");
        assert_eq!(told.len(), 1001, "each of them, and no flag in their place");
        told.sort();
        told.dedup();
        assert_eq!(told.len(), 1001, "each once");
        assert!(told.iter().all(|path| path.starts_with("many")));
        // Three files and two directories went; `dir` and `dir/many` stand.
        assert_eq!(removed, 5);

        // The listener goes after the third: the removal ends there.
        let mut heard = 0;
        let error = remove_as_rm_reporting(
            &root.join("dir"),
            false,
            Some(&root),
            options,
            None,
            &mut |_| {
                heard += 1;
                if heard == 3 {
                    Err(Stopped)
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert!(matches!(error, DeletionError::Cancelled { .. }), "{error}");
        assert_eq!(heard, 3, "nothing is told to nobody");

        // And held whole, for a caller that wants it so.
        let whole = rm(&root, "dir", true, false);
        unlock();
        assert_eq!(whole.failures.len(), 1001);
    }

    /// cr-uv2-3: a removal that is called off stops — before its operand
    /// is touched, and between one entry and the next. It was asked only
    /// when an entry FAILED, so a removal that went well went on to the
    /// end for a listener that had gone.
    #[test]
    fn a_removal_called_off_stops_where_it_is() {
        let options = RmOptions {
            recursive: true,
            force: false,
        };
        let never_told = &mut |failure: RemovalFailure| -> Result<(), Stopped> {
            panic!("nothing fails here: {failure:?}")
        };

        // Before anything: a file, and a tree, are left whole.
        let (_tmp, root) = tree();
        for (operand, stays) in [("a.txt", "a.txt"), ("dir", "dir/x.txt")] {
            let error = remove_as_rm_reporting(
                &root.join(operand),
                false,
                Some(&root),
                options,
                Some(&|| true),
                never_told,
            )
            .unwrap_err();
            assert!(matches!(error, DeletionError::Cancelled { .. }), "{error}");
            assert!(root.join(stays).exists(), "{operand} was not touched");
        }

        // Part way: asked once for the operand and once for each entry,
        // and called off at the twentieth question.
        fs::create_dir_all(root.join("many")).unwrap();
        for at in 0..100 {
            fs::write(root.join(format!("many/f{at:03}")), b"").unwrap();
        }
        let asked = std::cell::Cell::new(0);
        let error = remove_as_rm_reporting(
            &root.join("many"),
            false,
            Some(&root),
            options,
            Some(&|| {
                asked.set(asked.get() + 1);
                asked.get() >= 20
            }),
            never_told,
        )
        .unwrap_err();
        assert!(matches!(error, DeletionError::Cancelled { .. }), "{error}");
        let left = fs::read_dir(root.join("many")).unwrap().count();
        // Where every entry is asked about, eighteen went. Elsewhere the
        // tree is the standard library's to remove, whole or not at all.
        if cfg!(unix) {
            assert_eq!(left, 82, "it stopped between two entries");
        } else {
            assert_eq!(left, 100);
        }

        // And one that is never called off is removed.
        let removed = remove_as_rm_reporting(
            &root.join("many"),
            false,
            Some(&root),
            options,
            Some(&|| false),
            never_told,
        )
        .expect("removed");
        assert_eq!(removed, left as u64 + 1);
        assert!(!root.join("many").exists());
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
        use anchored::ByName as _;
        let here = std::ffi::CString::new(".").unwrap();
        let mut held = anchored::Passage::open_canonical(&root, Some(&root))
            .unwrap()
            .open_child(&here)
            .unwrap();
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
        let refused = anchored::Passage::open_canonical(&outside, Some(&root))
            .err()
            .expect("outside the root");
        assert_eq!(refused.kind(), std::io::ErrorKind::PermissionDenied);
    }

    /// cr-uv2-4: `rm` asks of a directory only what it needs of it. On the
    /// way to an operand that is to be SEARCHED: every such directory was
    /// opened to be read, and a file in a directory of mode 0300 — which
    /// `rm` removes — was `Permission denied`. And a directory that cannot
    /// be read is still removed when it is empty, as `rm` removes it.
    #[cfg(unix)]
    #[test]
    fn a_directory_is_asked_only_for_what_rm_needs_of_it() {
        use std::os::unix::fs::PermissionsExt as _;
        let (_tmp, root) = tree();
        let mode = |rel: &str, mode: u32| {
            fs::set_permissions(root.join(rel), fs::Permissions::from_mode(mode)).unwrap()
        };
        for dir in [
            "w/v/sub",
            "full/sub",
            "holds/sealed-empty",
            "blind",
            "searched",
        ] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        for file in [
            "w/known",
            "w/v/known",
            "w/v/sub/file",
            "full/sub/inner",
            "holds/a",
            "searched/known",
        ] {
            fs::write(root.join(file), b"x").unwrap();
        }
        mode("w/v", 0o300);
        mode("w", 0o300);
        if fs::read_dir(root.join("w")).is_ok() {
            mode("w", 0o755);
            mode("w/v", 0o755);
            eprintln!("skipping: running privileged, no mode stops a listing");
            return;
        }

        // Write and search, no read, one level and two above the operand.
        assert!(reasons(&rm(&root, "w/known", false, false)).is_empty());
        assert!(reasons(&rm(&root, "w/v/known", false, false)).is_empty());
        let tree_below = rm(&root, "w/v/sub", true, false);
        assert!(reasons(&tree_below).is_empty());
        assert_eq!(tree_below.removed, 2);
        mode("w", 0o755);
        mode("w/v", 0o755);
        assert_eq!(fs::read_dir(root.join("w/v")).unwrap().count(), 0);
        assert_eq!(fs::read_dir(root.join("w")).unwrap().count(), 1);

        // Search without write removes nothing, as ever.
        mode("searched", 0o100);
        let kept = rm(&root, "searched/known", false, false);
        mode("searched", 0o755);
        assert_eq!(
            reasons(&kept),
            [(String::new(), std::io::ErrorKind::PermissionDenied)]
        );
        assert!(root.join("searched/known").exists());

        // Not to be read, and empty: removed — as an operand, and below one.
        mode("blind", 0o000);
        let blind = rm(&root, "blind", true, false);
        assert!(reasons(&blind).is_empty() && blind.removed == 1);
        assert!(!root.join("blind").exists());
        mode("holds/sealed-empty", 0o000);
        let holds = rm(&root, "holds", true, false);
        assert!(reasons(&holds).is_empty(), "{:?}", reasons(&holds));
        assert_eq!(holds.removed, 3);
        assert!(!root.join("holds").exists());

        // Not to be read, and not empty: reported in those words, and left.
        mode("full/sub", 0o300);
        let full = rm(&root, "full", true, false);
        mode("full/sub", 0o755);
        assert_eq!(
            reasons(&full),
            [("sub".to_string(), std::io::ErrorKind::PermissionDenied)]
        );
        assert!(root.join("full/sub/inner").exists());
        assert_eq!(full.removed, 0);
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

#[cfg(all(test, unix))]
mod remover_tests {
    use super::*;
    use std::fs;

    /// Slice 4: `find -delete` removes an entry by its name in the
    /// directory that holds it — a file, a link as the link, a directory
    /// only when it is empty — and nothing through a link, or outside the
    /// root, or the root.
    #[test]
    fn an_entry_is_removed_by_its_name_where_it_is() {
        let tmp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(tmp.path()).unwrap().join("root");
        fs::create_dir_all(root.join("d/full")).unwrap();
        fs::create_dir_all(root.join("d/empty")).unwrap();
        fs::write(root.join("d/full/f"), b"x").unwrap();
        fs::write(root.join("d/a"), b"x").unwrap();
        fs::write(root.join("d/b"), b"x").unwrap();
        fs::create_dir_all(root.join("elsewhere")).unwrap();
        fs::write(root.join("elsewhere/kept"), b"x").unwrap();
        std::os::unix::fs::symlink("elsewhere", root.join("d/link")).unwrap();
        let outside = root.parent().unwrap().join("outside");
        fs::write(&outside, b"x").unwrap();

        let mut remover = Remover::below(&root);
        // Two files of one directory, the directory reached once.
        remover.remove(&root.join("d/a"), false).unwrap();
        remover.remove(&root.join("d/b"), false).unwrap();
        assert!(!root.join("d/a").exists() && !root.join("d/b").exists());
        // An entry of ANOTHER directory goes from that one — and one of
        // its name in the directory reached before stays.
        fs::write(root.join("d/same"), b"x").unwrap();
        fs::write(root.join("elsewhere/same"), b"x").unwrap();
        remover.remove(&root.join("elsewhere/same"), false).unwrap();
        assert!(root.join("d/same").exists() && !root.join("elsewhere/same").exists());
        remover.remove(&root.join("d/same"), false).unwrap();
        // A link goes as the link; what it points to stays.
        remover.remove(&root.join("d/link"), false).unwrap();
        assert!(fs::symlink_metadata(root.join("d/link")).is_err());
        assert!(root.join("elsewhere/kept").exists());
        // A directory goes only when it is empty, and says why not.
        let full = remover.remove(&root.join("d/full"), true).unwrap_err();
        assert_eq!(full.kind(), std::io::ErrorKind::DirectoryNotEmpty);
        assert!(root.join("d/full/f").exists());
        remover.remove(&root.join("d/empty"), true).unwrap();
        assert!(!root.join("d/empty").exists());
        // A directory asked for as a file, and a file as a directory.
        assert!(remover.remove(&root.join("d/full"), false).is_err());
        assert!(remover.remove(&root.join("d/full/f"), true).is_err());
        assert!(root.join("d/full/f").exists());

        // Nothing outside the root, and not the root.
        assert!(remover.remove(&outside, false).is_err());
        assert!(remover.remove(&root, true).is_err());
        assert!(outside.exists() && root.exists());

        // A directory swapped for a link after it was listed: what was
        // in it is not reached through the link.
        fs::remove_file(root.join("d/full/f")).unwrap();
        fs::remove_dir(root.join("d/full")).unwrap();
        std::os::unix::fs::symlink("../elsewhere", root.join("d/full")).unwrap();
        let mut remover = Remover::below(&root);
        assert!(remover.remove(&root.join("d/full/kept"), false).is_err());
        assert!(root.join("elsewhere/kept").exists());
    }
}
