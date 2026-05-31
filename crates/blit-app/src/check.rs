//! `blit check` — read-only tree comparison.
//!
//! Walks both source and destination trees with the same
//! `FileFilter` the transfer commands use, then classifies each
//! file:
//!   - matching        : same relative path, same size+mtime (or hash)
//!   - differing       : same relative path, content differs
//!   - missing-on-dest : source has it, destination doesn't
//!   - missing-on-src  : destination has it, source doesn't (skipped with `one_way`)
//!   - errors          : I/O failure during comparison
//!
//! Moved from `crates/blit-cli/src/check.rs` in A.0. Filter
//! construction (`build_filter_from_inputs` etc.) still lives in
//! `crate::transfers::mod` in the CLI; the CLI builds the
//! `FileFilter` and hands it to `compare_trees`. Tests that
//! exercise the algorithm directly travel with the function.
//!
//! `compare_trees` is `pub` (was `pub(crate)`) because it's now
//! consumed across a crate boundary. The TUI's future F4 Verify
//! pane will consume the same function with the same shape.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use blit_core::checksum::{hash_file, ChecksumType};
use blit_core::enumeration::{EntryKind, EnumeratedEntry, FileEnumerator};
use blit_core::fs_enum::FileFilter;
use eyre::{Context, Result};
use serde::Serialize;

use crate::display::format_bytes;

/// One source/destination file that differs. `reason` is the
/// human-readable explanation — pre-A.0 the CLI's `--json`
/// output emitted this verbatim; A.0 preserves the string shape
/// (size strings use the same `format_bytes` formatter).
#[derive(Debug, Clone, Serialize)]
pub struct DiffEntry {
    pub path: String,
    pub reason: String,
    pub src_size: u64,
    pub dst_size: u64,
}

/// Aggregated result of a `compare_trees` call.
#[derive(Debug, Default, Serialize)]
pub struct CheckResult {
    pub matching: usize,
    pub differing: Vec<DiffEntry>,
    pub missing_on_src: Vec<String>,
    pub missing_on_dest: Vec<String>,
    pub errors: Vec<(String, String)>,
}

/// Compare two local trees with the supplied filter. Synchronous;
/// callers that want to keep the async runtime free should drop
/// this on `spawn_blocking` (the CLI does).
pub fn compare_trees(
    src_root: &Path,
    dst_root: &Path,
    use_checksum: bool,
    one_way: bool,
    filter: FileFilter,
) -> Result<CheckResult> {
    let enumerator = FileEnumerator::new(filter);

    let src_entries = enumerator
        .enumerate_local(src_root)
        .with_context(|| format!("enumerate source {}", src_root.display()))?;
    let dst_entries = enumerator
        .enumerate_local(dst_root)
        .with_context(|| format!("enumerate destination {}", dst_root.display()))?;

    let src_map: HashMap<PathBuf, &EnumeratedEntry> = src_entries
        .iter()
        .map(|e| (e.relative_path.clone(), e))
        .collect();
    let dst_map: HashMap<PathBuf, &EnumeratedEntry> = dst_entries
        .iter()
        .map(|e| (e.relative_path.clone(), e))
        .collect();

    let mut result = CheckResult::default();

    for src_entry in &src_entries {
        if !matches!(src_entry.kind, EntryKind::File { .. }) {
            continue;
        }
        let rel = &src_entry.relative_path;
        let src_size = match src_entry.kind {
            EntryKind::File { size } => size,
            _ => continue,
        };

        match dst_map.get(rel) {
            None => {
                result
                    .missing_on_dest
                    .push(rel.to_string_lossy().into_owned());
            }
            Some(dst_entry) => {
                let dst_size = match dst_entry.kind {
                    EntryKind::File { size } => size,
                    _ => {
                        result.differing.push(DiffEntry {
                            path: rel.to_string_lossy().into_owned(),
                            reason: "type mismatch".into(),
                            src_size,
                            dst_size: 0,
                        });
                        continue;
                    }
                };

                if src_size != dst_size {
                    result.differing.push(DiffEntry {
                        path: rel.to_string_lossy().into_owned(),
                        reason: format!(
                            "size ({} vs {})",
                            format_bytes(src_size),
                            format_bytes(dst_size)
                        ),
                        src_size,
                        dst_size,
                    });
                    continue;
                }

                if use_checksum {
                    match compare_hashes(&src_entry.absolute_path, &dst_entry.absolute_path) {
                        Ok(true) => result.matching += 1,
                        Ok(false) => result.differing.push(DiffEntry {
                            path: rel.to_string_lossy().into_owned(),
                            reason: "hash mismatch".into(),
                            src_size,
                            dst_size,
                        }),
                        Err(e) => result
                            .errors
                            .push((rel.to_string_lossy().into_owned(), format!("{e:#}"))),
                    }
                } else {
                    let src_mtime = mtime_secs(&src_entry.metadata);
                    let dst_mtime = mtime_secs(&dst_entry.metadata);
                    match (src_mtime, dst_mtime) {
                        (Some(s), Some(d)) if s.abs_diff(d) <= 2 => result.matching += 1,
                        _ => result.differing.push(DiffEntry {
                            path: rel.to_string_lossy().into_owned(),
                            reason: "mtime differs".into(),
                            src_size,
                            dst_size,
                        }),
                    }
                }
            }
        }
    }

    if !one_way {
        for dst_entry in &dst_entries {
            if !matches!(dst_entry.kind, EntryKind::File { .. }) {
                continue;
            }
            if !src_map.contains_key(&dst_entry.relative_path) {
                result
                    .missing_on_src
                    .push(dst_entry.relative_path.to_string_lossy().into_owned());
            }
        }
    }

    Ok(result)
}

fn mtime_secs(metadata: &std::fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

fn compare_hashes(src: &Path, dst: &Path) -> Result<bool> {
    let s = hash_file(src, ChecksumType::Blake3)
        .with_context(|| format!("hashing {}", src.display()))?;
    let d = hash_file(dst, ChecksumType::Blake3)
        .with_context(|| format!("hashing {}", dst.display()))?;
    Ok(s == d)
}

#[cfg(unix)]
#[cfg(test)]
mod equivalence_tests {
    //! F12 regression tests pinning the documented `blit check`
    //! equivalence model. These don't go through clap; they call
    //! `compare_trees` directly so they're fast and deterministic.
    //! Moved from `blit-cli/src/check.rs` in A.0 alongside the
    //! function they exercise.

    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    fn write(path: &Path, body: &[u8]) {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn matching_files_report_zero_diffs() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write(&src.join("a.txt"), b"hello");
        write(&dst.join("a.txt"), b"hello");
        // Mtime sync so size+mtime equality holds.
        let mtime = std::fs::metadata(src.join("a.txt"))
            .unwrap()
            .modified()
            .unwrap();
        filetime::set_file_mtime(
            dst.join("a.txt"),
            filetime::FileTime::from_system_time(mtime),
        )
        .unwrap();

        let result = compare_trees(&src, &dst, false, false, FileFilter::default()).unwrap();
        assert_eq!(result.matching, 1);
        assert!(result.differing.is_empty());
        assert!(result.missing_on_dest.is_empty());
        assert!(result.missing_on_src.is_empty());
    }

    #[test]
    fn empty_directories_are_not_part_of_equivalence() {
        // Source has only an empty directory; destination is empty.
        // Equivalence model: matches transfer behavior, which doesn't
        // replicate empty directories. Result: zero diffs.
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(src.join("empty_dir")).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        let result = compare_trees(&src, &dst, false, false, FileFilter::default()).unwrap();
        assert_eq!(result.matching, 0);
        assert!(
            result.differing.is_empty()
                && result.missing_on_dest.is_empty()
                && result.missing_on_src.is_empty(),
            "empty dirs must not produce diff entries: {:#?}",
            result
        );
    }

    #[test]
    fn symlinks_are_skipped_silently() {
        // src has a symlink, dst doesn't (or vice versa). The
        // equivalence model ignores them — no diff entry produced.
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        // Symlink target doesn't need to exist for this test.
        symlink("nowhere", src.join("link")).unwrap();
        let result = compare_trees(&src, &dst, false, false, FileFilter::default()).unwrap();
        assert!(
            result.differing.is_empty() && result.missing_on_dest.is_empty(),
            "symlink-only difference must report identical: {:#?}",
            result
        );
    }

    #[test]
    fn file_vs_directory_at_same_path_diffs_on_file_side() {
        // src has a regular file at "x"; dst has a directory at "x".
        // The diff entry is keyed on the file side (only files
        // populate the equivalence model).
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write(&src.join("x"), b"file");
        std::fs::create_dir_all(dst.join("x")).unwrap();
        let result = compare_trees(&src, &dst, false, false, FileFilter::default()).unwrap();
        assert_eq!(
            result.differing.len(),
            1,
            "expected one diff: {:#?}",
            result
        );
        assert_eq!(result.differing[0].path, "x");
        assert!(result.differing[0].reason.contains("type mismatch"));
    }

    #[test]
    fn missing_on_dest_reported() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write(&src.join("only-here.txt"), b"x");
        std::fs::create_dir_all(&dst).unwrap();
        let result = compare_trees(&src, &dst, false, false, FileFilter::default()).unwrap();
        assert_eq!(result.missing_on_dest.len(), 1);
        assert_eq!(result.missing_on_dest[0], "only-here.txt");
    }

    #[test]
    fn one_way_ignores_extras_on_dest() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write(&src.join("a.txt"), b"x");
        let mtime = std::fs::metadata(src.join("a.txt"))
            .unwrap()
            .modified()
            .unwrap();
        write(&dst.join("a.txt"), b"x");
        write(&dst.join("extra.txt"), b"y");
        filetime::set_file_mtime(
            dst.join("a.txt"),
            filetime::FileTime::from_system_time(mtime),
        )
        .unwrap();

        let one_way = compare_trees(&src, &dst, false, true, FileFilter::default()).unwrap();
        assert_eq!(one_way.matching, 1);
        // missing_on_src is populated by compare_trees regardless,
        // but the print/exit logic ignores it under one_way.
        assert!(one_way.differing.is_empty());
        assert!(one_way.missing_on_dest.is_empty());
    }
}
