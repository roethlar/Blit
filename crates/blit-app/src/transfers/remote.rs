//! Remote transfer orchestration helpers.
//!
//! Every remote verb rides the unified transfer session; the old
//! PullSync orchestration (`run_pull_sync`, the local-manifest
//! enumeration, and the daemon-authored delete-list purge) was
//! deleted at otp-10c-2 with the `Push`/`PullSync` RPCs — mirror
//! deletions are in-session (the one delete rule, otp-6b), so no
//! delete list crosses the wire at all.
//!
//! - [`run_remote_pull`] + [`PullExecution`] +
//!   [`PullVerbOutcome`] — pull entry-point orchestration on the
//!   unified transfer session (otp-10b-2): one DESTINATION-role
//!   session, mirror deletions in-session (no post-RPC purge
//!   half — the old split's reason to exist is gone on this
//!   path). Presentation (progress monitor spawn, summary
//!   printing) stays in `blit-cli` until the M-C
//!   `AppProgressEvent` reshape lands.
//!
//! - [`run_delegated_pull`] + [`DelegatedPullExecution`] +
//!   [`DelegatedPullOutcome`] — delegated remote→remote
//!   orchestration. Builds the `DelegatedPullRequest`, connects
//!   to the destination's `BlitClient`, consumes the streamed
//!   payload (ManifestBatch / BytesProgress / Summary / Error),
//!   maps errors via [`map_delegated_error`], and returns the
//!   summary. The `on_started` callback fires once when the
//!   destination emits its `Started` event, giving the caller a
//!   live hook for verbose-mode diagnostics without baking
//!   presentation into the library.
//! - [`run_remote_push`] + [`PushExecution`] +
//!   [`PushExecutionOutcome`] — push entry-point orchestration.
//!   The library wraps the local source root in an
//!   `FsTransferSource` and runs the unified transfer session
//!   as its SOURCE (`run_push_session`, otp-10a) — the old
//!   per-direction `RemotePushClient::push` driver is no longer
//!   reachable from any verb and is deleted at otp-10c. A push
//!   source is local by construction: `--relay-via-cli` (the
//!   one remote-source push shape) was removed at otp-10c-1
//!   (D-2026-07-11-1). The CLI-side progress monitor stays in
//!   `blit-cli` (M-C `AppProgressEvent` reshape is its own
//!   pause point).
//!
//! No further `transfers/remote.rs` orchestration lives in
//! `blit-cli` after this slice — the CLI's `transfers/remote.rs`
//! retains only the clap-arg wrappers and presentation
//! (progress monitor + JSON / human printers).

use blit_core::generated::delegated_pull_error::Phase as DelegatedPullPhase;
use blit_core::generated::delegated_pull_progress::Payload as DelegatedPayload;
use blit_core::generated::{
    BytesProgress, ComparisonMode, DelegatedPullRequest, DelegatedPullStarted,
    DelegatedPullSummary, FilterSpec, MirrorMode, RemoteSourceLocator, TransferSummary,
};
use blit_core::remote::transfer::operation_spec::{
    delegated_spec_from_options, DelegatedSpecOptions,
};
use blit_core::remote::transfer::session_client::{
    run_pull_session, run_push_session, PullSessionOptions, PushSessionOptions,
};
use blit_core::remote::transfer::source::{FsTransferSource, TransferSource};
use blit_core::remote::transfer::RemoteTransferProgress;
use blit_core::remote::{RemoteEndpoint, RemotePath};
use eyre::{bail, eyre, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tonic::Code;

/// Inputs for [`run_remote_push`]. Primitive fields only — no
/// clap, no presentation. CLI builds this from `&TransferArgs`;
/// the TUI builds it directly.
///
/// `source` is the local source root (or single file) to push.
/// It is a `PathBuf` by construction: a push SOURCE is always
/// this host's filesystem — the removed `--relay-via-cli` was
/// the only remote-source push shape (otp-10c-1,
/// D-2026-07-11-1), so a remote source is unrepresentable here.
///
/// `filter` is the wire `FilterSpec` (the CLI builds it via
/// `blit_app::transfers::filter::build_spec`); it rides
/// `SessionOpen.filter`, where the session's SOURCE end applies
/// it through the universal `FilteredSource` chokepoint (R49 /
/// otp-6a) and the DESTINATION scopes mirror deletions with it.
/// `mirror_kind` communicates the user's `--delete-scope`
/// choice (R59 #1 F2: `--mirror --include …` deletes only
/// in-scope entries via `FilteredSubset`).
pub struct PushExecution {
    pub source: PathBuf,
    pub remote: RemoteEndpoint,
    pub filter: Option<FilterSpec>,
    pub mirror_mode: bool,
    pub mirror_kind: MirrorMode,
    pub force_grpc: bool,
    pub trace_data_plane: bool,
    pub require_complete_scan: bool,
    /// otp-10a: negotiate the resume block phase (`--resume`) — changed
    /// destination partials are patched block-wise instead of
    /// re-transferred whole. `resume_block_size` in bytes; 0 lets the
    /// DESTINATION choose.
    pub resume: bool,
    pub resume_block_size: u32,
    /// How the DESTINATION decides which files it needs. Copy/mirror
    /// verbs map their compare flags through the one
    /// `transfers::compare` mapping (otp-10b-2 — identical for push
    /// and pull; the SizeMtime default's same-size dest-newer skip is
    /// the standing owner question in STATE). Move verbs MUST pass
    /// `move_comparison_mode`'s result (codex otp-10a F1): move
    /// deletes the source on success, so every skip must be provably
    /// safe — `IgnoreTimes`, or `Checksum` (content-proven equal).
    pub compare_mode: ComparisonMode,
    /// Skip files that already exist at the destination, whatever
    /// their content (`--ignore-existing`) — the orthogonal compare
    /// axis, riding `SessionOpen.ignore_existing` (otp-10b-2; the old
    /// push driver silently ignored the flag).
    pub ignore_existing: bool,
    pub remote_label: String,
}

/// Output of [`run_remote_push`]. `summary` is the
/// destination-computed session [`TransferSummary`] (contract:
/// the end that wrote the bytes attests to them);
/// `in_stream_carrier_used` carries what the old report called
/// `tcp_fallback_used`. `destination` is the caller-supplied
/// `remote_label` echoed back — the printer consumes it.
/// `show_progress` is intentionally **not** here; it's a
/// CLI-side presentation hint that the CLI threads directly
/// into its own `DeferredPushState`.
pub struct PushExecutionOutcome {
    pub summary: TransferSummary,
    pub destination: String,
}

/// Run a remote push end-to-end (otp-10a: the push-shaped verb on the
/// unified transfer session): wrap the local source root in an
/// `FsTransferSource`, then initiate one SOURCE-role `Transfer`
/// session against the destination daemon via `run_push_session`. No
/// mirror-purge step exists on the push side — mirror deletes happen
/// on the DESTINATION (the one delete rule, otp-6b) and surface
/// through the returned summary's `entries_deleted`.
///
/// A source scan that skips unreadable files fails the call after the
/// transfer (the readable subset still lands) — same posture as the
/// old driver; `blit move`'s source-delete gate relies on that error.
///
/// `progress` is borrowed for the duration of the session. The caller
/// owns the channel + monitor task; this function never spawns or
/// awaits the monitor. Standard lifecycle:
///
/// ```text
/// let (handle, task) = spawn_progress_monitor(...);
/// let outcome = run_remote_push(execution, handle.as_ref()).await?;
/// drop(handle);
/// if let Some(t) = task { let _ = t.await; }
/// ```
///
/// Unlike the pull side, there is no need to split this into
/// pre-/post-purge halves — push has no post-RPC destructive
/// step on the caller's filesystem, so the monitor's lifetime
/// already lines up cleanly with the RPC.
pub async fn run_remote_push(
    execution: PushExecution,
    progress: Option<&RemoteTransferProgress>,
) -> Result<PushExecutionOutcome> {
    let source: Arc<dyn TransferSource> = Arc::new(FsTransferSource::new(execution.source));

    let options = PushSessionOptions {
        compare_mode: execution.compare_mode,
        ignore_existing: execution.ignore_existing,
        require_complete_scan: execution.require_complete_scan,
        // `--force-grpc`: the session's in-stream byte carrier is the
        // gRPC-fallback lane (otp-8).
        in_stream_bytes: execution.force_grpc,
        resume: execution.resume,
        resume_block_size: execution.resume_block_size,
        filter: execution.filter,
        mirror_enabled: execution.mirror_mode,
        mirror_kind: if execution.mirror_mode {
            execution.mirror_kind
        } else {
            MirrorMode::Off
        },
        progress: progress.cloned(),
        trace_data_plane: execution.trace_data_plane,
        ..PushSessionOptions::default()
    };

    let summary = run_push_session(&execution.remote, source, options)
        .await
        .with_context(|| format!("pushing to {}", execution.remote_label))?;

    Ok(PushExecutionOutcome {
        summary,
        destination: execution.remote_label,
    })
}

/// Inputs for [`run_remote_pull`] (otp-10b-2: the pull-shaped verb on
/// the unified transfer session). Primitive fields only — no clap, no
/// presentation. CLI builds this from `&TransferArgs`; the TUI builds
/// it directly. Field semantics match [`PushExecution`] exactly (the
/// one option surface, roles flipped): `filter` rides
/// `SessionOpen.filter` and is applied by the daemon SOURCE through
/// the universal `FilteredSource` chokepoint; `mirror_kind` carries
/// `--delete-scope`; `compare_mode` comes from the one
/// `transfers::compare` mapping.
pub struct PullExecution {
    pub remote: RemoteEndpoint,
    /// Local destination root — or, for a single-file pull, the target
    /// file path itself (the source manifests the file with an empty
    /// relative path, so the session writes AT this path — the old
    /// pull's exact convention).
    pub dest_root: PathBuf,
    pub filter: Option<FilterSpec>,
    pub mirror_mode: bool,
    pub mirror_kind: MirrorMode,
    pub force_grpc: bool,
    pub trace_data_plane: bool,
    /// R49-F2 / otp-9b F1: `blit move` sets this so the session
    /// refuses a partial source scan (`ScanIncomplete` at
    /// ManifestComplete) BEFORE the caller deletes the remote source.
    pub require_complete_scan: bool,
    pub resume: bool,
    pub resume_block_size: u32,
    /// See [`PushExecution::compare_mode`] — the same mapping serves
    /// both verbs; move verbs pass `move_comparison_mode`'s result.
    pub compare_mode: ComparisonMode,
    pub ignore_existing: bool,
    pub remote_label: String,
}

/// Output of [`run_remote_pull`]: the session [`TransferSummary`] this
/// DESTINATION computed (contract: the end that wrote the bytes is the
/// scorer — here that's us) plus the destination root echoed back for
/// the printer. Mirror deletions ran in-session (the one delete rule,
/// otp-6b) and are scored in `summary.entries_deleted` — there is no
/// post-transfer purge step and no separate purge stats.
pub struct PullVerbOutcome {
    pub summary: TransferSummary,
    pub dest_root: PathBuf,
}

/// Run a remote pull end-to-end (otp-10b-2): initiate one
/// DESTINATION-role `Transfer` session against the source daemon via
/// `run_pull_session`. The daemon becomes the SOURCE responder and
/// streams its module tree; this end diffs, receives, and (in mirror
/// mode) deletes extraneous local entries at SourceDone.
///
/// `progress` is borrowed for the duration of the session — the caller
/// owns the channel + monitor task, exactly as with
/// [`run_remote_push`]. Unlike the old pull there is no post-RPC
/// destructive step (deletes are in-session), so the monitor's
/// lifetime lines up with the one call and the run_pull_sync /
/// apply_pull_mirror_purge split does not apply here. Move's
/// remote-source delete remains a caller-side follow-up, gated by
/// `require_complete_scan`.
pub async fn run_remote_pull(
    execution: PullExecution,
    progress: Option<&RemoteTransferProgress>,
) -> Result<PullVerbOutcome> {
    // No pre-created destination directories: the session sink creates
    // each write target's parent chain itself (including the
    // single-file case, where `dest_root` IS the target file path and
    // must never be mkdir'd) — pinned by
    // `single_file_pull_lands_at_the_target_file_path` against a
    // missing parent. The old pull's explicit parent-creation step is
    // redundant on this path.
    let options = PullSessionOptions {
        compare_mode: execution.compare_mode,
        ignore_existing: execution.ignore_existing,
        require_complete_scan: execution.require_complete_scan,
        // `--force-grpc`: the session's in-stream byte carrier is the
        // gRPC-fallback lane (otp-8).
        in_stream_bytes: execution.force_grpc,
        resume: execution.resume,
        resume_block_size: execution.resume_block_size,
        filter: execution.filter,
        mirror_enabled: execution.mirror_mode,
        mirror_kind: if execution.mirror_mode {
            execution.mirror_kind
        } else {
            MirrorMode::Off
        },
        byte_progress: None,
        progress: progress.cloned(),
        trace_data_plane: execution.trace_data_plane,
    };
    let outcome = run_pull_session(&execution.remote, execution.dest_root.clone(), options)
        .await
        .with_context(|| {
            format!(
                "pulling from {} into {}",
                execution.remote_label,
                execution.dest_root.display()
            )
        })?;

    Ok(PullVerbOutcome {
        summary: outcome.summary,
        dest_root: execution.dest_root,
    })
}

/// Inputs for [`run_delegated_pull`]. Primitive fields only —
/// no clap, no presentation. CLI builds this from
/// `&TransferArgs`; the future TUI builds it directly.
pub struct DelegatedPullExecution {
    pub src: RemoteEndpoint,
    pub dst: RemoteEndpoint,
    pub options: DelegatedSpecOptions,
    pub trace_data_plane: bool,
    pub dst_label: String,
    /// Detach the transfer from the calling CLI. When true,
    /// the destination daemon's `tx.closed()` race disarms,
    /// so client disconnect no longer drops the transfer.
    /// The CLI can exit after observing the daemon's
    /// `Started` event. Only valid on remote→remote
    /// delegated transfers (any route with a local endpoint
    /// has the CLI in the byte path and rejects the flag
    /// upstream).
    pub detach: bool,
}

/// Output of [`run_delegated_pull`]. The `src` / `dst` endpoints
/// are echoed back so the caller's printer can reference them
/// without keeping its own copies.
pub struct DelegatedPullOutcome {
    pub summary: DelegatedPullSummary,
    pub src: RemoteEndpoint,
    pub dst: RemoteEndpoint,
}

/// Per-stream state tracked while consuming `BytesProgress`
/// messages. `files_completed` / `bytes_completed` are
/// monotonic counters from the daemon; we use them to compute
/// deltas against the CLI's [`RemoteTransferProgress`] channel.
/// This is the aggregate lane of the `ProgressEvent` contract
/// (see `blit_core::remote::transfer::progress`): only counters
/// are visible here, so file deltas ride `Payload.files` and no
/// `FileComplete` is ever emitted.
#[derive(Default)]
struct DelegatedBytesProgressState {
    files_completed: u64,
    bytes_completed: u64,
}

fn report_bytes_progress(
    progress: Option<&RemoteTransferProgress>,
    state: &mut DelegatedBytesProgressState,
    bytes: &BytesProgress,
) {
    if let Some(progress) = progress {
        let file_delta = bytes
            .files_completed
            .saturating_sub(state.files_completed)
            .try_into()
            .unwrap_or(usize::MAX);
        let byte_delta = bytes.bytes_completed.saturating_sub(state.bytes_completed);
        state.files_completed = state.files_completed.max(bytes.files_completed);
        state.bytes_completed = state.bytes_completed.max(bytes.bytes_completed);
        if file_delta > 0 || byte_delta > 0 {
            progress.report_payload(file_delta, byte_delta);
        }
    }
}

/// Map a daemon-side `DelegatedPullError` to a human-readable
/// CLI-facing report.
pub fn map_delegated_error(phase: i32, message: &str) -> eyre::Report {
    let phase = DelegatedPullPhase::try_from(phase).unwrap_or(DelegatedPullPhase::Unknown);
    match phase {
        DelegatedPullPhase::DelegationRejected => {
            eyre!("delegation rejected by destination daemon: {message}")
        }
        DelegatedPullPhase::ConnectSource => {
            // Remote→remote is delegated-only (D-2026-07-11-1 removed
            // the CLI relay), so the unreachable-source topology has no
            // flag to point at — the manual two-hop is the workaround.
            eyre!(
                "destination daemon cannot reach source ({message}); if this host can \
                 reach both daemons, pull to a local path first, then push it"
            )
        }
        DelegatedPullPhase::Negotiate => eyre!("source refused delegated pull: {message}"),
        DelegatedPullPhase::Transfer => eyre!("delegated transfer failed: {message}"),
        DelegatedPullPhase::Apply => {
            eyre!("destination failed to apply delegated transfer: {message}")
        }
        DelegatedPullPhase::Unknown => eyre!("delegated transfer failed: {message}"),
    }
}

/// Extract the `(module, destination_path)` pair the
/// `DelegatedPullRequest` needs from a parsed
/// [`RemoteEndpoint`]. Errors on `RemotePath::Discovery` —
/// remote destinations always require an explicit module or
/// root.
pub fn destination_spec_fields(dst: &RemoteEndpoint) -> Result<(String, String)> {
    match &dst.path {
        RemotePath::Module { module, rel_path } => {
            Ok((module.clone(), normalize_for_request(rel_path)))
        }
        RemotePath::Root { rel_path } => Ok((String::new(), normalize_for_request(rel_path))),
        RemotePath::Discovery => bail!(
            "remote destination must include a module or root (e.g., server:/module/ or server://path){}",
            crate::endpoints::local_path_hint(&dst.host)
        ),
    }
}

fn normalize_for_request(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        path.iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// Run a delegated remote→remote pull: build the request,
/// connect to the destination's `BlitClient`, stream the
/// payload, and return the destination's summary.
///
/// `progress` is borrowed for the duration of the call. The
/// library translates per-payload `BytesProgress` messages into
/// `report_payload` calls on the channel; CLI's printer
/// continues to consume `ProgressEvent` as before.
///
/// `on_started` fires exactly once if the destination emits a
/// `Started` event (it precedes the first byte). The callback
/// is the stopgap presentation hook: CLI prints
/// `[delegation] destination pulling from <ep> (<n> stream(s))`
/// in verbose mode; TUI passes a no-op. The M-C
/// `AppProgressEvent` reshape will replace the callback with
/// a stream variant that both consumers handle uniformly.
///
/// Errors from the destination's stream are mapped through
/// [`map_delegated_error`]; transport-level failures
/// (Unavailable, Unimplemented) get their own CLI-style hints
/// because they don't have a `phase`.
pub async fn run_delegated_pull<F>(
    execution: DelegatedPullExecution,
    progress: Option<&RemoteTransferProgress>,
    mut on_started: F,
) -> Result<DelegatedPullOutcome>
where
    F: FnMut(&DelegatedPullStarted),
{
    let spec = delegated_spec_from_options(&execution.src, &execution.options)?;
    let (dst_module, dst_destination_path) = destination_spec_fields(&execution.dst)?;

    let request = DelegatedPullRequest {
        dst_module,
        dst_destination_path,
        src: Some(RemoteSourceLocator {
            host: execution.src.host.clone(),
            port: execution.src.port as u32,
        }),
        spec: Some(spec),
        trace_data_plane: execution.trace_data_plane,
        detach: execution.detach,
    };

    let uri = execution.dst.control_plane_uri();
    let mut client = crate::client::connect_with_timeout(uri.clone())
        .await
        .with_context(|| format!("connecting to destination {}", execution.dst_label))?;

    let response = client.delegated_pull(request).await.map_err(|status| {
        if status.code() == Code::Unimplemented {
            eyre!(
                "destination daemon does not implement DelegatedPull; upgrade the destination \
                 daemon"
            )
        } else if status.code() == Code::Unavailable {
            eyre!(
                "destination daemon is unavailable for delegated pull ({})",
                status.message()
            )
        } else {
            eyre!(
                "delegated remote-to-remote transfer failed: {}",
                status.message()
            )
        }
    })?;
    let mut stream = response.into_inner();

    let mut summary: Option<DelegatedPullSummary> = None;
    let mut failure: Option<eyre::Report> = None;
    let mut bytes_progress_state = DelegatedBytesProgressState::default();

    loop {
        let message = match stream.message().await {
            Ok(Some(message)) => message,
            Ok(None) => break,
            Err(status) => {
                failure = Some(if status.code() == Code::Unavailable {
                    eyre!("delegation stream lost ({})", status.message())
                } else {
                    eyre!("delegation stream failed: {}", status.message())
                });
                break;
            }
        };
        match message.payload {
            Some(DelegatedPayload::Started(started)) => {
                on_started(&started);
            }
            Some(DelegatedPayload::ManifestBatch(batch)) => {
                if let Some(progress) = progress {
                    progress.report_manifest_batch(batch.file_count as usize);
                }
            }
            Some(DelegatedPayload::BytesProgress(bytes)) => {
                report_bytes_progress(progress, &mut bytes_progress_state, &bytes);
            }
            Some(DelegatedPayload::Summary(done)) => {
                summary = Some(done);
                break;
            }
            Some(DelegatedPayload::Error(error)) => {
                failure = Some(map_delegated_error(error.phase, &error.upstream_message));
                break;
            }
            None => {}
        }
    }

    if let Some(error) = failure {
        return Err(error);
    }

    let summary = summary.ok_or_else(|| eyre!("delegation ended before summary"))?;
    Ok(DelegatedPullOutcome {
        summary,
        src: execution.src,
        dst: execution.dst,
    })
}

/// "Fire and forget" variant of [`run_delegated_pull`] for the
/// CLI's `--detach` flow. Opens the delegated_pull RPC,
/// receives the first `Started` event (which now carries the
/// daemon-assigned `transfer_id` after m-jobs-3), and returns
/// without consuming the rest of the stream. Dropping the
/// returned tuple's response stream closes the receiver — but
/// the daemon-side spawn closure honors `execution.detach` and
/// completes the transfer regardless.
///
/// Returns the `DelegatedPullStarted` payload (which the CLI
/// uses to print the transfer id + cancel hint) plus the
/// destination endpoint so the caller can format display
/// strings without re-parsing.
///
/// Refuses to proceed if `execution.detach` is `false` — the
/// detached semantic is meaningless on a tx.closed-armed
/// daemon, and the caller would mistakenly return success
/// while the daemon drops the transfer the moment we drop the
/// stream.
pub async fn run_delegated_pull_until_started(
    execution: DelegatedPullExecution,
) -> Result<(DelegatedPullStarted, RemoteEndpoint)> {
    if !execution.detach {
        return Err(eyre!(
            "run_delegated_pull_until_started requires execution.detach=true"
        ));
    }

    let spec = delegated_spec_from_options(&execution.src, &execution.options)?;
    let (dst_module, dst_destination_path) = destination_spec_fields(&execution.dst)?;

    let request = DelegatedPullRequest {
        dst_module,
        dst_destination_path,
        src: Some(RemoteSourceLocator {
            host: execution.src.host.clone(),
            port: execution.src.port as u32,
        }),
        spec: Some(spec),
        trace_data_plane: execution.trace_data_plane,
        detach: execution.detach,
    };

    let uri = execution.dst.control_plane_uri();
    let mut client = crate::client::connect_with_timeout(uri.clone())
        .await
        .with_context(|| format!("connecting to destination {}", execution.dst_label))?;

    let response = client.delegated_pull(request).await.map_err(|status| {
        if status.code() == Code::Unimplemented {
            eyre!(
                "destination daemon does not implement DelegatedPull; \
                 cannot detach against this daemon"
            )
        } else if status.code() == Code::Unavailable {
            eyre!(
                "destination daemon is unavailable for delegated pull ({})",
                status.message()
            )
        } else {
            eyre!(
                "delegated remote-to-remote transfer failed: {}",
                status.message()
            )
        }
    })?;
    let mut stream = response.into_inner();

    // Read the first frame and resolve. Started is the
    // daemon's first emitted payload per the
    // DelegatedPullProgress protocol; anything else (or
    // stream end) is a clear error.
    //
    // Empty `transfer_id` is a daemon-too-old signal: the
    // `Started.transfer_id` field arrived in m-jobs-3 and
    // older daemons leave it empty (proto3 default). We
    // **must** refuse here rather than return success,
    // because an older daemon also doesn't honor the
    // `detach=true` we asked for — dropping `stream` after
    // Started would let its tx.closed() race drop the
    // transfer. The caller would print a detached-success
    // message with no usable id while the transfer was
    // already cancelled.
    match stream.message().await {
        Ok(Some(message)) => match message.payload {
            Some(DelegatedPayload::Started(started)) => {
                if started.transfer_id.is_empty() {
                    return Err(eyre!(
                        "destination daemon is older than m-jobs-3 and cannot detach \
                         this transfer (Started.transfer_id was empty, and dropping \
                         the stream would cancel the transfer on an older daemon). \
                         Upgrade the destination daemon, or retry without --detach."
                    ));
                }
                // Dropping `stream` here closes the receiver
                // → daemon's tx.closed() resolves. With
                // detach=true the daemon ignores that and
                // keeps the transfer running.
                drop(stream);
                Ok((started, execution.dst))
            }
            Some(DelegatedPayload::Error(error)) => {
                Err(map_delegated_error(error.phase, &error.upstream_message))
            }
            _ => Err(eyre!(
                "delegated pull emitted a non-Started payload before Started"
            )),
        },
        Ok(None) => Err(eyre!("delegated pull stream closed before Started")),
        Err(status) => Err(eyre!(
            "delegation stream failed before Started: {}",
            status.message()
        )),
    }
}

#[cfg(test)]
mod tests {
    //! Delegated-pull helper coverage. (The R46-F3
    //! `delete_listed_paths` containment tests that used to open
    //! this module died with the daemon-authored delete list at
    //! otp-10c-2 — mirror deletions are in-session and never cross
    //! the wire, so there is no list to contain.)

    use super::*;

    #[tokio::test]
    async fn run_delegated_pull_until_started_refuses_non_detach() {
        // Guard: if a caller asks for the "exit after Started"
        // path without setting `execution.detach = true`, the
        // function refuses synchronously instead of opening
        // the RPC. Otherwise dropping the stream after Started
        // would let the daemon's tx.closed() race drop the
        // transfer.
        use blit_core::remote::endpoint::RemoteEndpoint;
        use blit_core::remote::RemotePath;
        let endpoint = RemoteEndpoint {
            host: "127.0.0.1".to_string(),
            port: 1,
            path: RemotePath::Module {
                module: "m".to_string(),
                rel_path: PathBuf::new(),
            },
        };
        let execution = DelegatedPullExecution {
            src: endpoint.clone(),
            dst: endpoint,
            options: DelegatedSpecOptions::default(),
            trace_data_plane: false,
            dst_label: "x".to_string(),
            detach: false,
        };
        let err = run_delegated_pull_until_started(execution)
            .await
            .expect_err("non-detach execution must be refused");
        assert!(
            err.to_string().contains("requires execution.detach=true"),
            "got: {err}"
        );
    }

    // Delegated-pull helper tests — moved from
    // `crates/blit-cli/src/transfers/remote_remote_direct.rs::tests`
    // in the a0-delegated-execution slice so the helpers and their
    // coverage live together.

    use blit_core::remote::transfer::{ProgressEvent, RemoteTransferProgress};
    use tokio::sync::mpsc;

    fn delegated_endpoint(path: RemotePath) -> RemoteEndpoint {
        RemoteEndpoint {
            host: "localhost".to_string(),
            port: 9031,
            path,
        }
    }

    #[test]
    fn destination_fields_for_module_root_use_dot_path() {
        let dst = delegated_endpoint(RemotePath::Module {
            module: "mod".to_string(),
            rel_path: PathBuf::new(),
        });
        let (module, path) = destination_spec_fields(&dst).unwrap();
        assert_eq!(module, "mod");
        assert_eq!(path, ".");
    }

    #[test]
    fn destination_fields_for_subpath_normalize_forward_slashes() {
        let dst = delegated_endpoint(RemotePath::Module {
            module: "mod".to_string(),
            rel_path: PathBuf::from("a").join("b"),
        });
        let (module, path) = destination_spec_fields(&dst).unwrap();
        assert_eq!(module, "mod");
        assert_eq!(path, "a/b");
    }

    #[test]
    fn bytes_progress_reports_cumulative_values_as_deltas() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let progress = RemoteTransferProgress::new(tx);
        let mut state = DelegatedBytesProgressState::default();

        report_bytes_progress(
            Some(&progress),
            &mut state,
            &BytesProgress {
                files_completed: 1,
                files_total: 3,
                bytes_completed: 1024,
                bytes_total: 4096,
            },
        );
        report_bytes_progress(
            Some(&progress),
            &mut state,
            &BytesProgress {
                files_completed: 2,
                files_total: 3,
                bytes_completed: 4096,
                bytes_total: 4096,
            },
        );

        assert!(matches!(
            rx.try_recv().unwrap(),
            ProgressEvent::Payload {
                files: 1,
                bytes: 1024
            }
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            ProgressEvent::Payload {
                files: 1,
                bytes: 3072
            }
        ));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn bytes_progress_duplicate_cumulative_update_is_not_counted_twice() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let progress = RemoteTransferProgress::new(tx);
        let mut state = DelegatedBytesProgressState::default();
        let update = BytesProgress {
            files_completed: 1,
            files_total: 1,
            bytes_completed: 2048,
            bytes_total: 2048,
        };

        report_bytes_progress(Some(&progress), &mut state, &update);
        report_bytes_progress(Some(&progress), &mut state, &update);

        assert!(matches!(
            rx.try_recv().unwrap(),
            ProgressEvent::Payload {
                files: 1,
                bytes: 2048
            }
        ));
        assert!(rx.try_recv().is_err());
    }
}
