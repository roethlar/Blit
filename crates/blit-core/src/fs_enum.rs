use eyre::{Context, Result};
use once_cell::sync::OnceCell;
use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

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

/// File filter options. Single source-of-truth for what passes through
/// the transfer pipeline regardless of source/destination type.
///
/// Semantics:
/// - `include_files` (whitelist): if any pattern is set, the file must
///   match at least one to pass. Empty list disables the whitelist.
/// - `exclude_files` / `exclude_dirs`: matching files/dirs are blocked.
/// - `min_size` / `max_size`: size constraints applied after pattern checks.
/// - `min_age` / `max_age`: age constraints relative to `reference_time`
///   (set by the orchestrator at filter-build time, not by the leaf code).
/// - `files_from`: when present, only listed relative paths pass; all
///   other rules above are bypassed for the inclusion test.
#[derive(Debug)]
pub struct FileFilter {
    pub include_files: Vec<String>,
    pub exclude_files: Vec<String>,
    pub exclude_dirs: Vec<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub min_age: Option<Duration>,
    pub max_age: Option<Duration>,
    /// Set by orchestrator when building the filter (calculated, not hardcoded).
    pub reference_time: Option<SystemTime>,
    pub files_from: Option<HashSet<PathBuf>>,
    #[allow(dead_code)]
    compiled_includes: OnceCell<globset::GlobSet>,
    #[allow(dead_code)]
    compiled_files: OnceCell<globset::GlobSet>,
    #[allow(dead_code)]
    compiled_dirs: OnceCell<globset::GlobSet>,
}

impl FileFilter {
    /// Create a shallow clone that preserves user-specified patterns while
    /// dropping any compiled glob caches. This lets helpers reuse filters
    /// without sharing mutable compilation state.
    pub fn clone_without_cache(&self) -> Self {
        Self {
            include_files: self.include_files.clone(),
            exclude_files: self.exclude_files.clone(),
            exclude_dirs: self.exclude_dirs.clone(),
            min_size: self.min_size,
            max_size: self.max_size,
            min_age: self.min_age,
            max_age: self.max_age,
            reference_time: self.reference_time,
            files_from: self.files_from.clone(),
            compiled_includes: OnceCell::new(),
            compiled_files: OnceCell::new(),
            compiled_dirs: OnceCell::new(),
        }
    }

    /// True when no rules are configured — caller can skip filter checks entirely.
    pub fn is_empty(&self) -> bool {
        self.include_files.is_empty()
            && self.exclude_files.is_empty()
            && self.exclude_dirs.is_empty()
            && self.min_size.is_none()
            && self.max_size.is_none()
            && self.min_age.is_none()
            && self.max_age.is_none()
            && self.files_from.is_none()
    }

    /// Load a `--files-from` list (one relative path per line, blank lines
    /// and `#` comments skipped). Used by the orchestrator/CLI helper —
    /// the leaf TransferSource code should not parse files itself.
    pub fn load_files_from(path: &Path) -> Result<HashSet<PathBuf>> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("opening files-from list {}", path.display()))?;
        let mut set = HashSet::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            set.insert(PathBuf::from(trimmed));
        }
        Ok(set)
    }

    fn build_globset(patterns: &[String]) -> globset::GlobSet {
        let mut builder = globset::GlobSetBuilder::new();
        for pat in patterns {
            if let Ok(glob) = globset::Glob::new(pat) {
                builder.add(glob);
            }
        }
        builder.build().unwrap_or_else(|_| {
            globset::GlobSetBuilder::new()
                .build()
                .expect("empty globset")
        })
    }

    fn include_globs(&self) -> &globset::GlobSet {
        self.compiled_includes
            .get_or_init(|| Self::build_globset(&self.include_files))
    }

    fn file_globs(&self) -> &globset::GlobSet {
        self.compiled_files
            .get_or_init(|| Self::build_globset(&self.exclude_files))
    }

    fn dir_globs(&self) -> &globset::GlobSet {
        self.compiled_dirs
            .get_or_init(|| Self::build_globset(&self.exclude_dirs))
    }

    pub(crate) fn allows_dir(&self, path: &Path) -> bool {
        self.should_include_dir(path)
    }

    /// Filter check given only a relative path + manifest metadata.
    /// Used when we don't have an absolute on-disk path — e.g. the
    /// daemon evaluating "would this client-side file have been
    /// allowed by the source filter, if it had existed on the source?"
    /// during MirrorMode::FilteredSubset deletion scoping.
    ///
    /// Filename-based globs work because the filename component is the
    /// same regardless of which root the path is measured under.
    pub fn allows_relative(&self, rel_path: &Path, size: u64, mtime: Option<SystemTime>) -> bool {
        self.allows_entry(Some(rel_path), rel_path, size, mtime)
    }

    /// Full filter check. `rel_path` enables `files_from` matching;
    /// `mtime` enables age filtering. Both default to permissive when
    /// `None` (so back-compat callers via `allows_file` still work).
    pub fn allows_entry(
        &self,
        rel_path: Option<&Path>,
        abs_path: &Path,
        size: u64,
        mtime: Option<SystemTime>,
    ) -> bool {
        // files_from is exclusive: only listed paths pass. Other rules
        // are bypassed because the user explicitly enumerated targets.
        if let Some(ref allowed) = self.files_from {
            return rel_path.map(|p| allowed.contains(p)).unwrap_or(false);
        }

        let filename = abs_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let path_str = rel_path
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| abs_path.to_string_lossy().into_owned());

        // Whitelist: when include_files is non-empty, file must match one.
        // Match against both the relative path and the bare filename so
        // `--include '*.log'` works without users writing `**/*.log`.
        if !self.include_files.is_empty() {
            let gs = self.include_globs();
            let matched = gs.is_match(&filename)
                || gs.is_match(&path_str)
                || self
                    .include_files
                    .iter()
                    .any(|p| glob_match(p, &filename) || glob_match(p, &path_str));
            if !matched {
                return false;
            }
        }

        // Blacklist
        let gs = self.file_globs();
        if gs.is_match(&filename) || gs.is_match(&path_str) {
            return false;
        }
        for pattern in &self.exclude_files {
            if glob_match(pattern, &filename) || glob_match(pattern, &path_str) {
                return false;
            }
        }

        // Size
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

        // Age (only applied when both mtime and reference_time are present)
        if let (Some(mtime), Some(now)) = (mtime, self.reference_time) {
            if let Ok(age) = now.duration_since(mtime) {
                if let Some(min_age) = self.min_age {
                    if age < min_age {
                        return false;
                    }
                }
                if let Some(max_age) = self.max_age {
                    if age > max_age {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Check if a directory should be included
    fn should_include_dir(&self, path: &Path) -> bool {
        // files_from doesn't restrict dir traversal — we still need to
        // descend into directories to find listed files.
        if self.files_from.is_some() {
            return true;
        }

        let gs = self.dir_globs();
        if gs.is_match(path.to_string_lossy().as_ref()) {
            return false;
        }
        for pattern in &self.exclude_dirs {
            for component in path.components() {
                if let Some(component_str) = component.as_os_str().to_str() {
                    if glob_match(pattern, component_str) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

impl Default for FileFilter {
    fn default() -> Self {
        Self {
            include_files: Vec::new(),
            exclude_files: Vec::new(),
            exclude_dirs: Vec::new(),
            min_size: None,
            max_size: None,
            min_age: None,
            max_age: None,
            reference_time: None,
            files_from: None,
            compiled_includes: OnceCell::new(),
            compiled_files: OnceCell::new(),
            compiled_dirs: OnceCell::new(),
        }
    }
}

impl Clone for FileFilter {
    fn clone(&self) -> Self {
        self.clone_without_cache()
    }
}

/// Parse a human-readable size like "100K", "10M", "1G", "1.5Mi" into bytes.
/// SI suffixes (K=1000) and binary suffixes (Ki=1024) both supported.
pub fn parse_size(s: &str) -> Result<u64> {
    let s = s.trim();
    if s.is_empty() {
        eyre::bail!("empty size string");
    }
    let (num_str, multiplier) = if let Some(n) = s.strip_suffix("Ti") {
        (n, 1u64 << 40)
    } else if let Some(n) = s.strip_suffix("Gi") {
        (n, 1u64 << 30)
    } else if let Some(n) = s.strip_suffix("Mi") {
        (n, 1u64 << 20)
    } else if let Some(n) = s.strip_suffix("Ki") {
        (n, 1u64 << 10)
    } else if let Some(n) = s.strip_suffix('T') {
        (n, 1_000_000_000_000u64)
    } else if let Some(n) = s.strip_suffix('G') {
        (n, 1_000_000_000)
    } else if let Some(n) = s.strip_suffix('M') {
        (n, 1_000_000)
    } else if let Some(n) = s.strip_suffix('K') {
        (n, 1_000)
    } else {
        (s, 1)
    };
    let num: f64 = num_str
        .parse()
        .with_context(|| format!("invalid size number: {num_str}"))?;
    Ok((num * multiplier as f64) as u64)
}

/// Parse a duration like "30s", "5m", "1h", "7d", or compounds "1h30m".
/// A bare number is interpreted as seconds.
pub fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.is_empty() {
        eyre::bail!("empty duration string");
    }
    let mut total_secs: u64 = 0;
    let mut num_buf = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            num_buf.push(ch);
        } else {
            if num_buf.is_empty() {
                eyre::bail!("invalid duration: {s}");
            }
            let num: f64 = num_buf
                .parse()
                .with_context(|| format!("invalid number in duration: {s}"))?;
            num_buf.clear();
            let multiplier: u64 = match ch {
                's' => 1,
                'm' => 60,
                'h' => 3600,
                'd' => 86400,
                'w' => 604800,
                _ => eyre::bail!("unknown duration unit '{ch}' in: {s}"),
            };
            total_secs += (num * multiplier as f64) as u64;
        }
    }
    if !num_buf.is_empty() {
        let num: f64 = num_buf
            .parse()
            .with_context(|| format!("invalid number in duration: {s}"))?;
        total_secs += num as u64;
    }
    if total_secs == 0 {
        eyre::bail!("duration must be non-zero: {s}");
    }
    Ok(Duration::from_secs(total_secs))
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
