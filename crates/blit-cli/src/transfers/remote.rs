use crate::cli::TransferArgs;
use eyre::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, MissedTickBehavior};

use blit_app::transfers::remote::{
    apply_pull_mirror_purge, run_pull_sync, LocalPurgeStats, PullExecutionOutcome,
    PullSyncExecution,
};
use blit_core::remote::pull::PullSyncOptions;
use blit_core::remote::transfer::source::{
    FilteredSource, FsTransferSource, RemoteTransferSource, TransferSource,
};
use blit_core::remote::transfer::{ProgressEvent, RemoteTransferProgress};
use blit_core::remote::{
    RemoteEndpoint, RemotePullClient, RemotePullReport, RemotePushClient, RemotePushReport,
};

use super::endpoints::{format_remote_endpoint, Endpoint};

/// CLI-facing alias for the library's pull-outcome struct.
/// Field shape unchanged across the A.0 move — this preserves
/// the public name `DeferredPullState` that `transfers::mod`
/// already imports while letting the orchestration body live
/// in `blit-app`.
pub type DeferredPullState = PullExecutionOutcome;

/// Spawn the per-transfer progress monitor. `suppress_final_line=true`
/// lets move callers gate the post-transfer "[progress] final: …"
/// line so a transfer-looking success summary doesn't appear on
/// stdout before source-delete runs (and possibly fails). The
/// per-file / per-second progress lines still emit because the
/// user wants liveness signal during the transfer; only the
/// post-transfer "final:" line is gated (R53-F1).
pub(crate) fn spawn_progress_monitor_with_options(
    enabled: bool,
    verbose: bool,
    json: bool,
    suppress_final_line: bool,
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

        if started && !suppress_final_line {
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
        } else if !started && total_manifest > 0 {
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
    run_remote_push_transfer_inner(args, source, remote, mirror_mode, false)
        .await
        .map(|_| ())
}

/// R51-F4: move's variant of [`run_remote_push_transfer`]. Returns
/// the push report instead of printing inline so the caller can
/// defer output until after source-delete.
pub async fn run_remote_push_transfer_deferred(
    args: &TransferArgs,
    source: Endpoint,
    remote: RemoteEndpoint,
    mirror_mode: bool,
) -> Result<DeferredPushState> {
    run_remote_push_transfer_inner(args, source, remote, mirror_mode, true).await
}

pub struct DeferredPushState {
    pub report: blit_core::remote::push::RemotePushReport,
    pub destination: String,
    pub show_progress: bool,
}

pub fn print_deferred_push_result(args: &TransferArgs, state: &DeferredPushState) {
    if args.json {
        print_push_json(&state.report, &state.destination);
    } else {
        describe_push_result(&state.report, &state.destination, state.show_progress);
    }
}

async fn run_remote_push_transfer_inner(
    args: &TransferArgs,
    source: Endpoint,
    remote: RemoteEndpoint,
    mirror_mode: bool,
    defer_output: bool,
) -> Result<DeferredPushState> {
    let mut client = RemotePushClient::connect(remote.clone())
        .await
        .with_context(|| format!("connecting to {}", remote.control_plane_uri()))?;

    let show_progress = args.effective_progress() || args.verbose;
    let (progress_handle, progress_task) = spawn_progress_monitor_with_options(
        show_progress,
        args.verbose,
        args.json,
        defer_output, // R53-F1: suppress the final progress line on move
    );

    // Filter built by orchestrator-side helper from CLI args. The
    // universal `FilteredSource` wrapper (single chokepoint, see
    // remote/transfer/source.rs) applies it identically to local→remote,
    // remote→remote, and local→local — full src/dst combination parity.
    let filter = super::build_filter(args)?;
    let inner: Arc<dyn TransferSource> = match source {
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
    let transfer_source: Arc<dyn TransferSource> =
        Arc::new(FilteredSource::new(inner, filter.clone()));

    // R59 #1 F2: translate the user's --delete-scope flag to the wire
    // MirrorMode enum. Default to FilteredSubset so `push --include …
    // --mirror` deletes only files in scope. R59 #1 F1: require a
    // complete source scan for any mirror operation — a partial scan
    // could cause silent dest-side data loss when the daemon purges
    // entries it (wrongly) thinks are absent from the source.
    let mirror_kind = if mirror_mode {
        if args.delete_scope_all() {
            blit_core::generated::MirrorMode::All
        } else {
            blit_core::generated::MirrorMode::FilteredSubset
        }
    } else {
        blit_core::generated::MirrorMode::Off
    };
    let require_complete_scan = mirror_mode;

    let push_result = client
        .push(
            transfer_source.clone(),
            &filter,
            mirror_mode,
            mirror_kind,
            args.force_grpc,
            require_complete_scan,
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
    let destination = format_remote_endpoint(&remote);

    let state = DeferredPushState {
        report,
        destination,
        show_progress,
    };
    if !defer_output {
        print_deferred_push_result(args, &state);
    }
    Ok(state)
}

pub async fn run_remote_pull_transfer(
    args: &TransferArgs,
    remote: RemoteEndpoint,
    dest_root: &Path,
    mirror_mode: bool,
    require_complete_scan: bool,
) -> Result<()> {
    run_remote_pull_transfer_inner(
        args,
        remote,
        dest_root,
        mirror_mode,
        require_complete_scan,
        false, // emit success summary inline (copy/mirror default)
    )
    .await
    .map(|_| ())
}

/// R51-F4: move's variant of `run_remote_pull_transfer` — runs the
/// transfer and the (no-op for mirror=false) purge, but does NOT
/// emit the success summary. Caller is responsible for printing
/// after source-delete completes (or refusing to print on
/// source-delete failure). Returns the same `(report, purge_stats)`
/// state the inline printer uses so the deferred print can
/// produce byte-identical output.
pub async fn run_remote_pull_transfer_deferred(
    args: &TransferArgs,
    remote: RemoteEndpoint,
    dest_root: &Path,
    mirror_mode: bool,
    require_complete_scan: bool,
) -> Result<DeferredPullState> {
    run_remote_pull_transfer_inner(
        args,
        remote,
        dest_root,
        mirror_mode,
        require_complete_scan,
        true,
    )
    .await
}

// `DeferredPullState` is now a type alias for
// `blit_app::transfers::remote::PullExecutionOutcome` (see the
// top of this file). Same field shape, same callers — the
// orchestration body that builds it lives in `blit-app` after
// this A.0 sub-slice.

pub fn print_deferred_pull_result(args: &TransferArgs, state: &DeferredPullState) {
    if args.json {
        print_pull_json(
            &state.report,
            &state.actual_dest,
            state.mirror_purge_stats.as_ref(),
        );
    } else {
        describe_pull_result(&state.report, &state.actual_dest);
        if let Some(stats) = state.mirror_purge_stats.as_ref() {
            if stats.files_deleted > 0 || stats.dirs_deleted > 0 {
                println!(
                    "Mirror purge removed {} file(s) and {} directorie(s).",
                    stats.files_deleted, stats.dirs_deleted
                );
            }
        }
    }
}

async fn run_remote_pull_transfer_inner(
    args: &TransferArgs,
    remote: RemoteEndpoint,
    dest_root: &Path,
    mirror_mode: bool,
    require_complete_scan: bool,
    defer_output: bool,
) -> Result<DeferredPullState> {
    // Filter parity (Step 4B): build the wire FilterSpec here and
    // ship it on TransferOperationSpec. The daemon applies the same
    // rules during its source enumeration, so the file set the daemon
    // sees matches what `--exclude/--include/--min-size/...` would
    // have produced for an equivalent push.
    let filter_spec = super::build_filter_spec(args)?;

    let show_progress = args.effective_progress() || args.verbose;
    let (progress_handle, progress_task) = spawn_progress_monitor_with_options(
        show_progress,
        args.verbose,
        args.json,
        defer_output, // R53-F1: suppress final progress line on move
    );

    let execution = PullSyncExecution {
        remote: remote.clone(),
        dest_root: dest_root.to_path_buf(),
        options: PullSyncOptions {
            force_grpc: args.force_grpc,
            mirror_mode,
            delete_all_scope: args.delete_scope_all(),
            filter: Some(filter_spec),
            size_only: args.size_only,
            ignore_times: args.ignore_times,
            ignore_existing: args.ignore_existing,
            force: args.force,
            checksum: args.checksum,
            resume: args.resume,
            block_size: 0, // Use default (1 MiB)
            // R49-F2: move arms set this true so the daemon refuses
            // partial source scans before we delete the remote source.
            require_complete_scan,
        },
        compute_checksums: args.checksum,
        mirror_mode,
        remote_label: format_remote_endpoint(&remote),
    };

    // Lifecycle (round-2 fix for a0-pull-execution):
    //
    //   1. PullSync RPC with progress monitor live.
    //   2. Tear down progress channel + drain monitor task.
    //   3. Apply mirror-purge in the now-quiet state.
    //   4. Print summary (or defer to the move caller).
    //
    // Round-1 bundled steps 1 and 3 into a single library call,
    // which kept the monitor alive through purge and let stale
    // [progress] ticks emit during destructive cleanup. The
    // library now exposes the two halves separately so the CLI
    // (and TUI) can place the lifecycle boundary at step 2.
    //
    // R53-F1 (`suppress_final_line`) and R46-F6 (purge stats in
    // the same JSON document as the report) both still hold —
    // R46-F6 is about ordering relative to *printing*, which
    // still happens at the very end below.
    let sync_outcome = run_pull_sync(execution, progress_handle.as_ref()).await?;

    drop(progress_handle);
    if let Some(task) = progress_task {
        let _ = task.await;
    }

    let mirror_purge_stats = apply_pull_mirror_purge(&sync_outcome, mirror_mode).await?;

    let state = PullExecutionOutcome {
        report: sync_outcome.report,
        actual_dest: sync_outcome.actual_dest,
        mirror_purge_stats,
    };

    // R51-F4: when deferred, skip the inline print. The caller
    // (move) prints via `print_deferred_pull_result` after the
    // source-delete step succeeds — so a post-transfer failure
    // never leaves a success-looking JSON document on stdout.
    if !defer_output {
        print_deferred_pull_result(args, &state);
    }

    Ok(state)
}

// `enumerate_local_manifest`, `delete_listed_paths`,
// `LocalPurgeStats`, and the pull-execution orchestration
// (`PullSyncExecution` / `PullSyncOutcome` /
// `PullExecutionOutcome` plus `run_pull_sync` and
// `apply_pull_mirror_purge`) all live in
// `blit_app::transfers::remote`. CLI imports them at the top of
// this file; the inner function above is now a thin wrapper that
// handles clap arg → primitive input translation, the
// progress-monitor lifecycle (drop/await between pull_sync and
// purge), and presentation.

fn print_pull_json(
    report: &RemotePullReport,
    dest_root: &Path,
    mirror_purge_stats: Option<&LocalPurgeStats>,
) {
    use serde_json::json;
    // R46-F6: include mirror-purge stats inside the JSON document so
    // downstream tools see a single self-contained object instead
    // of having human-readable text appended after the JSON.
    let mirror = mirror_purge_stats.map(|s| {
        json!({
            "files_deleted": s.files_deleted,
            "dirs_deleted": s.dirs_deleted,
        })
    });
    let summary = json!({
        "operation": "pull",
        "destination": dest_root.to_string_lossy(),
        "files_transferred": report.summary.as_ref().map(|s| s.files_transferred).unwrap_or(report.files_transferred as u64),
        "bytes_transferred": report.summary.as_ref().map(|s| s.bytes_transferred).unwrap_or(report.bytes_transferred),
        "bytes_zero_copy": report.summary.as_ref().map(|s| s.bytes_zero_copy).unwrap_or(0u64),
        "tcp_fallback": report.summary.as_ref().map(|s| s.tcp_fallback_used).unwrap_or(false),
        "mirror_purge": mirror,
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

// R46-F3 safety tests for delete_listed_paths moved alongside
// the implementation in blit_app::transfers::remote::tests.
// The CLI now relies on those library-local tests; this
// module's test surface is reserved for CLI-entry-point
// behavior.
