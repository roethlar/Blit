use crate::cli::TransferArgs;
use crate::context::AppContext;
use eyre::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::time::{Duration, Instant};

use blit_core::orchestrator::{
    LocalMirrorOptions, LocalMirrorSummary, TransferOrchestrator, TransferOutcome,
};

use crate::util::format_bytes;

pub async fn run_local_transfer(
    ctx: &AppContext,
    args: &TransferArgs,
    src_path: &Path,
    dest_path: &Path,
    mirror: bool,
) -> Result<()> {
    if !src_path.exists() {
        bail!("source path does not exist: {}", src_path.display());
    }

    let options = build_local_options(ctx, args, mirror)?;
    let dry_run = options.dry_run;
    let null_sink = options.null_sink;
    let json_output = args.json;
    let verbose = options.verbose;
    let debug_mode = options.debug_mode;
    let workers = options.workers;
    if debug_mode {
        eprintln!(
            "[DEBUG] Worker limiter active – FAST planner auto-tuning capped to {workers} thread(s)."
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

    let src_clone = src_path.to_path_buf();
    let dest_clone = dest_path.to_path_buf();
    let start = Instant::now();

    let summary = tokio::task::spawn_blocking(move || {
        let orchestrator = TransferOrchestrator::new();
        orchestrator
            .execute_local_mirror(&src_clone, &dest_clone, options)
            .with_context(|| {
                format!(
                    "failed to {} from {} to {}",
                    if mirror { "mirror" } else { "copy" },
                    src_clone.display(),
                    dest_clone.display()
                )
            })
    })
    .await??;

    if let Some(pb) = progress_bar {
        pb.finish_and_clear();
    }

    let elapsed = start.elapsed();
    if json_output {
        print_summary_json(mirror, &summary, elapsed, src_path, dest_path);
    } else {
        print_summary(
            mirror, dry_run, null_sink, verbose, debug_mode, workers, &summary, elapsed,
        );
    }

    Ok(())
}

fn build_local_options(
    ctx: &AppContext,
    args: &TransferArgs,
    mirror: bool,
) -> Result<LocalMirrorOptions> {
    let mut options = LocalMirrorOptions {
        mirror,
        dry_run: args.dry_run,
        verbose: args.verbose,
        progress: args.effective_progress(),
        perf_history: ctx.perf_history_enabled,
        checksum: args.checksum,
        retries: args.retries,
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
        TransferOutcome::JournalSkip => {
            println!(
                "Up to date: filesystem journal reports no changes{} (in {:.2?})",
                suffix, duration
            );
            return;
        }
        TransferOutcome::UpToDate => {
            println!(
                "Up to date: {} files examined, 0 changed{} (in {:.2?})",
                summary.planned_files, suffix, duration
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
        println!(
            "• Throughput: {}/s | Workers used: {}",
            format_bytes(throughput as u64),
            workers
        );
    }
    if debug_mode {
        println!("• Debug limiter active – worker cap {} thread(s)", workers);
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
        TransferOutcome::JournalSkip => "journal_skip",
        TransferOutcome::UpToDate => "up_to_date",
        TransferOutcome::SourceEmpty => "source_empty",
    };
    let output = json!({
        "operation": if mirror { "mirror" } else { "copy" },
        "source": src.to_string_lossy(),
        "destination": dst.to_string_lossy(),
        "files_transferred": summary.copied_files,
        "files_examined": summary.planned_files,
        "total_bytes": summary.total_bytes,
        "deleted_files": summary.deleted_files,
        "deleted_dirs": summary.deleted_dirs,
        "duration_ms": duration.as_millis() as u64,
        "dry_run": summary.dry_run,
        "outcome": outcome,
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
