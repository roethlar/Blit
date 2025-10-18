use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand};
use std::time::{Duration, UNIX_EPOCH};

#[derive(Parser)]
#[command(name = "blit")]
#[command(about = "A fast, AI-built file transfer tool (v2)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Push files to a remote server
    Push { source: String, destination: String },
    /// Pull files from a remote server
    Pull { source: String, destination: String },
    /// Mirror a directory to a remote server
    Mirror { source: String, destination: String },
    /// List contents of a remote directory
    Ls { path: String },
    /// Diagnostics and tooling commands
    Diagnostics {
        #[command(subcommand)]
        command: DiagnosticsCommand,
    },
}

#[derive(Subcommand)]
enum DiagnosticsCommand {
    /// Show recent performance history captured locally
    Perf(PerfArgs),
}

#[derive(Args)]
struct PerfArgs {
    /// Number of recent records to display (0 = all)
    #[arg(long, default_value_t = 50)]
    limit: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Push {
            source,
            destination,
        } => {
            println!("Pushing from {} to {}", source, destination);
            // To be implemented in Phase 2
        }
        Commands::Pull {
            source,
            destination,
        } => {
            println!("Pulling from {} to {}", source, destination);
            // To be implemented in Phase 3
        }
        Commands::Mirror {
            source,
            destination,
        } => {
            println!("Mirroring from {} to {}", source, destination);
            // To be implemented in Phase 2
        }
        Commands::Ls { path } => {
            println!("Listing contents of {}", path);
            // To be implemented in Phase 3
        }
        Commands::Diagnostics { command } => match command {
            DiagnosticsCommand::Perf(PerfArgs { limit }) => {
                run_diagnostics_perf(*limit)?;
            }
        },
    }

    Ok(())
}

fn run_diagnostics_perf(limit: usize) -> Result<()> {
    use blit_core::perf_history::{config_dir, read_recent_records, TransferMode};

    let disabled = std::env::var("BLIT_DISABLE_PERF_HISTORY")
        .map(|v| matches!(v.trim(), "1" | "true" | "TRUE"))
        .unwrap_or(false);

    let history_path = config_dir()?.join("perf_local.jsonl");
    let records = read_recent_records(limit)?;

    println!(
        "Performance history (showing up to {} entries): {}",
        limit,
        records.len()
    );
    println!("History file: {}", history_path.display());
    println!(
        "Status: {}",
        if disabled {
            "disabled via BLIT_DISABLE_PERF_HISTORY"
        } else if records.is_empty() {
            "enabled (no entries yet)"
        } else {
            "enabled"
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
            TransferMode::Copy => "copy",
            TransferMode::Mirror => "mirror",
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
