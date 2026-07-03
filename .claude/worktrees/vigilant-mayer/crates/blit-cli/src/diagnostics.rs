use crate::cli::PerfArgs;
use crate::context::AppContext;
use blit_core::perf_history;
use chrono::{DateTime, Utc};
use eyre::Result;
use std::time::{Duration, UNIX_EPOCH};

pub fn run_diagnostics_perf(ctx: &mut AppContext, args: &PerfArgs) -> Result<()> {
    if args.enable {
        perf_history::set_perf_history_enabled(true)?;
        ctx.perf_history_enabled = true;
        println!("Performance history enabled (persisted).");
    }

    if args.disable {
        perf_history::set_perf_history_enabled(false)?;
        ctx.perf_history_enabled = false;
        println!("Performance history disabled (persisted).");
    }

    if args.clear {
        match perf_history::clear_history()? {
            true => println!("Cleared performance history log."),
            false => println!("No performance history log to clear."),
        }
    }

    // Refresh status from disk in case multiple toggles happened earlier.
    if let Ok(enabled) = perf_history::perf_history_enabled() {
        ctx.perf_history_enabled = enabled;
    }

    let history_path = perf_history::config_dir()?.join("perf_local.jsonl");
    let records = perf_history::read_recent_records(args.limit)?;

    println!(
        "Performance history (showing up to {} entries): {}",
        args.limit,
        records.len()
    );
    println!("History file: {}", history_path.display());
    println!(
        "Status: {}",
        if ctx.perf_history_enabled {
            if records.is_empty() {
                "enabled (no entries yet)"
            } else {
                "enabled"
            }
        } else {
            "disabled via CLI settings"
        }
    );

    if records.is_empty() {
        return Ok(());
    }

    let total_runs = records.len();
    let total_runs_f64 = total_runs as f64;
    let avg_planner = records
        .iter()
        .map(|r| r.planner_duration_ms as f64)
        .sum::<f64>()
        / total_runs_f64;
    let avg_transfer = records
        .iter()
        .map(|r| r.transfer_duration_ms as f64)
        .sum::<f64>()
        / total_runs_f64;
    let fast_path_runs = records.iter().filter(|r| r.fast_path.is_some()).count();
    let fast_pct = if total_runs == 0 {
        0.0
    } else {
        100.0 * fast_path_runs as f64 / total_runs_f64
    };

    println!(
        "Fast-path runs: {} ({:.1}%), streaming runs: {}",
        fast_path_runs,
        fast_pct,
        total_runs - fast_path_runs
    );
    println!(
        "Average planner: {:.1} ms | Average transfer: {:.1} ms",
        avg_planner, avg_transfer
    );

    if let Some(last) = records.last() {
        let millis = last.timestamp_epoch_ms.min(u64::MAX as u128) as u64;
        let timestamp = DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_millis(millis));
        let mode = match last.mode {
            perf_history::TransferMode::Copy => "copy",
            perf_history::TransferMode::Mirror => "mirror",
        };
        let fast_path_label = last.fast_path.as_deref().unwrap_or("streaming");

        println!("Most recent run:");
        println!(
            "  Timestamp : {}",
            timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!("  Mode      : {}", mode);
        println!("  Fast path : {}", fast_path_label);
        println!(
            "  Planned   : {} file(s), {} bytes",
            last.file_count, last.total_bytes
        );
        println!(
            "  Planner   : {} ms | Transfer: {} ms",
            last.planner_duration_ms, last.transfer_duration_ms
        );
        println!(
            "  Options   : checksum={} skip_unchanged={} workers={}",
            last.options.checksum, last.options.skip_unchanged, last.options.workers
        );
        if let Some(fs) = &last.source_fs {
            println!("  Source FS : {}", fs);
        }
        if let Some(fs) = &last.dest_fs {
            println!("  Dest FS   : {}", fs);
        }
    }

    Ok(())
}
