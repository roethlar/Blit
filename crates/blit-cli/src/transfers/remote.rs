use crate::cli::TransferArgs;
use crate::context::AppContext;
use eyre::{Context, Result};
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::{interval, MissedTickBehavior};

use blit_core::fs_enum::FileFilter;
use blit_core::remote::push::ProgressEvent;
use blit_core::remote::{
    RemoteEndpoint, RemotePullClient, RemotePullReport, RemotePushClient, RemotePushProgress,
    RemotePushReport,
};

use super::endpoints::format_remote_endpoint;

pub async fn run_remote_push_transfer(
    _ctx: &AppContext,
    args: &TransferArgs,
    source_path: &Path,
    remote: RemoteEndpoint,
    mirror_mode: bool,
) -> Result<()> {
    let mut client = RemotePushClient::connect(remote.clone())
        .await
        .with_context(|| format!("connecting to {}", remote.control_plane_uri()))?;

    let show_progress = args.progress || args.verbose;
    let mut progress_handle = None;
    let mut progress_task = None;

    if show_progress {
        let (tx, mut rx) = mpsc::unbounded_channel::<ProgressEvent>();
        let progress = RemotePushProgress::new(tx);
        let join = tokio::spawn(async move {
            let start = Instant::now();
            let mut total_manifest = 0usize;
            let mut total_files = 0usize;
            let mut total_bytes = 0u64;
            let mut prev_bytes = 0u64;
            let mut prev_instant = start;
            let mut started = false;
            let mut ticker = interval(Duration::from_secs(1));
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    biased;
                    event = rx.recv() => {
                        match event {
                            Some(ProgressEvent::ManifestBatch { files }) => {
                                total_manifest = total_manifest.saturating_add(files);
                            }
                            Some(ProgressEvent::Payload { files, bytes }) => {
                                if files > 0 {
                                    total_files = total_files.saturating_add(files);
                                }
                                if bytes > 0 {
                                    total_bytes = total_bytes.saturating_add(bytes);
                                    started = true;
                                }
                            }
                            None => break,
                        }
                    }
                    _ = ticker.tick() => {
                        if started {
                            let now = Instant::now();
                            let elapsed = now.duration_since(start).as_secs_f64().max(1e-6);
                            let window_elapsed = now
                                .duration_since(prev_instant)
                                .as_secs_f64()
                                .max(1e-6);
                            let window_bytes = total_bytes.saturating_sub(prev_bytes);
                            let avg_mib = (total_bytes as f64 / 1024.0 / 1024.0) / elapsed;
                            let current_mib =
                                (window_bytes as f64 / 1024.0 / 1024.0) / window_elapsed;
                            println!(
                                "[progress] {}/{} files • {:.2} MiB copied • {:.2} MiB/s avg • {:.2} MiB/s current",
                                total_files,
                                total_manifest,
                                total_bytes as f64 / (1024.0 * 1024.0),
                                avg_mib,
                                current_mib,
                            );
                            prev_instant = now;
                            prev_bytes = total_bytes;
                        } else if total_manifest > 0 {
                            println!(
                                "[progress] manifest enumerated {} file(s)…",
                                total_manifest
                            );
                        }
                    }
                }
            }

            if started {
                let elapsed = start.elapsed().as_secs_f64().max(1e-6);
                let avg_mib = (total_bytes as f64 / 1024.0 / 1024.0) / elapsed;
                println!(
                    "[progress] final: {} file(s) transferred • {:.2} MiB total • {:.2} MiB/s avg",
                    total_files,
                    total_bytes as f64 / (1024.0 * 1024.0),
                    avg_mib,
                );
            } else if total_manifest > 0 {
                println!("[progress] manifest enumerated {} file(s)", total_manifest);
            }
        });

        progress_handle = Some(progress);
        progress_task = Some(join);
    }

    let filter = FileFilter::default();
    let push_result = client
        .push(
            source_path,
            &filter,
            mirror_mode,
            args.force_grpc,
            progress_handle.as_ref(),
        )
        .await
        .with_context(|| {
            format!(
                "negotiating push manifest for {} -> {}",
                source_path.display(),
                format_remote_endpoint(&remote)
            )
        });

    drop(progress_handle);
    if let Some(task) = progress_task {
        let _ = task.await;
    }

    let report = push_result?;

    describe_push_result(&report, &format_remote_endpoint(&remote), show_progress);
    Ok(())
}

pub async fn run_remote_pull_transfer(
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

pub fn describe_pull_result(report: &RemotePullReport, dest_root: &Path) {
    println!(
        "Pull complete: {} file(s), {} bytes written to {}.",
        report.files_transferred,
        report.bytes_transferred,
        dest_root.display()
    );
}

pub fn describe_push_result(
    report: &RemotePushReport,
    destination: &str,
    show_first_payload: bool,
) {
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
