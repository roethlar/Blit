use anyhow::{Context, Result};
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
    pub fn enumerate_local_streaming<F>(&self, root: &Path, mut visit: F) -> Result<()>
    where
        F: FnMut(EnumeratedEntry) -> Result<()>,
    {
        if !root.exists() {
            anyhow::bail!("enumeration root does not exist: {}", root.display());
        }

        let mut filter = self.filter.clone_without_cache();

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
                    continue;
                }
            };

            let path = entry.path();

            if entry.depth() == 0 {
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

                if !filter.allows_file(path, metadata.len()) {
                    continue;
                }

                let size = metadata.len();

                visit(EnumeratedEntry {
                    absolute_path: path.to_path_buf(),
                    relative_path: relative_path(root, path),
                    metadata,
                    kind: EntryKind::File { size },
                })?;
            } else if entry.file_type().is_symlink() && self.include_symlinks {
                if self.follow_symlinks {
                    continue;
                }

                let metadata = fs::symlink_metadata(path)
                    .with_context(|| format!("symlink metadata {}", path.display()))?;

                if !filter.allows_file(path, 0) {
                    continue;
                }

                let target = fs::read_link(path).ok();

                visit(EnumeratedEntry {
                    absolute_path: path.to_path_buf(),
                    relative_path: relative_path(root, path),
                    metadata,
                    kind: EntryKind::Symlink { target },
                })?;
            }
        }

        Ok(())
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
