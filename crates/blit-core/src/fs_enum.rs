use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::enumeration::{EntryKind, EnumeratedEntry, FileEnumerator};
// Filesystem enumeration and categorization (Unix focus)

/// Entry with size information for categorization
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub is_directory: bool,
}

#[derive(Debug, Clone)]
pub struct SymlinkEntry {
    pub path: PathBuf,
    pub target: PathBuf,
    pub target_is_dir: bool,
}

/// Copy job with optional resume offset
#[derive(Debug, Clone)]
pub struct CopyJob {
    pub entry: FileEntry,
}

/// File filter options (robocopy-style compatibility)
#[derive(Default, Clone, Debug)]
pub struct FileFilter {
    pub exclude_files: Vec<String>,
    pub exclude_dirs: Vec<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    #[allow(dead_code)]
    compiled_files: Option<globset::GlobSet>,
    #[allow(dead_code)]
    compiled_dirs: Option<globset::GlobSet>,
}

impl FileFilter {
    /// Create a shallow clone that preserves user-specified patterns while
    /// dropping any compiled glob caches. This lets helpers reuse filters
    /// without sharing mutable compilation state.
    pub fn clone_without_cache(&self) -> Self {
        Self {
            exclude_files: self.exclude_files.clone(),
            exclude_dirs: self.exclude_dirs.clone(),
            min_size: self.min_size,
            max_size: self.max_size,
            compiled_files: None,
            compiled_dirs: None,
        }
    }

    fn ensure_compiled(&mut self) {
        if self.compiled_files.is_none() {
            let mut b = globset::GlobSetBuilder::new();
            for pat in &self.exclude_files {
                if let Ok(g) = globset::Glob::new(pat) {
                    b.add(g);
                }
            }
            self.compiled_files = b.build().ok();
        }
        if self.compiled_dirs.is_none() {
            let mut b = globset::GlobSetBuilder::new();
            for pat in &self.exclude_dirs {
                if let Ok(g) = globset::Glob::new(pat) {
                    b.add(g);
                }
            }
            self.compiled_dirs = b.build().ok();
        }
    }
    pub(crate) fn allows_file(&mut self, path: &Path, size: u64) -> bool {
        self.ensure_compiled();
        self.should_include_file(path, size)
    }

    pub(crate) fn allows_dir(&mut self, path: &Path) -> bool {
        self.ensure_compiled();
        self.should_include_dir(path)
    }
    /// Check if a file should be included
    fn should_include_file(&self, path: &Path, size: u64) -> bool {
        // Check file patterns using compiled globset if available; fallback to simple glob_match
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if let Some(gs) = &self.compiled_files {
            if gs.is_match(&filename) {
                return false;
            }
        } else {
            for pattern in &self.exclude_files {
                if glob_match(pattern, &filename) {
                    return false;
                }
            }
        }

        // Check size limits
        if let Some(min) = self.min_size {
            if size < min {
                return false;
            }
        }
        if let Some(max) = self.max_size {
            if size > max {
                return false;
            }
        }

        true
    }

    /// Check if a directory should be included
    fn should_include_dir(&self, path: &Path) -> bool {
        if let Some(gs) = &self.compiled_dirs {
            if gs.is_match(path.to_string_lossy().as_ref()) {
                return false;
            }
        } else {
            for pattern in &self.exclude_dirs {
                for component in path.components() {
                    if let Some(component_str) = component.as_os_str().to_str() {
                        if glob_match(pattern, component_str) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
}

/// Simple glob matching (supports * wildcards)
fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    // Simple wildcard matching
    if pattern.contains('*') {
        if pattern.starts_with('*') && pattern.ends_with('*') {
            let middle = &pattern[1..pattern.len() - 1];
            return text.contains(middle);
        } else if let Some(suffix) = pattern.strip_prefix('*') {
            return text.ends_with(suffix);
        } else if let Some(prefix) = pattern.strip_suffix('*') {
            return text.starts_with(prefix);
        }
    }

    // Exact match
    pattern == text
}

// All Windows-specific code removed.

/// Fast directory enumeration with filtering for non-Windows platforms
#[cfg(not(windows))]
/// Enumerate files under `root`, applying `filter` rules. Does not follow symlinks.
pub fn enumerate_directory_filtered(
    root: &Path,
    filter: &mut FileFilter,
) -> Result<Vec<FileEntry>> {
    let enumerator = FileEnumerator::new(filter.clone_without_cache());
    let entries = enumerator.enumerate_local(root)?;
    Ok(entries
        .into_iter()
        .filter_map(EnumeratedEntry::into_file_entry)
        .collect())
}

/// Enumerate symlinks under `root`, applying `filter` rules. Does not follow symlinks.
#[cfg(not(windows))]
pub fn enumerate_symlinks(root: &Path, filter: &mut FileFilter) -> Result<Vec<SymlinkEntry>> {
    let enumerator = FileEnumerator::new(filter.clone_without_cache()).include_symlinks(true);
    let entries = enumerator.enumerate_local(root)?;
    Ok(entries
        .into_iter()
        .filter_map(|entry| match entry.kind {
            EntryKind::Symlink { target } => {
                let target = target?;
                let target_is_dir = std::fs::metadata(&entry.absolute_path)
                    .map(|md| md.is_dir())
                    .unwrap_or(false);
                Some(SymlinkEntry {
                    path: entry.absolute_path,
                    target,
                    target_is_dir,
                })
            }
            _ => None,
        })
        .collect())
}

/// Windows implementation: use WalkDir without following reparse points.
#[cfg(windows)]
pub fn enumerate_directory_filtered(
    root: &Path,
    filter: &mut FileFilter,
) -> Result<Vec<FileEntry>> {
    use walkdir::WalkDir;

    let mut entries = Vec::new();
    filter.ensure_compiled();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                filter.should_include_dir(e.path())
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                let size = metadata.len();
                if filter.should_include_file(path, size) {
                    entries.push(FileEntry {
                        path: path.to_path_buf(),
                        size,
                        is_directory: false,
                    });
                }
            }
        }
    }

    Ok(entries)
}

#[cfg(windows)]
pub fn enumerate_directory_filtered(
    root: &Path,
    filter: &mut FileFilter,
) -> Result<Vec<FileEntry>> {
    let enumerator = FileEnumerator::new(filter.clone_without_cache());
    let entries = enumerator.enumerate_local(root)?;
    Ok(entries
        .into_iter()
        .filter_map(EnumeratedEntry::into_file_entry)
        .collect())
}

#[cfg(windows)]
pub fn enumerate_symlinks(root: &Path, filter: &mut FileFilter) -> Result<Vec<SymlinkEntry>> {
    let enumerator = FileEnumerator::new(filter.clone_without_cache()).include_symlinks(true);
    let entries = enumerator.enumerate_local(root)?;
    Ok(entries
        .into_iter()
        .filter_map(|entry| match entry.kind {
            EntryKind::Symlink { target } => {
                let target = target?;
                let target_is_dir = std::fs::metadata(&entry.absolute_path)
                    .map(|md| md.is_dir())
                    .unwrap_or(false);
                Some(SymlinkEntry {
                    path: entry.absolute_path,
                    target,
                    target_is_dir,
                })
            }
            _ => None,
        })
        .collect())
}

/// Categorize files by size for optimal copy strategy
pub fn categorize_files(entries: Vec<CopyJob>) -> (Vec<CopyJob>, Vec<CopyJob>, Vec<CopyJob>) {
    const SMALL_LIMIT: u64 = 1_048_576; // 1MB
    const MEDIUM_LIMIT: u64 = 104_857_600; // 100MB
    let mut small = Vec::new(); // < 1MB - tar streaming candidates
    let mut medium = Vec::new(); // 1-100MB - parallel copy
    let mut large = Vec::new(); // > 100MB - chunked copy

    for job in entries {
        if job.entry.size < SMALL_LIMIT {
            small.push(job);
        } else if job.entry.size < MEDIUM_LIMIT {
            medium.push(job);
        } else {
            large.push(job);
        }
    }

    (small, medium, large)
}

/// Enumerate files while following directory links and treating symlinked files as files.
/// Applies filters and avoids simple symlink cycles by tracking visited canonical directories.
/// Enumerate files while dereferencing symlinks. Filters are applied to final paths.
pub fn enumerate_directory_deref_filtered(
    root: &Path,
    filter: &mut FileFilter,
) -> Result<Vec<FileEntry>> {
    let enumerator = FileEnumerator::new(filter.clone_without_cache()).follow_symlinks(true);
    let entries = enumerator.enumerate_local(root)?;
    Ok(entries
        .into_iter()
        .filter_map(EnumeratedEntry::into_file_entry)
        .collect())
}
