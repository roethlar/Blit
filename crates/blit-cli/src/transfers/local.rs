use crate::cli::TransferArgs;
use crate::context::AppContext;
use eyre::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::time::{Duration, Instant};

use blit_core::orchestrator::{LocalMirrorOptions, LocalMirrorSummary, TransferOrchestrator};

use super::format_bytes;

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

    let options = build_local_options(ctx, args, mirror);
    let dry_run = options.dry_run;
    let verbose = options.verbose;
    let debug_mode = options.debug_mode;
    let workers = options.workers;
    if debug_mode {
        eprintln!(
            "[DEBUG] Worker limiter active – FAST planner auto-tuning capped to {workers} thread(s)."
        );
    }

    let progress_bar = if !args.progress {
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
    print_summary(
        mirror, dry_run, verbose, debug_mode, workers, &summary, elapsed,
    );

    Ok(())
}

fn build_local_options(ctx: &AppContext, args: &TransferArgs, mirror: bool) -> LocalMirrorOptions {
    let mut options = LocalMirrorOptions {
        mirror,
        dry_run: args.dry_run,
        verbose: args.verbose,
        progress: args.progress,
        perf_history: ctx.perf_history_enabled,
        checksum: args.checksum,
        ..LocalMirrorOptions::default()
    };
    if let Some(workers) = args.workers {
        options.workers = workers.max(1);
        options.debug_mode = true;
    }
    options
}

fn print_summary(
    mirror: bool,
    dry_run: bool,
    verbose: bool,
    debug_mode: bool,
    workers: usize,
    summary: &LocalMirrorSummary,
    elapsed: Duration,
) {
    let operation = if mirror { "Mirror" } else { "Copy" };
    let duration = if summary.duration.is_zero() {
        elapsed
    } else {
        summary.duration
    };

    let throughput = if duration.as_secs_f64() > 0.0 {
        summary.total_bytes as f64 / duration.as_secs_f64()
    } else {
        0.0
    };

    println!(
        "{}{} complete: {} files, {} in {:.2?}",
        operation,
        if dry_run { " (dry run)" } else { "" },
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

    println!(
        "• Throughput: {}/s | Workers used: {}",
        format_bytes(throughput as u64),
        workers
    );
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
