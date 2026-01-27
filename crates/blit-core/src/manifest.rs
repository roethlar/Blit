//! Unified manifest comparison for incremental transfers.
//!
//! This module provides manifest comparison logic used by both push and pull
//! operations to determine which files need to be transferred.

use crate::generated::FileHeader;
use std::collections::HashMap;

/// Status of a file after manifest comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    /// File exists on both sides with matching size and mtime.
    Unchanged,
    /// File exists on both sides but size or mtime differs.
    Modified,
    /// File exists on source but not on target.
    New,
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

/// Compare source manifest against target manifest to determine what needs transferring.
///
/// For push: source = client files, target = server files
/// For pull: source = server files, target = client files
///
/// # Arguments
/// * `source` - Files on the source side (what we have)
/// * `target` - Files on the target side (what exists at destination)
/// * `include_deletions` - If true, track files to delete for mirror mode
pub fn compare_manifests(
    source: &[FileHeader],
    target: &[FileHeader],
    include_deletions: bool,
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
                // File exists on target - compare size and mtime
                if src.size != target_size {
                    FileStatus::Modified
                } else if src.mtime_seconds > target_mtime {
                    // Source is newer
                    FileStatus::Modified
                } else {
                    FileStatus::Unchanged
                }
            }
        };

        if status != FileStatus::Unchanged {
            diff.bytes_to_transfer += src.size;
            diff.files_to_transfer.push(FileComparison {
                relative_path: src.relative_path.clone(),
                status,
                size: src.size,
            });
        }
    }

    // Track deletions for mirror mode
    if include_deletions {
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

    #[test]
    fn test_empty_manifests() {
        let diff = compare_manifests(&[], &[], false);
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

        let diff = compare_manifests(&source, &target, false);
        assert_eq!(diff.files_to_transfer.len(), 2);
        assert_eq!(diff.bytes_to_transfer, 300);
        assert!(diff.files_to_transfer.iter().all(|f| f.status == FileStatus::New));
    }

    #[test]
    fn test_unchanged_files() {
        let source = vec![header("a.txt", 100, 1000)];
        let target = vec![header("a.txt", 100, 1000)];

        let diff = compare_manifests(&source, &target, false);
        assert!(diff.files_to_transfer.is_empty());
        assert_eq!(diff.bytes_to_transfer, 0);
    }

    #[test]
    fn test_modified_by_size() {
        let source = vec![header("a.txt", 200, 1000)];
        let target = vec![header("a.txt", 100, 1000)];

        let diff = compare_manifests(&source, &target, false);
        assert_eq!(diff.files_to_transfer.len(), 1);
        assert_eq!(diff.files_to_transfer[0].status, FileStatus::Modified);
        assert_eq!(diff.bytes_to_transfer, 200);
    }

    #[test]
    fn test_modified_by_mtime() {
        let source = vec![header("a.txt", 100, 2000)];
        let target = vec![header("a.txt", 100, 1000)];

        let diff = compare_manifests(&source, &target, false);
        assert_eq!(diff.files_to_transfer.len(), 1);
        assert_eq!(diff.files_to_transfer[0].status, FileStatus::Modified);
    }

    #[test]
    fn test_target_newer_unchanged() {
        // If target is newer, we don't overwrite (source is not newer)
        let source = vec![header("a.txt", 100, 1000)];
        let target = vec![header("a.txt", 100, 2000)];

        let diff = compare_manifests(&source, &target, false);
        assert!(diff.files_to_transfer.is_empty());
    }

    #[test]
    fn test_deletions_for_mirror() {
        let source = vec![header("a.txt", 100, 1000)];
        let target = vec![
            header("a.txt", 100, 1000),
            header("b.txt", 200, 1000),
        ];

        let diff = compare_manifests(&source, &target, true);
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

        let diff = compare_manifests(&source, &target, true);
        assert_eq!(diff.files_to_transfer.len(), 2); // modified + new
        assert_eq!(diff.files_to_delete.len(), 1);   // deleted
        assert_eq!(diff.bytes_to_transfer, 500);     // 200 + 300
    }
}
