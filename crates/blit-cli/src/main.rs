use blit_core::fs_enum::FileFilter;
use blit_core::orchestrator::{LocalMirrorOptions, LocalMirrorSummary, TransferOrchestrator};
use blit_core::remote::{RemoteEndpoint, RemotePushClient, RemotePushReport};
use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand};
use eyre::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::{Duration, Instant, UNIX_EPOCH};

#[derive(Parser)]
#[command(name = "blit")]
#[command(about = "A fast, AI-built file transfer tool (v2)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Copy files locally from source to destination
    Copy(LocalArgs),
    /// Mirror a directory locally (including deletions at destination)
    Mirror(LocalArgs),
    /// Push files to a remote server
    Push { source: String, destination: String },
    /// Pull files from a remote server
    Pull { source: String, destination: String },
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

#[derive(Args)]
struct LocalArgs {
    /// Source path for the transfer
    source: String,
    /// Destination path for the transfer
    destination: String,
    /// Perform a dry run without making changes
    #[arg(long)]
    dry_run: bool,
    /// Force checksum comparison of files
    #[arg(long)]
    checksum: bool,
    /// Keep verbose logs from the orchestrator
    #[arg(long)]
    verbose: bool,
    /// Disable the interactive progress indicator
    #[arg(long)]
    no_progress: bool,
    /// Limit worker threads (advanced debugging only)
    #[arg(long, hide = true)]
    workers: Option<usize>,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    match &cli.command {
        Commands::Push {
            source,
            destination,
        } => run_remote_push(source, destination).await?,
        Commands::Copy(args) => run_local_transfer(args, false).await?,
        Commands::Mirror(args) => run_local_transfer(args, true).await?,
        Commands::Pull {
            source,
            destination,
        } => {
            println!("Pulling from {} to {}", source, destination);
            // To be implemented in Phase 3
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

async fn run_remote_push(source: &str, destination: &str) -> Result<()> {
    let endpoint = RemoteEndpoint::parse(destination)?;
    let mut client = RemotePushClient::connect(endpoint.clone())
        .await
        .with_context(|| format!("connecting to {}", endpoint.control_plane_uri()))?;

    let filter = FileFilter::default();
    let source_path = PathBuf::from(source);
    let report = client
        .push(&source_path, &filter, false)
        .await
        .with_context(|| {
            format!(
                "negotiating push manifest for {} -> blit://{}:{}/{}",
                source, endpoint.host, endpoint.port, endpoint.module
            )
        })?;

    describe_push_result(&report);

    Ok(())
}

fn describe_push_result(report: &RemotePushReport) {
    let file_count = report.files_requested.len();
    if file_count == 0 {
        println!("Remote already up to date; nothing to upload.");
    } else if report.fallback_used {
        println!(
            "Negotiation complete: {} file(s) scheduled; using gRPC data fallback.",
            file_count
        );
    } else if let Some(port) = report.data_port {
        println!(
            "Negotiation complete: {} file(s) scheduled; data port {} established.",
            file_count, port
        );
    } else {
        println!(
            "Negotiation complete: {} file(s) scheduled; awaiting server summary.",
            file_count
        );
    }

    let summary = &report.summary;
    println!(
        "Transfer complete: {} file(s), {} bytes (zero-copy {} bytes){}.",
        summary.files_transferred,
        summary.bytes_transferred,
        summary.bytes_zero_copy,
        if summary.tcp_fallback_used {
            " [gRPC fallback]"
        } else {
            ""
        }
    );
}

async fn run_local_transfer(args: &LocalArgs, mirror: bool) -> Result<()> {
    let src_path = PathBuf::from(&args.source);
    let dest_path = PathBuf::from(&args.destination);

    if !src_path.exists() {
        bail!("source path does not exist: {}", src_path.display());
    }

    let options = build_local_options(args, mirror);
    let dry_run = options.dry_run;
    let verbose = options.verbose;
    let debug_mode = options.debug_mode;
    let workers = options.workers;
    if debug_mode {
        eprintln!(
            "[DEBUG] Worker limiter active – FAST planner auto-tuning capped to {workers} thread(s)."
        );
    }

    let progress_bar = if args.no_progress {
        None
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("{spinner} {msg}")
                .unwrap()
                .tick_strings(&["⠁", "⠂", "⠄", "⠂"]),
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

    let src_clone = src_path.clone();
    let dest_clone = dest_path.clone();
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

fn build_local_options(args: &LocalArgs, mirror: bool) -> LocalMirrorOptions {
    let mut options = LocalMirrorOptions::default();
    options.mirror = mirror;
    options.dry_run = args.dry_run;
    options.verbose = args.verbose;
    options.progress = false;
    options.checksum = args.checksum;
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
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes == 0 {
        return "0 B".to_owned();
    }
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.2} {}", value, UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(prev) = &self.prev {
                std::env::set_var(self.key, prev);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
    }

    #[test]
    fn copy_local_transfers_file() -> Result<()> {
        let _env = EnvGuard::set("BLIT_DISABLE_PERF_HISTORY", "1");
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        std::fs::create_dir_all(&src)?;
        std::fs::write(src.join("hello.txt"), b"hello")?;

        let args = LocalArgs {
            source: src.to_string_lossy().into_owned(),
            destination: dest.to_string_lossy().into_owned(),
            dry_run: false,
            checksum: false,
            verbose: false,
            no_progress: true,
            workers: None,
        };

        runtime().block_on(run_local_transfer(&args, false))?;
        let copied = std::fs::read(dest.join("hello.txt"))?;
        assert_eq!(copied, b"hello");
        Ok(())
    }

    #[test]
    fn copy_local_dry_run_creates_no_files() -> Result<()> {
        let _env = EnvGuard::set("BLIT_DISABLE_PERF_HISTORY", "1");
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        std::fs::create_dir_all(&src)?;
        std::fs::write(src.join("hello.txt"), b"hello")?;

        let args = LocalArgs {
            source: src.to_string_lossy().into_owned(),
            destination: dest.to_string_lossy().into_owned(),
            dry_run: true,
            checksum: false,
            verbose: false,
            no_progress: true,
            workers: None,
        };

        runtime().block_on(run_local_transfer(&args, false))?;
        assert!(!dest.join("hello.txt").exists());
        Ok(())
    }
}
