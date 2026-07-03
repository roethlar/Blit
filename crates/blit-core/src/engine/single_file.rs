//! Single-file copy strategy. Moved from
//! `orchestrator/orchestrator.rs` at ue-r2-1c; the same slice later
//! adds the perf-history/predictor accounting this path lacked
//! (REV4 Design §2).

use std::path::{Path, PathBuf};
use std::time::Instant;

use eyre::{Context, Result};

use crate::generated::ComparisonMode;

use super::options::LocalMirrorOptions;
use super::summary::{LocalMirrorSummary, TransferOutcome};

/// Copy a single file source directly to `dest_root`, bypassing the
/// enumerator/planner/pipeline machinery which assumes `src_root` is a
/// directory. The CLI's destination resolver has already produced the final
/// target path, so this is a simple `copy_file` call.
pub(super) fn execute_single_file_copy(
    src_root: &Path,
    dest_root: &Path,
    options: &LocalMirrorOptions,
    start_time: Instant,
) -> Result<LocalMirrorSummary> {
    use crate::buffer::BufferSizer;
    use crate::copy::{copy_file, file_needs_copy_with_mode, resume_copy_file};
    use crate::logger::NoopLogger;
    use filetime::FileTime;

    let src_meta = std::fs::metadata(src_root)
        .with_context(|| format!("stat source file {}", src_root.display()))?;
    let size = src_meta.len();

    // R58-followup: route compare-mode for the single-file path
    // through the same translation the directory path uses
    // (orchestrator.rs:481). Pre-fix the short-circuit only looked
    // at `options.checksum`, so `--size-only` / `--ignore-times` /
    // `--force` were silently dropped — repro: copy src.txt dst.txt
    // --size-only re-copied even when sizes matched.
    let compare_mode = match options.compare_mode {
        crate::orchestrator::LocalCompareMode::Checksum => ComparisonMode::Checksum,
        crate::orchestrator::LocalCompareMode::SizeOnly => ComparisonMode::SizeOnly,
        crate::orchestrator::LocalCompareMode::Force => ComparisonMode::Force,
        crate::orchestrator::LocalCompareMode::IgnoreTimes => ComparisonMode::IgnoreTimes,
        crate::orchestrator::LocalCompareMode::SizeMtime => {
            if options.checksum {
                ComparisonMode::Checksum
            } else {
                ComparisonMode::SizeMtime
            }
        }
    };

    // R58-F5: the single-file strategy (engine dispatch)
    // bypasses the enumerator + planner, which is where the
    // streaming-pipeline path checks filter / ignore_existing.
    // Apply both here so single-file copies honor the same
    // CLI contract.
    //
    // Filter: the source root is itself the only entry. Run
    // `filter.allows_entry` against the source name. If excluded,
    // return a "scanned 1 / copied 0" summary so the user sees
    // "no work performed" rather than the file being copied
    // anyway.
    let src_name = src_root.file_name().map(PathBuf::from);
    let allows = match src_name {
        Some(name) => {
            let mtime = src_meta.modified().ok();
            options
                .filter
                .allows_entry(Some(&name), src_root, size, mtime)
        }
        None => true,
    };
    if !allows {
        return Ok(LocalMirrorSummary {
            planned_files: 0,
            copied_files: 0,
            total_bytes: 0,
            scanned_files: 1,
            scanned_bytes: size,
            duration: start_time.elapsed(),
            outcome: TransferOutcome::UpToDate,
            ..Default::default()
        });
    }

    // ignore_existing: if the destination file already exists,
    // skip the copy entirely. Matches the diff_planner behavior
    // for the streaming-pipeline path (diff_planner.rs).
    if options.ignore_existing && dest_root.exists() {
        return Ok(LocalMirrorSummary {
            planned_files: 0,
            copied_files: 0,
            total_bytes: 0,
            scanned_files: 1,
            scanned_bytes: size,
            duration: start_time.elapsed(),
            outcome: TransferOutcome::UpToDate,
            ..Default::default()
        });
    }

    if options.dry_run {
        return Ok(LocalMirrorSummary {
            planned_files: 1,
            copied_files: 1,
            total_bytes: size,
            scanned_files: 1,
            scanned_bytes: size,
            dry_run: true,
            duration: start_time.elapsed(),
            ..Default::default()
        });
    }

    if options.null_sink {
        return Ok(LocalMirrorSummary {
            planned_files: 1,
            copied_files: 1,
            total_bytes: size,
            scanned_files: 1,
            scanned_bytes: size,
            duration: start_time.elapsed(),
            ..Default::default()
        });
    }

    let mut did_copy = false;
    let mut clone_succeeded = false;
    let mut bytes_copied = 0u64;

    if options.resume {
        let outcome = resume_copy_file(src_root, dest_root, 0)
            .with_context(|| format!("resume copy {}", src_root.display()))?;
        did_copy = outcome.bytes_transferred > 0;
        bytes_copied = outcome.bytes_transferred;
    } else {
        let needs_copy = !options.skip_unchanged
            || file_needs_copy_with_mode(src_root, dest_root, compare_mode).unwrap_or(true);
        if needs_copy {
            let sizer = BufferSizer::default();
            let logger = NoopLogger;
            let outcome = copy_file(src_root, dest_root, &sizer, false, &logger)
                .with_context(|| format!("copy {}", src_root.display()))?;
            did_copy = true;
            clone_succeeded = outcome.clone_succeeded;
            bytes_copied = outcome.bytes_copied;
        }
    }

    if options.preserve_times && did_copy && !clone_succeeded {
        if let Ok(modified) = src_meta.modified() {
            let ft = FileTime::from_system_time(modified);
            // R42-F1: warn-don't-silence (was `let _ = ...`).
            if let Err(e) = filetime::set_file_mtime(dest_root, ft) {
                log::warn!("set mtime on {}: {}", dest_root.display(), e);
            }
        }
    }

    Ok(LocalMirrorSummary {
        planned_files: 1,
        copied_files: if did_copy { 1 } else { 0 },
        total_bytes: bytes_copied,
        // Single-file path always saw exactly one entry of `size`
        // bytes; whether we copied it or not is the
        // copied_files/total_bytes story, but the scan saw it.
        scanned_files: 1,
        scanned_bytes: size,
        duration: start_time.elapsed(),
        outcome: if did_copy {
            TransferOutcome::Transferred
        } else {
            TransferOutcome::UpToDate
        },
        ..Default::default()
    })
}
