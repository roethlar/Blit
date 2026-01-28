//! Unified manifest comparison for incremental transfers.
//!
//! This module provides manifest comparison logic used by both push and pull
//! operations to determine which files need to be transferred.

use crate::generated::FileHeader;
use std::collections::HashMap;

/// How to compare files between source and target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompareMode {
    /// Default: Compare size and mtime, skip if target is newer (safe).
    #[default]
    Default,
    /// Compare only by size, ignore modification time.
    SizeOnly,
    /// Transfer all files unconditionally.
    IgnoreTimes,
    /// Force transfer even if target is newer (dangerous for mirror).
    Force,
    /// Checksum mode: Transfer if checksums differ (slower but more accurate).
    /// For remote transfers, server computes checksums on demand.
    Checksum,
}

/// Status of a file after manifest comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    /// File exists on both sides with matching size and mtime.
    Unchanged,
    /// File exists on both sides but size or mtime differs.
    Modified,
    /// File exists on source but not on target.
    New,
    /// File exists on target and should be skipped (ignore_existing mode).
    SkippedExisting,
}

/// Result of comparing a single file.
#[derive(Debug, Clone)]
pub struct FileComparison {
    pub relative_path: String,
    pub status: FileStatus,
    /// Size of the source file (for transfer planning).
    pub size: u64,
}

/// Result of comparing two manifests.
#[derive(Debug, Default)]
pub struct ManifestDiff {
    /// Files that need to be transferred (new or modified).
    pub files_to_transfer: Vec<FileComparison>,
    /// Files that exist on target but not on source (for mirror mode deletion).
    pub files_to_delete: Vec<String>,
    /// Total bytes that need to be transferred.
    pub bytes_to_transfer: u64,
    /// Total files on source.
    pub source_file_count: usize,
    /// Total files on target.
    pub target_file_count: usize,
}

/// Options for manifest comparison.
#[derive(Debug, Clone, Default)]
pub struct CompareOptions {
    /// How to compare files.
    pub mode: CompareMode,
    /// If true, skip files that already exist on target (regardless of differences).
    pub ignore_existing: bool,
    /// If true, track files to delete for mirror mode.
    pub include_deletions: bool,
}

/// Compare source manifest against target manifest to determine what needs transferring.
///
/// For push: source = client files, target = server files
/// For pull: source = server files, target = client files
///
/// # Arguments
/// * `source` - Files on the source side (what we have)
/// * `target` - Files on the target side (what exists at destination)
/// * `options` - Comparison options controlling behavior
pub fn compare_manifests(
    source: &[FileHeader],
    target: &[FileHeader],
    options: &CompareOptions,
) -> ManifestDiff {
    let mut diff = ManifestDiff {
        source_file_count: source.len(),
        target_file_count: target.len(),
        ..Default::default()
    };

    // Build lookup from target manifest: path -> (size, mtime)
    let target_map: HashMap<&str, (u64, i64)> = target
        .iter()
        .map(|h| (h.relative_path.as_str(), (h.size, h.mtime_seconds)))
        .collect();

    // Compare each source file against target
    for src in source {
        let status = match target_map.get(src.relative_path.as_str()) {
            None => FileStatus::New,
            Some(&(target_size, target_mtime)) => {
                // File exists on target
                if options.ignore_existing {
                    // Skip all existing files regardless of differences
                    FileStatus::SkippedExisting
                } else {
                    compare_file(src, target_size, target_mtime, options.mode)
                }
            }
        };

        if status == FileStatus::New || status == FileStatus::Modified {
            diff.bytes_to_transfer += src.size;
            diff.files_to_transfer.push(FileComparison {
                relative_path: src.relative_path.clone(),
                status,
                size: src.size,
            });
        }
    }

    // Track deletions for mirror mode
    if options.include_deletions {
        let source_set: std::collections::HashSet<&str> =
            source.iter().map(|h| h.relative_path.as_str()).collect();

        for target_file in target {
            if !source_set.contains(target_file.relative_path.as_str()) {
                diff.files_to_delete.push(target_file.relative_path.clone());
            }
        }
    }

    diff
}

/// Compare a single file using the specified comparison mode.
fn compare_file(src: &FileHeader, target_size: u64, target_mtime: i64, mode: CompareMode) -> FileStatus {
    match mode {
        CompareMode::IgnoreTimes => {
            // Transfer all files unconditionally
            FileStatus::Modified
        }
        CompareMode::Force => {
            // Transfer if size differs OR source is different time (either direction)
            if src.size != target_size || src.mtime_seconds != target_mtime {
                FileStatus::Modified
            } else {
                FileStatus::Unchanged
            }
        }
        CompareMode::SizeOnly => {
            // Compare only by size, ignore mtime
            if src.size != target_size {
                FileStatus::Modified
            } else {
                FileStatus::Unchanged
            }
        }
        CompareMode::Default => {
            // Compare size and mtime, skip if target is newer (safe default)
            if src.size != target_size {
                FileStatus::Modified
            } else if src.mtime_seconds > target_mtime {
                // Source is newer - transfer
                FileStatus::Modified
            } else {
                // Target is same age or newer - skip (safe)
                FileStatus::Unchanged
            }
        }
        CompareMode::Checksum => {
            // Checksum mode: For manifest-based comparison, we transfer if size differs.
            // Full checksum comparison is done at the file copy level, not manifest level.
            // This ensures files go through the copy path where checksums are computed.
            if src.size != target_size {
                FileStatus::Modified
            } else {
                // Same size - mark as modified to trigger checksum comparison during copy
                FileStatus::Modified
            }
        }
    }
}

/// Build a manifest from FileHeader list, returning paths that need transfer.
pub fn files_needing_transfer(diff: &ManifestDiff) -> Vec<String> {
    diff.files_to_transfer
        .iter()
        .map(|f| f.relative_path.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(path: &str, size: u64, mtime: i64) -> FileHeader {
        FileHeader {
            relative_path: path.to_string(),
            size,
            mtime_seconds: mtime,
            permissions: 0o644,
        }
    }

    fn default_opts() -> CompareOptions {
        CompareOptions::default()
    }

    fn opts_with_deletions() -> CompareOptions {
        CompareOptions {
            include_deletions: true,
            ..Default::default()
        }
    }

    #[test]
    fn test_empty_manifests() {
        let diff = compare_manifests(&[], &[], &default_opts());
        assert!(diff.files_to_transfer.is_empty());
        assert!(diff.files_to_delete.is_empty());
        assert_eq!(diff.bytes_to_transfer, 0);
    }

    #[test]
    fn test_all_new_files() {
        let source = vec![
            header("a.txt", 100, 1000),
            header("b.txt", 200, 1000),
        ];
        let target = vec![];

        let diff = compare_manifests(&source, &target, &default_opts());
        assert_eq!(diff.files_to_transfer.len(), 2);
        assert_eq!(diff.bytes_to_transfer, 300);
        assert!(diff.files_to_transfer.iter().all(|f| f.status == FileStatus::New));
    }

    #[test]
    fn test_unchanged_files() {
        let source = vec![header("a.txt", 100, 1000)];
        let target = vec![header("a.txt", 100, 1000)];

        let diff = compare_manifests(&source, &target, &default_opts());
        assert!(diff.files_to_transfer.is_empty());
        assert_eq!(diff.bytes_to_transfer, 0);
    }

    #[test]
    fn test_modified_by_size() {
        let source = vec![header("a.txt", 200, 1000)];
        let target = vec![header("a.txt", 100, 1000)];

        let diff = compare_manifests(&source, &target, &default_opts());
        assert_eq!(diff.files_to_transfer.len(), 1);
        assert_eq!(diff.files_to_transfer[0].status, FileStatus::Modified);
        assert_eq!(diff.bytes_to_transfer, 200);
    }

    #[test]
    fn test_modified_by_mtime() {
        let source = vec![header("a.txt", 100, 2000)];
        let target = vec![header("a.txt", 100, 1000)];

        let diff = compare_manifests(&source, &target, &default_opts());
        assert_eq!(diff.files_to_transfer.len(), 1);
        assert_eq!(diff.files_to_transfer[0].status, FileStatus::Modified);
    }

    #[test]
    fn test_target_newer_unchanged() {
        // If target is newer, we don't overwrite (source is not newer) - safe default
        let source = vec![header("a.txt", 100, 1000)];
        let target = vec![header("a.txt", 100, 2000)];

        let diff = compare_manifests(&source, &target, &default_opts());
        assert!(diff.files_to_transfer.is_empty());
    }

    #[test]
    fn test_force_mode_overwrites_newer() {
        // Force mode should transfer even if target is newer
        let source = vec![header("a.txt", 100, 1000)];
        let target = vec![header("a.txt", 100, 2000)];

        let opts = CompareOptions {
            mode: CompareMode::Force,
            ..Default::default()
        };
        let diff = compare_manifests(&source, &target, &opts);
        assert_eq!(diff.files_to_transfer.len(), 1);
        assert_eq!(diff.files_to_transfer[0].status, FileStatus::Modified);
    }

    #[test]
    fn test_size_only_mode() {
        // Size-only ignores mtime differences
        let source = vec![header("a.txt", 100, 2000)];
        let target = vec![header("a.txt", 100, 1000)];

        let opts = CompareOptions {
            mode: CompareMode::SizeOnly,
            ..Default::default()
        };
        let diff = compare_manifests(&source, &target, &opts);
        assert!(diff.files_to_transfer.is_empty()); // Same size, so unchanged
    }

    #[test]
    fn test_ignore_times_mode() {
        // Ignore-times transfers everything unconditionally
        let source = vec![header("a.txt", 100, 1000)];
        let target = vec![header("a.txt", 100, 1000)]; // Identical file

        let opts = CompareOptions {
            mode: CompareMode::IgnoreTimes,
            ..Default::default()
        };
        let diff = compare_manifests(&source, &target, &opts);
        assert_eq!(diff.files_to_transfer.len(), 1);
    }

    #[test]
    fn test_ignore_existing() {
        // Ignore-existing skips all files that exist on target
        let source = vec![
            header("exists.txt", 200, 2000), // Modified, but should be skipped
            header("new.txt", 100, 1000),    // New, should transfer
        ];
        let target = vec![header("exists.txt", 100, 1000)];

        let opts = CompareOptions {
            ignore_existing: true,
            ..Default::default()
        };
        let diff = compare_manifests(&source, &target, &opts);
        assert_eq!(diff.files_to_transfer.len(), 1);
        assert_eq!(diff.files_to_transfer[0].relative_path, "new.txt");
    }

    #[test]
    fn test_deletions_for_mirror() {
        let source = vec![header("a.txt", 100, 1000)];
        let target = vec![
            header("a.txt", 100, 1000),
            header("b.txt", 200, 1000),
        ];

        let diff = compare_manifests(&source, &target, &opts_with_deletions());
        assert!(diff.files_to_transfer.is_empty());
        assert_eq!(diff.files_to_delete.len(), 1);
        assert_eq!(diff.files_to_delete[0], "b.txt");
    }

    #[test]
    fn test_mixed_scenario() {
        let source = vec![
            header("unchanged.txt", 100, 1000),
            header("modified.txt", 200, 2000),
            header("new.txt", 300, 1000),
        ];
        let target = vec![
            header("unchanged.txt", 100, 1000),
            header("modified.txt", 150, 1000),
            header("deleted.txt", 50, 1000),
        ];

        let diff = compare_manifests(&source, &target, &opts_with_deletions());
        assert_eq!(diff.files_to_transfer.len(), 2); // modified + new
        assert_eq!(diff.files_to_delete.len(), 1);   // deleted
        assert_eq!(diff.bytes_to_transfer, 500);     // 200 + 300
    }
}
