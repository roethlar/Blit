use crate::checksum::{self, ChecksumType};
use crate::generated::ComparisonMode;
use eyre::{Context, Result};
use std::path::Path;
use std::time::SystemTime;

/// Check if a file needs to be copied (for mirror mode)
pub fn file_needs_copy(src: &Path, dst: &Path, use_checksum: bool) -> Result<bool> {
    if !dst.exists() {
        return Ok(true);
    }

    let src_meta = src.metadata()?;
    let dst_meta = dst.metadata()?;

    if src_meta.len() != dst_meta.len() {
        return Ok(true);
    }

    if use_checksum {
        Ok(file_needs_copy_with_checksum_type(
            src,
            dst,
            Some(ChecksumType::Blake3),
        )?)
    } else {
        let src_time = src_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let dst_time = dst_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        Ok(src_time
            .duration_since(dst_time)
            .is_ok_and(|diff| diff.as_secs() > 2))
    }
}

/// Like file_needs_copy, but with explicit checksum selection.
pub fn file_needs_copy_with_checksum_type(
    src: &Path,
    dst: &Path,
    checksum: Option<ChecksumType>,
) -> Result<bool> {
    if !dst.exists() {
        return Ok(true);
    }
    let src_meta = src.metadata()?;
    let dst_meta = dst.metadata()?;
    if src_meta.len() != dst_meta.len() {
        return Ok(true);
    }

    let ph_bytes = 1024 * 1024; // 1 MiB
    let src_ph = checksum::partial_hash_first_last(src, ph_bytes)?;
    let dst_ph = checksum::partial_hash_first_last(dst, ph_bytes)?;
    if src_ph != dst_ph {
        return Ok(true);
    }
    match checksum {
        Some(ChecksumType::Blake3) => {
            let a = checksum::hash_file(src, ChecksumType::Blake3)?;
            let b = checksum::hash_file(dst, ChecksumType::Blake3)?;
            Ok(a != b)
        }
        Some(_) | None => {
            let src_time = src_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let dst_time = dst_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            Ok(src_time
                .duration_since(dst_time)
                .is_ok_and(|diff| diff.as_secs() > 2))
        }
    }
}

/// Per-mode comparison predicate. Returns `true` when the source file
/// should be transferred to the destination given the comparison mode.
///
/// Centralized so single-file copy, the streaming sink, and the diff
/// planner share one decision tree — R58 review-followup found three
/// sites that disagreed (single-file copy + sink both ignored
/// `compare_mode` entirely and second-guessed the planner with the
/// old SizeMtime-only helper, defeating `--force` / `--ignore-times`
/// / `--size-only`).
///
/// Variants:
///   - `SizeMtime` / `Unspecified`: copy when missing, when sizes
///     differ, or when source is newer than dest by >2s. The 2s
///     tolerance matches `file_needs_copy_with_checksum_type` and
///     FAT/exFAT mtime granularity.
///   - `Checksum`: copy when missing, sizes differ, or Blake3 hashes
///     differ. mtime is not consulted.
///   - `SizeOnly`: copy when missing or sizes differ; mtime ignored.
///   - `IgnoreTimes` / `Force`: always copy.
///
/// `ignore_existing` is orthogonal and must be handled by the caller
/// before invoking this function.
pub fn file_needs_copy_with_mode(src: &Path, dst: &Path, mode: ComparisonMode) -> Result<bool> {
    match mode {
        ComparisonMode::IgnoreTimes | ComparisonMode::Force => Ok(true),
        ComparisonMode::SizeOnly => {
            if !dst.exists() {
                return Ok(true);
            }
            let src_meta = src.metadata().context("stat source for size compare")?;
            let dst_meta = dst.metadata().context("stat dest for size compare")?;
            Ok(src_meta.len() != dst_meta.len())
        }
        ComparisonMode::Checksum => {
            if !dst.exists() {
                return Ok(true);
            }
            let src_meta = src.metadata().context("stat source for checksum compare")?;
            let dst_meta = dst.metadata().context("stat dest for checksum compare")?;
            if src_meta.len() != dst_meta.len() {
                return Ok(true);
            }
            let src_hash = checksum::hash_file(src, ChecksumType::Blake3)
                .with_context(|| format!("hashing source {}", src.display()))?;
            let dst_hash = checksum::hash_file(dst, ChecksumType::Blake3)
                .with_context(|| format!("hashing dest {}", dst.display()))?;
            Ok(src_hash != dst_hash)
        }
        // Unspecified folds to the historical default.
        ComparisonMode::Unspecified | ComparisonMode::SizeMtime => {
            file_needs_copy_with_checksum_type(src, dst, None)
        }
    }
}
