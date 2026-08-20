//! Client-side entry for initiating a unified transfer session.
//!
//! [`run_push_session`] declares the SOURCE role (push-equivalent,
//! otp-4): open the `Transfer` RPC, stream the manifest + payloads; the
//! daemon answers as the DESTINATION Responder. [`run_pull_session`]
//! declares the DESTINATION role (pull-equivalent, otp-5a): the daemon
//! answers as the SOURCE Responder and streams its module tree, which
//! this end diffs and writes. Both build a gRPC-backed [`FrameTransport`]
//! over `BlitClient::transfer` and run the matching role driver; role is
//! carried in `SessionOpen.initiator_role`, never a second code path.
//!
//! Verb wiring: the push-shaped verb (CLI `copy`/`mirror`/`move` to a
//! remote destination, TUI F1 push) rides [`run_push_session`] since
//! otp-10a via `blit_core::transfers::remote::run_remote_push`; the
//! pull-shaped verb (remote source → local destination, TUI F3 pull)
//! rides [`run_pull_session`] since otp-10b-2 via
//! `blit_core::transfers::remote::run_remote_pull`. Both push (otp-4b)
//! and pull (otp-5b) default to the TCP data plane; the in-stream
//! carrier is the requested fallback either direction.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use eyre::{eyre, Result};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Channel, Endpoint};

use crate::generated::blit_client::BlitClient;
use crate::generated::{
    ComparisonMode, FilterSpec, MirrorMode, ResumeSettings, SessionOpen, TransferRole,
    TransferSummary,
};
use crate::perf_history;
use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
use crate::remote::transfer::source::TransferSource;
use crate::remote::transfer::{
    ByteProgressSink, RemoteTransferProgress, SessionPhaseRole, TransferLifecycleOutcome,
    TransferLifecycleTrace,
};
use crate::transfer_plan::PlanOptions;
use crate::transfer_session::transport::{grpc_client_transport, GRPC_CHANNEL_FRAMES};
use crate::transfer_session::{
    run_destination, run_source, DestinationInstruments, DestinationOutcome,
    DestinationSessionConfig, DestinationTarget, HelloConfig, SessionEndpoint, SourceInstruments,
    SourceSessionConfig,
};

/// The push-shaped session options. The full verb surface rides here
/// since otp-10a (mirror, filters, progress, trace); the SOURCE owns
/// the planner knobs, the DESTINATION owns the compare decision.
pub struct PushSessionOptions {
    pub compare_mode: ComparisonMode,
    pub ignore_existing: bool,
    pub require_complete_scan: bool,
    /// Explicitly discard Windows attributes and named data streams at the
    /// SOURCE. False preserves strictly and may reject a non-Windows target.
    pub drop_windows_metadata: bool,
    pub plan_options: PlanOptions,
    /// Force the in-stream byte carrier instead of the TCP data plane
    /// (otp-4b). Default `false` = the responder grants a data plane and
    /// payloads ride TCP sockets; `true` is the diagnostics / unreachable
    /// data-plane fallback (`--force-grpc`-shaped).
    pub in_stream_bytes: bool,
    /// otp-7b: negotiate the resume block phase (`SessionOpen.resume`).
    /// Changed dest partials are then patched block-wise instead of
    /// re-transferred whole.
    pub resume: bool,
    /// Requested resume block size in bytes; `0` lets the DESTINATION
    /// choose (currently 1 MiB). The destination clamps to its
    /// carrier's bounds either way. Ignored unless `resume` is true.
    pub resume_block_size: u32,
    /// otp-10a: source-side scan filter, riding `SessionOpen.filter`
    /// (the session honors it since otp-6a — this is the client
    /// wiring; symmetric with [`PullSessionOptions::filter`]). This
    /// SOURCE applies it to its own scan through the universal
    /// `FilteredSource` chokepoint; the DESTINATION uses it to scope
    /// mirror deletions. `None` scans everything.
    pub filter: Option<FilterSpec>,
    /// otp-10a: mirror on the session (otp-6b's one delete rule — the
    /// daemon DESTINATION diffs the complete source manifest against
    /// its tree at SourceDone and deletes extraneous entries locally).
    /// Explicit enabled + scope per the contract; `MirrorMode::Off`
    /// with `mirror_enabled` set is refused at OPEN.
    pub mirror_enabled: bool,
    pub mirror_kind: MirrorMode,
    /// otp-10a: w6-1 progress events from this SOURCE's send side —
    /// need batches as the denominator, `Payload`/`FileComplete` per
    /// file sent on either carrier. The CLI progress line and the TUI
    /// footer consume these exactly as they did from the old driver.
    pub progress: Option<RemoteTransferProgress>,
    /// otp-10a: emit `[data-plane-client]` connect traces on the data
    /// plane sockets this SOURCE dials (`--trace-data-plane`).
    pub trace_data_plane: bool,
    /// Explicit process-local lifecycle context. Disabled by default.
    pub lifecycle_trace: TransferLifecycleTrace,
    /// ph-1: where this end records the finished session, and as what
    /// route. `None` (the default, and every test) records nothing —
    /// recording is opt-in per call, exactly as the local session's
    /// `LocalMirrorOptions::perf_history` flag is, so a unit test can
    /// never write to the operator's real store.
    pub perf: Option<perf_history::RecordSink>,
}

impl Default for PushSessionOptions {
    fn default() -> Self {
        Self {
            compare_mode: ComparisonMode::SizeMtime,
            ignore_existing: false,
            require_complete_scan: false,
            drop_windows_metadata: false,
            plan_options: PlanOptions::default(),
            in_stream_bytes: false,
            resume: false,
            resume_block_size: 0,
            filter: None,
            mirror_enabled: false,
            mirror_kind: MirrorMode::Off,
            progress: None,
            trace_data_plane: false,
            lifecycle_trace: TransferLifecycleTrace::disabled(),
            perf: None,
        }
    }
}

/// Connect to `endpoint`'s daemon and run one SOURCE-role transfer
/// session pushing `source`'s tree into the endpoint's module/path.
/// Returns the destination-computed [`TransferSummary`] (contract:
/// DESTINATION is the scorer).
pub async fn run_push_session(
    endpoint: &RemoteEndpoint,
    source: Arc<dyn TransferSource>,
    mut options: PushSessionOptions,
) -> Result<TransferSummary> {
    // ph-1: the perf facts this end needs, lifted before the option
    // fields are moved into `SessionOpen` / the instruments.
    let started = std::time::Instant::now();
    let perf = options.perf.take();
    let perf_mirror = options.mirror_enabled;
    let perf_compare = options.compare_mode;

    let lifecycle_trace = options.lifecycle_trace.clone();
    lifecycle_trace.attach_initiator_role(SessionPhaseRole::Source);

    // The responder resolves module→root; the initiator's own local
    // path never crosses the wire (contract §SessionOpen).
    let (module, path) = endpoint_module_path(endpoint)?;

    let mut client = connect_transfer_client_with_trace(endpoint, &lifecycle_trace).await?;

    let open = SessionOpen {
        initiator_role: TransferRole::Source as i32,
        module,
        path,
        compare_mode: options.compare_mode as i32,
        ignore_existing: options.ignore_existing,
        require_complete_scan: options.require_complete_scan,
        drop_windows_metadata: options.drop_windows_metadata,
        // otp-4b: default to the TCP data plane; the responder grants it
        // in SessionAccept unless this asks for the in-stream fallback.
        in_stream_bytes: options.in_stream_bytes,
        // otp-7b: resume rides the open (plan D6 — the flag is in the
        // open, so resume runs identically whichever end initiated).
        resume: options.resume.then_some(ResumeSettings {
            enabled: true,
            block_size: options.resume_block_size,
        }),
        // otp-10a: filter + mirror ride the open (otp-6a/6b session
        // support; this is the client wiring, symmetric with pull's
        // otp-9a).
        filter: options.filter,
        mirror_enabled: options.mirror_enabled,
        mirror_kind: options.mirror_kind as i32,
        ..Default::default()
    };

    // Open the bidi RPC: the request stream is fed by `out_tx`, the
    // response stream is the inbound half. The handler returns its
    // response stream immediately (it spawns the session), so this
    // await resolves before any frame flows — no deadlock.
    let (out_tx, out_rx) = mpsc::channel(GRPC_CHANNEL_FRAMES);
    lifecycle_trace.record("transfer_rpc_open_begin", None);
    let inbound = match client.transfer(ReceiverStream::new(out_rx)).await {
        Ok(response) => {
            lifecycle_trace.record(
                "transfer_rpc_open_end",
                Some(TransferLifecycleOutcome::Success),
            );
            response.into_inner()
        }
        Err(status) => {
            lifecycle_trace.record("transfer_rpc_open_end", Some(rpc_status_outcome(&status)));
            return Err(eyre::Report::new(transfer_open_refusal(status)));
        }
    };
    let transport = grpc_client_transport(out_tx, inbound);

    // otp-10a: own the unreadable-scan accumulator so a partial source
    // scan fails the push after the session completes — the old push
    // driver's exact posture (send what's readable, then error), which
    // `blit move`'s source-delete gate relies on: an error here means
    // move never deletes a source whose files were silently skipped.
    let unreadable: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();

    // ph-5: seed the sender-owned stream dial from what a previous run
    // on this exact route settled at, and capture what THIS run settles
    // at for the store. Both ride the perf sink: no sink (unit tests,
    // history disabled) means cold start and no recording — the same
    // opt-in the ph-1 record path uses. Any store problem is a cold
    // start, never an error.
    let stream_seed = perf.as_ref().and_then(|sink| {
        let store = crate::seed_store::SeedStore::for_history(&sink.store);
        match store.lookup_route_latest(&sink.route) {
            Ok(entry) => entry.and_then(|entry| entry.workers),
            Err(err) => {
                log::debug!("stream seed lookup failed (cold start): {err:?}");
                None
            }
        }
    });
    let settled_streams_cell = perf
        .as_ref()
        .map(|_| Arc::new(std::sync::atomic::AtomicU32::new(0)));

    let cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open),
        plan_options: options.plan_options,
        // The initiator dials the data plane on the same host it reached
        // the control plane on (contract §Transport: initiator dials).
        data_plane_host: Some(endpoint.host.clone()),
        instruments: SourceInstruments {
            progress: options.progress,
            unreadable: Some(Arc::clone(&unreadable)),
            trace_data_plane: options.trace_data_plane,
            session_phase_trace: Default::default(),
            lifecycle_trace,
            small_file_probe: Default::default(),
            // ph-1c: initiators record via their own summary return
            // path; the terminal hook exists for raced responders.
            on_terminal_summary: None,
            stream_seed,
            settled_streams_out: settled_streams_cell.clone(),
            #[cfg(test)]
            dial_test_samples: None,
            #[cfg(test)]
            dial_terminal_test_gate: None,
            #[cfg(test)]
            dial_proposal_test_gate: None,
            #[cfg(test)]
            dial_membership_test_gate: None,
        },
    };
    let summary = run_source(cfg, transport, source).await?;

    let unreadable = unreadable
        .lock()
        .map_err(|err| eyre!("unreadable-path accumulator poisoned: {err}"))?;
    if !unreadable.is_empty() {
        let preview: Vec<_> = unreadable.iter().take(5).cloned().collect();
        let mut message = format!(
            "{} file(s) were skipped due to permission or access errors: {}",
            unreadable.len(),
            preview.join(", ")
        );
        if unreadable.len() > preview.len() {
            message.push_str(&format!(" (and {} more)", unreadable.len() - preview.len()));
        }
        return Err(eyre!(message));
    }

    record_session_history(
        perf.as_ref(),
        SessionRecordFacts {
            mirror_enabled: perf_mirror,
            compare_mode: perf_compare,
            files: summary.files_transferred,
            bytes: summary.bytes_transferred,
            files_failed: summary.files_failed,
            elapsed: started.elapsed(),
        },
    );
    // ph-5 writer: persist what the dial settled at as this route's
    // `workers` seed (store-internal min-files gate; merge keeps the
    // route's checker seed). 0 = the mirror never armed — record
    // nothing rather than teach the floor from a run that never moved.
    if let (Some(sink), Some(cell)) = (perf.as_ref(), settled_streams_cell.as_ref()) {
        let settled = cell.load(std::sync::atomic::Ordering::Relaxed);
        if settled > 0 {
            let store = crate::seed_store::SeedStore::for_history(&sink.store);
            if let Err(err) = store.record_settled(
                &sink.route,
                crate::seed_store::SettledDials {
                    checkers: None,
                    workers: Some(settled),
                },
                summary.files_transferred.try_into().unwrap_or(usize::MAX),
                summary.bytes_transferred,
            ) {
                log::warn!("failed to update dial seed store: {err:?}");
            }
        }
    }
    Ok(summary)
}

/// The pull-shaped subset of session options the landed slices support.
/// Mirror and filters ride the open since otp-9a (the session honors
/// them since otp-6). The DESTINATION owns the compare decision; the
/// SOURCE owns the planner knobs (none cross the wire).
pub struct PullSessionOptions {
    pub compare_mode: ComparisonMode,
    pub ignore_existing: bool,
    pub require_complete_scan: bool,
    /// Explicitly discard Windows attributes and named data streams at the
    /// SOURCE. False preserves strictly and may reject a non-Windows target.
    pub drop_windows_metadata: bool,
    /// Force the in-stream byte carrier instead of the TCP data plane
    /// (otp-5b). Default `false` = the SOURCE responder grants a data
    /// plane and this DESTINATION initiator dials + receives over TCP
    /// sockets; `true` is the diagnostics / unreachable data-plane
    /// fallback. Symmetric with [`PushSessionOptions::in_stream_bytes`].
    pub in_stream_bytes: bool,
    /// otp-7b: negotiate the resume block phase — symmetric with
    /// [`PushSessionOptions::resume`] (plan D6: the flag is in the open,
    /// so resume runs identically whichever end initiated).
    pub resume: bool,
    /// Requested resume block size in bytes; `0` lets the DESTINATION
    /// (this end) choose. Ignored unless `resume` is true.
    pub resume_block_size: u32,
    /// otp-9a: source-side scan filter, riding `SessionOpen.filter`
    /// (the session honors it since otp-6a — this is the client
    /// wiring). `None` scans everything.
    pub filter: Option<FilterSpec>,
    /// otp-9a: mirror on the session (otp-6b's one delete rule — this
    /// DESTINATION diffs the complete source manifest against its tree
    /// at SourceDone and deletes extraneous entries locally). Explicit
    /// enabled + scope per the contract; `MirrorMode::Off` with
    /// `mirror_enabled` set is refused at OPEN.
    pub mirror_enabled: bool,
    pub mirror_kind: MirrorMode,
    /// otp-9a: live counter the session sink reports applied payload
    /// bytes against (the delegated dst daemon's jobs row, otp-9).
    pub byte_progress: Option<ByteProgressSink>,
    /// otp-10b-2: w6-1 progress events from this DESTINATION's receive
    /// side — need batches as the denominator, `Payload`/`FileComplete`
    /// per record received on either carrier. The CLI progress line and
    /// the TUI footer consume these exactly as they did from the old
    /// driver. Symmetric with [`PushSessionOptions::progress`].
    pub progress: Option<RemoteTransferProgress>,
    /// otp-10b-2: emit `[data-plane-client]` connect traces on the data
    /// plane sockets this DESTINATION dials (`--trace-data-plane`).
    pub trace_data_plane: bool,
    /// Explicit process-local lifecycle context. Disabled by default.
    pub lifecycle_trace: TransferLifecycleTrace,
    /// ph-1: see [`PushSessionOptions::perf`] — same opt-in posture,
    /// DESTINATION role.
    pub perf: Option<perf_history::RecordSink>,
}

impl Default for PullSessionOptions {
    fn default() -> Self {
        Self {
            compare_mode: ComparisonMode::SizeMtime,
            ignore_existing: false,
            require_complete_scan: false,
            drop_windows_metadata: false,
            in_stream_bytes: false,
            resume: false,
            resume_block_size: 0,
            filter: None,
            mirror_enabled: false,
            mirror_kind: MirrorMode::Off,
            byte_progress: None,
            progress: None,
            trace_data_plane: false,
            lifecycle_trace: TransferLifecycleTrace::disabled(),
            perf: None,
        }
    }
}

/// Connect to `endpoint`'s daemon and run one DESTINATION-role transfer
/// session pulling the endpoint's module/path tree into `dest_root`
/// (pull-equivalent, otp-5a). The client initiates and declares
/// DESTINATION, so the daemon becomes the SOURCE Responder (streaming
/// its module tree). Returns the [`DestinationOutcome`] this end
/// computed (contract: the DESTINATION is the scorer).
///
/// otp-5b: the default carrier is the TCP data plane — the SOURCE
/// responder binds+grants+accepts sockets while sending, and this
/// DESTINATION initiator dials + receives over them (the transport/role
/// decoupling). `PullSessionOptions::in_stream_bytes` forces the in-stream
/// fallback (diagnostics / unreachable data plane).
pub async fn run_pull_session(
    endpoint: &RemoteEndpoint,
    dest_root: PathBuf,
    options: PullSessionOptions,
) -> Result<DestinationOutcome> {
    options
        .lifecycle_trace
        .attach_initiator_role(SessionPhaseRole::Destination);
    let client = connect_transfer_client_with_trace(endpoint, &options.lifecycle_trace).await?;
    run_pull_session_with_client(client, endpoint, dest_root, options).await
}

/// [`run_pull_session`] over an already-connected client (otp-9b). The
/// delegated dst daemon connects separately so a connect failure keeps
/// its own error phase (`ConnectSource`) structurally, without string
/// matching on the session error.
pub async fn run_pull_session_with_client(
    mut client: BlitClient<Channel>,
    endpoint: &RemoteEndpoint,
    dest_root: PathBuf,
    mut options: PullSessionOptions,
) -> Result<DestinationOutcome> {
    // ph-1: same lift as the push side — the perf facts are read before
    // the option fields move into the open / the instruments.
    let started = std::time::Instant::now();
    let perf = options.perf.take();
    let perf_mirror = options.mirror_enabled;
    let perf_compare = options.compare_mode;

    let lifecycle_trace = options.lifecycle_trace.clone();
    lifecycle_trace.attach_initiator_role(SessionPhaseRole::Destination);
    let (module, path) = endpoint_module_path(endpoint)?;

    let open = SessionOpen {
        initiator_role: TransferRole::Destination as i32,
        module,
        path,
        compare_mode: options.compare_mode as i32,
        ignore_existing: options.ignore_existing,
        require_complete_scan: options.require_complete_scan,
        drop_windows_metadata: options.drop_windows_metadata,
        // otp-5b: default to the TCP data plane; the SOURCE responder
        // grants it in SessionAccept unless this asks for the in-stream
        // fallback.
        in_stream_bytes: options.in_stream_bytes,
        // otp-7b: resume rides the open, role-agnostic (plan D6).
        resume: options.resume.then_some(ResumeSettings {
            enabled: true,
            block_size: options.resume_block_size,
        }),
        // otp-9a: filter + mirror ride the open (otp-6a/6b session
        // support; this is the client wiring).
        filter: options.filter,
        mirror_enabled: options.mirror_enabled,
        mirror_kind: options.mirror_kind as i32,
        ..Default::default()
    };

    let (out_tx, out_rx) = mpsc::channel(GRPC_CHANNEL_FRAMES);
    lifecycle_trace.record("transfer_rpc_open_begin", None);
    let inbound = match client.transfer(ReceiverStream::new(out_rx)).await {
        Ok(response) => {
            lifecycle_trace.record(
                "transfer_rpc_open_end",
                Some(TransferLifecycleOutcome::Success),
            );
            response.into_inner()
        }
        Err(status) => {
            lifecycle_trace.record("transfer_rpc_open_end", Some(rpc_status_outcome(&status)));
            return Err(eyre::Report::new(transfer_open_refusal(status)));
        }
    };
    let transport = grpc_client_transport(out_tx, inbound);

    let cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open),
        // The initiator dials the data plane on the same host it reached
        // the control plane on (contract §Transport: initiator dials).
        data_plane_host: Some(endpoint.host.clone()),
        receiver_capacity: None,
        instruments: DestinationInstruments {
            progress: options.progress,
            byte_progress: options.byte_progress,
            trace_data_plane: options.trace_data_plane,
            session_phase_trace: Default::default(),
            lifecycle_trace,
            small_file_probe: Default::default(),
            // ph-1c: initiators record via their own outcome return
            // path; the terminal hook exists for raced responders.
            on_terminal_summary: None,
        },
        local_apply: None,
    };
    let outcome = run_destination(cfg, transport, DestinationTarget::Fixed(dest_root)).await?;

    record_session_history(
        perf.as_ref(),
        SessionRecordFacts {
            mirror_enabled: perf_mirror,
            compare_mode: perf_compare,
            files: outcome.summary.files_transferred,
            bytes: outcome.summary.bytes_transferred,
            files_failed: outcome.summary.files_failed,
            elapsed: started.elapsed(),
        },
    );
    Ok(outcome)
}

/// The facts one finished session end contributes to its
/// [`perf_history::PerformanceRecord`] (ph-1).
///
/// Grouped into a struct rather than threaded as loose parameters
/// because three call sites with three different summary messages
/// build the same row: the push SOURCE and the pull DESTINATION here
/// (`TransferSummary`), and the delegated coordinator in
/// `crate::transfers::remote` (`DelegatedPullSummary`, no session of
/// its own). One builder means the record shape cannot drift between
/// routes. Public because the daemon's served-session responder
/// (ph-1c) is a fourth call site: it must build the identical row
/// shape for its own store rather than growing a drift-prone copy.
#[derive(Debug, Clone, Copy)]
pub struct SessionRecordFacts {
    pub mirror_enabled: bool,
    pub compare_mode: ComparisonMode,
    /// Destination-attested file and byte counts.
    pub files: u64,
    pub bytes: u64,
    /// Destination-contained per-file failures (pfc-4).
    pub files_failed: u64,
    /// Wall time this end spent on the session.
    pub elapsed: Duration,
}

/// Map the wire comparison policy onto the perf-history snapshot enum.
///
/// Deliberately NOT shared with
/// `LocalCompareMode::resolve_compare_snapshot`: that one maps from the
/// process-local `LocalCompareMode` **plus** the legacy `--checksum`
/// bool, so it has a different input type and an extra precedence rule.
/// A shared `CompareModeSnapshot::from_proto` would only serve one of
/// the two sites, and folding the local site through it would mean
/// resolving `LocalCompareMode` to `ComparisonMode` first — a second
/// hop that buys nothing. The variant mapping below is the same
/// one-to-one correspondence, written against the proto enum.
/// `Unspecified` is the wire's documented "treat as SIZE_MTIME"
/// default (blit.proto §ComparisonMode).
fn compare_snapshot(mode: ComparisonMode) -> perf_history::CompareModeSnapshot {
    use perf_history::CompareModeSnapshot;
    match mode {
        ComparisonMode::Unspecified | ComparisonMode::SizeMtime => CompareModeSnapshot::SizeMtime,
        ComparisonMode::Checksum => CompareModeSnapshot::Checksum,
        ComparisonMode::SizeOnly => CompareModeSnapshot::SizeOnly,
        ComparisonMode::Force => CompareModeSnapshot::Force,
        ComparisonMode::IgnoreTimes => CompareModeSnapshot::IgnoreTimes,
    }
}

/// Build the [`perf_history::PerformanceRecord`] for one finished
/// session end, without touching disk — split from the writer for the
/// same reason `build_local_record` is: the record-shape contract stays
/// unit-testable without a live daemon.
pub fn build_session_record(
    route: &perf_history::RouteTag,
    facts: SessionRecordFacts,
) -> perf_history::PerformanceRecord {
    use perf_history::{OptionSnapshot, PerformanceRecord, TransferMode};
    let snapshot = OptionSnapshot {
        // A session never runs a dry plan: `--dry-run` never reaches
        // the wire.
        dry_run: false,
        // The engine-era option axes retired at otp-11b; the persisted
        // snapshot schema keeps the fields — record the historical
        // defaults (the only values production ever produced), exactly
        // as `build_local_record` does.
        preserve_symlinks: true,
        include_symlinks: true,
        skip_unchanged: true,
        checksum: matches!(facts.compare_mode, ComparisonMode::Checksum),
        compare_mode: compare_snapshot(facts.compare_mode),
        // Remote worker counts are dial-managed by the live session
        // controllers on each end and are not a client-side option, so
        // there is no client-observed count to record here.
        workers: 0,
    };
    let mode = if facts.mirror_enabled {
        TransferMode::Mirror
    } else {
        TransferMode::Copy
    };
    PerformanceRecord::new(
        mode,
        None,
        None,
        facts.files as usize,
        facts.bytes,
        snapshot,
        Some("session".to_string()),
        // The session has no separate planner phase to attribute; the
        // whole wall time lands in `transfer_duration_ms` (D3, the same
        // split `build_local_record` collapsed).
        0,
        facts.elapsed.as_millis(),
        0,
        facts.files_failed.try_into().unwrap_or(u32::MAX),
    )
    .with_route(route.clone())
}

/// Append one finished session end's record to its sink, if it has one.
///
/// ph-1 posture: recording must NEVER fail or delay the transfer result
/// beyond the append itself, so a store error is logged and dropped —
/// the caller has already computed its summary.
pub fn record_session_history(sink: Option<&perf_history::RecordSink>, facts: SessionRecordFacts) {
    let Some(sink) = sink else {
        return;
    };
    let record = build_session_record(&sink.route, facts);
    if let Err(err) = sink.store.append(&record) {
        log::warn!("failed to update performance history: {err:?}");
    }
}

/// Derive the wire `(module, path)` from a resolved endpoint. Empty
/// module targets the daemon's default root export; a discovery-form
/// endpoint is not resolvable to a transfer target. The path is
/// POSIX-normalized (review otp-10a F2): a `rel_path` that went through
/// `PathBuf::join` (the CLI's rsync destination-resolution rule does)
/// carries native `\` separators on Windows, and `to_string_lossy`
/// would put them on the wire verbatim — a Unix daemon then creates a
/// literal `sub\dir` entry. Every wire-bound relative path routes
/// through `path_posix` (the win-1 rule).
///
/// `pub(crate)` since ph-1: the perf-history `peer_key` for a remote
/// route is derived from the very `(module, path)` pair the session
/// addressed, so the key cannot drift from what was transferred.
pub(crate) fn endpoint_module_path(endpoint: &RemoteEndpoint) -> Result<(String, String)> {
    use crate::path_posix::relative_path_to_posix;
    match &endpoint.path {
        RemotePath::Module { module, rel_path } => {
            Ok((module.clone(), relative_path_to_posix(rel_path)))
        }
        RemotePath::Root { rel_path } => Ok((String::new(), relative_path_to_posix(rel_path))),
        RemotePath::Discovery => Err(eyre!(
            "a transfer session needs a resolved module or root endpoint, not a discovery form"
        )),
    }
}

#[cfg(test)]
mod endpoint_module_path_tests {
    use super::*;
    use std::path::PathBuf;

    fn endpoint(rel_path: PathBuf) -> RemoteEndpoint {
        RemoteEndpoint {
            host: "h".into(),
            port: 9031,
            path: RemotePath::Module {
                module: "m".into(),
                rel_path,
            },
        }
    }

    /// review otp-10a F2: a rel_path assembled via `PathBuf::join` (the
    /// rsync destination-resolution rule appends the source file name
    /// this way) must reach the wire in POSIX form on every platform —
    /// on Windows the joined form carries a native `\` that would
    /// otherwise land verbatim in `SessionOpen.path`.
    #[test]
    fn joined_rel_path_reaches_the_wire_in_posix_form() {
        let rel = PathBuf::from("sub").join("dir").join("file.txt");
        let (module, path) = endpoint_module_path(&endpoint(rel)).expect("module form resolves");
        assert_eq!(module, "m");
        assert_eq!(path, "sub/dir/file.txt");
    }

    /// Empty rel_path is the module-root identity ("" on the wire).
    #[test]
    fn empty_rel_path_is_the_module_root() {
        let (_, path) = endpoint_module_path(&endpoint(PathBuf::new())).expect("resolves");
        assert_eq!(path, "");
    }
}

/// ph-1 record-shape coverage. A push or pull session cannot be built
/// without a live daemon, so the shape contract is pinned on the
/// builder both session paths run through rather than faked around one.
#[cfg(test)]
mod session_record_tests {
    use super::*;
    use crate::perf_history::{
        CompareModeSnapshot, Initiator, LocalRole, RouteTag, RunKind, Topology, TransferMode,
    };

    fn route() -> RouteTag {
        RouteTag {
            topology: Topology::Remote,
            local_role: LocalRole::Source,
            initiator: Initiator::Cli,
            peer_key: Some("host:/mod/sub".to_string()),
        }
    }

    fn facts() -> SessionRecordFacts {
        SessionRecordFacts {
            mirror_enabled: false,
            compare_mode: ComparisonMode::SizeMtime,
            files: 12,
            bytes: 3400,
            files_failed: 0,
            elapsed: Duration::from_millis(250),
        }
    }

    /// Every comparison policy the wire can carry maps onto a
    /// snapshot variant. The enumeration is built by probing the
    /// proto's whole discriminant space, so a new variant joins the
    /// list automatically, and the expectation is an exhaustive
    /// `match` with no wildcard arm, so that new variant is a compile
    /// error here until it is mapped deliberately — never a silent
    /// slide into the SizeMtime bucket (R59 #5's contamination shape).
    #[test]
    fn every_wire_compare_mode_maps_onto_a_snapshot() {
        let variants: Vec<ComparisonMode> = (0..64)
            .filter_map(|value| ComparisonMode::try_from(value).ok())
            .collect();
        assert_eq!(
            variants.len(),
            6,
            "proto ComparisonMode variant set changed: {variants:?}"
        );
        for mode in variants {
            let expected = match mode {
                // The wire documents UNSPECIFIED as "treat as SIZE_MTIME".
                ComparisonMode::Unspecified | ComparisonMode::SizeMtime => {
                    CompareModeSnapshot::SizeMtime
                }
                ComparisonMode::Checksum => CompareModeSnapshot::Checksum,
                ComparisonMode::SizeOnly => CompareModeSnapshot::SizeOnly,
                ComparisonMode::Force => CompareModeSnapshot::Force,
                ComparisonMode::IgnoreTimes => CompareModeSnapshot::IgnoreTimes,
            };
            assert_eq!(compare_snapshot(mode), expected, "mapping for {mode:?}");
        }
    }

    /// The route the caller experienced lands on the record verbatim —
    /// a record without it is invisible to every per-route aggregate
    /// and seed lookup ph-2/ph-3 build on.
    #[test]
    fn session_record_carries_the_callers_route() {
        let record = build_session_record(&route(), facts());
        assert_eq!(record.topology, Topology::Remote);
        assert_eq!(record.local_role, LocalRole::Source);
        assert_eq!(record.initiator, Initiator::Cli);
        assert_eq!(record.peer_key.as_deref(), Some("host:/mod/sub"));
        assert_eq!(record.fast_path.as_deref(), Some("session"));
        // A session never carries a dry plan, so the lane is Real —
        // the filter every production consumer keys on (R56-F1).
        assert_eq!(record.run_kind, RunKind::Real);
        assert_eq!(record.file_count, 12);
        assert_eq!(record.total_bytes, 3400);
        assert_eq!(record.transfer_duration_ms, 250);
        assert_eq!(record.options.workers, 0);
    }

    /// Mirror intent comes off the session's own mirror flag, not the
    /// verb name — a push and a pull built from the same flag record
    /// the same mode.
    #[test]
    fn session_record_mode_follows_the_mirror_flag() {
        let copy = build_session_record(&route(), facts());
        assert_eq!(copy.mode, TransferMode::Copy);
        let mirror = build_session_record(
            &route(),
            SessionRecordFacts {
                mirror_enabled: true,
                ..facts()
            },
        );
        assert_eq!(mirror.mode, TransferMode::Mirror);
    }

    /// The wire counts failures in `u64`; the record's `error_count` is
    /// `u32`. A saturating conversion keeps a catastrophic run's record
    /// writable (and honest about "very many") instead of wrapping to a
    /// small number — `as` would report 0 for exactly 2^32 failures.
    #[test]
    fn session_record_clamps_the_failure_count() {
        let record = build_session_record(
            &route(),
            SessionRecordFacts {
                files_failed: u64::from(u32::MAX) + 1,
                ..facts()
            },
        );
        assert_eq!(record.error_count, u32::MAX);
        let exact = build_session_record(
            &route(),
            SessionRecordFacts {
                files_failed: 7,
                ..facts()
            },
        );
        assert_eq!(exact.error_count, 7);
    }

    /// The legacy `checksum` bool and the full `compare_mode` snapshot
    /// stay consistent: the bool is true exactly for the checksum
    /// policy, so a pre-R59 consumer reading only the bool is not
    /// misled by a `--size-only` run.
    #[test]
    fn session_record_snapshot_tracks_the_compare_policy() {
        let checksum = build_session_record(
            &route(),
            SessionRecordFacts {
                compare_mode: ComparisonMode::Checksum,
                ..facts()
            },
        );
        assert!(checksum.options.checksum);
        assert_eq!(checksum.options.compare_mode, CompareModeSnapshot::Checksum);

        let size_only = build_session_record(
            &route(),
            SessionRecordFacts {
                compare_mode: ComparisonMode::SizeOnly,
                ..facts()
            },
        );
        assert!(!size_only.options.checksum);
        assert_eq!(
            size_only.options.compare_mode,
            CompareModeSnapshot::SizeOnly
        );
    }
}

/// The `Transfer` RPC failed at OPEN — before any session frame flowed.
/// A distinct error type (not a bare `SessionFault`) so callers can
/// classify EVERY open-time failure structurally as a negotiation
/// failure (review otp-9b F3 — the old typed `PullSyncError` boundary
/// treated every pre-response RPC failure as NEGOTIATE); the inner
/// fault still carries the closest session code for the message.
#[derive(Debug)]
pub struct TransferOpenRefusal(pub crate::transfer_session::SessionFault);

impl std::fmt::Display for TransferOpenRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TransferOpenRefusal {}

/// Map an open-time gRPC status onto the `SessionFault` code the same
/// refusal would carry as a session frame. An `Unimplemented`
/// Transfer only means a pre-session peer — the same refusal shape a
/// contract mismatch carries; `PermissionDenied` is the peer's own
/// delegation/ACL gate; anything else keeps INTERNAL, with the
/// [`TransferOpenRefusal`] wrapper preserving the open-phase identity.
fn transfer_open_refusal(status: tonic::Status) -> TransferOpenRefusal {
    use crate::generated::session_error::Code;
    let code = match status.code() {
        tonic::Code::Unimplemented => Code::BuildMismatch,
        tonic::Code::PermissionDenied => Code::DelegationRefused,
        _ => Code::Internal,
    };
    TransferOpenRefusal(crate::transfer_session::SessionFault::refusal(
        code,
        format!("opening Transfer RPC: {}", status.message()),
    ))
}

fn rpc_status_outcome(status: &tonic::Status) -> TransferLifecycleOutcome {
    match status.code() {
        tonic::Code::PermissionDenied
        | tonic::Code::Unauthenticated
        | tonic::Code::Unimplemented
        | tonic::Code::FailedPrecondition
        | tonic::Code::InvalidArgument => TransferLifecycleOutcome::Refused,
        _ => TransferLifecycleOutcome::Error,
    }
}

/// Build a `BlitClient` over `endpoint`'s control-plane URI with a
/// bounded connect (audit-2's 30 s policy, inherited from the old
/// drivers' connect path).
/// `pub` since otp-9b: the delegated dst daemon connects separately
/// from running the session so connect failures keep their own phase.
pub async fn connect_transfer_client(endpoint: &RemoteEndpoint) -> Result<BlitClient<Channel>> {
    connect_transfer_client_with_trace(endpoint, &TransferLifecycleTrace::disabled()).await
}

/// [`connect_transfer_client`] with explicit lifecycle boundaries for an
/// initiating process.
pub async fn connect_transfer_client_with_trace(
    endpoint: &RemoteEndpoint,
    lifecycle_trace: &TransferLifecycleTrace,
) -> Result<BlitClient<Channel>> {
    let uri = endpoint.control_plane_uri();
    lifecycle_trace.record("control_connect_begin", None);
    let conn = match Endpoint::from_shared(uri.clone()) {
        Ok(endpoint) => endpoint.connect_timeout(Duration::from_secs(30)),
        Err(err) => {
            lifecycle_trace.record("control_connect_end", Some(TransferLifecycleOutcome::Error));
            return Err(eyre!("invalid endpoint uri {uri}: {err}"));
        }
    };
    let channel = match tokio::time::timeout(Duration::from_secs(30), conn.connect()).await {
        Ok(Ok(channel)) => channel,
        Ok(Err(err)) => {
            lifecycle_trace.record("control_connect_end", Some(TransferLifecycleOutcome::Error));
            return Err(eyre!("connecting to {uri}: {err}"));
        }
        Err(_) => {
            lifecycle_trace.record("control_connect_end", Some(TransferLifecycleOutcome::Error));
            return Err(eyre!("timed out connecting to {uri}"));
        }
    };
    lifecycle_trace.record(
        "control_connect_end",
        Some(TransferLifecycleOutcome::Success),
    );
    Ok(BlitClient::new(channel))
}
