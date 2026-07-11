use crate::cli::TransferArgs;
use eyre::Result;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, MissedTickBehavior};

use blit_app::transfers::remote::{
    apply_pull_mirror_purge, run_pull_sync, run_remote_push, LocalPurgeStats, PullExecutionOutcome,
    PullSyncExecution, PushExecution,
};
use blit_core::remote::pull::PullSyncOptions;
use blit_core::remote::transfer::{ProgressEvent, ProgressTotals, RemoteTransferProgress};
use blit_core::remote::{RemoteEndpoint, RemotePullReport};

use blit_app::endpoints::{format_remote_endpoint, Endpoint};

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
        // w6-1: fold through the shared accumulator in blit-core — the
        // per-direction folding rules (and the CLI's byte double-count
        // on TCP pulls, design-1) are gone with the contract.
        let mut totals = ProgressTotals::default();
        let mut prev_bytes = 0u64;
        let mut prev_instant = start;
        let mut ticker = interval(Duration::from_secs(1));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                biased;
                event = rx.recv() => {
                    match event {
                        Some(event) => {
                            totals.apply(&event);
                            if let ProgressEvent::FileComplete { path } = &event {
                                if json {
                                    // `bytes` stays in the JSON shape for
                                    // stream compatibility; per-event bytes
                                    // no longer exist under the contract
                                    // (they ride Payload events), so it is
                                    // always 0.
                                    eprintln!(
                                        "{{\"event\":\"file_complete\",\"path\":\"{}\",\"bytes\":0}}",
                                        path.replace('\\', "\\\\").replace('"', "\\\""),
                                    );
                                } else if verbose {
                                    println!("{}", path);
                                }
                            }
                        }
                        None => break,
                    }
                }
                _ = ticker.tick() => {
                    if totals.started() {
                        let now = Instant::now();
                        let elapsed = now.duration_since(start).as_secs_f64().max(1e-6);
                        let window_elapsed = now.duration_since(prev_instant).as_secs_f64().max(1e-6);
                        let window_bytes = totals.bytes.saturating_sub(prev_bytes);
                        let avg_bps = (totals.bytes as f64) / elapsed;
                        let current_bps = (window_bytes as f64) / window_elapsed;
                        if json {
                            eprintln!(
                                "{{\"event\":\"progress\",\"files\":{},\"total_files\":{},\"bytes_copied\":{},\"avg_bytes_sec\":{:.0},\"current_bytes_sec\":{:.0}}}",
                                totals.files, totals.manifest_files, totals.bytes, avg_bps, current_bps
                            );
                        } else {
                            let avg_mib = avg_bps / (1024.0 * 1024.0);
                            let current_mib = current_bps / (1024.0 * 1024.0);
                            println!(
                                "[progress] {}/{} files \u{2022} {:.2} MiB copied \u{2022} {:.2} MiB/s avg \u{2022} {:.2} MiB/s current",
                                totals.files,
                                totals.manifest_files,
                                totals.bytes as f64 / (1024.0 * 1024.0),
                                avg_mib,
                                current_mib,
                            );
                        }
                        prev_instant = now;
                        prev_bytes = totals.bytes;
                    } else if totals.manifest_files > 0 {
                        if json {
                            eprintln!(
                                "{{\"event\":\"manifest\",\"total_files\":{}}}",
                                totals.manifest_files
                            );
                        } else {
                            println!(
                                "[progress] manifest enumerated {} file(s)\u{2026}",
                                totals.manifest_files
                            );
                        }
                    }
                }
            }
        }

        if totals.started() && !suppress_final_line {
            let elapsed = start.elapsed().as_secs_f64().max(1e-6);
            let avg_bps = (totals.bytes as f64) / elapsed;
            if json {
                eprintln!(
                    "{{\"event\":\"final\",\"files_transferred\":{},\"total_bytes\":{},\"avg_bytes_sec\":{:.0}}}",
                    totals.files, totals.bytes, avg_bps
                );
            } else {
                let avg_mib = avg_bps / (1024.0 * 1024.0);
                println!(
                    "[progress] final: {} file(s) transferred \u{2022} {:.2} MiB total \u{2022} {:.2} MiB/s avg",
                    totals.files,
                    totals.bytes as f64 / (1024.0 * 1024.0),
                    avg_mib,
                );
            }
        } else if !totals.started() && totals.manifest_files > 0 {
            if json {
                eprintln!(
                    "{{\"event\":\"manifest\",\"total_files\":{}}}",
                    totals.manifest_files
                );
            } else {
                println!(
                    "[progress] manifest enumerated {} file(s)",
                    totals.manifest_files
                );
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
    run_remote_push_transfer_inner(
        args,
        source,
        remote,
        mirror_mode,
        blit_core::generated::ComparisonMode::SizeMtime,
        false,
    )
    .await
    .map(|_| ())
}

/// R51-F4: move's variant of [`run_remote_push_transfer`]. Returns
/// the push summary instead of printing inline so the caller can
/// defer output until after source-delete.
///
/// codex otp-10a F1: move pushes with `IgnoreTimes` (transfer every
/// file unconditionally). Move deletes the source on success, so a
/// compare-mode skip of a same-size file whose content differs would
/// destroy the only copy of the source bytes; always-transfer makes
/// the delete safe by construction. Copy/mirror keep `SizeMtime`
/// (whose same-size dest-newer skip is the standing owner question).
pub async fn run_remote_push_transfer_deferred(
    args: &TransferArgs,
    source: Endpoint,
    remote: RemoteEndpoint,
    mirror_mode: bool,
) -> Result<DeferredPushState> {
    run_remote_push_transfer_inner(
        args,
        source,
        remote,
        mirror_mode,
        blit_core::generated::ComparisonMode::IgnoreTimes,
        true,
    )
    .await
}

pub struct DeferredPushState {
    pub summary: blit_core::generated::TransferSummary,
    pub destination: String,
}

pub fn print_deferred_push_result(args: &TransferArgs, state: &DeferredPushState) {
    if args.json {
        print_push_json(&state.summary, &state.destination);
    } else {
        describe_push_result(&state.summary, &state.destination);
    }
}

/// otp-10a: a failed session names the file a fault touched
/// (D-2026-07-09-1) — extract that end-of-operation summary from the
/// error chain, so the operator sees which file to re-run for without
/// digging through it. Applies to both fault shapes: a `SessionFault`
/// raised by a running session and a `TransferOpenRefusal` from a
/// session that never opened (whose inner fault never names a file —
/// `end_of_operation_summary` then returns `None`). Extraction is
/// split from the printing so the chain-walking is unit-pinned
/// (codex otp-10a F7).
fn session_fault_summary(err: &eyre::Report) -> Option<String> {
    use blit_core::remote::transfer::session_client::TransferOpenRefusal;
    use blit_core::transfer_session::SessionFault;
    err.chain()
        .find_map(|cause| {
            cause
                .downcast_ref::<SessionFault>()
                .or_else(|| cause.downcast_ref::<TransferOpenRefusal>().map(|r| &r.0))
        })
        .and_then(|fault| fault.end_of_operation_summary())
}

fn emit_session_fault_summary(err: &eyre::Report) {
    if let Some(line) = session_fault_summary(err) {
        eprintln!("{line}");
    }
}

async fn run_remote_push_transfer_inner(
    args: &TransferArgs,
    source: Endpoint,
    remote: RemoteEndpoint,
    mirror_mode: bool,
    compare_mode: blit_core::generated::ComparisonMode,
    defer_output: bool,
) -> Result<DeferredPushState> {
    let show_progress = args.effective_progress() || args.verbose;
    let (progress_handle, progress_task) = spawn_progress_monitor_with_options(
        show_progress,
        args.verbose,
        args.json,
        defer_output, // R53-F1: suppress the final progress line on move
    );

    // Filter parity: the wire FilterSpec rides `SessionOpen.filter`
    // (otp-10a); the session's SOURCE end applies it through the
    // universal `FilteredSource` chokepoint and the daemon DESTINATION
    // scopes mirror deletions with it — identical rules to what
    // `--exclude/--include/--min-size/...` produce on pull.
    let filter_spec = super::build_filter_spec(args)?;

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

    let execution = PushExecution {
        source,
        remote: remote.clone(),
        filter: Some(filter_spec),
        mirror_mode,
        mirror_kind,
        force_grpc: args.force_grpc,
        trace_data_plane: args.trace_data_plane,
        require_complete_scan: mirror_mode,
        resume: args.resume,
        resume_block_size: 0, // destination default (1 MiB)
        compare_mode,
        remote_label: format_remote_endpoint(&remote),
    };

    // Push has no caller-side destructive step (mirror-delete is
    // daemon-side and surfaces via the summary), so unlike the pull
    // lifecycle there is no need to drop the progress handle
    // *before* a follow-up library call — the monitor's lifetime
    // already matches the RPC.
    let outcome = run_remote_push(execution, progress_handle.as_ref()).await;

    drop(progress_handle);
    if let Some(task) = progress_task {
        let _ = task.await;
    }

    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(err) => {
            emit_session_fault_summary(&err);
            return Err(err);
        }
    };
    let state = DeferredPushState {
        summary: outcome.summary,
        destination: outcome.destination,
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

fn print_push_json(summary: &blit_core::generated::TransferSummary, destination: &str) {
    use serde_json::json;
    // otp-10a: the push verb reports the session's destination-computed
    // summary. Keys that only the deleted driver could fill
    // (files_requested, bytes_zero_copy, first_payload_ms) are gone;
    // files_resumed is new with push-side --resume.
    let summary = json!({
        "operation": "push",
        "destination": destination,
        "files_transferred": summary.files_transferred,
        "bytes_transferred": summary.bytes_transferred,
        "files_resumed": summary.files_resumed,
        "entries_deleted": summary.entries_deleted,
        "tcp_fallback": summary.in_stream_carrier_used,
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

pub fn describe_push_result(summary: &blit_core::generated::TransferSummary, destination: &str) {
    // otp-10a: the session's DESTINATION is the scorer; the old
    // negotiation-phase lines (file counts scheduled, data port) died
    // with the per-direction driver. `[gRPC fallback]` keeps its exact
    // wording — it marks the session's in-stream byte carrier now.
    if summary.files_transferred == 0 && summary.files_resumed == 0 {
        println!(
            "Remote already up to date; nothing to upload ({}).",
            destination
        );
    }
    let resumed = if summary.files_resumed > 0 {
        format!(" ({} resumed block-wise)", summary.files_resumed)
    } else {
        String::new()
    };
    println!(
        "Transfer complete: {} file(s), {} bytes{}{}.",
        summary.files_transferred,
        summary.bytes_transferred,
        resumed,
        if summary.in_stream_carrier_used {
            " [gRPC fallback]"
        } else {
            ""
        }
    );
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

#[cfg(test)]
mod session_fault_summary_tests {
    use super::session_fault_summary;
    use blit_core::generated::session_error::Code;
    use blit_core::remote::transfer::session_client::TransferOpenRefusal;
    use blit_core::transfer_session::SessionFault;

    fn fault_with_path(path: &str) -> SessionFault {
        SessionFault {
            code: Code::Internal,
            message: "'big.bin' hit EOF with 42 bytes still promised".into(),
            local_build_id: String::new(),
            peer_build_id: String::new(),
            peer_notified: true,
            relative_path: Some(path.into()),
            io_kind: None,
        }
    }

    /// The verb-level print's contract (D-2026-07-09-1 Q2): the
    /// summary extracted from a real, context-wrapped verb error names
    /// the affected file and suggests a re-run.
    #[test]
    fn names_the_file_and_suggests_a_rerun_through_context_layers() {
        let err = eyre::Report::new(fault_with_path("big.bin"))
            .wrap_err("pushing to 127.0.0.1:9031:/test/");
        let line = session_fault_summary(&err).expect("fault with a path yields a summary");
        assert!(line.contains("affected file: big.bin"), "got: {line}");
        assert!(line.contains("re-run"), "got: {line}");
    }

    /// An open-time refusal wraps its fault in `TransferOpenRefusal`;
    /// the extraction must reach through it. Open faults carry no file
    /// (nothing transferred yet) — no summary, nothing printed.
    #[test]
    fn open_refusals_without_a_file_yield_no_summary() {
        let mut fault = fault_with_path("x");
        fault.relative_path = None;
        let err = eyre::Report::new(TransferOpenRefusal(fault)).wrap_err("pushing to host:/mod/");
        assert!(session_fault_summary(&err).is_none());
    }

    /// A refusal whose inner fault DOES name a file still summarizes —
    /// the downcast reaches the inner fault through the wrapper.
    #[test]
    fn open_refusal_with_a_file_summarizes_through_the_wrapper() {
        let err = eyre::Report::new(TransferOpenRefusal(fault_with_path("nested/f.txt")))
            .wrap_err("pushing");
        let line = session_fault_summary(&err).expect("inner fault names a file");
        assert!(line.contains("affected file: nested/f.txt"), "got: {line}");
    }

    /// Non-session errors (connect failures, arg errors) never print a
    /// transfer-abort block.
    #[test]
    fn plain_errors_yield_no_summary() {
        assert!(session_fault_summary(&eyre::eyre!("connection refused")).is_none());
    }
}
