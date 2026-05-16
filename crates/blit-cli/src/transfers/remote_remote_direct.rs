use crate::cli::TransferArgs;
use eyre::Result;

use blit_app::transfers::remote::{
    run_delegated_pull, DelegatedPullExecution, DelegatedPullOutcome,
};
use blit_core::generated::DelegatedPullSummary;
use blit_core::remote::pull::PullSyncOptions;
use blit_core::remote::RemoteEndpoint;

use super::endpoints::format_remote_endpoint;
use super::remote::spawn_progress_monitor_with_options;

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
    require_complete_scan: bool,
) -> Result<()> {
    // Copy/mirror callers: `--relay-via-cli` is a valid escape
    // hatch, so error messages mention it.
    run_remote_to_remote_direct_inner(
        args,
        src,
        dst,
        mirror_mode,
        require_complete_scan,
        false, // defer_output
        true,  // relay_fallback_suggestable
    )
    .await
    .map(|_| ())
}

/// R51-F4: move's variant of [`run_remote_to_remote_direct`].
/// Returns the delegated summary instead of printing inline so
/// the caller can defer output until after source-delete.
///
/// R53-F2: move refuses `--relay-via-cli` (R50-F1), so error
/// messages must not point users at it — they'd be sent to a
/// flag the same command rejects.
pub async fn run_remote_to_remote_direct_deferred(
    args: &TransferArgs,
    src: RemoteEndpoint,
    dst: RemoteEndpoint,
    mirror_mode: bool,
    require_complete_scan: bool,
) -> Result<DeferredDelegatedState> {
    run_remote_to_remote_direct_inner(
        args,
        src,
        dst,
        mirror_mode,
        require_complete_scan,
        true,  // defer_output
        false, // relay_fallback_suggestable — move refuses --relay-via-cli
    )
    .await
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
    require_complete_scan: bool,
    defer_output: bool,
    relay_fallback_suggestable: bool,
) -> Result<DeferredDelegatedState> {
    let filter_spec = super::build_filter_spec(args)?;
    let options = PullSyncOptions {
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
        block_size: 0,
        // R49-F2: move arms set this true so the daemon refuses
        // partial source scans before we delete the source.
        require_complete_scan,
    };

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
        relay_fallback_suggestable,
        dst_label,
    };

    // CLI-side presentation hook for the destination's `Started`
    // event. M-C's `AppProgressEvent` reshape will replace the
    // callback with a stream variant that both CLI and TUI
    // handle uniformly; the closure is the stopgap.
    let verbose_human = args.verbose && !args.json;
    let outcome = run_delegated_pull(execution, progress_handle.as_ref(), |started| {
        if verbose_human {
            eprintln!(
                "[delegation] destination pulling from {} ({} stream(s))",
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
        print_deferred_delegated_result(args, &state);
    }
    Ok(state)
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
