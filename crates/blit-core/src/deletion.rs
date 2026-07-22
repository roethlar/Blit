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
            Ok(()) => stats.files += 1,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error)
                if error.kind() == std::io::ErrorKind::IsADirectory
                    && options.directory_mode == DirectoryMode::Recursive =>
            {
                #[cfg(windows)]
                crate::win_fs::clear_readonly_recursive(file);
                match std::fs::remove_dir_all(file) {
                    Ok(()) => stats.dirs += 1,
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
            Ok(()) => stats.dirs += 1,
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
