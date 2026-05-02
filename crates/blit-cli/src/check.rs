//! `blit check` — read-only tree comparison.
//!
//! Walks both source and destination trees with the same `FileFilter` the
//! transfer commands use, then classifies each file:
//!   - matching        : same relative path, same size+mtime (or hash)
//!   - differing       : same relative path, content differs
//!   - missing-on-dest : source has it, destination doesn't
//!   - missing-on-src  : destination has it, source doesn't (skipped with --one-way)
//!   - errors          : I/O failure during comparison
//!
//! Exit code: 0 = identical, 1 = differences found, 2 = errors.
//!
//! Filter handling goes through the same `transfers::build_filter_from_inputs`
//! helper used by copy/mirror/move so behavior is uniform across commands.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::UNIX_EPOCH;

use eyre::{bail, Context, Result};
use serde::Serialize;

use blit_core::checksum::{hash_file, ChecksumType};
use blit_core::enumeration::{EntryKind, EnumeratedEntry, FileEnumerator};
use blit_core::fs_enum::FileFilter;

use crate::cli::CheckArgs;
use crate::transfers::{build_filter_from_inputs, FilterInputs};
use crate::util::format_bytes;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DiffEntry {
    pub(crate) path: String,
    pub(crate) reason: String,
    pub(crate) src_size: u64,
    pub(crate) dst_size: u64,
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct CheckResult {
    pub(crate) matching: usize,
    pub(crate) differing: Vec<DiffEntry>,
    pub(crate) missing_on_src: Vec<String>,
    pub(crate) missing_on_dest: Vec<String>,
    pub(crate) errors: Vec<(String, String)>,
}

pub async fn run_check(args: &CheckArgs) -> Result<ExitCode> {
    let src = PathBuf::from(&args.source);
    let dst = PathBuf::from(&args.destination);
    if !src.exists() {
        bail!("source path does not exist: {}", src.display());
    }
    if !dst.exists() {
        bail!("destination path does not exist: {}", dst.display());
    }

    // Build filter via the same chokepoint that copy/mirror/move use, so
    // `blit check --exclude '*.tmp'` matches `blit copy --exclude '*.tmp'`.
    let filter = build_filter_from_inputs(&FilterInputs {
        include: &args.include,
        exclude: &args.exclude,
        files_from: args.files_from.as_ref(),
        min_size: args.min_size.as_deref(),
        max_size: args.max_size.as_deref(),
        min_age: args.min_age.as_deref(),
        max_age: args.max_age.as_deref(),
    })?;

    let use_checksum = args.checksum;
    let one_way = args.one_way;
    let json = args.json;

    let result = tokio::task::spawn_blocking(move || {
        compare_trees(&src, &dst, use_checksum, one_way, filter)
    })
    .await
    .context("check task panicked")??;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print_result(&result, one_way);
    }

    if !result.errors.is_empty() {
        Ok(ExitCode::from(2))
    } else if !result.differing.is_empty()
        || !result.missing_on_dest.is_empty()
        || (!one_way && !result.missing_on_src.is_empty())
    {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

pub(crate) fn compare_trees(
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

fn print_result(result: &CheckResult, one_way: bool) {
    let total_diffs = result.differing.len()
        + result.missing_on_dest.len()
        + if one_way {
            0
        } else {
            result.missing_on_src.len()
        };

    if total_diffs == 0 && result.errors.is_empty() {
        println!(
            "Check complete: {} files match, no differences found.",
            result.matching
        );
        return;
    }

    println!(
        "Check complete: {} matching, {} difference(s) found.",
        result.matching, total_diffs
    );

    if !result.differing.is_empty() {
        println!("\nDiffering ({}):", result.differing.len());
        for entry in &result.differing {
            println!("  * {} ({})", entry.path, entry.reason);
        }
    }
    if !result.missing_on_dest.is_empty() {
        println!(
            "\nMissing on destination ({}):",
            result.missing_on_dest.len()
        );
        for path in &result.missing_on_dest {
            println!("  + {path}");
        }
    }
    if !one_way && !result.missing_on_src.is_empty() {
        println!("\nMissing on source ({}):", result.missing_on_src.len());
        for path in &result.missing_on_src {
            println!("  - {path}");
        }
    }
    if !result.errors.is_empty() {
        println!("\nErrors ({}):", result.errors.len());
        for (path, err) in &result.errors {
            println!("  ! {path}: {err}");
        }
    }
}

#[cfg(unix)]
#[cfg(test)]
mod equivalence_tests {
    //! F12 regression tests pinning the documented `blit check`
    //! equivalence model. These don't go through clap; they call
    //! `compare_trees` directly so they're fast and deterministic.

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
