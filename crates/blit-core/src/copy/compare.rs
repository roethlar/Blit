use crate::checksum::{self, ChecksumType};
use crate::generated::ComparisonMode;
use eyre::{Context, Result};
use std::path::Path;
use std::time::SystemTime;

/// What stands at `dst`, when it is something a copy can be THE SAME AS:
/// a regular file (or a link to one). `None` when nothing is there — and
/// when what is there is no file (cr-ecm12-3). A directory has a size and
/// a time too: where the two happened to pass the test — Btrfs gives a
/// directory holding one entry a size of twenty bytes or so — a directory
/// standing in a file's place was taken for the file already copied, and
/// the run reported one file copied, no bytes, exit 0. Such an entry
/// needs copying, and the write says why it cannot be.
fn comparable(dst: &Path) -> Option<std::fs::Metadata> {
    std::fs::metadata(dst)
        .ok()
        .filter(std::fs::Metadata::is_file)
}

/// Check if a file needs to be copied (for mirror mode)
pub fn file_needs_copy(src: &Path, dst: &Path, use_checksum: bool) -> Result<bool> {
    let src_meta = src.metadata()?;
    let Some(dst_meta) = comparable(dst) else {
        return Ok(true);
    };

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
    let src_meta = src.metadata()?;
    let Some(dst_meta) = comparable(dst) else {
        return Ok(true);
    };
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
            let src_meta = src.metadata().context("stat source for size compare")?;
            let Some(dst_meta) = comparable(dst) else {
                return Ok(true);
            };
            Ok(src_meta.len() != dst_meta.len())
        }
        ComparisonMode::Checksum => {
            let src_meta = src.metadata().context("stat source for checksum compare")?;
            let Some(dst_meta) = comparable(dst) else {
                return Ok(true);
            };
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

/// [`file_needs_copy_with_mode`] on an already-OPENED source handle
/// (ssc-4, SOURCE_SIDE_CONTAINMENT D-C): every source fact — size, mtime,
/// hash — comes from `src` and `src_meta` (the handle's own metadata),
/// never from a path re-stat that could describe a replacement inode.
/// Leaves the handle rewound to offset 0.
pub fn file_needs_copy_with_mode_opened(
    src: &mut std::fs::File,
    src_meta: &std::fs::Metadata,
    dst: &Path,
    mode: ComparisonMode,
) -> Result<bool> {
    use std::io::{Read, Seek, SeekFrom};
    match mode {
        ComparisonMode::IgnoreTimes | ComparisonMode::Force => Ok(true),
        ComparisonMode::SizeOnly => {
            let Some(dst_meta) = comparable(dst) else {
                return Ok(true);
            };
            Ok(src_meta.len() != dst_meta.len())
        }
        ComparisonMode::Checksum => {
            let Some(dst_meta) = comparable(dst) else {
                return Ok(true);
            };
            if src_meta.len() != dst_meta.len() {
                return Ok(true);
            }
            src.seek(SeekFrom::Start(0))?;
            let src_hash = checksum::hash_reader(src, ChecksumType::Blake3)
                .context("hashing the opened source")?;
            src.seek(SeekFrom::Start(0))?;
            let dst_hash = checksum::hash_file(dst, ChecksumType::Blake3)
                .with_context(|| format!("hashing dest {}", dst.display()))?;
            let _ = Read::by_ref(src);
            Ok(src_hash != dst_hash)
        }
        ComparisonMode::SizeMtime | ComparisonMode::Unspecified => {
            let Some(dst_meta) = comparable(dst) else {
                return Ok(true);
            };
            if src_meta.len() != dst_meta.len() {
                return Ok(true);
            }
            let src_time = src_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let dst_time = dst_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            Ok(src_time
                .duration_since(dst_time)
                .is_ok_and(|diff| diff.as_secs() > 2))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// cr-ecm12-3: a directory standing where a file is to go is never
    /// "the file, already copied" — whatever size and time the filesystem
    /// gives it. The source here is made to have exactly the directory's
    /// size and its time, so the comparison has nothing else to go by.
    #[test]
    fn a_directory_is_never_the_same_as_the_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Report.txt");
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("inside"), b"x").unwrap();
        let there = std::fs::metadata(&dir).unwrap();
        let src = tmp.path().join("source");
        std::fs::write(&src, vec![b'x'; there.len() as usize]).unwrap();
        filetime::set_file_mtime(
            &src,
            filetime::FileTime::from_last_modification_time(&there),
        )
        .unwrap();
        let here = std::fs::metadata(&src).unwrap();
        assert_eq!(here.len(), there.len(), "the same size");
        assert_eq!(
            here.modified().unwrap(),
            there.modified().unwrap(),
            "the same time"
        );

        for mode in [
            ComparisonMode::SizeMtime,
            ComparisonMode::Unspecified,
            ComparisonMode::SizeOnly,
            ComparisonMode::Checksum,
        ] {
            assert!(
                file_needs_copy_with_mode(&src, &dir, mode).unwrap(),
                "{mode:?}: by path"
            );
            let mut opened = std::fs::File::open(&src).unwrap();
            assert!(
                file_needs_copy_with_mode_opened(&mut opened, &here, &dir, mode).unwrap(),
                "{mode:?}: by the opened source"
            );
        }
        assert!(file_needs_copy(&src, &dir, false).unwrap());
        assert!(file_needs_copy(&src, &dir, true).unwrap());

        // A regular file of that size and time is the same, as ever.
        let copy = tmp.path().join("copy");
        std::fs::copy(&src, &copy).unwrap();
        filetime::set_file_mtime(
            &copy,
            filetime::FileTime::from_last_modification_time(&there),
        )
        .unwrap();
        assert!(!file_needs_copy_with_mode(&src, &copy, ComparisonMode::SizeMtime).unwrap());
        assert!(!file_needs_copy(&src, &copy, true).unwrap());
        // And nothing there needs copying.
        assert!(file_needs_copy(&src, &tmp.path().join("missing"), false).unwrap());
    }
}
