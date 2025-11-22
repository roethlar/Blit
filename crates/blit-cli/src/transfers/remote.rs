use crate::cli::TransferArgs;
use crate::context::AppContext;
use eyre::{eyre, Context, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, MissedTickBehavior};

use blit_core::fs_enum::FileFilter;
use blit_core::remote::transfer::{ProgressEvent, RemoteTransferProgress};
use blit_core::remote::{
    RemoteEndpoint, RemotePullClient, RemotePullProgress, RemotePullReport, RemotePushClient,
    RemotePushProgress, RemotePushReport,
};
use blit_core::remote::transfer::source::{FsTransferSource, RemoteTransferSource, TransferSource};
use std::sync::Arc;

use super::endpoints::{format_remote_endpoint, Endpoint};

fn spawn_progress_monitor(
    enabled: bool,
) -> (Option<RemoteTransferProgress>, Option<JoinHandle<()>>) {
    if !enabled {
        return (None, None);
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<ProgressEvent>();
    let progress = RemoteTransferProgress::new(tx);
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
                        let window_elapsed = now.duration_since(prev_instant).as_secs_f64().max(1e-6);
                        let window_bytes = total_bytes.saturating_sub(prev_bytes);
                        let avg_mib = (total_bytes as f64 / 1024.0 / 1024.0) / elapsed;
                        let current_mib = (window_bytes as f64 / 1024.0 / 1024.0) / window_elapsed;
                        println!(
                            "[progress] {}/{} files \u{2022} {:.2} MiB copied \u{2022} {:.2} MiB/s avg \u{2022} {:.2} MiB/s current",
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
                            "[progress] manifest enumerated {} file(s)\u{2026}",
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
                "[progress] final: {} file(s) transferred \u{2022} {:.2} MiB total \u{2022} {:.2} MiB/s avg",
                total_files,
                total_bytes as f64 / (1024.0 * 1024.0),
                avg_mib,
            );
        } else if total_manifest > 0 {
            println!("[progress] manifest enumerated {} file(s)", total_manifest);
        }
    });

    (Some(progress), Some(join))
}

pub async fn run_remote_push_transfer(
    _ctx: &AppContext,
    args: &TransferArgs,
    source: Endpoint,
    remote: RemoteEndpoint,
    mirror_mode: bool,
) -> Result<()> {
    let mut client = RemotePushClient::connect(remote.clone())
        .await
        .with_context(|| format!("connecting to {}", remote.control_plane_uri()))?;

    let show_progress = args.progress || args.verbose;
    let (progress_handle, progress_task) = spawn_progress_monitor(show_progress);

    let filter = FileFilter::default();
    let transfer_source: Arc<dyn TransferSource> = match source {
        Endpoint::Local(path) => Arc::new(FsTransferSource::new(path)),
        Endpoint::Remote(endpoint) => {
            let client = RemotePullClient::connect(endpoint.clone())
                .await
                .with_context(|| format!("connecting to source {}", endpoint.control_plane_uri()))?;
            // Use the relative path from the endpoint as the root
            let root = match &endpoint.path {
                blit_core::remote::RemotePath::Module { rel_path, .. } => rel_path.clone(),
                blit_core::remote::RemotePath::Root { rel_path } => rel_path.clone(),
                blit_core::remote::RemotePath::Discovery => PathBuf::from("."),
            };
            Arc::new(RemoteTransferSource::new(client, root))
        }
    };

    let push_result = client
        .push(
            transfer_source.clone(),
            &filter,
            mirror_mode,
            args.force_grpc,
            progress_handle.as_ref(),
            args.trace_data_plane,
        )
        .await
        .with_context(|| {
            format!(
                "negotiating push manifest for {} -> {}",
                transfer_source.root().display(),
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
    mirror_mode: bool,
) -> Result<()> {
    let mut client = RemotePullClient::connect(remote.clone())
        .await
        .with_context(|| format!("connecting to {}", remote.control_plane_uri()))?;

    let show_progress = args.progress || args.verbose;
    let (progress_handle, progress_task) = spawn_progress_monitor(show_progress);

    let report = client
        .pull(
            dest_root,
            args.force_grpc,
            mirror_mode,
            progress_handle.as_ref(),
        )
        .await
        .with_context(|| {
            format!(
                "pulling from {} into {}",
                format_remote_endpoint(&remote),
                dest_root.display()
            )
        })?;

    drop(progress_handle);
    if let Some(task) = progress_task {
        let _ = task.await;
    }

    describe_pull_result(&report, dest_root);

    if mirror_mode {
        let stats = purge_extraneous_local(dest_root, &report.downloaded_paths).await?;
        println!(
            "Mirror purge removed {} file(s) and {} directorie(s).",
            stats.files_deleted, stats.dirs_deleted
        );
    }

    Ok(())
}

struct LocalPurgeStats {
    files_deleted: u64,
    dirs_deleted: u64,
}

async fn purge_extraneous_local(
    dest_root: &Path,
    keep_paths: &[PathBuf],
) -> Result<LocalPurgeStats> {
    use std::collections::HashSet;
    use walkdir::WalkDir;

    let keep_set: HashSet<PathBuf> = keep_paths.iter().cloned().collect();
    let root = dest_root.to_path_buf();

    let extraneous_files = tokio::task::spawn_blocking(move || {
        let mut extras = Vec::new();
        for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Ok(rel) = entry.path().strip_prefix(&root) {
                    if keep_set.is_empty() || !keep_set.contains(rel) {
                        extras.push(entry.path().to_path_buf());
                    }
                }
            }
        }
        extras
    })
    .await
    .map_err(|err| eyre!("enumeration task failed: {}", err))?;

    let mut stats = LocalPurgeStats {
        files_deleted: 0,
        dirs_deleted: 0,
    };

    for file_path in extraneous_files {
        if tokio::fs::remove_file(&file_path).await.is_ok() {
            stats.files_deleted += 1;
        }
    }

    let root_for_dirs = dest_root.to_path_buf();
    let dirs = tokio::task::spawn_blocking(move || {
        let mut dirs = Vec::new();
        for entry in WalkDir::new(&root_for_dirs)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_dir() {
                dirs.push(entry.path().to_path_buf());
            }
        }
        dirs.sort_by(|a, b| b.components().count().cmp(&a.components().count()));
        dirs
    })
    .await
    .map_err(|err| eyre!("enumeration task failed: {}", err))?;

    for dir in dirs {
        if dir == dest_root {
            continue;
        }
        if tokio::fs::remove_dir(&dir).await.is_ok() {
            stats.dirs_deleted += 1;
        }
    }

    Ok(stats)
}

pub fn describe_pull_result(report: &RemotePullReport, dest_root: &Path) {
    if let Some(summary) = &report.summary {
        println!(
            "Pull complete: {} file(s), {} bytes (zero-copy {} bytes){} -> {}.",
            summary.files_transferred,
            summary.bytes_transferred,
            summary.bytes_zero_copy,
            if summary.tcp_fallback_used {
                " [gRPC fallback]"
            } else {
                ""
            },
            dest_root.display()
        );
    } else {
        println!(
            "Pull complete: {} file(s), {} bytes written to {}.",
            report.files_transferred,
            report.bytes_transferred,
            dest_root.display()
        );
    }
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
