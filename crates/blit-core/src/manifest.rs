//! Unified manifest comparison for incremental transfers.
//!
//! This module provides manifest comparison logic used by both push and pull
//! operations to determine which files need to be transferred.

use crate::generated::{ComparisonMode, FileHeader};

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

/// Canonical mapping from the wire enum. `Unspecified` folds to the
/// historical default, matching `NormalizedTransferOperation` and the
/// diff planner's defensive handling.
impl From<ComparisonMode> for CompareMode {
    fn from(mode: ComparisonMode) -> Self {
        match mode {
            ComparisonMode::Checksum => CompareMode::Checksum,
            ComparisonMode::SizeOnly => CompareMode::SizeOnly,
            ComparisonMode::IgnoreTimes => CompareMode::IgnoreTimes,
            ComparisonMode::Force => CompareMode::Force,
            ComparisonMode::Unspecified | ComparisonMode::SizeMtime => CompareMode::Default,
        }
    }
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

/// Options for the per-entry manifest comparison.
/// (`include_deletions` and the materialized `compare_manifests`
/// aggregate died at otp-11b with their last caller — session
/// deletions are the otp-6b mirror pass, a whole-tree diff at
/// SourceDone, never a per-entry flag.)
#[derive(Debug, Clone, Default)]
pub struct CompareOptions {
    /// How to compare files.
    pub mode: CompareMode,
    /// If true, skip files that already exist on target (regardless of differences).
    pub ignore_existing: bool,
}

/// Status of one source header against the target's view of the same
/// path — `Some((size, mtime_seconds, checksum))` when the target has
/// the path, `None` when it doesn't. The single owner of the
/// mode-aware header-vs-target decision: the unified
/// `transfer_session` destination diff (which stats its own
/// filesystem per entry instead of materializing a full target
/// manifest) calls it for every manifest entry.
pub fn header_transfer_status(
    src: &FileHeader,
    target: Option<(u64, i64, &[u8])>,
    options: &CompareOptions,
) -> FileStatus {
    match target {
        None => FileStatus::New,
        Some((target_size, target_mtime, target_checksum)) => {
            // File exists on target
            if options.ignore_existing {
                // Skip all existing files regardless of differences
                FileStatus::SkippedExisting
            } else {
                compare_file(
                    src,
                    target_size,
                    target_mtime,
                    target_checksum,
                    options.mode,
                )
            }
        }
    }
}

/// Compare a single file using the specified comparison mode.
fn compare_file(
    src: &FileHeader,
    target_size: u64,
    target_mtime: i64,
    target_checksum: &[u8],
    mode: CompareMode,
) -> FileStatus {
    match mode {
        CompareMode::IgnoreTimes => {
            // Transfer all files unconditionally
            FileStatus::Modified
        }
        CompareMode::Force => {
            // R58-F9: Force means "transfer regardless of target
            // state" per the proto contract (proto/blit.proto:443)
            // and the diff_planner's always-copy behavior. The
            // size/mtime comparison previously here disagreed with
            // both — if the user said --force, the manifest layer
            // should NOT second-guess them. Always Modified.
            let _ = (target_size, target_mtime, target_checksum);
            FileStatus::Modified
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
            // Checksum mode: Compare using checksums if available
            if src.size != target_size {
                FileStatus::Modified
            } else if !src.checksum.is_empty() && !target_checksum.is_empty() {
                // Both have checksums - compare them
                if src.checksum == target_checksum {
                    FileStatus::Unchanged
                } else {
                    FileStatus::Modified
                }
            } else {
                // Checksums not available - must transfer for verification
                // (This happens when server checksums are disabled)
                FileStatus::Modified
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! Direct pins on `header_transfer_status` — the live per-entry
    //! compare owner every session diff routes through. Converted 1:1
    //! from the retired `compare_manifests` test block at otp-11b
    //! (the three aggregate-shape tests — empty manifests, deletion
    //! tracking, the mixed scenario — retired with the aggregate;
    //! deletions are pinned on the session mirror pass).

    use super::*;

    fn header(path: &str, size: u64, mtime: i64) -> FileHeader {
        header_with_checksum(path, size, mtime, vec![])
    }

    fn header_with_checksum(path: &str, size: u64, mtime: i64, checksum: Vec<u8>) -> FileHeader {
        FileHeader {
            relative_path: path.to_string(),
            size,
            mtime_seconds: mtime,
            permissions: 0o644,
            checksum,
        }
    }

    fn status(src: &FileHeader, target: Option<&FileHeader>, opts: &CompareOptions) -> FileStatus {
        header_transfer_status(
            src,
            target.map(|t| (t.size, t.mtime_seconds, t.checksum.as_slice())),
            opts,
        )
    }

    fn mode_opts(mode: CompareMode) -> CompareOptions {
        CompareOptions {
            mode,
            ..Default::default()
        }
    }

    #[test]
    fn absent_target_is_new() {
        let src = header("a.txt", 100, 1000);
        assert_eq!(
            status(&src, None, &CompareOptions::default()),
            FileStatus::New
        );
    }

    #[test]
    fn matching_size_and_mtime_is_unchanged() {
        let src = header("a.txt", 100, 1000);
        let dst = header("a.txt", 100, 1000);
        assert_eq!(
            status(&src, Some(&dst), &CompareOptions::default()),
            FileStatus::Unchanged
        );
    }

    #[test]
    fn size_difference_is_modified() {
        let src = header("a.txt", 200, 1000);
        let dst = header("a.txt", 100, 1000);
        assert_eq!(
            status(&src, Some(&dst), &CompareOptions::default()),
            FileStatus::Modified
        );
    }

    #[test]
    fn newer_source_mtime_is_modified() {
        let src = header("a.txt", 100, 2000);
        let dst = header("a.txt", 100, 1000);
        assert_eq!(
            status(&src, Some(&dst), &CompareOptions::default()),
            FileStatus::Modified
        );
    }

    /// The data-safe default (otp-4a owner ack): same size, NEWER
    /// destination — skip, never clobber.
    #[test]
    fn newer_target_is_unchanged_safe_default() {
        let src = header("a.txt", 100, 1000);
        let dst = header("a.txt", 100, 2000);
        assert_eq!(
            status(&src, Some(&dst), &CompareOptions::default()),
            FileStatus::Unchanged
        );
    }

    /// R58-F9: Force means "transfer regardless of target state" — the
    /// compare layer must not second-guess, even a newer target.
    #[test]
    fn force_overwrites_newer_target() {
        let src = header("a.txt", 100, 1000);
        let dst = header("a.txt", 100, 2000);
        assert_eq!(
            status(&src, Some(&dst), &mode_opts(CompareMode::Force)),
            FileStatus::Modified
        );
    }

    #[test]
    fn size_only_ignores_mtime_difference() {
        let src = header("a.txt", 100, 2000);
        let dst = header("a.txt", 100, 1000);
        assert_eq!(
            status(&src, Some(&dst), &mode_opts(CompareMode::SizeOnly)),
            FileStatus::Unchanged
        );
    }

    #[test]
    fn ignore_times_transfers_identical_file() {
        let src = header("a.txt", 100, 1000);
        let dst = header("a.txt", 100, 1000);
        assert_eq!(
            status(&src, Some(&dst), &mode_opts(CompareMode::IgnoreTimes)),
            FileStatus::Modified
        );
    }

    /// `--ignore-existing`: any existing target skips regardless of
    /// differences; absent targets still transfer.
    #[test]
    fn ignore_existing_skips_existing_but_not_new() {
        let opts = CompareOptions {
            ignore_existing: true,
            ..Default::default()
        };
        let modified_src = header("exists.txt", 200, 2000);
        let dst = header("exists.txt", 100, 1000);
        assert_eq!(
            status(&modified_src, Some(&dst), &opts),
            FileStatus::SkippedExisting
        );
        let new_src = header("new.txt", 100, 1000);
        assert_eq!(status(&new_src, None, &opts), FileStatus::New);
    }

    /// Checksum mode: content-equal skips even with a different mtime
    /// (the cell `--checksum` exists for).
    #[test]
    fn checksum_same_checksum_skips_despite_mtime() {
        let sum = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let src = header_with_checksum("a.txt", 100, 1000, sum.clone());
        let dst = header_with_checksum("a.txt", 100, 2000, sum);
        assert_eq!(
            status(&src, Some(&dst), &mode_opts(CompareMode::Checksum)),
            FileStatus::Unchanged
        );
    }

    #[test]
    fn checksum_different_checksum_transfers() {
        let src = header_with_checksum("a.txt", 100, 1000, vec![1, 2, 3, 4]);
        let dst = header_with_checksum("a.txt", 100, 1000, vec![5, 6, 7, 8]);
        assert_eq!(
            status(&src, Some(&dst), &mode_opts(CompareMode::Checksum)),
            FileStatus::Modified
        );
    }

    /// Missing checksums under Checksum mode transfer conservatively
    /// (server checksums disabled ⇒ content cannot be verified).
    #[test]
    fn checksum_missing_checksums_transfer_conservatively() {
        let src = header("a.txt", 100, 1000);
        let dst = header("a.txt", 100, 1000);
        assert_eq!(
            status(&src, Some(&dst), &mode_opts(CompareMode::Checksum)),
            FileStatus::Modified
        );
    }

    /// `ignore_existing` wins BEFORE the mode is consulted — even
    /// Checksum-mode candidates skip when the target exists.
    #[test]
    fn ignore_existing_wins_over_checksum_mode() {
        let opts = CompareOptions {
            mode: CompareMode::Checksum,
            ignore_existing: true,
        };
        let src = header_with_checksum("a.txt", 100, 1000, vec![1, 2]);
        let dst = header_with_checksum("a.txt", 100, 1000, vec![3, 4]);
        assert_eq!(status(&src, Some(&dst), &opts), FileStatus::SkippedExisting);
    }

    /// Every mode reports an absent target as New (the mode only
    /// applies once a target exists).
    #[test]
    fn absent_target_is_new_in_every_mode() {
        for mode in [
            CompareMode::Default,
            CompareMode::SizeOnly,
            CompareMode::IgnoreTimes,
            CompareMode::Force,
            CompareMode::Checksum,
        ] {
            let src = header("a.txt", 100, 1000);
            assert_eq!(
                status(&src, None, &mode_opts(mode)),
                FileStatus::New,
                "mode {mode:?}"
            );
        }
    }

    #[test]
    fn checksum_size_difference_transfers_without_hashing() {
        let sum = vec![1, 2, 3, 4];
        let src = header_with_checksum("a.txt", 200, 1000, sum.clone());
        let dst = header_with_checksum("a.txt", 100, 1000, sum);
        assert_eq!(
            status(&src, Some(&dst), &mode_opts(CompareMode::Checksum)),
            FileStatus::Modified
        );
    }
}
