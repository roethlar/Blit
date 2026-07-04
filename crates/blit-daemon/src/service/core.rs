use super::admin::{
    delete_rel_paths, filesystem_stats_for_path, list_completions, sanitize_request_paths,
    split_completion_prefix, stream_disk_usage, stream_find_entries,
};
use super::pull_sync::handle_pull_sync_stream;
use super::push::handle_push_stream;
use super::util::{
    metadata_mtime_seconds, resolve_contained_path, resolve_module, resolve_relative_path,
};
use super::{DiskUsageSender, FindSender};
use crate::active_jobs::{ActiveJobKind, ActiveJobs, CancelOutcome};
use crate::metrics::TransferMetrics;
use crate::runtime::{ModuleConfig, RootExport};
use blit_core::generated::blit_server::Blit;
pub use blit_core::generated::blit_server::BlitServer;
use blit_core::generated::{
    daemon_event, ActiveTransfer, CancelJobRequest, CancelJobResponse, ClearRecentRequest,
    ClearRecentResponse, ClientPullMessage, ClientPushRequest, CompletionRequest,
    CompletionResponse, Counters, DaemonEvent, DaemonState, DelegatedPullProgress,
    DelegatedPullRequest, DiskUsageEntry, DiskUsageRequest, FileInfo, FilesystemStatsRequest,
    FilesystemStatsResponse, FindEntry, FindRequest, GetStateRequest, ListModulesRequest,
    ListModulesResponse, ListRequest, ListResponse, ModuleInfo, PurgeRequest, PurgeResponse,
    ServerPullMessage, ServerPushResponse, SubscribeRequest, TransferComplete, TransferError,
    TransferProgress, TransferRecord, TransferStarted,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status, Streaming};

/// Capacity of the daemon's `Subscribe` event broadcast channel.
/// Sized for a handful of subscribers (operator TUI + maybe a
/// Prometheus scraper bridge) plus burst headroom — enough that a
/// momentary stall on the subscriber side doesn't immediately drop
/// events. Slow consumers that lag more than this many events behind
/// receive a `tonic::Status::aborted` and re-subscribe.
const SUBSCRIBE_BROADCAST_CAPACITY: usize = 256;

/// Capacity of the per-subscriber `mpsc` buffer behind the
/// c-5a Subscribe forwarder. Sized so a momentary client stall
/// (one or two tick intervals' worth of matching events) doesn't
/// back up into the forwarder. Smaller than
/// `SUBSCRIBE_BROADCAST_CAPACITY` because the filter is already
/// applied — the buffer holds only events the client wanted.
/// A client whose mpsc fills causes the forwarder to block on
/// `send().await`, which eventually triggers a broadcast Lagged
/// when global event rate exceeds the broadcast ring capacity —
/// the correct "this client really is too slow" signal.
const SUBSCRIBE_MPSC_CAPACITY: usize = 64;

/// Cadence of the c-4 progress ticker. Default 100ms (10 Hz) —
/// matches the TUI_DESIGN.md §6.2 step-3 estimate. The cost is one
/// broadcast event per active transfer per tick, so at typical
/// active-counts of 1-4 transfers we send 10-40 events/sec.
/// Subscribers that can't keep up get the c-2 Lagged → Status::aborted
/// path; the broadcast itself never blocks the ticker.
pub const DEFAULT_PROGRESS_TICK_MS: u64 = 100;

pub struct BlitService {
    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    default_root: Option<RootExport>,
    force_grpc_data: bool,
    server_checksums_enabled: bool,
    metrics: Arc<TransferMetrics>,
    /// Delegation gate config. The handler reads it on every
    /// `DelegatedPull` request; default-disabled means no caller can
    /// make this daemon initiate outbound connects until the operator
    /// flips `[delegation] allow_delegated_pull = true`.
    pub(crate) delegation: Arc<crate::delegation_gate::DelegationConfig>,
    /// Always-on registry of in-flight transfers. Populated
    /// from the dispatch boundary in this file; read by
    /// `GetState.active[]` once that RPC lands (milestone B
    /// sub-slice). See `crate::active_jobs`.
    pub(crate) active_jobs: ActiveJobs,
    /// Daemon-side event broadcast channel feeding the `Subscribe`
    /// RPC. Producers (dispatch sites in this file) send `DaemonEvent`
    /// payloads; subscribers receive their own `Receiver` via
    /// `events_tx.subscribe()`. Default capacity
    /// `SUBSCRIBE_BROADCAST_CAPACITY` (256) — slow subscribers that
    /// lag past that get a `tonic::Status::aborted` and re-subscribe.
    events_tx: broadcast::Sender<DaemonEvent>,
    /// Wall-clock at construction. `GetState.uptime_seconds`
    /// reports `Instant::now().duration_since(started_at)`.
    /// Captured once so a clock jump between construction and
    /// the GetState call doesn't show up as negative uptime.
    started_at: std::time::Instant,
}

impl BlitService {
    pub(crate) fn from_runtime(
        modules: HashMap<String, ModuleConfig>,
        default_root: Option<RootExport>,
        force_grpc_data: bool,
        server_checksums_enabled: bool,
        metrics: Arc<TransferMetrics>,
        delegation: crate::delegation_gate::DelegationConfig,
    ) -> Self {
        let (events_tx, _) = broadcast::channel(SUBSCRIBE_BROADCAST_CAPACITY);
        Self {
            modules: Arc::new(Mutex::new(modules)),
            default_root,
            force_grpc_data,
            server_checksums_enabled,
            metrics,
            delegation: Arc::new(delegation),
            active_jobs: ActiveJobs::new(),
            events_tx,
            started_at: std::time::Instant::now(),
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn with_modules(
        modules: HashMap<String, ModuleConfig>,
        force_grpc_data: bool,
    ) -> Self {
        Self::from_runtime(
            modules,
            None,
            force_grpc_data,
            true,
            TransferMetrics::disabled(),
            crate::delegation_gate::DelegationConfig::default(),
        )
    }

    /// Inner purge body. Extracted from the trait method so the
    /// `--metrics` completion log can wrap a single call site and
    /// branch on Result without duplicating the response shape.
    /// §3.1 followup.
    async fn purge_inner(&self, req: PurgeRequest) -> Result<Response<PurgeResponse>, Status> {
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
        if module.read_only {
            return Err(Status::permission_denied(format!(
                "module '{}' is read-only",
                module.name
            )));
        }
        let sanitized = sanitize_request_paths(req.paths_to_delete)?;
        if sanitized.is_empty() {
            return Ok(Response::new(PurgeResponse { files_deleted: 0 }));
        }
        let stats = delete_rel_paths(
            module.path.clone(),
            module.canonical_root.clone(),
            sanitized,
        )
        .await?;
        Ok(Response::new(PurgeResponse {
            files_deleted: stats.total(),
        }))
    }

    /// Send a `TransferStarted` event onto the broadcast channel.
    /// Called from each RPC dispatch site immediately after the
    /// `ActiveJob` is registered, with the same values that
    /// populated the row. A `SendError` return from `broadcast::Sender::send`
    /// just means there are no current subscribers — that is the
    /// normal state and we ignore it.
    ///
    /// Caller-passed values rather than re-reading from the
    /// `ActiveJobGuard`: the dispatch site already has all the
    /// inputs in scope as locals, and using them here avoids a
    /// table lookup + clone on every transfer. Module/path are
    /// empty strings for streaming RPCs at registration time;
    /// that matches `GetState.active[]`'s view of the same row.
    pub(crate) fn emit_transfer_started(
        &self,
        guard: &crate::active_jobs::ActiveJobGuard,
        kind: ActiveJobKind,
        peer: &str,
        module: &str,
        path: &str,
    ) {
        let event = DaemonEvent {
            payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                transfer_id: guard.transfer_id().to_string(),
                kind: kind.to_wire() as i32,
                peer: peer.to_string(),
                module: module.to_string(),
                path: path.to_string(),
                start_unix_ms: guard.start_unix_ms(),
            })),
        };
        // c-5b: emit via ActiveJobs so the event lands in the
        // per-row ring (for future replay) AND on the
        // broadcast (for live subscribers). Both happen under
        // the table lock — see emit_event rustdoc.
        self.active_jobs
            .emit_event(&self.events_tx, guard.transfer_id(), event);
    }

    /// Clone of the broadcast sender, handed to spawn closures
    /// that need to fire terminal `TransferComplete` /
    /// `TransferError` events after `record_outcome` but before
    /// the guard drops. The closures don't otherwise see `&self`,
    /// so the dispatch site clones this once and moves the clone
    /// into the spawned task. Each `broadcast::Sender::clone` is
    /// an Arc bump; effectively free.
    pub(crate) fn events_tx(&self) -> broadcast::Sender<DaemonEvent> {
        self.events_tx.clone()
    }
}

/// Drive one tick of the c-4 progress emitter. For each row in the
/// `active_jobs` table, computes throughput since the previous tick
/// and broadcasts a `TransferProgress` event with the row's current
/// byte count.
///
/// Factored as a free function so the daemon's tokio interval task
/// can drive it on a timer AND tests can call it directly without
/// standing up a runtime + timer.
///
/// Returns the number of events emitted (one per active row).
pub(crate) fn tick_progress_once(
    active_jobs: &crate::active_jobs::ActiveJobs,
    events_tx: &broadcast::Sender<DaemonEvent>,
) -> usize {
    // c-5b: dispatch through `tick_progress_emit`, which
    // builds AND broadcasts AND pushes to the per-row event
    // ring all under the same table lock. The lock window
    // serializes against c-3 terminal-event Drop (which also
    // takes the table lock) and against c-5b subscribe
    // snapshots (which use the same lock), so progress events
    // can never be observed after a same-id terminal event
    // and can never be missed/duplicated by replaying
    // subscribers.
    active_jobs.tick_progress_emit(events_tx, |sample| DaemonEvent {
        payload: Some(daemon_event::Payload::TransferProgress(TransferProgress {
            transfer_id: sample.transfer_id.clone(),
            bytes_completed: sample.bytes_completed,
            // bytes_total and files_* land in follow-up C sub-
            // slices that wire the manifest stage and the
            // files counter.
            bytes_total: 0,
            files_completed: 0,
            files_total: 0,
            throughput_bps: sample.throughput_bps,
        })),
    })
}

/// Spawn the long-running progress ticker. Called from the daemon
/// binary's `main` after the service is constructed; the returned
/// `JoinHandle` lives as long as the daemon process.
///
/// Tests don't spawn this — they call [`tick_progress_once`]
/// directly so the test ordering is deterministic and doesn't pick
/// up arbitrary background events.
#[allow(dead_code)]
pub fn spawn_progress_ticker(svc: &BlitService) -> tokio::task::JoinHandle<()> {
    let active_jobs = svc.active_jobs.clone();
    let events_tx = svc.events_tx.clone();
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_millis(DEFAULT_PROGRESS_TICK_MS));
        // Skip behavior: if the daemon is paused (e.g. by a long
        // single-threaded operation) we don't want to fire a burst
        // of catch-up ticks afterwards.
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let _ = tick_progress_once(&active_jobs, &events_tx);
        }
    })
}

/// c-5a: decide whether a `DaemonEvent` should be forwarded to a
/// subscriber whose `SubscribeRequest.transfer_id_filter` is set
/// to `filter`. Empty filter accepts everything. Non-empty
/// filter accepts only transfer-scoped events whose `transfer_id`
/// matches.
///
/// Non-transfer-scoped events (future variants: `ModuleListChanged`,
/// `DaemonHeartbeat`) bypass the filter entirely so a subscriber
/// tracking a specific transfer still sees daemon-wide state
/// updates relevant to context — they're cheap and provide
/// orientation.
pub(crate) fn event_matches_filter(event: &DaemonEvent, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    // Exhaustive match over current variants. Future
    // non-transfer-scoped variants (ModuleListChanged,
    // DaemonHeartbeat) should be added with the explicit
    // policy "bypass the filter and always forward" so
    // subscribers tracking one transfer still see daemon-wide
    // context. Adding any future variant here is a compile-
    // forced decision — exactly what we want.
    match event.payload.as_ref() {
        Some(daemon_event::Payload::TransferStarted(e)) => e.transfer_id == filter,
        Some(daemon_event::Payload::TransferProgress(e)) => e.transfer_id == filter,
        Some(daemon_event::Payload::TransferComplete(e)) => e.transfer_id == filter,
        Some(daemon_event::Payload::TransferError(e)) => e.transfer_id == filter,
        None => false,
    }
}

/// Build the terminal event for a transfer that's draining. Called
/// from each RPC's spawn closure after `record_outcome` and before
/// `drop(job)`, with the same `(ok, error_message)` pair that the
/// ActiveJobs ring records. Pairs with `emit_transfer_started` on
/// the receive side: every transfer that emitted Started will also
/// emit either Complete or Error.
///
/// Sourced from the guard so byte total and duration match what
/// `GetState.recent[]` will surface on the same row.
pub(crate) fn build_transfer_finished_event(
    guard: &crate::active_jobs::ActiveJobGuard,
    ok: bool,
    error_message: Option<&str>,
) -> DaemonEvent {
    if ok {
        DaemonEvent {
            payload: Some(daemon_event::Payload::TransferComplete(TransferComplete {
                transfer_id: guard.transfer_id().to_string(),
                bytes: guard.bytes_completed_load(),
                // `files` is wired in a follow-up C sub-slice
                // (file-level counter analogous to bytes).
                files: 0,
                duration_ms: guard.elapsed_ms(),
                // `tcp_fallback_used` plumbs through the handler's
                // result struct in a follow-up; false today.
                tcp_fallback_used: false,
            })),
        }
    } else {
        DaemonEvent {
            payload: Some(daemon_event::Payload::TransferError(TransferError {
                transfer_id: guard.transfer_id().to_string(),
                message: error_message.unwrap_or("").to_string(),
            })),
        }
    }
}

#[tonic::async_trait]
impl Blit for BlitService {
    type PushStream = ReceiverStream<Result<ServerPushResponse, Status>>;
    type PullSyncStream = ReceiverStream<Result<ServerPullMessage, Status>>;
    type FindStream = ReceiverStream<Result<FindEntry, Status>>;
    type DiskUsageStream = ReceiverStream<Result<DiskUsageEntry, Status>>;
    type DelegatedPullStream = ReceiverStream<Result<DelegatedPullProgress, Status>>;
    type SubscribeStream =
        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<DaemonEvent, Status>> + Send>>;

    async fn subscribe(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let req = request.into_inner();
        let transfer_id_filter = req.transfer_id_filter;
        // c-5b: atomically register a broadcast Receiver AND
        // snapshot the per-row event ring (if replay_recent &&
        // filter is non-empty AND the row exists). Both happen
        // under the table lock so no event can be observed
        // both via replay and via broadcast — see emit_event /
        // subscribe_with_ring rustdoc for the full ordering.
        let (mut broadcast_rx, replay) = self.active_jobs.subscribe_with_ring(
            &self.events_tx,
            &transfer_id_filter,
            req.replay_recent,
        );

        // c-5a round 2: per-subscriber forwarder. The round-1
        // shape (returning `BroadcastStream::filter_map` directly)
        // still advanced the subscriber's broadcast cursor
        // through every event — so a `jobs watch <id>` consumer
        // could be aborted with Lagged when unrelated transfers
        // overflowed the global ring, even though the filter
        // rejected those events anyway.
        //
        // Fix: spawn a task that eagerly drains the broadcast
        // (cursor stays caught up independent of client read
        // pace), applies the filter, and forwards only matching
        // events into a bounded per-subscriber `mpsc`. The mpsc
        // receiver is what tonic streams to the client.
        //
        // Lagged semantics now mean "the FORWARDER couldn't
        // keep up with global event rate" — a daemon-side CPU
        // problem, not "this client is slow on its filtered
        // subset." If the client is slow on the matching
        // subset the mpsc fills first, the forwarder's
        // `send().await` blocks, and Lagged eventually fires
        // through the normal broadcast over-capacity path —
        // which is the correct "this client really is too
        // slow" signal.
        let (tx, rx) = mpsc::channel::<Result<DaemonEvent, Status>>(SUBSCRIBE_MPSC_CAPACITY);
        tokio::spawn(async move {
            // c-5b: drain replay events first (empty Vec when
            // replay_recent=false or filter is empty or row
            // doesn't exist). The forwarder then transitions
            // to live broadcast forwarding. Note that replay
            // events have ALREADY been deduped against the
            // broadcast Receiver under the table lock — the
            // ordering contract in `subscribe_with_ring` and
            // `emit_event` ensures any event in `replay` is
            // NOT also in `broadcast_rx`.
            for event in replay {
                if tx.send(Ok(event)).await.is_err() {
                    // Client dropped before consuming replay.
                    return;
                }
            }
            loop {
                // c-5a round 3: race broadcast recv against
                // `tx.closed()` so the forwarder exits as soon
                // as the client drops the stream — not only
                // when a matching event happens to arrive.
                // Without this race a filtered watcher that
                // disconnects during a quiet period (no
                // further matching events) leaks a task + a
                // live broadcast Receiver indefinitely, since
                // unrelated events are filtered with `continue`
                // and never touch `tx`.
                let recv = tokio::select! {
                    biased;
                    () = tx.closed() => break,
                    recv = broadcast_rx.recv() => recv,
                };
                match recv {
                    Ok(event) => {
                        if !event_matches_filter(&event, &transfer_id_filter) {
                            continue;
                        }
                        if tx.send(Ok(event)).await.is_err() {
                            // Client dropped between filter and send.
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        let _ = tx
                            .send(Err(Status::aborted(format!(
                                "subscriber lagged {n} events; re-subscribe and refresh via GetState"
                            ))))
                            .await;
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        // Daemon shutdown — broadcast Sender
                        // dropped. Stream ends cleanly.
                        break;
                    }
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn push(
        &self,
        request: Request<Streaming<ClientPushRequest>>,
    ) -> Result<Response<Self::PushStream>, Status> {
        let peer = peer_addr_string(&request);
        let modules = Arc::clone(&self.modules);
        let (tx, rx) = mpsc::channel(32);
        let stream = request.into_inner();
        let force_grpc_data = self.force_grpc_data;
        let default_root = self.default_root.clone();
        // Counter increments at the dispatch boundary — single chokepoint
        // per RPC, no reach-in to the transfer pipeline. No-op when
        // metrics are disabled (default). The active-transfers gauge
        // uses an RAII guard so panic/cancellation can't leak it
        // (F5 of docs/reviews/codebase_review_2026-05-01.md).
        let metrics = Arc::clone(&self.metrics);
        metrics.inc_push();
        let guard = Arc::clone(&metrics).enter_transfer();
        // ActiveJobs row registered with empty module/path —
        // those arrive in the first stream frame; the handler
        // calls `job.set_endpoint(...)` once the header is
        // parsed (b-2-set-endpoint).
        let job = self.active_jobs.register(
            ActiveJobKind::Push,
            peer.clone(),
            String::new(),
            String::new(),
        );
        // Subscribe event — fired with the empty module/path the
        // row currently carries. Subscribers reconcile to the
        // populated endpoint by reading GetState.active[] after
        // the first stream frame; a separate "endpoint resolved"
        // event family member is a future C slice.
        self.emit_transfer_started(&job, ActiveJobKind::Push, &peer, "", "");
        // §3.1 / D5: capture start time so `--metrics` can emit a
        // per-RPC duration line at completion.
        let started = std::time::Instant::now();
        // c-3: clone the broadcast Sender into the spawn closure
        // so the terminal-event emit (TransferComplete on success,
        // TransferError on failure) can fire without `&self`.
        let events_tx = self.events_tx();

        tokio::spawn(async move {
            // `guard` and `job` are moved into the task; their
            // Drop fires no matter how the task ends.
            let guard = guard;
            let job = job;
            // w4-3: cloned off the guard so the select can hold a
            // `cancelled()` future while the handler borrows `job`.
            let cancel_token = job.cancellation_token().clone();
            // w4-3: race the handler against client hangup and the
            // row's cancel token instead of bare-awaiting it. Pre-fix
            // a client that disconnected during a send-free compute
            // phase left this task running the full handler for a
            // dead peer — the mechanism `delegated_pull` has had
            // since R30-F2. See `resolve_streaming_outcome`.
            let (ok, err_msg) = resolve_streaming_outcome(
                handle_push_stream(
                    modules,
                    default_root,
                    stream,
                    tx.clone(),
                    force_grpc_data,
                    &job,
                ),
                &tx,
                &cancel_token,
                &metrics,
            )
            .await;
            // Record the outcome before dropping the
            // ActiveJob guard — Drop builds the recent-runs
            // TransferRecord and reads this cell. If we
            // dropped the guard first the record would say
            // "cancelled before outcome recorded."
            job.record_outcome(ok, err_msg.clone());
            // c-3 round 2: build the terminal event from the
            // still-alive guard (we need its byte counter +
            // start_unix_ms), drain the daemon's bookkeeping
            // (active row + metrics gauge + error counter),
            // and ONLY THEN broadcast. A subscriber that races
            // GetState immediately after seeing the event will
            // observe the transfer already drained from
            // active[] and present in recent[] — the event
            // signals reconcilable state, not "about to drain."
            let finished_event = build_transfer_finished_event(&job, ok, err_msg.as_deref());
            drop(job);
            // §3.1 followup: drop the active-transfer guard BEFORE the
            // completion log so `active=N` reflects state AFTER the
            // just-finished RPC is removed from the gauge. Pre-fix
            // a single-transfer log showed `active=1`, which is
            // misleading for an end-of-RPC summary.
            drop(guard);
            let _ = events_tx.send(finished_event);
            metrics.log_completion("push", started.elapsed(), ok);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn pull_sync(
        &self,
        request: Request<Streaming<ClientPullMessage>>,
    ) -> Result<Response<Self::PullSyncStream>, Status> {
        let peer = peer_addr_string(&request);
        let modules = Arc::clone(&self.modules);
        let (tx, rx) = mpsc::channel(32);
        let stream = request.into_inner();
        let force_grpc_data = self.force_grpc_data;
        let default_root = self.default_root.clone();
        let server_checksums_enabled = self.server_checksums_enabled;
        let metrics = Arc::clone(&self.metrics);
        metrics.inc_pull();
        let guard = Arc::clone(&metrics).enter_transfer();
        // Same shape as `push` above: module + path arrive in
        // the first stream frame; handler calls
        // `job.set_endpoint(...)` after parsing the spec.
        let job = self.active_jobs.register(
            ActiveJobKind::PullSync,
            peer.clone(),
            String::new(),
            String::new(),
        );
        // Subscribe event with empty module/path — same caveat
        // as the push site above. Subscribers reconcile via
        // GetState.active[].
        self.emit_transfer_started(&job, ActiveJobKind::PullSync, &peer, "", "");
        let started = std::time::Instant::now();
        let events_tx = self.events_tx();

        tokio::spawn(async move {
            let guard = guard;
            let job = job;
            // w4-3: same handler-vs-hangup-vs-cancel race as the push
            // site above — pull_sync's enumerate+checksum collection
            // is the longest send-free compute window of the three
            // transfer RPCs, so it was the most exposed to running to
            // completion for a client that had already disconnected.
            let cancel_token = job.cancellation_token().clone();
            let (ok, err_msg) = resolve_streaming_outcome(
                handle_pull_sync_stream(
                    modules,
                    default_root,
                    stream,
                    tx.clone(),
                    force_grpc_data,
                    server_checksums_enabled,
                    &job,
                ),
                &tx,
                &cancel_token,
                &metrics,
            )
            .await;
            job.record_outcome(ok, err_msg.clone());
            // c-3 round 2: same ordering as push/pull — build,
            // drain, then broadcast so subscribers can race
            // GetState and see reconcilable state.
            let finished_event = build_transfer_finished_event(&job, ok, err_msg.as_deref());
            drop(guard);
            drop(job);
            let _ = events_tx.send(finished_event);
            metrics.log_completion("pull_sync", started.elapsed(), ok);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn delegated_pull(
        &self,
        request: Request<DelegatedPullRequest>,
    ) -> Result<Response<Self::DelegatedPullStream>, Status> {
        let peer = peer_addr_string(&request);
        let req = request.into_inner();
        // ActiveJobs row mirrors the metrics gauge — both are
        // owned by the spawned task so the row drains on every
        // termination path (success, handler failure, client
        // hangup). Module + dst path come straight off the
        // request; they're synchronously available here unlike
        // the streaming RPCs (push, pull_sync), which register
        // with empty endpoint strings and have their handlers
        // fill them in via `ActiveJobGuard::set_endpoint` once
        // the first stream frame parses.
        let job = self.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            peer.clone(),
            req.dst_module.clone(),
            req.dst_destination_path.clone(),
        );
        // Subscribe event — module/path are populated for
        // delegated_pull at dispatch time (unlike push/pull_sync).
        self.emit_transfer_started(
            &job,
            ActiveJobKind::DelegatedPull,
            &peer,
            &req.dst_module,
            &req.dst_destination_path,
        );
        // Captured before `req` moves into the handler call.
        // Drives the conditional select arm below: when
        // `detach=true` the `tx.closed()` race is disabled, so
        // a CLI disconnect no longer drops the transfer
        // future. The transfer then runs to completion,
        // failure, or `CancelJob(transfer_id)` regardless of
        // client connection state.
        let detach = req.detach;
        let transfer_id_for_started = job.transfer_id().to_string();
        // c-1b: byte-progress sink fed by the data-plane write
        // loop inside `pull_sync_with_spec`. Reports land on the
        // same atomic the table row holds, so GetState sees live
        // progress while the transfer is in flight.
        let byte_progress = job.bytes_counter();
        let modules = Arc::clone(&self.modules);
        let default_root = self.default_root.clone();
        let delegation = Arc::clone(&self.delegation);
        let metrics = Arc::clone(&self.metrics);
        let metrics_for_log = Arc::clone(&self.metrics);
        let (tx, rx) = mpsc::channel(32);
        // §3.1 followup: cover delegated_pull in the per-RPC summary
        // log too. The handler increments `pull_ops` + the active
        // gauge inside `run_delegated_pull` (delegated_pull.rs:227),
        // so without this site `delegated_pull` would count toward
        // `pull_ops` but never emit its own completion line.
        let started = std::time::Instant::now();

        // R30-F2: race the handler against tx.closed() so a CLI
        // disconnect drops the inner pull future. tonic's response
        // stream drops the mpsc Receiver when the client cancels;
        // that closes the Sender, and tx.closed() resolves. The
        // handler's pull_sync_with_spec future is then dropped,
        // which propagates cancellation through the existing pull
        // cancellation path (data plane connection drop, manifest
        // task cleanup). Without this race the spawned task would
        // continue to write — and post-R30-F1 to delete — on dst
        // after the operator has Ctrl-C'd.
        //
        // Cloning tx for the handler so the original tx survives
        // long enough for tx.closed() to be the racing future.
        let handler_tx = tx.clone();
        // Clone the cancellation token off the guard before
        // moving the guard into the spawn task. The future's
        // select needs a `.cancelled()` future; cloning the
        // token (cheap, internal Arc) lets us hold the
        // cancelled-future on its own line.
        let cancel_token = job.cancellation_token().clone();
        let events_tx = self.events_tx();
        tokio::spawn(async move {
            // `job` moves into the spawned task alongside the
            // metrics guard; its Drop runs on every exit path
            // from the select below.
            let job = job;
            // Three-way race (the tx.closed arm is gated by
            // `!detach` — see m-jobs-3):
            //   tx.closed()             → client hung up (R30-F2);
            //                              disabled when detach=true
            //   cancel_token.cancelled() → `CancelJob` RPC fired the
            //                              token from another task
            //                              (m-jobs-1)
            //   handle_delegated_pull → handler ran to completion or
            //                              failure
            //
            // Outcome encoding:
            //   None         → cancelled (client OR CancelJob)
            //   Some(true)   → handler returned success
            //   Some(false)  → handler returned failure (phased
            //                  error already sent to client over
            //                  handler_tx)
            // audit-10: the handler branch is ordered FIRST in the
            // `biased` select inside `resolve_transfer_outcome`, so
            // a handler that has run to completion wins even if the
            // cancel token fires (or the client hangs up) at the same
            // instant. A still-running (Pending) handler still yields to
            // a hangup / `CancelJob`. See that helper for the rationale.
            let outcome: Option<bool> = resolve_transfer_outcome(
                super::delegated_pull::handle_delegated_pull(
                    req,
                    modules,
                    default_root,
                    delegation,
                    metrics,
                    handler_tx,
                    transfer_id_for_started,
                    byte_progress,
                ),
                tx.closed(),
                cancel_token.cancelled(),
                detach,
            )
            .await;
            // Map the select outcome onto the ActiveJobs ring
            // shape:
            //   Some(true)  → ok, no error
            //   Some(false) → handler-failure; the handler
            //                  already sent the phased error to
            //                  the client and surfaced it via
            //                  `metrics.inc_error` below. We
            //                  don't have the message string at
            //                  this level — the C milestone
            //                  routes structured errors. Use a
            //                  short marker.
            //   None        → client hangup or CancelJob.
            //                  Distinguish by inspecting the
            //                  cancellation token: if it was
            //                  cancelled, the cause was
            //                  CancelJob; otherwise it was the
            //                  client.
            let (job_ok, job_err) = match outcome {
                Some(true) => (true, None),
                Some(false) => (false, Some("delegated_pull handler failed".to_string())),
                None if cancel_token.is_cancelled() => {
                    (false, Some("cancelled via CancelJob".to_string()))
                }
                None => (false, Some("client cancelled".to_string())),
            };
            job.record_outcome(job_ok, job_err.clone());
            // c-3 round 2: build the terminal event while the
            // guard is still alive (we need its byte counter
            // + start_unix_ms), but defer the broadcast until
            // AFTER the active row is dropped AND the error
            // counter has been incremented. Without this
            // ordering a subscriber that races GetState after
            // seeing the event could still see active[] or
            // stale counters.
            let finished_event = build_transfer_finished_event(&job, job_ok, job_err.as_deref());
            drop(job);
            // The handler's RAII guard releases the active gauge as
            // its scope ends with the spawn task above, so by the
            // time we log here `active` already excludes this RPC.
            //
            // R-followup: distinguish handler-failure from client-
            // cancellation. Both log `ok=false` (the transfer didn't
            // complete), but only the former is a daemon error and
            // increments `errors`. Pre-fix every non-cancelled run
            // logged `ok=true` regardless of whether
            // `run_delegated_pull` returned Err — a real failure
            // could log `delegated_pull ok … errors=N` with N
            // unchanged.
            let ok = matches!(outcome, Some(true));
            if matches!(outcome, Some(false)) {
                metrics_for_log.inc_error();
            }
            // All bookkeeping (active row, metrics gauge,
            // error counter) is committed — safe to broadcast.
            let _ = events_tx.send(finished_event);
            metrics_for_log.log_completion("delegated_pull", started.elapsed(), ok);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn list(&self, request: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;

        let requested = if req.path.trim().is_empty() {
            PathBuf::from(".")
        } else {
            resolve_relative_path(&req.path)?
        };

        let target = resolve_contained_path(&module, &requested)?;
        let response_entries =
            tokio::task::spawn_blocking(move || -> Result<Vec<FileInfo>, Status> {
                let metadata = fs::metadata(&target).map_err(|err| {
                    Status::internal(format!("stat {}: {}", target.display(), err))
                })?;

                if metadata.is_file() {
                    let name = requested
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| ".".to_string());
                    let info = FileInfo {
                        name,
                        is_dir: false,
                        size: metadata.len(),
                        mtime_seconds: metadata_mtime_seconds(&metadata).unwrap_or(0),
                    };
                    Ok(vec![info])
                } else if metadata.is_dir() {
                    let mut infos = Vec::new();
                    let entries = fs::read_dir(&target).map_err(|err| {
                        Status::internal(format!("read_dir {}: {}", target.display(), err))
                    })?;
                    for entry in entries {
                        let entry = entry.map_err(|err| {
                            Status::internal(format!(
                                "read_dir entry {}: {}",
                                target.display(),
                                err
                            ))
                        })?;
                        let path = entry.path();
                        let meta = entry.metadata().map_err(|err| {
                            Status::internal(format!("metadata {}: {}", path.display(), err))
                        })?;
                        let name = entry.file_name().to_string_lossy().into_owned();
                        infos.push(FileInfo {
                            name,
                            is_dir: meta.is_dir(),
                            size: meta.len(),
                            mtime_seconds: metadata_mtime_seconds(&meta).unwrap_or(0),
                        });
                    }
                    infos.sort_by(|a, b| a.name.cmp(&b.name));
                    Ok(infos)
                } else {
                    Err(Status::invalid_argument(format!(
                        "unsupported path type for list: {}",
                        target.display()
                    )))
                }
            })
            .await
            .map_err(|err| Status::internal(format!("list task failed: {}", err)))??;

        Ok(Response::new(ListResponse {
            entries: response_entries,
        }))
    }

    async fn purge(
        &self,
        request: Request<PurgeRequest>,
    ) -> Result<Response<PurgeResponse>, Status> {
        let req = request.into_inner();
        // F5: counters mark dispatch attempts (matching push/pull
        // semantics). Previously inc_purge fired only after a
        // successful delete, contradicting the metrics-module doc.
        self.metrics.inc_purge();
        // §3.1 followup: purge needs its own completion line.
        // Pre-fix `purge_ops` was visible only on later push/pull
        // logs, never on the purge RPC itself.
        let started = std::time::Instant::now();
        let result = self.purge_inner(req).await;
        let ok = result.is_ok();
        if result.is_err() {
            self.metrics.inc_error();
        }
        self.metrics.log_completion("purge", started.elapsed(), ok);
        result
    }

    async fn complete_path(
        &self,
        request: Request<CompletionRequest>,
    ) -> Result<Response<CompletionResponse>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
        if !req.include_files && !req.include_directories {
            return Err(Status::invalid_argument(
                "at least one of include_files or include_directories must be true",
            ));
        }

        let (dir_rel, display_prefix, leaf_prefix) =
            split_completion_prefix(req.path_prefix.as_str())?;
        let search_root = resolve_contained_path(&module, &dir_rel)?;
        let include_files = req.include_files;
        let include_dirs = req.include_directories;

        let entries = tokio::task::spawn_blocking(move || {
            list_completions(
                &search_root,
                &display_prefix,
                &leaf_prefix,
                include_files,
                include_dirs,
            )
        })
        .await
        .map_err(|err| Status::internal(format!("completion task failed: {}", err)))??;

        Ok(Response::new(CompletionResponse {
            completions: entries,
        }))
    }

    async fn list_modules(
        &self,
        _request: Request<ListModulesRequest>,
    ) -> Result<Response<ListModulesResponse>, Status> {
        let guard = self.modules.lock().await;
        let mut modules: Vec<ModuleInfo> = guard
            .values()
            .map(|module| ModuleInfo {
                name: module.name.clone(),
                path: module.path.to_string_lossy().into_owned(),
                read_only: module.read_only,
            })
            .collect();
        modules.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Response::new(ListModulesResponse { modules }))
    }

    async fn get_state(
        &self,
        request: Request<GetStateRequest>,
    ) -> Result<Response<DaemonState>, Status> {
        use std::sync::atomic::Ordering;
        let req = request.into_inner();
        // `recent_limit` semantics (`proto/blit.proto`): 0 means
        // "use the daemon's default" — return the full ring as
        // sized by `with_recent_limit`. A non-zero value caps
        // the response to the most-recent N entries; the daemon
        // doesn't grow the ring for one request, just truncates
        // for this response.
        let modules = {
            let guard = self.modules.lock().await;
            let mut ms: Vec<ModuleInfo> = guard
                .values()
                .map(|module| ModuleInfo {
                    name: module.name.clone(),
                    path: module.path.to_string_lossy().into_owned(),
                    read_only: module.read_only,
                })
                .collect();
            ms.sort_by(|a, b| a.name.cmp(&b.name));
            ms
        };

        let active: Vec<ActiveTransfer> = self
            .active_jobs
            .snapshot()
            .into_iter()
            .map(|j| ActiveTransfer {
                transfer_id: j.transfer_id,
                kind: j.kind.to_wire() as i32,
                peer: j.peer,
                module: j.module,
                path: j.path,
                start_unix_ms: j.start_unix_ms,
                // `bytes_completed` reads from the per-row atomic
                // (c-1a-byte-counter-api). Stays at zero until
                // c-1b wires the data-plane receive loop to call
                // `ByteProgressSink::report`; the wire shape is
                // already correct.
                bytes_completed: j.bytes_completed,
                // `bytes_total` lands in a subsequent C slice
                // from the manifest stage.
                bytes_total: 0,
            })
            .collect();

        // `recent()` returns oldest-first; truncate from the
        // front when capping to the most-recent N. `as usize`
        // is well-defined: even on 32-bit targets the proto u32
        // can't exceed `usize::MAX`.
        let mut recent_rows = self.active_jobs.recent();
        if req.recent_limit > 0 {
            let limit = req.recent_limit as usize;
            if recent_rows.len() > limit {
                let drop_n = recent_rows.len() - limit;
                recent_rows.drain(0..drop_n);
            }
        }
        let recent: Vec<TransferRecord> = recent_rows
            .into_iter()
            .map(|r| TransferRecord {
                transfer_id: r.transfer_id,
                kind: r.kind.to_wire() as i32,
                peer: r.peer,
                module: r.module,
                path: r.path,
                start_unix_ms: r.start_unix_ms,
                duration_ms: r.duration_ms,
                // Final byte count from the per-row atomic
                // (c-1a-byte-counter-api). Zero until c-1b
                // wires the data-plane receive loop.
                bytes: r.bytes,
                // `files` lands in a subsequent C slice.
                files: 0,
                ok: r.ok,
                error_message: r.error_message,
            })
            .collect();

        let counters = Counters {
            push_operations_total: self.metrics.push_operations.load(Ordering::Relaxed),
            pull_operations_total: self.metrics.pull_operations.load(Ordering::Relaxed),
            purge_operations_total: self.metrics.purge_operations.load(Ordering::Relaxed),
            active_transfers: self.metrics.active_transfers.load(Ordering::Relaxed),
            transfer_errors_total: self.metrics.transfer_errors.load(Ordering::Relaxed),
        };

        let uptime_seconds = self.started_at.elapsed().as_secs();

        Ok(Response::new(DaemonState {
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds,
            modules,
            active,
            recent,
            counters: Some(counters),
            delegation_enabled: self.delegation.allow_delegated_pull,
        }))
    }

    async fn cancel_job(
        &self,
        request: Request<CancelJobRequest>,
    ) -> Result<Response<CancelJobResponse>, Status> {
        // Capture the caller's address before consuming the request:
        // audit-9 authorizes the cancel against the peer that started
        // the transfer.
        let caller = request.remote_addr();
        let req = request.into_inner();
        if req.transfer_id.trim().is_empty() {
            return Err(Status::invalid_argument(
                "CancelJobRequest.transfer_id must not be empty",
            ));
        }
        // `ActiveJobs::cancel_authorized` is synchronous and short — the
        // critical section is one `HashMap::get` + an IP comparison +
        // (when authorized and cancellable) one
        // `CancellationToken::cancel()`. No async work to do.
        match self.active_jobs.cancel_authorized(&req.transfer_id, caller) {
            CancelOutcome::Cancelled => Ok(Response::new(CancelJobResponse {
                transfer_id: req.transfer_id,
            })),
            CancelOutcome::Unsupported => Err(Status::failed_precondition(format!(
                "transfer '{}' is not cancellable from another client (CLI is in the byte path; \
                 cancel from the originating client instead)",
                req.transfer_id
            ))),
            CancelOutcome::NotFound => Err(Status::not_found(format!(
                "no active transfer matches transfer_id '{}'",
                req.transfer_id
            ))),
            CancelOutcome::Unauthorized => Err(Status::permission_denied(format!(
                "transfer '{}' may only be cancelled by the peer that started it",
                req.transfer_id
            ))),
        }
    }

    /// rec-2: clear the recent-transfers list. Wipes the in-memory
    /// recent ring (what `GetState.recent[]` reads) and triggers the
    /// persistence writer to rewrite `recents.jsonl` empty. Deliberately
    /// does NOT touch the planner's `perf_local.jsonl` — `clear_recent`
    /// on `ActiveJobs` only ever references the recents ring + its own
    /// store. `ClearRecentRequest` is empty (the operator clears the
    /// whole list); the response carries the count removed.
    async fn clear_recent(
        &self,
        _request: Request<ClearRecentRequest>,
    ) -> Result<Response<ClearRecentResponse>, Status> {
        let cleared = self.active_jobs.clear_recent();
        Ok(Response::new(ClearRecentResponse {
            cleared: cleared as u32,
        }))
    }

    async fn disk_usage(
        &self,
        request: Request<DiskUsageRequest>,
    ) -> Result<Response<Self::DiskUsageStream>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
        let start_rel = if req.start_path.trim().is_empty() {
            PathBuf::from(".")
        } else {
            resolve_relative_path(req.start_path.trim())?
        };

        let max_depth = if req.max_depth == 0 {
            None
        } else {
            Some(req.max_depth as usize)
        };

        let (tx, rx): (
            DiskUsageSender,
            mpsc::Receiver<Result<DiskUsageEntry, Status>>,
        ) = mpsc::channel(32);

        let module_root = module.path.clone();
        tokio::spawn(async move {
            let err_sender = tx.clone();
            let result = tokio::task::spawn_blocking(move || {
                stream_disk_usage(module_root, start_rel, max_depth, &tx)
            })
            .await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(status)) => {
                    let _ = err_sender.send(Err(status)).await;
                }
                Err(join_err) => {
                    let _ = err_sender
                        .send(Err(Status::internal(format!(
                            "disk usage worker failed: {}",
                            join_err
                        ))))
                        .await;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn find(
        &self,
        request: Request<FindRequest>,
    ) -> Result<Response<Self::FindStream>, Status> {
        let req = request.into_inner();
        if !req.include_files && !req.include_directories {
            return Err(Status::invalid_argument(
                "at least one of include_files or include_directories must be true",
            ));
        }
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
        let (tx, rx): (FindSender, mpsc::Receiver<Result<FindEntry, Status>>) = mpsc::channel(32);

        let start_rel = if req.start_path.trim().is_empty() {
            PathBuf::from(".")
        } else {
            resolve_relative_path(&req.start_path)?
        };

        let module_root = module.path.clone();
        let pattern = req.pattern.clone();
        let case_sensitive = req.case_sensitive;
        let include_files = req.include_files;
        let include_dirs = req.include_directories;
        let max_results = if req.max_results == 0 {
            None
        } else {
            Some(req.max_results as usize)
        };

        tokio::spawn(async move {
            let err_sender = tx.clone();
            let result = tokio::task::spawn_blocking(move || {
                stream_find_entries(
                    module_root,
                    start_rel,
                    pattern,
                    case_sensitive,
                    include_files,
                    include_dirs,
                    max_results,
                    &tx,
                )
            })
            .await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(status)) => {
                    let _ = err_sender.send(Err(status)).await;
                }
                Err(join_err) => {
                    let _ = err_sender
                        .send(Err(Status::internal(format!(
                            "find worker failed: {}",
                            join_err
                        ))))
                        .await;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn filesystem_stats(
        &self,
        request: Request<FilesystemStatsRequest>,
    ) -> Result<Response<FilesystemStatsResponse>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
        let stats = filesystem_stats_for_path(&module.path)?;

        Ok(Response::new(stats))
    }
}

/// Format the remote peer of a tonic request as `<ip>:<port>`,
/// or `"unknown"` when the transport didn't surface one (eg.
/// in-process tests that bypass the network).
fn peer_addr_string<T>(request: &Request<T>) -> String {
    request
        .remote_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Resolve a transfer's terminal outcome from its three racing
/// conditions, giving handler completion priority (audit-10).
///
/// Extracted for `delegated_pull` (R30-F2 / m-jobs-1) and generalized
/// in w4-3 to be the single owner of the biased select every transfer
/// RPC races through — `delegated_pull` calls it directly (handler
/// output `bool`), while `push` / `pull_sync` go through
/// [`resolve_streaming_outcome`] (handler output `Result<(), Status>`).
///
/// The select is `biased` with the **handler branch first**: when the
/// handler future is `Ready`, its result wins even if the cancel token
/// has also just fired or the client just hung up. A handler that is
/// still `Pending` yields to a client hangup (only when `!detach`) or a
/// `CancelJob` cancel, both of which resolve to `None` so the caller
/// records the cancellation.
///
/// Pre-audit-10 the cancel branch was evaluated before the handler, so a
/// transfer that completed at the same instant `CancelJob` fired its
/// token was mis-recorded as "cancelled via CancelJob" despite having
/// actually succeeded. Ordering completion first makes a real result
/// (success *or* failure) authoritative over a simultaneous cancel.
///
/// Returns `Some(output)` when the handler completed, or `None` for a
/// client hangup or cancel.
async fn resolve_transfer_outcome<T, H, C, K>(
    handler: H,
    tx_closed: C,
    cancelled: K,
    detach: bool,
) -> Option<T>
where
    H: std::future::Future<Output = T>,
    C: std::future::Future<Output = ()>,
    K: std::future::Future<Output = ()>,
{
    tokio::select! {
        biased;
        output = handler => Some(output),
        _ = tx_closed, if !detach => None,
        _ = cancelled => None,
    }
}

/// w4-3: resolve a streaming transfer RPC's (`push` / `pull_sync`)
/// terminal outcome, racing the handler against client hangup and the
/// row's `CancelJob` token via [`resolve_transfer_outcome`].
///
/// Pre-w4-3 these dispatchers bare-awaited their handlers, so a client
/// that disconnected during a send-free compute phase (pull_sync's
/// enumerate+checksum collection, push's mirror purge) left the daemon
/// running the whole remaining handler for a dead peer — unbounded,
/// unobservable work that `CancelJob` also refused to touch
/// (async-daemon-handlers-blind-to-disconnect-in-compute-phases).
/// Dropping the handler future propagates through the existing
/// cancellation paths: the push data-plane accept task is
/// `AbortOnDrop`-wrapped and its workers live in a `JoinSet` (w4-1),
/// and pull_sync's payload feeder exits when its channel closes. An
/// in-flight `spawn_blocking` enumeration/checksum batch still runs to
/// its natural end with the result discarded — making that window
/// abortable is the finding's stated follow-up slice.
///
/// The streaming RPCs have no `detach` mode (the client is inherently
/// attached to the byte path), so the hangup arm is always armed —
/// hence the hardcoded `detach: false`.
///
/// Returns the `(ok, error_message)` pair the ActiveJobs ring records:
/// - handler completed → its result via [`outcome_from_status`]; an
///   `Err` is counted (`inc_error`) and forwarded to the
///   still-connected client, exactly as the pre-w4-3 dispatchers did.
/// - client hung up → `(false, "client cancelled")`; nothing is sent —
///   the receiver is gone, that's what ended the race.
/// - cancel token fired → `(false, "cancelled via CancelJob")`, and the
///   still-connected client gets a terminal `Status::cancelled`. Today
///   `ActiveJobKind::supports_cancellation` keeps `CancelJob` dispatch
///   gated off for push/pull_sync, so this arm is armed but
///   production-unreachable until that policy flips — wired per the
///   w4-3 slice spec so a future flip is policy-only.
async fn resolve_streaming_outcome<T, H>(
    handler: H,
    tx: &mpsc::Sender<Result<T, Status>>,
    cancel_token: &CancellationToken,
    metrics: &TransferMetrics,
) -> (bool, Option<String>)
where
    H: std::future::Future<Output = Result<(), Status>>,
{
    let outcome =
        resolve_transfer_outcome(handler, tx.closed(), cancel_token.cancelled(), false).await;
    match outcome {
        Some(result) => {
            let (ok, err_msg) = outcome_from_status(&result);
            if let Err(status) = result {
                metrics.inc_error();
                let _ = tx.send(Err(status)).await;
            }
            (ok, err_msg)
        }
        // Same disambiguation the delegated_pull closure uses: a fired
        // token means the cause was CancelJob; otherwise the client
        // hung up.
        None if cancel_token.is_cancelled() => {
            let _ = tx
                .send(Err(Status::cancelled("transfer cancelled via CancelJob")))
                .await;
            (false, Some("cancelled via CancelJob".to_string()))
        }
        None => (false, Some("client cancelled".to_string())),
    }
}

/// Translate a handler's `Result<_, Status>` into the
/// `(ok, error_message)` pair the ActiveJobs guard expects.
/// Used inside [`resolve_streaming_outcome`] for the `push` /
/// `pull_sync` dispatchers. `delegated_pull` has its own shape
/// (handler returns `bool` inside a select) and inlines the
/// equivalent mapping there.
fn outcome_from_status<T>(result: &Result<T, Status>) -> (bool, Option<String>) {
    match result {
        Ok(_) => (true, None),
        Err(status) => (false, Some(status.message().to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_jobs::ActiveJobKind;
    use blit_core::generated::TransferKind as WireKind;

    fn empty_service() -> BlitService {
        BlitService::with_modules(HashMap::new(), false)
    }

    /// audit-10: a handler that has completed must win the `biased`
    /// select even when the cancel token (and the client-hangup signal)
    /// have ALSO fired — otherwise a transfer that succeeded at the same
    /// instant `CancelJob` fired gets mis-recorded as cancelled.
    /// (Helper renamed `resolve_delegated_pull_outcome` →
    /// `resolve_transfer_outcome` in w4-3; same select, now generic.)
    #[tokio::test]
    async fn resolve_pull_handler_completion_wins_over_simultaneous_cancel() {
        use std::future::ready;
        // Handler ready(success); client hung up; cancel fired — all
        // simultaneously. Handler-first ordering must yield Some(true).
        let outcome = resolve_transfer_outcome(ready(true), ready(()), ready(()), false).await;
        assert_eq!(outcome, Some(true), "ready success must win the race");

        // The same holds for a handler that completed with failure: a
        // real result beats a simultaneous cancel.
        let outcome = resolve_transfer_outcome(ready(false), ready(()), ready(()), false).await;
        assert_eq!(outcome, Some(false), "ready failure must win the race");
    }

    /// audit-10: a still-running (Pending) handler must still yield to a
    /// `CancelJob` cancel — the fix must not make transfers
    /// uncancellable.
    #[tokio::test]
    async fn resolve_pull_pending_handler_yields_to_cancel() {
        use std::future::{pending, ready};
        let outcome = resolve_transfer_outcome(
            pending::<bool>(), // handler still running
            pending::<()>(),   // client still connected
            ready(()),         // CancelJob fired
            false,
        )
        .await;
        assert_eq!(outcome, None, "a running handler must yield to cancel");
    }

    /// audit-10 / m-jobs-3: with `detach = true` the client-hangup branch
    /// is disabled, so a closed tx must NOT terminate the pull.
    #[tokio::test]
    async fn resolve_pull_detach_disables_client_hangup() {
        use std::future::{pending, ready};
        let fut = resolve_transfer_outcome(
            pending::<bool>(), // handler still running
            ready(()),         // client hung up...
            pending::<()>(),   // ...but no cancel
            true,              // detached
        );
        // tx_closed is ready but gated off by detach; nothing else is
        // ready, so the outcome must not resolve.
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(50), fut)
                .await
                .is_err(),
            "detach=true must keep a client hangup from ending the pull"
        );
    }

    /// w4-3: a client hangup (dropped response `Receiver`) must resolve
    /// a still-running streaming handler as `(false, "client
    /// cancelled")` instead of letting it run to completion for a dead
    /// peer. Pre-fix the push/pull_sync dispatchers bare-awaited the
    /// handler, so with a `pending()` handler this future would never
    /// resolve — the test would hang, not merely assert-fail.
    /// Deterministic: the receiver is dropped before the race starts,
    /// so `tx.closed()` is already level-set.
    #[tokio::test]
    async fn streaming_hangup_resolves_pending_handler_as_client_cancelled() {
        use std::future::pending;
        let (tx, rx) = mpsc::channel::<Result<(), Status>>(1);
        drop(rx); // client hung up: tonic drops the ReceiverStream
        let token = CancellationToken::new();
        let metrics = TransferMetrics::disabled();
        let (ok, err) =
            resolve_streaming_outcome(pending::<Result<(), Status>>(), &tx, &token, &metrics).await;
        assert!(!ok, "a hangup-terminated transfer must record ok=false");
        assert_eq!(err.as_deref(), Some("client cancelled"));
    }

    /// w4-3: a fired row token must resolve a still-running streaming
    /// handler as `(false, "cancelled via CancelJob")` and deliver a
    /// terminal `Status::cancelled` to the still-connected client.
    /// (Production dispatch keeps `CancelJob` gated off for
    /// push/pull_sync via `supports_cancellation`; this pins the
    /// handler-side capability so a future policy flip is policy-only.)
    #[tokio::test]
    async fn streaming_canceljob_resolves_pending_handler_and_notifies_client() {
        use std::future::pending;
        let (tx, mut rx) = mpsc::channel::<Result<(), Status>>(1);
        let token = CancellationToken::new();
        token.cancel();
        let metrics = TransferMetrics::disabled();
        let (ok, err) =
            resolve_streaming_outcome(pending::<Result<(), Status>>(), &tx, &token, &metrics).await;
        assert!(!ok);
        assert_eq!(err.as_deref(), Some("cancelled via CancelJob"));
        let sent = rx.recv().await.expect("client must get a terminal frame");
        let status = sent.expect_err("terminal frame must be an error status");
        assert_eq!(status.code(), tonic::Code::Cancelled);
    }

    /// w4-3 extends audit-10's guarantee to the streaming dispatchers:
    /// a handler that has completed must win the race even when the
    /// client has hung up AND the token has fired at the same instant —
    /// a real success must not be mis-recorded as a cancellation.
    #[tokio::test]
    async fn streaming_completed_handler_wins_simultaneous_races() {
        use std::future::ready;
        let (tx, rx) = mpsc::channel::<Result<(), Status>>(1);
        drop(rx);
        let token = CancellationToken::new();
        token.cancel();
        let metrics = TransferMetrics::disabled();
        let (ok, err) = resolve_streaming_outcome(ready(Ok(())), &tx, &token, &metrics).await;
        assert!(ok, "a completed handler must beat simultaneous cancels");
        assert_eq!(err, None);
    }

    /// w4-3: the pre-existing dispatcher error path must survive the
    /// rewire — a handler `Err` is recorded with the status message and
    /// the status itself is forwarded to the still-connected client.
    #[tokio::test]
    async fn streaming_handler_error_recorded_and_forwarded_to_client() {
        use std::future::ready;
        let (tx, mut rx) = mpsc::channel::<Result<(), Status>>(1);
        let token = CancellationToken::new();
        let metrics = TransferMetrics::disabled();
        let (ok, err) =
            resolve_streaming_outcome(ready(Err(Status::internal("boom"))), &tx, &token, &metrics)
                .await;
        assert!(!ok);
        assert_eq!(err.as_deref(), Some("boom"));
        let status = rx
            .recv()
            .await
            .expect("client must get the terminal frame")
            .expect_err("terminal frame must be an error status");
        assert_eq!(status.code(), tonic::Code::Internal);
        assert_eq!(status.message(), "boom");
    }

    /// w4-3: a clean success sends nothing extra on the response
    /// channel (the handler already sent its own summary frames) and
    /// records `(true, None)`.
    #[tokio::test]
    async fn streaming_handler_success_records_ok_and_sends_nothing() {
        use std::future::ready;
        let (tx, mut rx) = mpsc::channel::<Result<(), Status>>(1);
        let token = CancellationToken::new();
        let metrics = TransferMetrics::disabled();
        let (ok, err) = resolve_streaming_outcome(ready(Ok(())), &tx, &token, &metrics).await;
        assert!(ok);
        assert!(err.is_none());
        drop(tx);
        assert!(
            rx.recv().await.is_none(),
            "success must not push extra frames at the client"
        );
    }

    #[tokio::test]
    async fn get_state_empty_daemon_returns_zero_active_and_recent() {
        let svc = empty_service();
        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 0 }))
            .await
            .expect("get_state ok");
        let state = resp.into_inner();
        assert_eq!(state.version, env!("CARGO_PKG_VERSION"));
        assert!(state.active.is_empty());
        assert!(state.recent.is_empty());
        assert!(state.modules.is_empty());
        // Counters present but zero because `with_modules` builds
        // a metrics-disabled service.
        let counters = state.counters.expect("counters present");
        assert_eq!(counters.push_operations_total, 0);
        assert_eq!(counters.pull_operations_total, 0);
        assert_eq!(counters.transfer_errors_total, 0);
    }

    #[tokio::test]
    async fn get_state_surfaces_live_active_row_and_recent_row() {
        let svc = empty_service();
        // Live row.
        let guard = svc.active_jobs.register(
            ActiveJobKind::Pull,
            "10.0.0.5:443".to_string(),
            "mod-a".to_string(),
            "sub/dir".to_string(),
        );

        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 0 }))
            .await
            .expect("get_state ok");
        let state = resp.into_inner();
        assert_eq!(state.active.len(), 1);
        let row = &state.active[0];
        assert_eq!(row.kind, WireKind::Pull as i32);
        assert_eq!(row.peer, "10.0.0.5:443");
        assert_eq!(row.module, "mod-a");
        assert_eq!(row.path, "sub/dir");
        // Byte-level fields are zero with no reports against
        // the per-row counter — c-1a wired the atomic but no
        // call site reports against it yet (c-1b lands the
        // data-plane wiring); `bytes_total` lands in a later C
        // slice from the manifest stage.
        assert_eq!(row.bytes_completed, 0);
        assert_eq!(row.bytes_total, 0);

        // Confirm the wire field tracks the atomic by
        // reporting through the sink and re-snapshotting.
        guard.bytes_counter().report(4096);
        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 0 }))
            .await
            .expect("get_state ok");
        let state2 = resp.into_inner();
        assert_eq!(state2.active.len(), 1);
        assert_eq!(state2.active[0].bytes_completed, 4096);

        // Drop the active row + record outcome → it should now
        // appear in `recent[]`.
        guard.record_outcome(true, None);
        drop(guard);

        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 0 }))
            .await
            .expect("get_state ok");
        let state = resp.into_inner();
        assert!(state.active.is_empty());
        assert_eq!(state.recent.len(), 1);
        let rec = &state.recent[0];
        assert_eq!(rec.kind, WireKind::Pull as i32);
        assert!(rec.ok);
        assert_eq!(rec.error_message, "");
        // `bytes` is the final value of the per-row atomic
        // captured at Drop. The earlier `bytes_counter().report(4096)`
        // is what lands here; `files` is a later C slice.
        assert_eq!(rec.bytes, 4096);
        assert_eq!(rec.files, 0);
    }

    #[tokio::test]
    async fn get_state_recent_limit_truncates_to_most_recent_n() {
        // Push 5 records into the ring, request recent_limit=3,
        // expect the 3 most recent in oldest-first order.
        let svc = empty_service();
        for i in 0..5u32 {
            let guard = svc.active_jobs.register(
                ActiveJobKind::Pull,
                format!("peer{i}"),
                "mod".to_string(),
                "/".to_string(),
            );
            guard.record_outcome(true, None);
        }

        // recent_limit=3 → most-recent 3 only.
        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 3 }))
            .await
            .expect("get_state ok");
        let state = resp.into_inner();
        assert_eq!(state.recent.len(), 3);
        // Oldest-first within the truncated window: peer2, peer3, peer4.
        assert_eq!(state.recent[0].peer, "peer2");
        assert_eq!(state.recent[1].peer, "peer3");
        assert_eq!(state.recent[2].peer, "peer4");

        // recent_limit=0 → full ring (the daemon's default).
        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 0 }))
            .await
            .expect("get_state ok");
        let state = resp.into_inner();
        assert_eq!(state.recent.len(), 5);
        assert_eq!(state.recent[0].peer, "peer0");
        assert_eq!(state.recent[4].peer, "peer4");

        // recent_limit larger than what the daemon has → returns
        // everything (no error).
        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 999 }))
            .await
            .expect("get_state ok");
        let state = resp.into_inner();
        assert_eq!(state.recent.len(), 5);
    }

    #[tokio::test]
    async fn cancel_job_ok_for_delegated_pull() {
        let svc = empty_service();
        let guard = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "mod".to_string(),
            "/".to_string(),
        );
        let id = guard.transfer_id().to_string();
        let token = guard.cancellation_token().clone();

        let resp = svc
            .cancel_job(Request::new(CancelJobRequest {
                transfer_id: id.clone(),
            }))
            .await
            .expect("cancel_job ok");
        assert_eq!(resp.into_inner().transfer_id, id);
        assert!(token.is_cancelled(), "delegated_pull token must be fired");

        // The active row stays in the table until Drop runs;
        // letting the guard fall out of scope here.
        drop(guard);
    }

    #[tokio::test]
    async fn cancel_job_failed_precondition_for_non_delegated_kind() {
        let svc = empty_service();
        for kind in [
            ActiveJobKind::Push,
            ActiveJobKind::Pull,
            ActiveJobKind::PullSync,
        ] {
            let guard =
                svc.active_jobs
                    .register(kind, "p".to_string(), "mod".to_string(), "/".to_string());
            let id = guard.transfer_id().to_string();
            let token = guard.cancellation_token().clone();

            let err = svc
                .cancel_job(Request::new(CancelJobRequest {
                    transfer_id: id.clone(),
                }))
                .await
                .expect_err("non-delegated kind must reject CancelJob");
            assert_eq!(err.code(), tonic::Code::FailedPrecondition);
            assert!(
                !token.is_cancelled(),
                "{}: token must NOT be fired when CancelJob is unsupported",
                kind.as_str()
            );
            drop(guard);
        }
    }

    #[tokio::test]
    async fn cancel_job_not_found_for_unknown_transfer_id() {
        let svc = empty_service();
        let err = svc
            .cancel_job(Request::new(CancelJobRequest {
                transfer_id: "t-nope".to_string(),
            }))
            .await
            .expect_err("unknown id must NotFound");
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn cancel_job_invalid_argument_for_empty_id() {
        let svc = empty_service();
        let err = svc
            .cancel_job(Request::new(CancelJobRequest {
                transfer_id: String::new(),
            }))
            .await
            .expect_err("empty id must InvalidArgument");
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    /// rec-2: `ClearRecent` returns the number of entries removed and
    /// empties `GetState.recent[]`. (Persistence isn't armed in the
    /// test service, so this exercises the ring-clearing + response
    /// path; the on-disk + perf_local-untouched behavior is covered by
    /// `active_jobs::clear_recent_empties_store_but_not_perf_local`.)
    #[tokio::test]
    async fn clear_recent_empties_recent_and_reports_count() {
        let svc = empty_service();
        // Two completed transfers → recent[] has two rows.
        for _ in 0..2 {
            let guard = svc.active_jobs.register(
                ActiveJobKind::Pull,
                "10.0.0.5:443".to_string(),
                "mod".to_string(),
                "p".to_string(),
            );
            guard.record_outcome(true, None);
            drop(guard);
        }
        let before = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 0 }))
            .await
            .expect("get_state ok")
            .into_inner();
        assert_eq!(before.recent.len(), 2);

        let resp = svc
            .clear_recent(Request::new(ClearRecentRequest {}))
            .await
            .expect("clear_recent ok")
            .into_inner();
        assert_eq!(resp.cleared, 2, "response reports entries removed");

        let after = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 0 }))
            .await
            .expect("get_state ok")
            .into_inner();
        assert!(after.recent.is_empty(), "recent[] emptied after clear");

        // Idempotent: a second clear removes nothing.
        let resp2 = svc
            .clear_recent(Request::new(ClearRecentRequest {}))
            .await
            .expect("clear_recent ok")
            .into_inner();
        assert_eq!(resp2.cleared, 0);
    }

    #[tokio::test]
    async fn subscribe_delivers_transfer_started_event_to_subscriber() {
        use tokio_stream::StreamExt;

        let svc = empty_service();
        // Subscribe BEFORE firing the event. This is the
        // ordering the production code guarantees too — the
        // RPC handler subscribes synchronously, then the
        // stream is returned to the caller.
        let resp = svc
            .subscribe(Request::new(SubscribeRequest {
                replay_recent: false,
                event_mask: 0,
                transfer_id_filter: String::new(),
            }))
            .await
            .expect("subscribe returns Ok");
        let mut stream = resp.into_inner();

        // Fire a TransferStarted by registering a job on the
        // service's table — same path the real dispatch site
        // takes.
        let guard = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "10.0.0.5:443".to_string(),
            "mod-a".to_string(),
            "sub/dir".to_string(),
        );
        svc.emit_transfer_started(
            &guard,
            ActiveJobKind::DelegatedPull,
            "10.0.0.5:443",
            "mod-a",
            "sub/dir",
        );
        let id = guard.transfer_id().to_string();

        // First (and only) frame should be a TransferStarted.
        let frame = stream
            .next()
            .await
            .expect("stream yields a frame")
            .expect("frame is Ok");
        let payload = frame.payload.expect("frame has payload");
        match payload {
            daemon_event::Payload::TransferStarted(ev) => {
                assert_eq!(ev.transfer_id, id);
                assert_eq!(ev.kind, WireKind::DelegatedPull as i32);
                assert_eq!(ev.peer, "10.0.0.5:443");
                assert_eq!(ev.module, "mod-a");
                assert_eq!(ev.path, "sub/dir");
                assert!(ev.start_unix_ms > 0);
            }
            other => panic!("expected TransferStarted, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn subscribe_delivers_to_multiple_subscribers() {
        use tokio_stream::StreamExt;

        let svc = empty_service();
        let mut stream_a = svc
            .subscribe(Request::new(SubscribeRequest {
                replay_recent: false,
                event_mask: 0,
                transfer_id_filter: String::new(),
            }))
            .await
            .expect("subscribe a")
            .into_inner();
        let mut stream_b = svc
            .subscribe(Request::new(SubscribeRequest {
                replay_recent: false,
                event_mask: 0,
                transfer_id_filter: String::new(),
            }))
            .await
            .expect("subscribe b")
            .into_inner();

        let guard = svc.active_jobs.register(
            ActiveJobKind::Pull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        svc.emit_transfer_started(&guard, ActiveJobKind::Pull, "p", "m", "/");

        let event_a = stream_a
            .next()
            .await
            .expect("a yields")
            .expect("a frame ok");
        let event_b = stream_b
            .next()
            .await
            .expect("b yields")
            .expect("b frame ok");
        let id_a = match event_a.payload.unwrap() {
            daemon_event::Payload::TransferStarted(e) => e.transfer_id,
            other => panic!("expected TransferStarted, got {other:?}"),
        };
        let id_b = match event_b.payload.unwrap() {
            daemon_event::Payload::TransferStarted(e) => e.transfer_id,
            other => panic!("expected TransferStarted, got {other:?}"),
        };
        assert_eq!(id_a, id_b, "both subscribers see the same transfer_id");
    }

    #[tokio::test]
    async fn build_transfer_finished_event_ok_emits_transfer_complete() {
        // Drives the c-3 builder directly. Asserts:
        // - TransferComplete variant is selected when ok=true.
        // - bytes field reflects the per-row counter.
        // - duration_ms is non-zero (start_unix_ms < unix_ms_now()).
        // - files/tcp_fallback_used follow the documented zero/
        //   false defaults until follow-up slices wire them.
        let svc = empty_service();
        let guard = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        // Land some bytes against the per-row atomic so the
        // emitted event carries a non-zero `bytes` field.
        guard.bytes_counter().report(2048);

        let ev = build_transfer_finished_event(&guard, true, None);
        match ev.payload.unwrap() {
            daemon_event::Payload::TransferComplete(c) => {
                assert_eq!(c.transfer_id, guard.transfer_id());
                assert_eq!(c.bytes, 2048);
                assert_eq!(c.files, 0);
                // duration_ms is `unix_ms_now() - start_unix_ms`
                // — small (test runs fast) but not negative.
                let _ = c.duration_ms;
                assert!(!c.tcp_fallback_used);
            }
            other => panic!("expected TransferComplete, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn terminal_event_observable_only_after_active_row_drained() {
        // c-3 round-2 regression: build → drain → broadcast.
        // A subscriber that observes the terminal event MUST
        // be able to immediately query GetState and see the
        // row already moved out of active[].
        use tokio_stream::StreamExt;

        let svc = empty_service();
        let mut stream = svc
            .subscribe(Request::new(SubscribeRequest {
                replay_recent: false,
                event_mask: 0,
                transfer_id_filter: String::new(),
            }))
            .await
            .expect("subscribe ok")
            .into_inner();

        // Replay the spawn-closure ordering: register → record
        // outcome → build event → drop guard → broadcast.
        let guard = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        let id = guard.transfer_id().to_string();
        guard.record_outcome(true, None);
        let event = build_transfer_finished_event(&guard, true, None);
        // Snapshot BEFORE drop — row should be present.
        assert_eq!(svc.active_jobs.snapshot().len(), 1);
        drop(guard);
        // Snapshot AFTER drop — row drained.
        assert!(
            svc.active_jobs.snapshot().is_empty(),
            "Drop must drain the active row before broadcast"
        );
        // Now broadcast. A subscriber's `next()` cannot
        // resolve any earlier than this `send()` call, so by
        // the time the subscriber sees the event the active
        // row is already gone (recent[] now holds it).
        let _ = svc.events_tx().send(event);

        let frame = stream
            .next()
            .await
            .expect("stream yields")
            .expect("frame ok");
        // At event-receipt time, an immediate GetState query
        // would race and might see the recent ring populated;
        // the load-bearing invariant is that active is
        // already drained.
        assert!(
            svc.active_jobs.snapshot().is_empty(),
            "active[] must be empty when subscriber sees terminal event"
        );
        // recent[] must already carry the row.
        let recent = svc.active_jobs.recent();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].transfer_id, id);
        match frame.payload.expect("payload present") {
            daemon_event::Payload::TransferComplete(c) => {
                assert_eq!(c.transfer_id, id);
            }
            other => panic!("expected TransferComplete, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn build_transfer_finished_event_err_emits_transfer_error() {
        let svc = empty_service();
        let guard = svc.active_jobs.register(
            ActiveJobKind::Push,
            "p".to_string(),
            String::new(),
            String::new(),
        );
        let ev = build_transfer_finished_event(&guard, false, Some("module not found"));
        match ev.payload.unwrap() {
            daemon_event::Payload::TransferError(err) => {
                assert_eq!(err.transfer_id, guard.transfer_id());
                assert_eq!(err.message, "module not found");
            }
            other => panic!("expected TransferError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn tick_progress_emits_transfer_progress_per_active_row() {
        use tokio_stream::StreamExt;

        let svc = empty_service();
        let mut stream = svc
            .subscribe(Request::new(SubscribeRequest {
                replay_recent: false,
                event_mask: 0,
                transfer_id_filter: String::new(),
            }))
            .await
            .expect("subscribe ok")
            .into_inner();

        // Register two active rows and seed some bytes on one so
        // it has non-zero progress.
        let g1 = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "p1".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        g1.bytes_counter().report(4096);
        let g2 = svc.active_jobs.register(
            ActiveJobKind::Pull,
            "p2".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        // ActiveJobs::register emits no events directly — only the
        // dispatch site calls emit_transfer_started, which we
        // skip here. Drain whatever happened to land (e.g.
        // background events) is unnecessary; the broadcast queue
        // was empty when we subscribed and we haven't fired any
        // started events.

        let n = tick_progress_once(&svc.active_jobs, &svc.events_tx);
        assert_eq!(n, 2, "one progress event per active row");

        // Collect both frames the ticker broadcast.
        let mut ids = Vec::new();
        for _ in 0..2 {
            let frame = stream
                .next()
                .await
                .expect("ticker frame yields")
                .expect("frame ok");
            match frame.payload.expect("payload") {
                daemon_event::Payload::TransferProgress(p) => {
                    ids.push((p.transfer_id, p.bytes_completed));
                }
                other => panic!("expected TransferProgress, got {other:?}"),
            }
        }
        ids.sort_by(|a, b| a.0.cmp(&b.0));
        let id1 = g1.transfer_id().to_string();
        let id2 = g2.transfer_id().to_string();
        let mut expected = vec![(id1, 4096u64), (id2, 0u64)];
        expected.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(ids, expected);
    }

    #[tokio::test]
    async fn tick_progress_throughput_reflects_delta_between_ticks() {
        let svc = empty_service();
        let guard = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        // Subscribe so the broadcast doesn't have zero receivers
        // (it would still succeed silently, but a real consumer
        // matches the production code path).
        let _stream = svc
            .subscribe(Request::new(SubscribeRequest {
                replay_recent: false,
                event_mask: 0,
                transfer_id_filter: String::new(),
            }))
            .await
            .expect("subscribe ok")
            .into_inner();

        // First tick — establishes the baseline at 0 bytes.
        let _ = tick_progress_once(&svc.active_jobs, &svc.events_tx);

        // Report bytes, sleep ~50ms so delta_ms is non-zero, then
        // tick again. The second tick should show a non-zero
        // throughput corresponding to the bytes reported.
        guard.bytes_counter().report(50 * 1024);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let samples = svc.active_jobs.snapshot_progress_samples();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].bytes_completed, 50 * 1024);
        // throughput_bps = 50 KiB / ~50ms ≈ 1 MiB/s. Loose bound
        // since sleep timing isn't precise; just confirm > 0 and
        // < a sane ceiling (10x expected).
        assert!(samples[0].throughput_bps > 0);
        assert!(samples[0].throughput_bps < 100 * 1024 * 1024);
    }

    #[tokio::test]
    async fn progress_event_cannot_arrive_after_terminal_for_same_transfer() {
        // c-4 round-2 regression: the progress ticker and the
        // c-3 terminal emit BOTH acquire the active-jobs table
        // lock for their critical sections. The lock serializes
        // them, so a TransferProgress for a given transfer_id
        // cannot be broadcast after the corresponding
        // TransferComplete/Error.
        //
        // We deterministically force the worst-case interleave:
        // the ticker is racing against a thread that's about to
        // run the c-3 build-then-drop-then-broadcast sequence
        // (the spawn-closure pattern). Whichever side acquires
        // the lock first, the invariant must hold.
        use std::sync::atomic::AtomicUsize;
        use tokio_stream::StreamExt;

        let svc = empty_service();
        let mut stream = svc
            .subscribe(Request::new(SubscribeRequest {
                replay_recent: false,
                event_mask: 0,
                transfer_id_filter: String::new(),
            }))
            .await
            .expect("subscribe ok")
            .into_inner();

        let guard = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        let id = guard.transfer_id().to_string();
        guard.bytes_counter().report(1024);
        guard.record_outcome(true, None);

        // Build the terminal event BEFORE racing the threads
        // so we can hand it to the dropper. This mirrors the
        // spawn-closure ordering: build, then drop, then send.
        let finished_event = build_transfer_finished_event(&guard, true, None);
        let events_tx = svc.events_tx.clone();
        let active_jobs = svc.active_jobs.clone();

        // Two blocking tasks racing on the lock. We don't
        // synchronize their start with a barrier — running
        // them naked exercises both interleavings across
        // multiple test runs (rustc's test runner shuffles).
        // What matters is that for EITHER interleaving the
        // invariant holds. We then check the stream order.
        let ticker_events_tx = events_tx.clone();
        let ticker = tokio::task::spawn_blocking(move || {
            tick_progress_once(&active_jobs, &ticker_events_tx);
        });
        let dropper = tokio::task::spawn_blocking(move || {
            // Match c-3 round 2's spawn-closure order: drop the
            // guard (releases the row under the lock), then
            // broadcast the pre-built terminal event.
            drop(guard);
            let _ = events_tx.send(finished_event);
        });

        ticker.await.expect("ticker join");
        dropper.await.expect("dropper join");

        // Walk the subscriber's stream and assert the ordering
        // invariant: any TransferProgress for `id` must come
        // BEFORE any TransferComplete/Error for the same id.
        // We can see 0 or 1 progress events depending on which
        // task won the lock race; we must see exactly 1
        // terminal event.
        let mut seen_terminal = false;
        let progress_count = AtomicUsize::new(0);
        // Drain up to 3 frames (max possible: 1 progress + 1
        // terminal; an extra await guards against test-runner
        // jitter).
        for _ in 0..3 {
            let frame =
                match tokio::time::timeout(std::time::Duration::from_millis(100), stream.next())
                    .await
                {
                    Ok(Some(f)) => f.expect("frame ok"),
                    _ => break,
                };
            match frame.payload.expect("payload") {
                daemon_event::Payload::TransferProgress(p) if p.transfer_id == id => {
                    assert!(
                        !seen_terminal,
                        "progress event after terminal for same transfer_id"
                    );
                    progress_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                daemon_event::Payload::TransferComplete(c) if c.transfer_id == id => {
                    assert!(!seen_terminal, "two terminal events for same id");
                    seen_terminal = true;
                }
                daemon_event::Payload::TransferError(e) if e.transfer_id == id => {
                    assert!(!seen_terminal, "two terminal events for same id");
                    seen_terminal = true;
                }
                _ => {
                    // Other events (none expected) — ignore.
                }
            }
        }
        assert!(seen_terminal, "terminal event must arrive on the stream");
        // progress_count is 0 if dropper won the lock race
        // (row gone before ticker iterates), 1 if ticker won.
        // Both are valid outcomes; the invariant we care about
        // is just "no progress after terminal" which the inner
        // assertion enforces.
    }

    #[tokio::test]
    async fn tick_progress_emits_zero_events_when_no_active_rows() {
        let svc = empty_service();
        let n = tick_progress_once(&svc.active_jobs, &svc.events_tx);
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn event_matches_filter_empty_filter_accepts_everything() {
        let svc = empty_service();
        let g = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        let ev = DaemonEvent {
            payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                transfer_id: g.transfer_id().to_string(),
                kind: WireKind::DelegatedPull as i32,
                peer: String::new(),
                module: String::new(),
                path: String::new(),
                start_unix_ms: g.start_unix_ms(),
            })),
        };
        assert!(event_matches_filter(&ev, ""));
    }

    #[tokio::test]
    async fn event_matches_filter_matches_only_target_transfer() {
        let svc = empty_service();
        let g_a = svc.active_jobs.register(
            ActiveJobKind::Pull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        let g_b = svc.active_jobs.register(
            ActiveJobKind::Pull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        let id_a = g_a.transfer_id().to_string();
        let id_b = g_b.transfer_id().to_string();
        let mk_started = |g: &crate::active_jobs::ActiveJobGuard| DaemonEvent {
            payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                transfer_id: g.transfer_id().to_string(),
                kind: WireKind::Pull as i32,
                peer: String::new(),
                module: String::new(),
                path: String::new(),
                start_unix_ms: g.start_unix_ms(),
            })),
        };
        let ev_a = mk_started(&g_a);
        let ev_b = mk_started(&g_b);

        assert!(event_matches_filter(&ev_a, &id_a));
        assert!(!event_matches_filter(&ev_b, &id_a));
        assert!(event_matches_filter(&ev_b, &id_b));
        assert!(!event_matches_filter(&ev_a, &id_b));

        // Cross-variant: progress events honor the same filter.
        let p_a = DaemonEvent {
            payload: Some(daemon_event::Payload::TransferProgress(TransferProgress {
                transfer_id: id_a.clone(),
                bytes_completed: 0,
                bytes_total: 0,
                files_completed: 0,
                files_total: 0,
                throughput_bps: 0,
            })),
        };
        assert!(event_matches_filter(&p_a, &id_a));
        assert!(!event_matches_filter(&p_a, &id_b));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn subscribe_replay_recent_replays_per_row_ring_to_late_joiner() {
        // c-5b regression: a subscriber that joins AFTER
        // emit_transfer_started has fired (and any early
        // TransferProgress events have fired) MUST see them
        // on connect when SubscribeRequest.replay_recent =
        // true. Without replay the forwarder would only see
        // events that arrive after subscribe registration.
        use tokio_stream::StreamExt;

        let svc = empty_service();
        let guard = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        let id = guard.transfer_id().to_string();
        // Fire the started + a couple of progress events BEFORE
        // anyone subscribes. They land in the row's event ring.
        svc.emit_transfer_started(&guard, ActiveJobKind::DelegatedPull, "p", "m", "/");
        guard.bytes_counter().report(1024);
        tick_progress_once(&svc.active_jobs, &svc.events_tx);
        guard.bytes_counter().report(2048);
        tick_progress_once(&svc.active_jobs, &svc.events_tx);

        // Now subscribe with replay_recent=true. The
        // forwarder should drain the row's ring (Started +
        // 2 progress) to us before any live broadcast.
        let mut stream = svc
            .subscribe(Request::new(SubscribeRequest {
                event_mask: 0,
                replay_recent: true,
                transfer_id_filter: id.clone(),
            }))
            .await
            .expect("subscribe ok")
            .into_inner();

        let mut seen = Vec::new();
        for _ in 0..3 {
            let frame = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
                .await
                .expect("replay frame did not arrive in time")
                .expect("stream did not end prematurely")
                .expect("frame ok");
            match frame.payload.expect("payload") {
                daemon_event::Payload::TransferStarted(_) => seen.push("started"),
                daemon_event::Payload::TransferProgress(_) => seen.push("progress"),
                other => panic!("unexpected variant in replay: {other:?}"),
            }
        }
        // Order is the order the events were emitted: Started,
        // Progress, Progress.
        assert_eq!(seen, vec!["started", "progress", "progress"]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn subscribe_without_replay_recent_skips_ring() {
        // Default `replay_recent=false` behavior: the row's
        // event ring stays put, but the subscriber only sees
        // events from subscribe time onward.
        use tokio_stream::StreamExt;

        let svc = empty_service();
        let guard = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        let id = guard.transfer_id().to_string();
        svc.emit_transfer_started(&guard, ActiveJobKind::DelegatedPull, "p", "m", "/");

        let mut stream = svc
            .subscribe(Request::new(SubscribeRequest {
                event_mask: 0,
                replay_recent: false,
                transfer_id_filter: id.clone(),
            }))
            .await
            .expect("subscribe ok")
            .into_inner();

        // No replay → no Started frame. A short timeout
        // should fire with no frame received.
        let result =
            tokio::time::timeout(std::time::Duration::from_millis(100), stream.next()).await;
        assert!(
            result.is_err(),
            "replay_recent=false should NOT replay the ring; got: {:?}",
            result
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn filtered_subscriber_forwarder_exits_on_client_disconnect() {
        // c-5a round-3 regression: when a filtered subscriber
        // drops its stream during a quiet period (no further
        // matching events ever fire), the forwarder MUST
        // notice and exit — otherwise it leaks a task and a
        // live broadcast Receiver indefinitely.
        //
        // The signal we measure is `events_tx.receiver_count()`:
        // each forwarder owns one broadcast Receiver, so the
        // count drops by exactly 1 when the forwarder exits.

        let svc = empty_service();
        let g_a = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );

        let baseline = svc.events_tx.receiver_count();
        let stream = svc
            .subscribe(Request::new(SubscribeRequest {
                replay_recent: false,
                event_mask: 0,
                transfer_id_filter: g_a.transfer_id().to_string(),
            }))
            .await
            .expect("subscribe ok")
            .into_inner();
        // Subscribe spawned a forwarder which now holds a
        // broadcast::Receiver. Yield so the spawned task has
        // a chance to subscribe before we measure.
        tokio::task::yield_now().await;
        let with_subscriber = svc.events_tx.receiver_count();
        assert_eq!(
            with_subscriber,
            baseline + 1,
            "subscribe should add exactly one broadcast Receiver"
        );

        // Client drops the stream. The forwarder is sitting
        // on `broadcast_rx.recv().await`; with the `tx.closed()`
        // race wired up it should observe the channel close
        // and exit, dropping its Receiver and decrementing the
        // count back to baseline.
        drop(stream);

        // Poll the receiver count for up to 1s. The forwarder
        // typically exits on the very next runtime tick; the
        // loop tolerates jitter under CI load.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while std::time::Instant::now() < deadline {
            if svc.events_tx.receiver_count() == baseline {
                return;
            }
            tokio::task::yield_now().await;
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!(
            "forwarder did not exit within 1s after client disconnect; \
             receiver_count {} != baseline {}",
            svc.events_tx.receiver_count(),
            baseline
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn filtered_subscriber_survives_overflow_of_other_transfer_events() {
        // c-5a round-2 regression: a subscriber with a
        // transfer_id_filter MUST not be aborted with Lagged
        // when unrelated transfers' events overflow the global
        // broadcast ring at production-realistic interleaved
        // rates. The forwarder eagerly drains unrelated events
        // (filter rejects → no mpsc traffic), so the client's
        // stream only sees the matching one.
        //
        // Requires multi-thread runtime: the forwarder spawned
        // inside `subscribe` needs to make progress in parallel
        // with the emit task. Under the default `current_thread`
        // runtime, a tight sync emit loop would starve the
        // forwarder regardless of design.
        use tokio_stream::StreamExt;

        let svc = empty_service();
        let g_a = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "a".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        let g_b = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "b".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        let id_a = g_a.transfer_id().to_string();
        let id_b = g_b.transfer_id().to_string();
        let start_b = g_b.start_unix_ms();

        let mut stream = svc
            .subscribe(Request::new(SubscribeRequest {
                replay_recent: false,
                event_mask: 0,
                transfer_id_filter: id_a.clone(),
            }))
            .await
            .expect("subscribe ok")
            .into_inner();

        // Spawn the id_b emitter on its own task so the
        // runtime schedules it alongside the forwarder. Emit
        // > SUBSCRIBE_BROADCAST_CAPACITY events; the forwarder
        // drains each (filter rejects → no mpsc traffic) and
        // its broadcast cursor stays caught up. A tight sync
        // emit loop in the test task itself would starve the
        // forwarder regardless of design (the runtime can't
        // preempt sync code); production naturally interleaves
        // because emit sites live on different async tasks
        // (RPC handlers, the progress ticker).
        let overflow_count = SUBSCRIBE_BROADCAST_CAPACITY + 50;
        let events_tx = svc.events_tx.clone();
        let emit_task = tokio::spawn(async move {
            for _ in 0..overflow_count {
                let event = DaemonEvent {
                    payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
                        transfer_id: id_b.clone(),
                        kind: ActiveJobKind::DelegatedPull.to_wire() as i32,
                        peer: "b".to_string(),
                        module: "m".to_string(),
                        path: "/".to_string(),
                        start_unix_ms: start_b,
                    })),
                };
                let _ = events_tx.send(event);
                tokio::task::yield_now().await;
            }
        });
        emit_task.await.expect("emit task ok");

        // Fire one event for id_a. It must reach the
        // subscriber's stream, not be lost to Lagged.
        svc.emit_transfer_started(&g_a, ActiveJobKind::DelegatedPull, "a", "m", "/");

        let frame = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
            .await
            .expect("filtered subscriber did not receive event in time")
            .expect("stream did not end prematurely")
            .expect(
                "expected the id_a frame, not Status::aborted — \
             filtered events for id_b should not cause Lagged",
            );
        match frame.payload.unwrap() {
            daemon_event::Payload::TransferStarted(e) => assert_eq!(e.transfer_id, id_a),
            other => panic!("expected id_a TransferStarted, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn subscribe_with_transfer_id_filter_drops_other_transfer_events() {
        // End-to-end via the real subscribe handler: subscribe
        // with filter=id_a, fire Started events for two
        // transfers, assert only id_a's reaches the subscriber.
        use tokio_stream::StreamExt;

        let svc = empty_service();
        let g_a = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        let g_b = svc.active_jobs.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        let id_a = g_a.transfer_id().to_string();
        let _id_b = g_b.transfer_id().to_string();

        let mut stream = svc
            .subscribe(Request::new(SubscribeRequest {
                replay_recent: false,
                event_mask: 0,
                transfer_id_filter: id_a.clone(),
            }))
            .await
            .expect("subscribe ok")
            .into_inner();

        // Fire two events; only id_a's should reach the
        // subscriber's stream.
        svc.emit_transfer_started(&g_a, ActiveJobKind::DelegatedPull, "p", "m", "/");
        svc.emit_transfer_started(&g_b, ActiveJobKind::DelegatedPull, "p", "m", "/");

        let frame = stream.next().await.expect("first frame").expect("frame ok");
        match frame.payload.unwrap() {
            daemon_event::Payload::TransferStarted(e) => assert_eq!(e.transfer_id, id_a),
            other => panic!("expected id_a TransferStarted, got {other:?}"),
        }

        // No second frame within a short window — id_b
        // filtered out.
        let next = tokio::time::timeout(std::time::Duration::from_millis(50), stream.next()).await;
        assert!(
            next.is_err(),
            "expected no further frame (id_b should be filtered), got: {:?}",
            next
        );
    }

    #[tokio::test]
    async fn subscribe_drops_event_silently_when_no_subscribers() {
        // No subscribers attached → broadcast::send returns
        // SendError, which `emit_transfer_started` ignores.
        // The test just asserts the call doesn't panic.
        let svc = empty_service();
        let guard = svc.active_jobs.register(
            ActiveJobKind::Push,
            "p".to_string(),
            String::new(),
            String::new(),
        );
        svc.emit_transfer_started(&guard, ActiveJobKind::Push, "p", "", "");
        // Drop the guard cleanly to keep the table snapshot
        // assertion below precise.
        drop(guard);
        assert!(svc.active_jobs.snapshot().is_empty());
    }

    #[tokio::test]
    async fn get_state_failure_record_carries_error_message() {
        let svc = empty_service();
        {
            let guard = svc.active_jobs.register(
                ActiveJobKind::Push,
                "p".to_string(),
                String::new(),
                String::new(),
            );
            guard.record_outcome(false, Some("module not found".to_string()));
        }
        let resp = svc
            .get_state(Request::new(GetStateRequest { recent_limit: 0 }))
            .await
            .expect("get_state ok");
        let state = resp.into_inner();
        assert_eq!(state.recent.len(), 1);
        assert!(!state.recent[0].ok);
        assert_eq!(state.recent[0].error_message, "module not found");
    }
}
