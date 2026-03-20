//! Integrity verification: compare source and destination trees.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process;
use std::time::UNIX_EPOCH;

use eyre::{bail, Context, Result};
use serde::Serialize;

use blit_core::checksum::{hash_file, ChecksumType};
use blit_core::enumeration::{EntryKind, EnumeratedEntry, FileEnumerator};
use blit_core::filter::{parse_duration, parse_size, FilterRules};
use blit_core::fs_enum::FileFilter;

use crate::cli::CheckArgs;
use crate::transfers::format_bytes;

#[derive(Debug, Clone, Serialize)]
struct DiffEntry {
    path: String,
    reason: String,
    src_size: u64,
    dst_size: u64,
}

#[derive(Debug, Default, Serialize)]
struct CheckResult {
    matching: usize,
    differing: Vec<DiffEntry>,
    missing_on_src: Vec<String>,
    missing_on_dest: Vec<String>,
    errors: Vec<(String, String)>,
}

pub async fn run_check(args: &CheckArgs) -> Result<()> {
    let src_path = PathBuf::from(&args.source);
    let dst_path = PathBuf::from(&args.destination);

    if !src_path.exists() {
        bail!("source path does not exist: {}", src_path.display());
    }
    if !dst_path.exists() {
        bail!("destination path does not exist: {}", dst_path.display());
    }

    let filter_rules = build_filter_rules(args)?;
    let use_checksum = args.checksum;
    let one_way = args.one_way;
    let json = args.json;

    let result = tokio::task::spawn_blocking(move || {
        compare_trees(&src_path, &dst_path, use_checksum, one_way, filter_rules)
    })
    .await??;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print_result(&result, one_way);
    }

    // Exit code: 0=identical, 1=differences, 2=errors
    if !result.errors.is_empty() {
        process::exit(2);
    }
    if !result.differing.is_empty()
        || !result.missing_on_dest.is_empty()
        || (!one_way && !result.missing_on_src.is_empty())
    {
        process::exit(1);
    }

    Ok(())
}

fn compare_trees(
    src_root: &Path,
    dst_root: &Path,
    use_checksum: bool,
    one_way: bool,
    filter_rules: Option<FilterRules>,
) -> Result<CheckResult> {
    let filter = FileFilter::default();
    let mut enumerator = FileEnumerator::new(filter);
    if let Some(rules) = filter_rules {
        enumerator = enumerator.with_filter_rules(rules);
    }

    let src_entries = enumerator.enumerate_local(src_root)?;
    let dst_entries = enumerator.enumerate_local(dst_root)?;

    // Build maps by relative path
    let src_map: HashMap<PathBuf, &EnumeratedEntry> = src_entries
        .iter()
        .map(|e| (e.relative_path.clone(), e))
        .collect();
    let dst_map: HashMap<PathBuf, &EnumeratedEntry> = dst_entries
        .iter()
        .map(|e| (e.relative_path.clone(), e))
        .collect();

    let mut result = CheckResult::default();

    // Check all source entries against destination
    for src_entry in &src_entries {
        let rel = &src_entry.relative_path;

        // Skip directories for comparison
        if matches!(src_entry.kind, EntryKind::Directory) {
            continue;
        }

        let src_size = match &src_entry.kind {
            EntryKind::File { size } => *size,
            _ => continue,
        };

        match dst_map.get(rel) {
            None => {
                result
                    .missing_on_dest
                    .push(rel.to_string_lossy().into_owned());
            }
            Some(dst_entry) => {
                let dst_size = match &dst_entry.kind {
                    EntryKind::File { size } => *size,
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
                        Ok(false) => {
                            result.differing.push(DiffEntry {
                                path: rel.to_string_lossy().into_owned(),
                                reason: "hash mismatch".into(),
                                src_size,
                                dst_size,
                            });
                        }
                        Err(e) => {
                            result.errors.push((
                                rel.to_string_lossy().into_owned(),
                                format!("{e:#}"),
                            ));
                        }
                    }
                } else {
                    // Compare by mtime
                    let src_mtime = src_entry
                        .metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_secs());
                    let dst_mtime = dst_entry
                        .metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_secs());

                    match (src_mtime, dst_mtime) {
                        (Some(s), Some(d)) if s.abs_diff(d) <= 2 => {
                            result.matching += 1;
                        }
                        _ => {
                            result.differing.push(DiffEntry {
                                path: rel.to_string_lossy().into_owned(),
                                reason: "mtime differs".into(),
                                src_size,
                                dst_size,
                            });
                        }
                    }
                }
            }
        }
    }

    // Check for entries only in destination (unless one-way)
    if !one_way {
        for dst_entry in &dst_entries {
            if matches!(dst_entry.kind, EntryKind::Directory) {
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

fn compare_hashes(src: &Path, dst: &Path) -> Result<bool> {
    let src_hash = hash_file(src, ChecksumType::Blake3)
        .with_context(|| format!("hashing {}", src.display()))?;
    let dst_hash = hash_file(dst, ChecksumType::Blake3)
        .with_context(|| format!("hashing {}", dst.display()))?;
    Ok(src_hash == dst_hash)
}

fn print_result(result: &CheckResult, one_way: bool) {
    let total_differences = result.differing.len()
        + result.missing_on_dest.len()
        + if one_way {
            0
        } else {
            result.missing_on_src.len()
        };

    if total_differences == 0 && result.errors.is_empty() {
        println!(
            "Check complete: {} files match, no differences found.",
            result.matching
        );
        return;
    }

    println!(
        "Check complete: {} matching, {} difference(s) found.",
        result.matching, total_differences
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

fn build_filter_rules(args: &CheckArgs) -> Result<Option<FilterRules>> {
    let mut rules = FilterRules::new();

    for pattern in &args.include {
        rules = rules.include(pattern)?;
    }
    for pattern in &args.exclude {
        rules = rules.exclude(pattern)?;
    }

    if let Some(ref s) = args.min_size {
        rules = rules.min_size(parse_size(s)?);
    }
    if let Some(ref s) = args.max_size {
        rules = rules.max_size(parse_size(s)?);
    }
    if let Some(ref s) = args.min_age {
        rules = rules.min_age(parse_duration(s)?);
    }
    if let Some(ref s) = args.max_age {
        rules = rules.max_age(parse_duration(s)?);
    }

    if rules.is_empty() {
        Ok(None)
    } else {
        Ok(Some(rules))
    }
}
