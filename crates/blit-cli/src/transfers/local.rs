use crate::cli::TransferArgs;
use crate::context::AppContext;
use blit_app::display::format_bytes;
use blit_core::transfer_session::{LocalMirrorOptions, LocalMirrorSummary, TransferOutcome};
use eyre::{bail, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::time::{Duration, Instant};

/// Convenience wrapper for callers that always want the summary
/// printed inline. Most CLI paths (copy / mirror) want this; move
/// uses [`run_local_transfer_deferred`] so it can suppress the
/// "success" output until after the source-delete decision is
/// made (R49-F3).
pub async fn run_local_transfer(
    ctx: &AppContext,
    args: &TransferArgs,
    src_path: &Path,
    dest_path: &Path,
    mirror: bool,
) -> Result<LocalMirrorSummary> {
    run_local_transfer_inner(ctx, args, src_path, dest_path, mirror, false, false).await
}

/// Same as [`run_local_transfer`] but for the MOVE verb: the caller
/// takes ownership of when (and whether) to print the final summary
/// (a failure during source-delete can then surface without first
/// emitting a successful-looking JSON document on stdout — R49-F3),
/// and the compare maps through the move rule (codex otp-10b-2 F3):
/// transfer unconditionally, or `--checksum` for the one skip that is
/// content-proven safe — a SizeMtime skip of a same-size same-mtime
/// changed file followed by the source-delete would destroy the only
/// copy, the same otp-10a F1 hazard the remote move verbs closed.
pub async fn run_local_transfer_deferred(
    ctx: &AppContext,
    args: &TransferArgs,
    src_path: &Path,
    dest_path: &Path,
    mirror: bool,
) -> Result<LocalMirrorSummary> {
    run_local_transfer_inner(ctx, args, src_path, dest_path, mirror, true, true).await
}

/// Print the standard summary block for a completed local
/// transfer. Exposed for `run_local_transfer_deferred` callers
/// (move) that need to emit output AFTER their own follow-up
/// (source-delete) succeeds. Mirrors the inline print in
/// `run_local_transfer_inner` so deferred + inline callers
/// produce byte-identical output.
pub fn print_local_transfer_summary(
    ctx: &AppContext,
    args: &TransferArgs,
    mirror: bool,
    summary: &LocalMirrorSummary,
    elapsed: Duration,
    src_path: &Path,
    dest_path: &Path,
) -> Result<()> {
    // Only presentation fields are read here; the compare mode (and
    // thus the move_verb flag) is irrelevant to printing.
    let options = build_local_options(ctx, args, mirror, false)?;
    if args.json {
        print_summary_json(mirror, summary, elapsed, src_path, dest_path);
    } else {
        print_summary(
            mirror,
            options.dry_run,
            options.null_sink,
            options.verbose,
            options.debug_mode,
            options.workers,
            summary,
            elapsed,
        );
    }
    Ok(())
}

async fn run_local_transfer_inner(
    ctx: &AppContext,
    args: &TransferArgs,
    src_path: &Path,
    dest_path: &Path,
    mirror: bool,
    defer_output: bool,
    move_verb: bool,
) -> Result<LocalMirrorSummary> {
    if !src_path.exists() {
        bail!("source path does not exist: {}", src_path.display());
    }

    let options = build_local_options(ctx, args, mirror, move_verb)?;
    let dry_run = options.dry_run;
    let null_sink = options.null_sink;
    let json_output = args.json;
    let verbose = options.verbose;
    let debug_mode = options.debug_mode;
    let workers = options.workers;
    if debug_mode {
        eprintln!(
            "blit: debug: worker limiter active – local apply pipeline capped to {workers} worker(s)."
        );
    }

    let progress_bar = if !args.effective_progress() {
        None
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("{spinner} {msg}")
                .unwrap()
                .tick_strings(&["-", "\\", "|", "/"]),
        );
        pb.enable_steady_tick(Duration::from_millis(120));
        pb.set_message(format!(
            "{} {} → {}",
            if mirror { "Mirroring" } else { "Copying" },
            src_path.display(),
            dest_path.display()
        ));
        Some(pb)
    };

    let start = Instant::now();
    let summary = blit_app::transfers::local::run(src_path, dest_path, options).await?;

    if let Some(pb) = progress_bar {
        pb.finish_and_clear();
    }

    let elapsed = start.elapsed();
    if !defer_output {
        if json_output {
            print_summary_json(mirror, &summary, elapsed, src_path, dest_path);
        } else {
            print_summary(
                mirror, dry_run, null_sink, verbose, debug_mode, workers, &summary, elapsed,
            );
        }
    }

    Ok(summary)
}

fn build_local_options(
    ctx: &AppContext,
    args: &TransferArgs,
    mirror: bool,
    move_verb: bool,
) -> Result<LocalMirrorOptions> {
    use blit_core::transfer_session::{LocalCompareMode, LocalMirrorDeleteScope};

    // R58-F7: translate the per-flag CLI args into the unified
    // LocalCompareMode enum. The session then resolves it onto the
    // proper ComparisonMode for the diff_planner. Pre-fix only
    // --checksum was honored; --size-only / --ignore-times /
    // --force were silently dropped.
    //
    // Priority follows the pull-side ordering at
    // pull.rs:538-547: ignore_times > force > size_only >
    // checksum > default. This keeps local and pull behaviorally
    // identical when given the same flag combination.
    //
    // codex otp-10b-2 F3: a MOVE maps through the move rule instead
    // (IgnoreTimes, or Checksum when asked) — the local twin of
    // `blit_app::transfers::compare::move_comparison_mode`. Today the
    // non-mirror local path copies unconditionally regardless of the
    // compare mode (probed live at the F3 adjudication), so this is
    // defense-in-depth; it becomes load-bearing at otp-11, when local
    // transfers ride the session and its diff WOULD skip a same-size
    // same-mtime changed file — which move's source-delete then turns
    // into data loss. Pinned by
    // `local_move_lands_source_bytes_over_same_size_same_mtime_destination`.
    // The metadata flags are rejected on move upstream (R54-F2 gates).
    let compare_mode = if move_verb {
        if args.checksum {
            LocalCompareMode::Checksum
        } else {
            LocalCompareMode::IgnoreTimes
        }
    } else if args.ignore_times {
        LocalCompareMode::IgnoreTimes
    } else if args.force {
        LocalCompareMode::Force
    } else if args.size_only {
        LocalCompareMode::SizeOnly
    } else if args.checksum {
        LocalCompareMode::Checksum
    } else {
        LocalCompareMode::SizeMtime
    };

    // R58-F6: --delete-scope is now plumbed through to local
    // mirror. The CLI exposes `subset` (default — filter scope)
    // and `all`. Pre-fix LocalMirrorOptions had no field for
    // this and apply_mirror_deletions always operated through
    // the user's filter, then failed with ENOTEMPTY on dirs
    // containing excluded contents.
    let delete_scope = if args.delete_scope_all() {
        LocalMirrorDeleteScope::All
    } else {
        LocalMirrorDeleteScope::FilteredSubset
    };

    let mut options = LocalMirrorOptions {
        mirror,
        dry_run: args.dry_run,
        verbose: args.verbose,
        progress: args.effective_progress(),
        perf_history: ctx.perf_history_enabled,
        checksum: args.checksum,
        ignore_existing: args.ignore_existing,
        drop_windows_metadata: args.drop_windows_metadata,
        compare_mode,
        delete_scope,
        resume: args.resume,
        null_sink: args.null,
        filter: super::build_filter(args)?,
        ..LocalMirrorOptions::default()
    };
    if let Some(workers) = args.workers {
        options.workers = workers.max(1);
        options.debug_mode = true;
    }
    Ok(options)
}

/// Threshold below which the `• Throughput / Workers used` line is noise:
/// short transfers (startup-dominated) or single-file copies produce
/// misleading numbers (e.g. "184 B/s" on an NVMe). Keep it for bulk
/// transfers where it's meaningful.
const THROUGHPUT_LINE_MIN_BYTES: u64 = 1024 * 1024; // 1 MiB

fn print_summary(
    mirror: bool,
    dry_run: bool,
    null_sink: bool,
    verbose: bool,
    debug_mode: bool,
    workers: usize,
    summary: &LocalMirrorSummary,
    elapsed: Duration,
) {
    let operation = if mirror { "Mirror" } else { "Copy" };
    let suffix = if dry_run {
        " (dry run)"
    } else if null_sink {
        " (null sink — writes discarded)"
    } else {
        ""
    };
    let duration = if summary.duration.is_zero() {
        elapsed
    } else {
        summary.duration
    };

    // Distinguish the three legitimate zero-files cases from the normal
    // "transferred N files" case. Previously all four printed identically,
    // which masked two classes of bugs (rsync-semantics, single-file noop).
    match summary.outcome {
        TransferOutcome::UpToDate => {
            println!(
                "Up to date: {} files examined, 0 changed{} (in {:.2?})",
                summary.scanned_files, suffix, duration
            );
            return;
        }
        TransferOutcome::SourceEmpty => {
            println!(
                "Source is empty: 0 files copied{} (in {:.2?})",
                suffix, duration
            );
            return;
        }
        TransferOutcome::Transferred => {}
    }

    println!(
        "{}{} complete: {} files, {} in {:.2?}",
        operation,
        suffix,
        summary.copied_files,
        format_bytes(summary.total_bytes),
        duration
    );

    if summary.deleted_files > 0 || summary.deleted_dirs > 0 {
        println!(
            "• Deleted: {} file(s), {} dir(s)",
            summary.deleted_files, summary.deleted_dirs
        );
    }

    // Suppress throughput/workers noise on small transfers where startup
    // dominates wall time and the numbers are meaningless. Keep it for
    // bulk transfers where it's actually informative.
    let show_throughput =
        verbose || summary.total_bytes >= THROUGHPUT_LINE_MIN_BYTES || summary.copied_files > 1;
    if show_throughput {
        let throughput = if duration.as_secs_f64() > 0.0 {
            summary.total_bytes as f64 / duration.as_secs_f64()
        } else {
            0.0
        };
        // codex otp-11b B4: the session's apply pipeline runs one sink
        // worker unless the hidden debug limiter widened it — print
        // the EFFECTIVE count, not the options default (num_cpus).
        println!(
            "• Throughput: {}/s | Workers used: {}",
            format_bytes(throughput as u64),
            if debug_mode { workers } else { 1 }
        );
    }
    if debug_mode {
        println!("• Debug limiter active – worker cap {} worker(s)", workers);
    }

    if verbose {
        println!(
            "• Planned {} file(s), total bytes {}",
            summary.planned_files,
            format_bytes(summary.total_bytes)
        );
        if summary.tar_shard_tasks > 0 || summary.raw_bundle_tasks > 0 || summary.large_tasks > 0 {
            println!(
                "• Planner mix: {} tar shard(s) [{} file(s), {}], {} bundle(s) [{} file(s), {}], {} large task(s) [{}]",
                summary.tar_shard_tasks,
                summary.tar_shard_files,
                format_bytes(summary.tar_shard_bytes),
                summary.raw_bundle_tasks,
                summary.raw_bundle_files,
                format_bytes(summary.raw_bundle_bytes),
                summary.large_tasks,
                format_bytes(summary.large_bytes),
            );
        }
    }
}

fn print_summary_json(
    mirror: bool,
    summary: &LocalMirrorSummary,
    elapsed: Duration,
    src: &Path,
    dst: &Path,
) {
    use serde_json::json;
    let duration = if summary.duration.is_zero() {
        elapsed
    } else {
        summary.duration
    };
    let outcome = match summary.outcome {
        TransferOutcome::Transferred => "transferred",
        TransferOutcome::UpToDate => "up_to_date",
        TransferOutcome::SourceEmpty => "source_empty",
    };
    let output = json!({
        "operation": if mirror { "mirror" } else { "copy" },
        "source": src.to_string_lossy(),
        "destination": dst.to_string_lossy(),
        "files_transferred": summary.copied_files,
        "files_examined": summary.scanned_files,
        "total_bytes": summary.total_bytes,
        "deleted_files": summary.deleted_files,
        "deleted_dirs": summary.deleted_dirs,
        "duration_ms": duration.as_millis() as u64,
        "dry_run": summary.dry_run,
        "outcome": outcome,
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
