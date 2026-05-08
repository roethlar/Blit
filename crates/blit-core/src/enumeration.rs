use eyre::{bail, Context, Result};
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::fs_enum::{FileEntry, FileFilter};

/// Describes the kind of entry returned by the enumerator.
#[derive(Debug, Clone)]
pub enum EntryKind {
    File { size: u64 },
    Directory,
    Symlink { target: Option<PathBuf> },
}

/// One non-root WalkDir error suppressed during enumeration.
/// Surfaced via [`EnumerationOutcome`] so destructive follow-ups
/// (mirror-deletion, move) can refuse to act on an incomplete scan.
#[derive(Debug, Clone)]
pub struct SuppressedScanError {
    /// Path from the WalkDir error if it was attached, else
    /// `"<unknown>"`. Best-effort and not necessarily a clean
    /// relative path.
    pub path: String,
    /// IO error kind (PermissionDenied, NotFound, etc.) if the
    /// underlying WalkDir error wrapped one. WalkDir's loop and
    /// recursion errors do not carry an IO kind.
    pub kind: Option<std::io::ErrorKind>,
    /// Human-readable error message from WalkDir.
    pub message: String,
}

/// Outcome of a [`FileEnumerator::enumerate_local_streaming_capturing`]
/// run. `suppressed_errors` is empty on a fully clean walk; non-empty
/// means at least one subtree was skipped (most commonly because of
/// EACCES / ENOENT during traversal). Callers must NOT use the
/// resulting header set to drive destructive destination work
/// without first inspecting this field.
#[derive(Debug, Default, Clone)]
pub struct EnumerationOutcome {
    pub suppressed_errors: Vec<SuppressedScanError>,
}

impl EnumerationOutcome {
    /// True iff the walk completed without dropping any subtree.
    pub fn is_complete(&self) -> bool {
        self.suppressed_errors.is_empty()
    }
}

/// Result of filesystem enumeration. `absolute_path` is the full path on disk,
/// `relative_path` is the path relative to the enumeration root, and
/// `metadata` always refers to the filesystem object (captured via
/// `metadata()` for files/dirs and `symlink_metadata()` for symlinks).
#[derive(Debug, Clone)]
pub struct EnumeratedEntry {
    pub absolute_path: PathBuf,
    pub relative_path: PathBuf,
    pub metadata: Metadata,
    pub kind: EntryKind,
}

/// Unified filesystem enumerator used by both local and remote paths. The
/// enumerator honours the same filtering logic as the CLI and can be configured
/// to follow symlinks or include them as standalone entries.
#[derive(Debug, Clone)]
pub struct FileEnumerator {
    filter: FileFilter,
    follow_symlinks: bool,
    include_symlinks: bool,
}

impl FileEnumerator {
    pub fn new(filter: FileFilter) -> Self {
        Self {
            filter,
            follow_symlinks: false,
            include_symlinks: false,
        }
    }

    /// Configure whether symlinks should be followed during traversal.
    pub fn follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }

    /// Configure whether symlinks should be returned as entries. When combined
    /// with `follow_symlinks`, only one of these should typically be enabled.
    pub fn include_symlinks(mut self, include: bool) -> Self {
        self.include_symlinks = include;
        self
    }

    /// Enumerate local filesystem entries beneath `root`, applying the
    /// configured filters.
    pub fn enumerate_local(&self, root: &Path) -> Result<Vec<EnumeratedEntry>> {
        let mut results = Vec::new();
        self.enumerate_local_streaming(root, |entry| {
            results.push(entry);
            Ok(())
        })?;
        Ok(results)
    }

    /// Enumerate entries and invoke `visit` for each discovered item.
    ///
    /// **Suppressed errors**: non-root WalkDir errors (e.g.
    /// permission-denied on a child directory) are silently
    /// skipped to keep the scan resilient. Callers that need to
    /// know about these — most importantly anything driving
    /// mirror-deletion, where "not seen during scan" must NOT mean
    /// "delete from destination" — must use
    /// [`enumerate_local_streaming_capturing`] instead, which
    /// surfaces the suppressed paths so the caller can refuse
    /// destructive follow-up work.
    pub fn enumerate_local_streaming<F>(&self, root: &Path, visit: F) -> Result<()>
    where
        F: FnMut(EnumeratedEntry) -> Result<()>,
    {
        let outcome = self.enumerate_local_streaming_capturing(root, visit)?;
        // Drop the captured errors — any caller using this entry
        // point has already opted into "best-effort" semantics.
        let _ = outcome;
        Ok(())
    }

    /// Same as [`enumerate_local_streaming`] but returns an
    /// [`EnumerationOutcome`] enumerating any non-root errors that
    /// were suppressed during the walk. R46-F2 (data-loss): any
    /// caller that uses the resulting headers to drive
    /// destructive behavior on the *destination* (mirror-delete,
    /// move-then-delete-source) MUST inspect
    /// `outcome.suppressed_errors` and either refuse to delete or
    /// fail the operation — otherwise an unreadable source
    /// subtree would silently translate into the destination
    /// subtree being deleted.
    pub fn enumerate_local_streaming_capturing<F>(
        &self,
        root: &Path,
        mut visit: F,
    ) -> Result<EnumerationOutcome>
    where
        F: FnMut(EnumeratedEntry) -> Result<()>,
    {
        if !root.exists() {
            bail!("enumeration root does not exist: {}", root.display());
        }

        let filter = self.filter.clone_without_cache();
        let mut outcome = EnumerationOutcome::default();

        let mut walker = WalkDir::new(root)
            .follow_links(self.follow_symlinks)
            .into_iter();

        while let Some(next) = walker.next() {
            let entry = match next {
                Ok(e) => e,
                Err(err) => {
                    if err.depth() == 0 {
                        return Err(err.into());
                    }
                    // Non-root walkdir error: capture it so callers
                    // that drive destructive work can detect the
                    // incomplete scan. Pre-R46-F2 this `continue`
                    // silently dropped permission-denied
                    // subdirectories, and a follow-up
                    // mirror-deletion would treat the unscanned
                    // subtree as "absent at source" and delete the
                    // corresponding destination subtree.
                    let path_display = err
                        .path()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "<unknown>".to_string());
                    let io_kind = err.io_error().map(|e| e.kind());
                    outcome.suppressed_errors.push(SuppressedScanError {
                        path: path_display,
                        kind: io_kind,
                        message: err.to_string(),
                    });
                    continue;
                }
            };

            let path = entry.path();

            if entry.depth() == 0 {
                // If the root is itself a regular file (not a directory),
                // emit it as an entry — otherwise single-file sources get
                // silently dropped. Directory roots are still skipped to
                // avoid emitting the root as a child of itself.
                if entry.file_type().is_file() {
                    let metadata = entry
                        .metadata()
                        .with_context(|| format!("stat file root {}", path.display()))?;
                    let size = metadata.len();
                    let mtime = metadata.modified().ok();
                    let rel = PathBuf::new();
                    if filter.allows_entry(Some(&rel), path, size, mtime) {
                        visit(EnumeratedEntry {
                            absolute_path: path.to_path_buf(),
                            // Empty relative path so src_root.join(rel) === src_root
                            // and dest_root.join(rel) === dest_root.
                            relative_path: rel,
                            metadata,
                            kind: EntryKind::File { size },
                        })?;
                    }
                }
                continue;
            }

            if entry.file_type().is_dir() {
                if !filter.allows_dir(path) {
                    walker.skip_current_dir();
                    continue;
                }

                let metadata = entry
                    .metadata()
                    .with_context(|| format!("stat directory {}", path.display()))?;

                visit(EnumeratedEntry {
                    absolute_path: path.to_path_buf(),
                    relative_path: relative_path(root, path),
                    metadata,
                    kind: EntryKind::Directory,
                })?;
            } else if entry.file_type().is_file() {
                let metadata = entry
                    .metadata()
                    .with_context(|| format!("stat file {}", path.display()))?;
                let size = metadata.len();
                let mtime = metadata.modified().ok();
                let rel = relative_path(root, path);

                if !filter.allows_entry(Some(&rel), path, size, mtime) {
                    continue;
                }

                visit(EnumeratedEntry {
                    absolute_path: path.to_path_buf(),
                    relative_path: rel,
                    metadata,
                    kind: EntryKind::File { size },
                })?;
            } else if entry.file_type().is_symlink() && self.include_symlinks {
                if self.follow_symlinks {
                    continue;
                }

                let metadata = fs::symlink_metadata(path)
                    .with_context(|| format!("symlink metadata {}", path.display()))?;
                let mtime = metadata.modified().ok();
                let rel = relative_path(root, path);

                if !filter.allows_entry(Some(&rel), path, 0, mtime) {
                    continue;
                }

                let target = fs::read_link(path).ok();

                visit(EnumeratedEntry {
                    absolute_path: path.to_path_buf(),
                    relative_path: rel,
                    metadata,
                    kind: EntryKind::Symlink { target },
                })?;
            }
        }

        Ok(outcome)
    }
}

fn relative_path(root: &Path, path: &Path) -> PathBuf {
    match path.strip_prefix(root) {
        Ok(rel) if rel.as_os_str().is_empty() => PathBuf::from("."),
        Ok(rel) => rel.to_path_buf(),
        Err(_) => PathBuf::from(path),
    }
}

impl EnumeratedEntry {
    pub fn into_file_entry(self) -> Option<FileEntry> {
        match self.kind {
            EntryKind::File { size } => Some(FileEntry {
                path: self.absolute_path,
                size,
                is_directory: false,
            }),
            EntryKind::Directory => Some(FileEntry {
                path: self.absolute_path,
                size: 0,
                is_directory: true,
            }),
            EntryKind::Symlink { .. } => None,
        }
    }
}

#[cfg(test)]
impl FileEnumerator {
    pub fn enumerate_local_with_symlinks(&self, root: &Path) -> Result<Vec<EnumeratedEntry>> {
        self.clone().include_symlinks(true).enumerate_local(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn enumerate_file_root_emits_file() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("solo.txt");
        std::fs::write(&file, b"hello").unwrap();

        let enumerator = FileEnumerator::new(FileFilter::default());
        let entries = enumerator.enumerate_local(&file).unwrap();

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert!(matches!(entry.kind, EntryKind::File { size: 5 }));
        assert_eq!(entry.absolute_path, file);
        // Empty relative path so downstream join(src_root, rel) == src_root.
        assert_eq!(entry.relative_path, PathBuf::new());
    }

    #[test]
    fn enumerate_empty_dir_root_emits_nothing() {
        let tmp = tempdir().unwrap();
        let enumerator = FileEnumerator::new(FileFilter::default());
        let entries = enumerator.enumerate_local(tmp.path()).unwrap();
        assert!(entries.is_empty(), "empty dir should yield no entries");
    }

    #[test]
    fn enumerate_dir_root_does_not_emit_self() {
        // Pre-existing behavior: the directory root itself is never emitted;
        // only its contents are. Regression guard for the depth-0 skip.
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("a.txt"), b"a").unwrap();
        std::fs::write(tmp.path().join("b.txt"), b"b").unwrap();

        let enumerator = FileEnumerator::new(FileFilter::default());
        let entries = enumerator.enumerate_local(tmp.path()).unwrap();

        // Two files, no Directory entry for the root itself.
        assert_eq!(entries.len(), 2);
        assert!(entries
            .iter()
            .all(|e| matches!(e.kind, EntryKind::File { .. })));
    }

    #[test]
    fn enumerate_file_root_respects_filter() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("blocked.log");
        std::fs::write(&file, b"data").unwrap();

        let mut filter = FileFilter::default();
        filter.exclude_files.push("*.log".to_string());
        let enumerator = FileEnumerator::new(filter);
        let entries = enumerator.enumerate_local(&file).unwrap();

        assert!(
            entries.is_empty(),
            "excluded file at root should be skipped"
        );
    }
}
