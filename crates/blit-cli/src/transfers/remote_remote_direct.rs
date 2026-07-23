use crate::cli::TransferArgs;
use eyre::Result;

use blit_app::transfers::remote::{
    run_delegated_pull, run_delegated_pull_until_started, DelegatedPullExecution,
    DelegatedPullOutcome,
};
use blit_core::generated::DelegatedPullSummary;
use blit_core::remote::transfer::{operation_spec::DelegatedSpecOptions, TransferLifecycleTrace};
use blit_core::remote::RemoteEndpoint;

use super::remote::spawn_progress_monitor_with_options;
use blit_app::endpoints::format_remote_endpoint;

/// CLI-facing alias for the library's delegated-pull outcome.
/// Field shape unchanged across the A.0 move; preserves the
/// public name `DeferredDelegatedState` that `transfers::mod`
/// already imports.
pub type DeferredDelegatedState = DelegatedPullOutcome;

pub async fn run_remote_to_remote_direct(
    args: &TransferArgs,
    src: RemoteEndpoint,
    dst: RemoteEndpoint,
    mirror_mode: bool,
    move_verb: bool,
    lifecycle_trace: &TransferLifecycleTrace,
) -> Result<()> {
    run_remote_to_remote_direct_inner(
        args,
        src,
        dst,
        mirror_mode,
        move_verb,
        false, // defer_output
        lifecycle_trace,
    )
    .await
    .map(|_| ())
}

/// R51-F4: move's variant of [`run_remote_to_remote_direct`].
/// Returns the delegated summary instead of printing inline so
/// the caller can defer output until after source-delete.
pub async fn run_remote_to_remote_direct_deferred(
    args: &TransferArgs,
    src: RemoteEndpoint,
    dst: RemoteEndpoint,
    mirror_mode: bool,
    move_verb: bool,
    lifecycle_trace: &TransferLifecycleTrace,
) -> Result<DeferredDelegatedState> {
    run_remote_to_remote_direct_inner(
        args,
        src,
        dst,
        mirror_mode,
        move_verb,
        true, // defer_output
        lifecycle_trace,
    )
    .await
}

/// The delegated wire options, extracted for pinnability (codex
/// otp-10b-2 F2). A MOVE deletes the remote source after the
/// delegated transfer, so its compare must never produce a
/// metadata-shaped skip: `ignore_times` is forced on unless the user
/// asked for `--checksum` (a content-proven skip is safe) —
/// `delegated_spec_from_options` maps ignore_times with top
/// precedence, so this reproduces the verbs' `move_comparison_mode`
/// through the delegated wire spec. Pre-fix a delegated move rode the
/// SizeMtime default: a same-size changed destination file was
/// skipped and the source-delete destroyed the only copy — the exact
/// otp-10a F1 hazard on the one route the cutover slice missed.
/// `require_complete_scan` rides the same flag (R49-F2).
fn delegated_pull_options(
    args: &TransferArgs,
    filter_spec: blit_core::generated::FilterSpec,
    mirror_mode: bool,
    move_verb: bool,
) -> DelegatedSpecOptions {
    DelegatedSpecOptions {
        force_grpc: args.force_grpc,
        mirror_mode,
        delete_all_scope: args.delete_scope_all(),
        filter: Some(filter_spec),
        size_only: args.size_only,
        ignore_times: args.ignore_times || (move_verb && !args.checksum),
        ignore_existing: args.ignore_existing,
        force: args.force,
        checksum: args.checksum,
        resume: args.resume,
        block_size: 0,
        require_complete_scan: move_verb,
        drop_windows_metadata: args.drop_windows_metadata,
    }
}

// `DeferredDelegatedState` is now a type alias for
// `blit_app::transfers::remote::DelegatedPullOutcome` (see the
// top of this file). Same field shape, same callers — the
// orchestration body that builds it lives in `blit-app` after
// this A.0 sub-slice.

pub fn print_deferred_delegated_result(args: &TransferArgs, state: &DeferredDelegatedState) {
    if args.json {
        print_delegated_json(&state.summary, &state.src, &state.dst);
    } else {
        describe_delegated_result(&state.summary, &state.src, &state.dst);
    }
}

async fn run_remote_to_remote_direct_inner(
    args: &TransferArgs,
    src: RemoteEndpoint,
    dst: RemoteEndpoint,
    mirror_mode: bool,
    move_verb: bool,
    defer_output: bool,
    lifecycle_trace: &TransferLifecycleTrace,
) -> Result<DeferredDelegatedState> {
    let filter_spec = super::build_filter_spec(args)?;
    let options = delegated_pull_options(args, filter_spec, mirror_mode, move_verb);

    let show_progress = args.effective_progress() || args.verbose;
    let (progress_handle, progress_task) = spawn_progress_monitor_with_options(
        show_progress,
        args.verbose,
        args.json,
        defer_output, // R53-F1: suppress final progress line on move
    );

    let dst_label = format_remote_endpoint(&dst);
    let execution = DelegatedPullExecution {
        src,
        dst,
        options,
        trace_data_plane: args.trace_data_plane,
        dst_label,
        // `--detach` is only honored on remote→remote
        // delegated pulls (this code path). `run_transfer`
        // rejects the flag on push / pull / pull_sync routes
        // upstream, so we don't need to gate it here — but
        // the daemon also refuses to honor it on those
        // RPCs, so a misbehaving caller can't escape the
        // CLI in-byte-path guarantee.
        detach: args.detach,
        lifecycle_trace: lifecycle_trace.clone(),
    };

    // --detach exit-after-Started path. Opens the stream
    // just long enough to learn the daemon-assigned
    // transfer_id (which arrives on the Started event after
    // m-jobs-3) and then drops the receiver. The daemon's
    // tx.closed race is disarmed by `detach=true`, so the
    // transfer continues. We synthesize a zero-summary
    // outcome so the existing callers (`run_remote_to_remote_direct`
    // which discards it; `_deferred` which is rejected up
    // front for detach via run_move's gate) see a stable
    // shape.
    if args.detach {
        // Tear down the progress monitor before printing —
        // same posture as the non-detach success path, so
        // any in-flight `[progress]` line doesn't get
        // interleaved with the detach output.
        drop(progress_handle);
        if let Some(task) = progress_task {
            let _ = task.await;
        }

        let dst_for_state = execution.dst.clone();
        // The cancel/status hint references the destination
        // host as the argument to `blit jobs`. Derive it
        // from the parsed `RemoteEndpoint` rather than the
        // raw CLI input — string-splitting `args.destination`
        // breaks `host:port:/module/path` (port dropped) and
        // bracketed IPv6 (`[::1]:9031:/m/p` truncates to
        // just `[`). `host_port_display` handles both via
        // the same helper `RemoteEndpoint::display` already
        // uses.
        let dst_host_hint = dst_for_state.host_port_display();

        let (started, _dst) = run_delegated_pull_until_started(execution).await?;
        let transfer_id = started.transfer_id.clone();
        let summary = DelegatedPullSummary {
            files_transferred: 0,
            bytes_transferred: 0,
            bytes_zero_copy: 0,
            tcp_fallback_used: false,
            entries_deleted: 0,
            source_peer_observed: started.source_data_plane_endpoint.clone(),
        };
        let state = DeferredDelegatedState {
            summary,
            src: dst_for_state.clone(), // source endpoint not surfaced on Started
            dst: dst_for_state,
        };
        super::render_result(lifecycle_trace, || {
            if args.json {
                print_detach_json(&transfer_id);
            } else {
                print_detach_human(&transfer_id, &dst_host_hint);
            }
            Ok(())
        })?;
        return Ok(state);
    }

    // CLI-side presentation hook for the destination's `Started`
    // event. M-C's `AppProgressEvent` reshape will replace the
    // callback with a stream variant that both CLI and TUI
    // handle uniformly; the closure is the stopgap.
    let verbose_human = args.verbose && !args.json;
    let outcome = run_delegated_pull(execution, progress_handle.as_ref(), |started| {
        if verbose_human {
            eprintln!(
                "blit: delegation: destination pulling from {} ({} stream(s))",
                started.source_data_plane_endpoint, started.stream_count
            );
        }
    })
    .await;

    drop(progress_handle);
    if let Some(task) = progress_task {
        let _ = task.await;
    }

    let state = outcome?;
    if !defer_output {
        super::render_result(lifecycle_trace, || {
            print_deferred_delegated_result(args, &state);
            Ok(())
        })?;
    }
    Ok(state)
}

fn print_detach_human(transfer_id: &str, dst_host_hint: &str) {
    eprintln!(
        "Detached transfer {transfer_id}; daemon owns it to completion or cancel.\n  cancel: blit jobs cancel {dst_host_hint} {transfer_id}\n  status: blit jobs list {dst_host_hint}"
    );
}

fn print_detach_json(transfer_id: &str) {
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "outcome": "detached",
            "transfer_id": transfer_id,
        }))
        .unwrap_or_default()
    );
}

fn print_delegated_json(
    summary: &DelegatedPullSummary,
    src: &RemoteEndpoint,
    dst: &RemoteEndpoint,
) {
    use serde_json::json;
    let body = json!({
        "operation": "delegated_pull",
        "source": format_remote_endpoint(src),
        "destination": format_remote_endpoint(dst),
        "files_transferred": summary.files_transferred,
        "bytes_transferred": summary.bytes_transferred,
        "bytes_zero_copy": summary.bytes_zero_copy,
        "entries_deleted": summary.entries_deleted,
        "tcp_fallback": summary.tcp_fallback_used,
        "source_peer_observed": summary.source_peer_observed,
    });
    println!("{}", serde_json::to_string_pretty(&body).unwrap());
}

fn describe_delegated_result(
    summary: &DelegatedPullSummary,
    src: &RemoteEndpoint,
    dst: &RemoteEndpoint,
) {
    println!(
        "Delegated remote-to-remote transfer complete: {} file(s), {} bytes (zero-copy {} bytes){} from {} to {}.",
        summary.files_transferred,
        summary.bytes_transferred,
        summary.bytes_zero_copy,
        if summary.tcp_fallback_used {
            " [gRPC fallback]"
        } else {
            ""
        },
        format_remote_endpoint(src),
        format_remote_endpoint(dst)
    );
    if summary.entries_deleted > 0 {
        println!(
            "Mirror purge removed {} entrie(s) on destination.",
            summary.entries_deleted
        );
    }
}

// Unit tests for `destination_spec_fields`,
// `report_bytes_progress`, and `DelegatedBytesProgressState`
// moved to `blit_app::transfers::remote::tests` alongside the
// implementations — see the a0-remote-helpers round-1 reopen
// for the test-locality precedent.

#[cfg(test)]
mod delegated_options_tests {
    use super::*;
    use blit_core::generated::{ComparisonMode, FilterSpec};
    use blit_core::remote::transfer::operation_spec::delegated_spec_from_options;
    use blit_core::remote::RemoteEndpoint;

    /// Minimal args for the option-mapping pins; only the compare
    /// flags vary per test.
    fn args(checksum: bool, size_only: bool) -> TransferArgs {
        TransferArgs {
            source: "a:/m/".into(),
            destination: "b:/m/".into(),
            dry_run: false,
            checksum,
            size_only,
            ignore_times: false,
            ignore_existing: false,
            force: false,
            verbose: false,
            progress: false,
            yes: true,
            workers: None,
            trace_data_plane: false,
            force_grpc: false,
            detach: false,
            resume: false,
            drop_windows_metadata: false,
            retry: 0,
            wait: 5,
            null: false,
            json: false,
            exclude: vec![],
            include: vec![],
            files_from: None,
            min_size: None,
            max_size: None,
            min_age: None,
            max_age: None,
            delete_scope: "subset".into(),
        }
    }

    fn wire_compare(options: &DelegatedSpecOptions) -> i32 {
        let ep = RemoteEndpoint::parse("h:9031:/m/").expect("endpoint");
        delegated_spec_from_options(&ep, options)
            .expect("spec")
            .compare_mode
    }

    /// codex otp-10b-2 F2: a delegated MOVE must never ride a
    /// metadata-shaped compare — the dst daemon would skip a
    /// same-size changed file and the CLI then deletes the remote
    /// source (the otp-10a F1 data loss on the delegated route).
    /// The wire spec must carry IGNORE_TIMES (or CHECKSUM when the
    /// user asked — a content-proven skip is safe).
    #[test]
    fn delegated_move_transfers_unconditionally_on_the_wire() {
        let opts = delegated_pull_options(&args(false, false), FilterSpec::default(), false, true);
        assert!(opts.ignore_times, "move must force ignore_times");
        assert!(opts.require_complete_scan, "move refuses partial scans");
        assert_eq!(wire_compare(&opts), ComparisonMode::IgnoreTimes as i32);

        let with_checksum =
            delegated_pull_options(&args(true, false), FilterSpec::default(), false, true);
        assert!(
            !with_checksum.ignore_times,
            "--checksum is the one safe skip: content-proven equality"
        );
        assert_eq!(
            wire_compare(&with_checksum),
            ComparisonMode::Checksum as i32
        );
    }

    /// Copy/mirror keep the user's flags untouched (the old
    /// delegated behavior) — no forced ignore_times, no scan gate.
    #[test]
    fn delegated_copy_passes_compare_flags_through() {
        let opts = delegated_pull_options(&args(false, true), FilterSpec::default(), false, false);
        assert!(!opts.ignore_times);
        assert!(!opts.require_complete_scan);
        assert_eq!(wire_compare(&opts), ComparisonMode::SizeOnly as i32);

        let default_opts =
            delegated_pull_options(&args(false, false), FilterSpec::default(), false, false);
        assert_eq!(
            wire_compare(&default_opts),
            ComparisonMode::SizeMtime as i32
        );
    }
}
