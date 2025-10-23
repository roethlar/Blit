use blit_core::fs_enum::FileFilter;
use blit_core::generated::blit_client::BlitClient;
use blit_core::generated::{ListModulesRequest, ListRequest};
use blit_core::orchestrator::{LocalMirrorOptions, LocalMirrorSummary, TransferOrchestrator};
use blit_core::perf_history;
use blit_core::remote::{
    RemoteEndpoint, RemotePath, RemotePullClient, RemotePullReport, RemotePushClient,
    RemotePushReport,
};
use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand};
use eyre::{bail, eyre, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

struct AppContext {
    perf_history_enabled: bool,
}

impl AppContext {
    fn load() -> Self {
        let perf_history_enabled = match perf_history::perf_history_enabled() {
            Ok(enabled) => enabled,
            Err(err) => {
                eprintln!(
                    "[warn] failed to read performance history settings (defaulting to enabled): {err:?}"
                );
                true
            }
        };
        Self {
            perf_history_enabled,
        }
    }
}

#[derive(Parser)]
#[command(name = "blit")]
#[command(about = "A fast, AI-built file transfer tool (v2)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Copy files between local and/or remote locations
    Copy(TransferArgs),
    /// Mirror a directory (including deletions at destination)
    Mirror(TransferArgs),
    /// Move a directory or file (mirror + remove source)
    Move(TransferArgs),
    /// Discover daemons advertising via mDNS
    Scan(ScanArgs),
    /// List modules or paths on a remote daemon
    List(ListArgs),
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
    /// Enable performance history capture
    #[arg(long, conflicts_with = "disable")]
    enable: bool,
    /// Disable performance history capture
    #[arg(long, conflicts_with = "enable")]
    disable: bool,
    /// Remove the stored performance history file
    #[arg(long)]
    clear: bool,
}

#[derive(Args)]
struct TransferArgs {
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
    /// Show an interactive progress indicator
    #[arg(long)]
    progress: bool,
    /// Limit worker threads (advanced debugging only)
    #[arg(long, hide = true)]
    workers: Option<usize>,
}

#[derive(Args)]
struct ScanArgs {
    /// Seconds to wait for mDNS responses
    #[arg(long, default_value_t = 2)]
    wait: u64,
}

#[derive(Args)]
struct ListArgs {
    /// Remote location to list (e.g., server:/module/, server:/module/path, server)
    target: String,
}

#[derive(Copy, Clone)]
enum TransferKind {
    Copy,
    Mirror,
}

enum Endpoint {
    Local(PathBuf),
    Remote(RemoteEndpoint),
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    let mut ctx = AppContext::load();

    match &cli.command {
        Commands::Copy(args) => run_transfer(&ctx, args, TransferKind::Copy).await?,
        Commands::Mirror(args) => run_transfer(&ctx, args, TransferKind::Mirror).await?,
        Commands::Move(args) => run_move(&ctx, args).await?,
        Commands::Scan(args) => run_scan(args).await?,
        Commands::List(args) => run_list(args).await?,
        Commands::Diagnostics { command } => match command {
            DiagnosticsCommand::Perf(args) => {
                run_diagnostics_perf(&mut ctx, args)?;
            }
        },
    }

    Ok(())
}

fn run_diagnostics_perf(ctx: &mut AppContext, args: &PerfArgs) -> Result<()> {
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

async fn run_transfer(ctx: &AppContext, args: &TransferArgs, mode: TransferKind) -> Result<()> {
    let src_endpoint = parse_transfer_endpoint(&args.source)?;
    let dst_endpoint = parse_transfer_endpoint(&args.destination)?;

    match (src_endpoint, dst_endpoint) {
        (Endpoint::Local(src_path), Endpoint::Local(dst_path)) => {
            if !src_path.exists() {
                bail!("source path does not exist: {}", src_path.display());
            }
            run_local_transfer(
                ctx,
                args,
                &src_path,
                &dst_path,
                matches!(mode, TransferKind::Mirror),
            )
            .await
        }
        (Endpoint::Local(src_path), Endpoint::Remote(remote)) => {
            if !src_path.exists() {
                bail!("source path does not exist: {}", src_path.display());
            }
            ensure_remote_transfer_supported(args)?;
            ensure_remote_destination_supported(&remote)?;
            run_remote_push_transfer(
                ctx,
                args,
                &src_path,
                remote,
                matches!(mode, TransferKind::Mirror),
            )
            .await
        }
        (Endpoint::Remote(remote), Endpoint::Local(dst_path)) => {
            if matches!(mode, TransferKind::Mirror) {
                bail!("remote-to-local mirror is not supported yet");
            }
            ensure_remote_transfer_supported(args)?;
            ensure_remote_source_supported(&remote)?;
            run_remote_pull_transfer(ctx, args, remote, &dst_path).await
        }
        (Endpoint::Remote(_), Endpoint::Remote(_)) => {
            bail!("remote-to-remote transfers are not supported yet")
        }
    }
}

async fn run_move(ctx: &AppContext, args: &TransferArgs) -> Result<()> {
    let src_endpoint = parse_transfer_endpoint(&args.source)?;
    let dst_endpoint = parse_transfer_endpoint(&args.destination)?;

    match (src_endpoint, dst_endpoint) {
        (Endpoint::Local(src_path), Endpoint::Local(dst_path)) => {
            if args.dry_run {
                bail!("move does not support --dry-run");
            }
            if !src_path.exists() {
                bail!("source path does not exist: {}", src_path.display());
            }
            run_local_transfer(ctx, args, &src_path, &dst_path, true).await?;

            if src_path.is_dir() {
                fs::remove_dir_all(&src_path)
                    .with_context(|| format!("removing {}", src_path.display()))?;
            } else if src_path.is_file() {
                fs::remove_file(&src_path)
                    .with_context(|| format!("removing {}", src_path.display()))?;
            }
            Ok(())
        }
        _ => bail!("remote moves are not supported yet"),
    }
}

async fn run_scan(args: &ScanArgs) -> Result<()> {
    let _ = args;
    bail!("`blit scan` is not implemented yet (pending Phase 3 work)");
}

async fn run_list(args: &ListArgs) -> Result<()> {
    let endpoint = match parse_transfer_endpoint(&args.target) {
        Ok(endpoint) => endpoint,
        Err(_) => {
            // Treat as local path fallback
            let path = PathBuf::from(&args.target);
            if !path.exists() {
                bail!("path does not exist: {}", path.display());
            }
            list_local_path(&path)?;
            return Ok(());
        }
    };

    match endpoint {
        Endpoint::Local(path) => {
            if !path.exists() {
                bail!("path does not exist: {}", path.display());
            }
            list_local_path(&path)?;
            Ok(())
        }
        Endpoint::Remote(remote) => run_remote_list(remote).await,
    }
}

fn parse_transfer_endpoint(input: &str) -> Result<Endpoint> {
    match RemoteEndpoint::parse(input) {
        Ok(endpoint) => Ok(Endpoint::Remote(endpoint)),
        Err(err) => {
            if input.contains("://") || input.contains(":/") {
                Err(err)
            } else {
                Ok(Endpoint::Local(PathBuf::from(input)))
            }
        }
    }
}

fn ensure_remote_transfer_supported(args: &TransferArgs) -> Result<()> {
    if args.dry_run {
        bail!("--dry-run is not supported for remote transfers");
    }
    if args.checksum {
        bail!("--checksum is not supported for remote transfers");
    }
    if args.workers.is_some() {
        bail!("--workers limiter is not supported for remote transfers");
    }
    Ok(())
}

fn ensure_remote_destination_supported(remote: &RemoteEndpoint) -> Result<()> {
    match &remote.path {
        RemotePath::Module { .. } => Ok(()),
        RemotePath::Root { .. } => bail!(
            "root exports (server://...) are not supported yet; configure daemon root export first"
        ),
        RemotePath::Discovery => {
            bail!("remote destination must include a module (e.g., server:/module/)",)
        }
    }
}

fn ensure_remote_source_supported(remote: &RemoteEndpoint) -> Result<()> {
    match remote.path {
        RemotePath::Module { .. } => Ok(()),
        RemotePath::Root { .. } => bail!(
            "root exports (server://...) are not supported yet; configure daemon root export first"
        ),
        RemotePath::Discovery => {
            bail!("remote source must include a module (e.g., server:/module/)")
        }
    }
}

fn format_remote_endpoint(remote: &RemoteEndpoint) -> String {
    remote.display()
}

async fn run_remote_push_transfer(
    _ctx: &AppContext,
    _args: &TransferArgs,
    source_path: &Path,
    remote: RemoteEndpoint,
    mirror_mode: bool,
) -> Result<()> {
    let mut client = RemotePushClient::connect(remote.clone())
        .await
        .with_context(|| format!("connecting to {}", remote.control_plane_uri()))?;

    let filter = FileFilter::default();
    let report = client
        .push(source_path, &filter, mirror_mode)
        .await
        .with_context(|| {
            format!(
                "negotiating push manifest for {} -> {}",
                source_path.display(),
                format_remote_endpoint(&remote)
            )
        })?;

    describe_push_result(&report, &format_remote_endpoint(&remote));
    Ok(())
}

async fn run_remote_pull_transfer(
    _ctx: &AppContext,
    _args: &TransferArgs,
    remote: RemoteEndpoint,
    dest_root: &Path,
) -> Result<()> {
    let mut client = RemotePullClient::connect(remote.clone())
        .await
        .with_context(|| format!("connecting to {}", remote.control_plane_uri()))?;

    let report = client.pull(dest_root).await.with_context(|| {
        format!(
            "pulling from {} into {}",
            format_remote_endpoint(&remote),
            dest_root.display()
        )
    })?;

    describe_pull_result(&report, dest_root);
    Ok(())
}

fn describe_pull_result(report: &RemotePullReport, dest_root: &Path) {
    println!(
        "Pull complete: {} file(s), {} bytes written to {}.",
        report.files_transferred,
        report.bytes_transferred,
        dest_root.display()
    );
}

fn describe_push_result(report: &RemotePushReport, destination: &str) {
    let file_count = report.files_requested.len();
    if file_count == 0 {
        println!(
            "Remote already up to date; nothing to upload ({}).",
            destination
        );
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
    println!("Destination: {}", destination);
}

fn list_local_path(path: &Path) -> Result<()> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("reading metadata for {}", path.display()))?;
    if metadata.is_file() {
        println!(
            "FILE {:>12} {}",
            format_bytes(metadata.len()),
            path.display()
        );
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(path)
        .with_context(|| format!("reading directory {}", path.display()))?
        .collect::<Result<_, _>>()
        .with_context(|| format!("iterating directory {}", path.display()))?;
    entries.sort_by_key(|entry| entry.path());
    println!("Listing {}:", path.display());
    for entry in entries {
        let entry_path = entry.path();
        let meta = entry
            .metadata()
            .with_context(|| format!("metadata {}", entry_path.display()))?;
        let name = entry_path.file_name().unwrap_or_default().to_string_lossy();
        if meta.is_dir() {
            println!("DIR  {:>12} {}/", "-", name);
        } else {
            println!("FILE {:>12} {}", format_bytes(meta.len()), name);
        }
    }
    Ok(())
}

async fn run_remote_list(remote: RemoteEndpoint) -> Result<()> {
    let uri = remote.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to {}", uri))?;

    match &remote.path {
        RemotePath::Discovery => {
            let response = client
                .list_modules(ListModulesRequest {})
                .await
                .map_err(|status| eyre!(status.message().to_string()))?
                .into_inner();
            if response.modules.is_empty() {
                println!("No modules exported by {}", remote.display());
            } else {
                println!("Modules on {}:", remote.display());
                for module in response.modules {
                    println!(
                        "{}\t{}\t{}",
                        module.name,
                        module.path,
                        if module.read_only { "read-only" } else { "rw" }
                    );
                }
            }
            Ok(())
        }
        RemotePath::Module { module, rel_path } => {
            let path_str = if rel_path.as_os_str().is_empty() {
                String::new()
            } else {
                rel_path
                    .iter()
                    .map(|component| component.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/")
            };
            let response = client
                .list(ListRequest {
                    module: module.clone(),
                    path: path_str.clone(),
                })
                .await
                .map_err(|status| eyre!(status.message().to_string()))?
                .into_inner();
            if response.entries.is_empty() {
                println!(
                    "No entries under {}:/{}",
                    module,
                    if path_str.is_empty() { "" } else { &path_str }
                );
            } else {
                println!(
                    "Listing {}:/{}:",
                    module,
                    if path_str.is_empty() { "" } else { &path_str }
                );
                for entry in response.entries {
                    let indicator = if entry.is_dir { "DIR " } else { "FILE" };
                    println!(
                        "{} {:>12} {}",
                        indicator,
                        if entry.is_dir {
                            "-".to_string()
                        } else {
                            format_bytes(entry.size)
                        },
                        entry.name
                    );
                }
            }
            Ok(())
        }
        RemotePath::Root { .. } => bail!("listing root exports is not supported yet"),
    }
}

async fn run_local_transfer(
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

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
    }

    #[test]
    fn copy_local_transfers_file() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        std::fs::create_dir_all(&src)?;
        std::fs::write(src.join("hello.txt"), b"hello")?;
        let ctx = AppContext {
            perf_history_enabled: false,
        };

        let args = TransferArgs {
            source: src.to_string_lossy().into_owned(),
            destination: dest.to_string_lossy().into_owned(),
            dry_run: false,
            checksum: false,
            verbose: false,
            progress: false,
            workers: None,
        };

        runtime().block_on(run_local_transfer(&ctx, &args, &src, &dest, false))?;
        let copied = std::fs::read(dest.join("hello.txt"))?;
        assert_eq!(copied, b"hello");
        Ok(())
    }

    #[test]
    fn copy_local_dry_run_creates_no_files() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        std::fs::create_dir_all(&src)?;
        std::fs::write(src.join("hello.txt"), b"hello")?;
        let ctx = AppContext {
            perf_history_enabled: false,
        };

        let args = TransferArgs {
            source: src.to_string_lossy().into_owned(),
            destination: dest.to_string_lossy().into_owned(),
            dry_run: true,
            checksum: false,
            verbose: false,
            progress: false,
            workers: None,
        };

        runtime().block_on(run_local_transfer(&ctx, &args, &src, &dest, false))?;
        assert!(!dest.join("hello.txt").exists());
        Ok(())
    }
}
