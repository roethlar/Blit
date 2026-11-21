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
/// `target` is checked against `canonical_root` before anything is read.
/// `Err` is for what is not about one entry: a target outside the root, a
/// cancelled operation.
pub fn remove_as_rm(
    target: &Path,
    canonical_root: Option<&Path>,
    options: RmOptions,
    abort: Option<&AtomicBool>,
) -> Result<Removal, DeletionError> {
    const OPERATION: &str = "rm";
    if let Some(root) = canonical_root {
        crate::path_safety::verify_contained(root, target).map_err(|source| {
            DeletionError::Containment {
                operation: OPERATION,
                target: target.to_path_buf(),
                source,
            }
        })?;
    }
    let mut removal = Removal::default();
    let fail = |removal: &mut Removal, relative_path: PathBuf, error: std::io::Error| {
        if removal.failures.len() < MAX_LISTED_REMOVAL_FAILURES {
            removal.failures.push(RemovalFailure {
                relative_path,
                error,
            });
        } else {
            removal.failures_truncated = true;
        }
    };
    let metadata = match std::fs::symlink_metadata(target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && options.force => {
            return Ok(removal);
        }
        Err(error) => {
            fail(&mut removal, PathBuf::new(), error);
            return Ok(removal);
        }
    };
    if !metadata.is_dir() {
        #[cfg(windows)]
        crate::win_fs::clear_readonly_recursive(target);
        match std::fs::remove_file(target) {
            Ok(()) => removal.removed += 1,
            Err(error) => fail(&mut removal, PathBuf::new(), error),
        }
        return Ok(removal);
    }
    if !options.recursive {
        // `rm`'s words for it, on every platform.
        fail(
            &mut removal,
            PathBuf::new(),
            std::io::Error::new(std::io::ErrorKind::IsADirectory, "Is a directory"),
        );
        return Ok(removal);
    }
    // A recursive removal clears read-only attributes through the tree
    // first, as the purge it replaces did; the empty-only removals of
    // other callers never do.
    #[cfg(windows)]
    crate::win_fs::clear_readonly_recursive(target);

    /// One directory being emptied.
    struct Frame {
        relative_path: PathBuf,
        /// Its name in the directory above it; empty for the operand.
        name: std::ffi::OsString,
        entries: std::fs::ReadDir,
        /// The directory held open, to remove what is in it by name.
        held: Option<HeldDirectory>,
        /// Something below could not be removed, so this cannot be either.
        kept: bool,
    }
    let mut frames: Vec<Frame> = Vec::new();
    match std::fs::read_dir(target) {
        Ok(entries) => frames.push(Frame {
            relative_path: PathBuf::new(),
            name: std::ffi::OsString::new(),
            entries,
            held: HeldDirectory::open(target),
            kept: false,
        }),
        Err(error) => {
            fail(&mut removal, PathBuf::new(), error);
            return Ok(removal);
        }
    }
    while let Some(frame) = frames.last_mut() {
        if abort.is_some_and(|abort| abort.load(Ordering::Acquire)) {
            return Err(DeletionError::Cancelled {
                operation: OPERATION,
            });
        }
        let Some(entry) = frame.entries.next() else {
            // Emptied, or as empty as it will get.
            let done = frames.pop().expect("the frame just read");
            let mut kept = done.kept;
            if !kept {
                // By its name in the directory above, when that is held.
                let gone = match frames.last().and_then(|above| above.held.as_ref()) {
                    Some(above) => above.remove(&done.name, true),
                    None => std::fs::remove_dir(target.join(&done.relative_path)),
                };
                match gone {
                    Ok(()) => removal.removed += 1,
                    Err(error) => {
                        fail(&mut removal, done.relative_path, error);
                        kept = true;
                    }
                }
            }
            if let Some(above) = frames.last_mut() {
                above.kept |= kept;
            }
            continue;
        };
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                let relative_path = frame.relative_path.clone();
                frame.kept = true;
                fail(&mut removal, relative_path, error);
                continue;
            }
        };
        let name = entry.file_name();
        let relative_path = frame.relative_path.join(&name);
        // `DirEntry::file_type` never follows a link.
        let is_dir = match entry.file_type() {
            Ok(kind) => kind.is_dir(),
            Err(error) => {
                frame.kept = true;
                fail(&mut removal, relative_path, error);
                continue;
            }
        };
        if is_dir {
            let path = entry.path();
            match std::fs::read_dir(&path) {
                Ok(entries) => frames.push(Frame {
                    relative_path,
                    name,
                    entries,
                    held: HeldDirectory::open(&path),
                    kept: false,
                }),
                Err(error) => {
                    frame.kept = true;
                    fail(&mut removal, relative_path, error);
                }
            }
            continue;
        }
        let gone = match &frame.held {
            Some(held) => held.remove(&name, false),
            None => std::fs::remove_file(entry.path()),
        };
        match gone {
            Ok(()) => removal.removed += 1,
            Err(error) => {
                frame.kept = true;
                fail(&mut removal, relative_path, error);
            }
        }
    }
    Ok(removal)
}

/// A directory held open while it is emptied, so that each entry in it is
/// removed BY NAME from the directory itself — as `rm` removes it, and as
/// the standard library's own recursive removal does. Removing by the
/// entry's whole path makes the system walk that path again for every
/// file: over 200,000 files in a module a dozen folders deep that was a
/// fifth of the whole removal (296 ms against 245 ms, measured).
///
/// Unix only. Elsewhere, and where the directory cannot be opened this
/// way, entries are removed by path — the same removal, slower.
struct HeldDirectory {
    #[cfg(unix)]
    directory: std::fs::File,
}

impl HeldDirectory {
    #[cfg(unix)]
    fn open(path: &Path) -> Option<Self> {
        use std::os::unix::fs::OpenOptionsExt as _;
        // The directory itself, never what a link in its place points to.
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)
            .ok()
            .map(|directory| Self { directory })
    }

    #[cfg(not(unix))]
    fn open(_: &Path) -> Option<Self> {
        None
    }

    /// Remove the entry called `name` in this directory: an empty
    /// directory when `directory`, anything else otherwise — a link as
    /// the link it is.
    #[cfg(unix)]
    fn remove(&self, name: &std::ffi::OsStr, directory: bool) -> std::io::Result<()> {
        use std::os::unix::ffi::OsStrExt as _;
        use std::os::unix::io::AsRawFd as _;
        let name = std::ffi::CString::new(name.as_bytes())
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
        let flags = if directory { libc::AT_REMOVEDIR } else { 0 };
        // SAFETY: the descriptor is this open directory's for the length
        // of the call, and `name` is a NUL-terminated string that outlives
        // it; `unlinkat` reads both and keeps neither.
        if unsafe { libc::unlinkat(self.directory.as_raw_fd(), name.as_ptr(), flags) } == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    #[cfg(not(unix))]
    fn remove(&self, _: &std::ffi::OsStr, _: bool) -> std::io::Result<()> {
        Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
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

    /// A target outside the root is refused before anything is read, and a
    /// cancelled removal stops.
    #[test]
    fn a_target_outside_the_root_is_refused_whole() {
        let (_tmp, root) = tree();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("victim.txt"), b"x").unwrap();
        let error = remove_as_rm(
            &outside.path().join("victim.txt"),
            Some(&root),
            RmOptions {
                recursive: true,
                force: true,
            },
            None,
        )
        .unwrap_err();
        assert!(
            matches!(error, DeletionError::Containment { .. }),
            "{error}"
        );
        assert!(outside.path().join("victim.txt").exists());

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
