use crate::cli::TransferArgs;
use crate::context::AppContext;
use eyre::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use blit_core::fs_enum::FileFilter;
use blit_core::orchestrator::{LocalMirrorOptions, LocalMirrorSummary, TransferOrchestrator};
use blit_core::remote::{
    RemoteEndpoint, RemotePath, RemotePullClient, RemotePullReport, RemotePushClient,
    RemotePushReport,
};

#[derive(Copy, Clone)]
pub enum TransferKind {
    Copy,
    Mirror,
}

pub enum Endpoint {
    Local(PathBuf),
    Remote(RemoteEndpoint),
}

pub async fn run_transfer(ctx: &AppContext, args: &TransferArgs, mode: TransferKind) -> Result<()> {
    let src_endpoint = parse_transfer_endpoint(&args.source)?;
    let dst_endpoint = parse_transfer_endpoint(&args.destination)?;

    let operation = match mode {
        TransferKind::Copy => "copy",
        TransferKind::Mirror => "mirror",
    };
    let transfer_scope = match (&src_endpoint, &dst_endpoint) {
        (Endpoint::Local(src_path), Endpoint::Local(dst_path)) => {
            format!("{} -> {}", src_path.display(), dst_path.display())
        }
        (Endpoint::Local(src_path), Endpoint::Remote(remote)) => {
            format!(
                "{} -> {}",
                src_path.display(),
                format_remote_endpoint(remote)
            )
        }
        (Endpoint::Remote(remote), Endpoint::Local(dst_path)) => {
            format!(
                "{} -> {}",
                format_remote_endpoint(remote),
                dst_path.display()
            )
        }
        (Endpoint::Remote(a), Endpoint::Remote(b)) => {
            format!(
                "{} -> {}",
                format_remote_endpoint(a),
                format_remote_endpoint(b)
            )
        }
    };
    println!(
        "blit v{}: starting {} {}",
        env!("CARGO_PKG_VERSION"),
        operation,
        transfer_scope
    );

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

pub async fn run_move(ctx: &AppContext, args: &TransferArgs) -> Result<()> {
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

pub fn parse_transfer_endpoint(input: &str) -> Result<Endpoint> {
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

pub(crate) fn format_remote_endpoint(remote: &RemoteEndpoint) -> String {
    remote.display()
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
        RemotePath::Module { .. } | RemotePath::Root { .. } => Ok(()),
        RemotePath::Discovery => {
            bail!("remote destination must include a module or root (e.g., server:/module/ or server://path)")
        }
    }
}

fn ensure_remote_source_supported(remote: &RemoteEndpoint) -> Result<()> {
    match remote.path {
        RemotePath::Module { .. } | RemotePath::Root { .. } => Ok(()),
        RemotePath::Discovery => {
            bail!("remote source must include a module or root (e.g., server:/module/ or server://path)")
        }
    }
}

async fn run_remote_push_transfer(
    _ctx: &AppContext,
    args: &TransferArgs,
    source_path: &Path,
    remote: RemoteEndpoint,
    mirror_mode: bool,
) -> Result<()> {
    let mut client = RemotePushClient::connect(remote.clone())
        .await
        .with_context(|| format!("connecting to {}", remote.control_plane_uri()))?;

    let filter = FileFilter::default();
    let report = client
        .push(source_path, &filter, mirror_mode, args.force_grpc)
        .await
        .with_context(|| {
            format!(
                "negotiating push manifest for {} -> {}",
                source_path.display(),
                format_remote_endpoint(&remote)
            )
        })?;

    describe_push_result(
        &report,
        &format_remote_endpoint(&remote),
        args.progress || args.verbose,
    );
    Ok(())
}

async fn run_remote_pull_transfer(
    _ctx: &AppContext,
    args: &TransferArgs,
    remote: RemoteEndpoint,
    dest_root: &Path,
) -> Result<()> {
    let mut client = RemotePullClient::connect(remote.clone())
        .await
        .with_context(|| format!("connecting to {}", remote.control_plane_uri()))?;

    let report = client
        .pull(dest_root, args.force_grpc)
        .await
        .with_context(|| {
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

fn describe_push_result(report: &RemotePushReport, destination: &str, show_first_payload: bool) {
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
    if show_first_payload {
        if let Some(elapsed) = report.first_payload_elapsed {
            println!("First payload dispatched after {:.2?}.", elapsed);
        }
    }
    if summary.entries_deleted > 0 {
        let plural = if summary.entries_deleted == 1 {
            ""
        } else {
            "s"
        };
        println!(
            "Remote purge removed {} entr{}.",
            summary.entries_deleted, plural
        );
    }
    println!("Destination: {}", destination);
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

pub(crate) fn format_bytes(bytes: u64) -> String {
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
    use crate::cli::TransferArgs;
    use crate::context::AppContext;
    use eyre::Result;
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
            force_grpc: false,
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
            force_grpc: false,
        };

        runtime().block_on(run_local_transfer(&ctx, &args, &src, &dest, false))?;
        assert!(!dest.join("hello.txt").exists());
        Ok(())
    }
}
