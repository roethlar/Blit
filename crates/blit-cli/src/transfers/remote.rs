use crate::cli::TransferArgs;
use eyre::{eyre, Context, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, MissedTickBehavior};

use blit_core::fs_enum::FileFilter;
use blit_core::generated::FileHeader;
use blit_core::remote::pull::PullSyncOptions;
use blit_core::remote::transfer::source::{FsTransferSource, RemoteTransferSource, TransferSource};
use blit_core::remote::transfer::{ProgressEvent, RemoteTransferProgress};
use blit_core::remote::{
    RemoteEndpoint, RemotePath, RemotePullClient, RemotePullReport, RemotePushClient,
    RemotePushReport,
};
use std::sync::Arc;

use super::endpoints::{format_remote_endpoint, Endpoint};

/// Compute the actual destination path for a pull operation using rsync-style semantics:
/// - If dest exists and is a directory, append the source's basename to create dest/basename/
/// - Otherwise use dest as-is (will be created as the target)
///
/// Example: `copy server://path/release ~/Downloads/` -> `~/Downloads/release/`
fn compute_pull_destination(dest: &Path, remote: &RemoteEndpoint) -> Result<PathBuf> {
    // Get the source path's basename from the remote endpoint
    let source_basename = match &remote.path {
        RemotePath::Module { rel_path, .. } | RemotePath::Root { rel_path } => {
            rel_path.file_name().map(|s| s.to_os_string())
        }
        RemotePath::Discovery => None,
    };

    // If dest exists and is a directory, and we have a source basename, append it
    if dest.is_dir() {
        if let Some(basename) = source_basename {
            // Don't append if basename is empty or "."
            let basename_str = basename.to_string_lossy();
            if !basename_str.is_empty() && basename_str != "." {
                return Ok(dest.join(basename));
            }
        }
    }

    Ok(dest.to_path_buf())
}

fn spawn_progress_monitor(
    enabled: bool,
    verbose: bool,
    json: bool,
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
                        Some(ProgressEvent::FileComplete { path, bytes }) => {
                            total_files = total_files.saturating_add(1);
                            total_bytes = total_bytes.saturating_add(bytes);
                            started = true;
                            if json {
                                eprintln!(
                                    "{{\"event\":\"file_complete\",\"path\":\"{}\",\"bytes\":{}}}",
                                    path.replace('\\', "\\\\").replace('"', "\\\""),
                                    bytes
                                );
                            } else if verbose {
                                println!("{}", path);
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
                        let avg_bps = (total_bytes as f64) / elapsed;
                        let current_bps = (window_bytes as f64) / window_elapsed;
                        if json {
                            eprintln!(
                                "{{\"event\":\"progress\",\"files\":{},\"total_files\":{},\"bytes_copied\":{},\"avg_bytes_sec\":{:.0},\"current_bytes_sec\":{:.0}}}",
                                total_files, total_manifest, total_bytes, avg_bps, current_bps
                            );
                        } else {
                            let avg_mib = avg_bps / (1024.0 * 1024.0);
                            let current_mib = current_bps / (1024.0 * 1024.0);
                            println!(
                                "[progress] {}/{} files \u{2022} {:.2} MiB copied \u{2022} {:.2} MiB/s avg \u{2022} {:.2} MiB/s current",
                                total_files,
                                total_manifest,
                                total_bytes as f64 / (1024.0 * 1024.0),
                                avg_mib,
                                current_mib,
                            );
                        }
                        prev_instant = now;
                        prev_bytes = total_bytes;
                    } else if total_manifest > 0 {
                        if json {
                            eprintln!(
                                "{{\"event\":\"manifest\",\"total_files\":{}}}",
                                total_manifest
                            );
                        } else {
                            println!(
                                "[progress] manifest enumerated {} file(s)\u{2026}",
                                total_manifest
                            );
                        }
                    }
                }
            }
        }

        if started {
            let elapsed = start.elapsed().as_secs_f64().max(1e-6);
            let avg_bps = (total_bytes as f64) / elapsed;
            if json {
                eprintln!(
                    "{{\"event\":\"final\",\"files_transferred\":{},\"total_bytes\":{},\"avg_bytes_sec\":{:.0}}}",
                    total_files, total_bytes, avg_bps
                );
            } else {
                let avg_mib = avg_bps / (1024.0 * 1024.0);
                println!(
                    "[progress] final: {} file(s) transferred \u{2022} {:.2} MiB total \u{2022} {:.2} MiB/s avg",
                    total_files,
                    total_bytes as f64 / (1024.0 * 1024.0),
                    avg_mib,
                );
            }
        } else if total_manifest > 0 {
            if json {
                eprintln!(
                    "{{\"event\":\"manifest\",\"total_files\":{}}}",
                    total_manifest
                );
            } else {
                println!("[progress] manifest enumerated {} file(s)", total_manifest);
            }
        }
    });

    (Some(progress), Some(join))
}

pub async fn run_remote_push_transfer(
    args: &TransferArgs,
    source: Endpoint,
    remote: RemoteEndpoint,
    mirror_mode: bool,
) -> Result<()> {
    let mut client = RemotePushClient::connect(remote.clone())
        .await
        .with_context(|| format!("connecting to {}", remote.control_plane_uri()))?;

    let show_progress = args.progress || args.verbose;
    let (progress_handle, progress_task) =
        spawn_progress_monitor(show_progress, args.verbose, args.json);

    let filter = FileFilter::default();
    let transfer_source: Arc<dyn TransferSource> = match source {
        Endpoint::Local(path) => Arc::new(FsTransferSource::new(path)),
        Endpoint::Remote(endpoint) => {
            let client = RemotePullClient::connect(endpoint.clone())
                .await
                .with_context(|| {
                    format!("connecting to source {}", endpoint.control_plane_uri())
                })?;
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

    if args.json {
        print_push_json(&report, &format_remote_endpoint(&remote));
    } else {
        describe_push_result(&report, &format_remote_endpoint(&remote), show_progress);
    }
    Ok(())
}

pub async fn run_remote_pull_transfer(
    args: &TransferArgs,
    remote: RemoteEndpoint,
    dest_root: &Path,
    mirror_mode: bool,
) -> Result<()> {
    let mut client = RemotePullClient::connect(remote.clone())
        .await
        .with_context(|| format!("connecting to {}", remote.control_plane_uri()))?;

    // Compute actual destination path using rsync-style semantics:
    // - If dest exists and is a directory, append source basename
    // - Otherwise use dest as-is
    let actual_dest = compute_pull_destination(dest_root, &remote)?;

    // Enumerate local files to build manifest
    // Compute checksums if --checksum mode is requested
    let local_manifest = enumerate_local_manifest(&actual_dest, args.checksum).await?;

    let show_progress = args.progress || args.verbose;
    let (progress_handle, progress_task) =
        spawn_progress_monitor(show_progress, args.verbose, args.json);

    // Build comparison options from CLI args
    let pull_opts = PullSyncOptions {
        force_grpc: args.force_grpc,
        mirror_mode,
        size_only: args.size_only,
        ignore_times: args.ignore_times,
        ignore_existing: args.ignore_existing,
        force: args.force,
        checksum: args.checksum,
        resume: args.resume,
        block_size: 0, // Use default (1 MiB)
    };

    // Use PullSync - sends local manifest to server, server compares and only sends what's needed
    let report = client
        .pull_sync(
            &actual_dest,
            local_manifest,
            &pull_opts,
            mirror_mode, // track_paths for mirror mode deletion
            progress_handle.as_ref(),
        )
        .await
        .with_context(|| {
            format!(
                "pulling from {} into {}",
                format_remote_endpoint(&remote),
                actual_dest.display()
            )
        })?;

    drop(progress_handle);
    if let Some(task) = progress_task {
        let _ = task.await;
    }

    if args.json {
        print_pull_json(&report, &actual_dest);
    } else {
        describe_pull_result(&report, &actual_dest);
    }

    // Handle mirror mode deletions based on server's entries_deleted count
    if mirror_mode {
        if let Some(ref summary) = report.summary {
            if summary.entries_deleted > 0 {
                // The server told us how many files should be deleted locally
                // We need to delete local files not in the remote manifest
                let remote_paths: Vec<PathBuf> = report.downloaded_paths.to_vec();
                let stats = purge_extraneous_local(&actual_dest, &remote_paths).await?;
                if stats.files_deleted > 0 || stats.dirs_deleted > 0 {
                    println!(
                        "Mirror purge removed {} file(s) and {} directorie(s).",
                        stats.files_deleted, stats.dirs_deleted
                    );
                }
            }
        }
    }

    Ok(())
}

/// Enumerate local files to build a manifest for comparison with remote.
/// When `compute_checksums` is true, computes Blake3 checksums for each file.
async fn enumerate_local_manifest(root: &Path, compute_checksums: bool) -> Result<Vec<FileHeader>> {
    use blit_core::checksum::{hash_file, ChecksumType};
    use rayon::prelude::*;
    use walkdir::WalkDir;

    if !root.exists() {
        return Ok(Vec::new());
    }

    let root_path = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        // First, collect all file entries
        let entries: Vec<_> = WalkDir::new(&root_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();

        // Process files in parallel when computing checksums
        let manifest: Vec<FileHeader> = if compute_checksums {
            entries
                .into_par_iter()
                .filter_map(|entry| {
                    let path = entry.path();
                    let rel = path.strip_prefix(&root_path).ok()?;
                    let relative_path = rel
                        .iter()
                        .map(|c| c.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");

                    let meta = std::fs::metadata(path).ok()?;
                    let mtime_seconds = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);

                    // Compute Blake3 checksum
                    let checksum = hash_file(path, ChecksumType::Blake3).ok()?;

                    Some(FileHeader {
                        relative_path,
                        size: meta.len(),
                        mtime_seconds,
                        permissions: 0,
                        checksum,
                    })
                })
                .collect()
        } else {
            // No checksums - use sequential iteration (faster for metadata-only)
            entries
                .into_iter()
                .filter_map(|entry| {
                    let path = entry.path();
                    let rel = path.strip_prefix(&root_path).ok()?;
                    let relative_path = rel
                        .iter()
                        .map(|c| c.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");

                    let meta = std::fs::metadata(path).ok()?;
                    let mtime_seconds = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);

                    Some(FileHeader {
                        relative_path,
                        size: meta.len(),
                        mtime_seconds,
                        permissions: 0,
                        checksum: vec![],
                    })
                })
                .collect()
        };

        Ok(manifest)
    })
    .await
    .map_err(|err| eyre!("manifest enumeration task failed: {}", err))?
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
        dirs.sort_by_key(|b| std::cmp::Reverse(b.components().count()));
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

fn print_pull_json(report: &RemotePullReport, dest_root: &Path) {
    use serde_json::json;
    let summary = json!({
        "operation": "pull",
        "destination": dest_root.to_string_lossy(),
        "files_transferred": report.summary.as_ref().map(|s| s.files_transferred).unwrap_or(report.files_transferred as u64),
        "bytes_transferred": report.summary.as_ref().map(|s| s.bytes_transferred).unwrap_or(report.bytes_transferred),
        "bytes_zero_copy": report.summary.as_ref().map(|s| s.bytes_zero_copy).unwrap_or(0u64),
        "tcp_fallback": report.summary.as_ref().map(|s| s.tcp_fallback_used).unwrap_or(false),
    });
    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
}

fn print_push_json(report: &RemotePushReport, destination: &str) {
    use serde_json::json;
    let summary = json!({
        "operation": "push",
        "destination": destination,
        "files_requested": report.files_requested.len(),
        "files_transferred": report.summary.files_transferred,
        "bytes_transferred": report.summary.bytes_transferred,
        "bytes_zero_copy": report.summary.bytes_zero_copy,
        "entries_deleted": report.summary.entries_deleted,
        "tcp_fallback": report.summary.tcp_fallback_used,
        "first_payload_ms": report.first_payload_elapsed.map(|d| d.as_millis() as u64),
    });
    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
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
