//! In-memory registry of in-flight transfers on the daemon.
//!
//! Milestone B of `docs/plan/TUI_DESIGN.md` §6.3: the always-on
//! `ActiveJobs` table that `GetState.active[]` will read from
//! once the RPC lands in a later sub-slice. Populated at the
//! dispatch boundary in `service/core.rs`; rows are drained on
//! RPC completion via the RAII guard returned by [`register`].
//!
//! Scope so far:
//!
//! - `b-1-active-jobs`: table struct + `ActiveJob` row +
//!   `ActiveJobKind`, `register(...) -> ActiveJobGuard` with
//!   a synchronous Drop that removes the row, `snapshot()`
//!   for the future `GetState` reader, and wiring at the
//!   `pull` and `delegated_pull` dispatch sites.
//! - `b-2-set-endpoint`: `ActiveJobGuard::set_endpoint(module,
//!   path)` for the streaming-RPC case. Push and pull_sync now
//!   register at dispatch with empty module/path strings and
//!   their handlers fill the row once the first stream frame
//!   parses. All four `ActiveJobKind` variants are now
//!   actually constructed on the wire path.
//! - `b-3-recent-ring`: bounded recent-runs ring of
//!   [`TransferRecord`] entries on `ActiveJobs`, pushed by
//!   Drop alongside the table removal. Outcome capture via
//!   [`ActiveJobGuard::record_outcome`]; spawn closures in
//!   `service/core.rs` call it before dropping the guard.
//!   Default ring depth [`DEFAULT_RECENT_LIMIT`] (50);
//!   configurable via `ActiveJobs::with_recent_limit`.
//! - `b-4-getstate`: `GetState` RPC reads from `snapshot()` +
//!   `recent()`. No active_jobs changes — wire-layer only.
//! - `b-5-jobs-list`: CLI `blit jobs list <remote>` consumes
//!   `GetState`. Also wire-layer only.
//! - `m-jobs-1-cancel-token`: per-row [`CancellationToken`]
//!   plumbing. `register` mints a token, `cancel(id)` fires
//!   it, `ActiveJobGuard::cancellation_token()` exposes it
//!   to handlers. `delegated_pull` spawn closure now races
//!   the token against the handler future, so a
//!   forthcoming `CancelJob` RPC can drop in-flight
//!   delegated transfers.
//! - `c-1a-byte-counter-api`: per-row [`Arc<AtomicU64>`] for
//!   `bytes_completed`. [`ActiveJobGuard::bytes_counter`]
//!   hands out a clonable
//!   [`blit_core::remote::transfer::ByteProgressSink`] wrapping
//!   that Arc. Reports against the sink land in
//!   `GetState.active[].bytes_completed` and (on Drop)
//!   `GetState.recent[].bytes`. The sink type lives in
//!   `blit-core` so the data-plane write loop
//!   (`receive_stream_double_buffered`) can take it as a
//!   parameter in c-1b without `blit-core` depending on
//!   `blit-daemon` (the dependency goes the other way).
//!   This slice adds only the registry-side machinery; the
//!   sink is never called yet — current behavior is
//!   unchanged except the proto byte fields now carry the
//!   (still zero) atomic value instead of a hardcoded zero.
//!
//! Out of scope (next sub-slices):
//!
//! - `CancelJob` RPC + CLI verb (`m-jobs-2-cancel-rpc`).
//! - `detach` field on `DelegatedPullRequest` + spawn-closure
//!   lifecycle change (`m-jobs-3-detach`).
//! - Per-job event ring inside each row (`m-jobs-4-events`).
//! - `SubscribeRequest.transfer_id_filter` proto field
//!   (`m-jobs-5-subscribe-filter`).
//! - `blit jobs watch` polling CLI (`m-jobs-6-watch`).
//! - Data-plane wiring of `ByteProgressSink`
//!   (`c-1b-byte-counter-wiring`): `receive_stream_double_buffered`
//!   grows an optional `&ByteProgressSink` and
//!   `handle_delegated_pull` passes the counter through.
//! - Throughput EWMA, files-completed counter, bytes_total
//!   wiring from the manifest stage (subsequent C sub-slices).
//!
//! ## Locking
//!
//! The table is guarded by [`std::sync::Mutex`] rather than
//! `tokio::sync::Mutex`. The protected work is purely
//! in-memory (HashMap insert / remove / cloned-values
//! collect) so the critical section is short — bounded by the
//! number of active transfers, which is small relative to the
//! cost of any single transfer. A standard mutex gives Drop a
//! deterministic synchronous removal path: after
//! `ActiveJobGuard` is dropped, the row is gone. An async
//! mutex would force Drop to either spawn an unawaited
//! cleanup task or use `try_lock` with a fallback — both leak
//! the RAII contract `GetState.active[]` will rely on.
//! Round-1 of this slice used `tokio::sync::Mutex` + the
//! try_lock-then-spawn pattern; the reviewer caught it.

use blit_core::generated::DaemonEvent;
use blit_core::remote::transfer::ByteProgressSink;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

/// Default depth of the recent-runs ring buffer. Mirrors the
/// `GetStateRequest.recent_limit = 0 → 50` default the design
/// doc (§6.3) calls for. Carried as a constant here so b-3
/// can land before the proto types.
pub const DEFAULT_RECENT_LIMIT: usize = 50;

/// Capacity of each `ActiveJob`'s per-row event ring. Holds the
/// most recent N events emitted for that transfer so a c-5b
/// `Subscribe(replay_recent=true, transfer_id_filter=X)` can
/// replay the in-flight history to a late-joining client.
///
/// Sized to comfortably hold one TransferStarted + many
/// TransferProgress events (10 Hz × a few seconds of history
/// is plenty for a TUI joining mid-transfer). Terminal events
/// never land here — they're emitted AFTER row drain (c-3
/// round 2), at which point the ring is gone with the row.
pub const JOB_EVENT_RING_CAP: usize = 64;

/// What kind of transfer a row represents. Mirrors the
/// dispatch sites in `service/core.rs`. When milestone C
/// introduces the `TransferStarted.Kind` wire enum, the
/// conversion will live in the GetState handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveJobKind {
    Push,
    Pull,
    PullSync,
    DelegatedPull,
}

impl ActiveJobKind {
    /// Stable, lowercased name used by logs / future wire
    /// serialization (e.g. `GetState.active[].kind` once the
    /// proto enum is mapped in b-3/b-4).
    #[allow(dead_code)]
    pub fn as_str(self) -> &'static str {
        match self {
            ActiveJobKind::Push => "push",
            ActiveJobKind::Pull => "pull",
            ActiveJobKind::PullSync => "pull_sync",
            ActiveJobKind::DelegatedPull => "delegated_pull",
        }
    }

    /// Wire-shape conversion to the `TransferKind` proto enum.
    /// Used by the `GetState` handler.
    pub fn to_wire(self) -> blit_core::generated::TransferKind {
        use blit_core::generated::TransferKind as Wire;
        match self {
            ActiveJobKind::Push => Wire::Push,
            ActiveJobKind::Pull => Wire::Pull,
            ActiveJobKind::PullSync => Wire::PullSync,
            ActiveJobKind::DelegatedPull => Wire::DelegatedPull,
        }
    }

    /// Whether the daemon side honors a cancellation token for
    /// this kind of transfer. Only `DelegatedPull` today —
    /// push / pull / pull_sync have the CLI in the byte path
    /// (a client-side cancel already drops the handler future
    /// via `tx.closed()`), so `CancelJob` from another client
    /// has no meaningful semantic. M-Jobs may flip this for
    /// future locally-spawned daemon transfers.
    pub fn supports_cancellation(self) -> bool {
        matches!(self, ActiveJobKind::DelegatedPull)
    }
}

/// Outcome of an [`ActiveJobs::cancel`] call. The upcoming
/// `CancelJob` RPC handler will map each variant onto a
/// distinct gRPC status:
///
/// - `Cancelled` → `Code::Ok` with a body acknowledging the
///   cancel was fired.
/// - `Unsupported` → `Code::FailedPrecondition` — the
///   transfer kind doesn't support cancellation today
///   (push / pull / pull_sync; the CLI is in the byte path).
/// - `NotFound` → `Code::NotFound` — no active row matches
///   the requested transfer_id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum CancelOutcome {
    Cancelled,
    Unsupported,
    NotFound,
}

/// One row of the `ActiveJobs` table. Fields mirror the
/// `ActiveTransfer` proto message planned for `GetState` in
/// §6.3 of the TUI design doc; missing wire fields
/// (`bytes_completed`, `bytes_total`) land in milestone C.
///
/// Fields are `#[allow(dead_code)]` for this slice because
/// the read consumer (`GetState` handler) lands in b-4. The
/// `snapshot()` test in this module exercises them so the
/// shape is locked in now.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ActiveJob {
    pub transfer_id: String,
    pub kind: ActiveJobKind,
    /// Remote address of the connecting peer, as observed by
    /// tonic (`"<ip>:<port>"`) — or `"unknown"` when the
    /// transport didn't surface one (in-process tests).
    pub peer: String,
    /// Module name on the daemon. Empty string for streaming
    /// RPCs whose module arrives in the first frame and hasn't
    /// been populated yet — see the b-2 follow-up.
    pub module: String,
    /// Module-relative path the transfer targets. Same
    /// "empty until first frame" caveat as `module` for
    /// streaming RPCs.
    pub path: String,
    /// Unix milliseconds at which the row was registered.
    pub start_unix_ms: u64,
    /// Cumulative bytes the data plane has reported writing for
    /// this transfer. Read from the per-row atomic at
    /// `snapshot()` time. Zero until c-1b wires the receive
    /// loop to call [`ByteProgressSink::report`]; the field is
    /// already plumbed onto the wire (`ActiveTransfer.bytes_completed`)
    /// so future Subscribe events and GetState consumers don't
    /// see a shape change.
    pub bytes_completed: u64,
}

/// One entry in the recent-runs ring buffer. Fields mirror
/// the `TransferRecord` proto message planned for
/// `GetState.recent[]` in §6.3 of the TUI design doc. Missing
/// wire fields (`bytes`, `files`) land in milestone C from
/// the write-loop instrumentation.
///
/// Fields are `#[allow(dead_code)]` for this slice because
/// the read consumer (`GetState` handler) lands in b-4; the
/// `recent()` tests in this module exercise them so the
/// shape is locked in now.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct TransferRecord {
    pub transfer_id: String,
    pub kind: ActiveJobKind,
    pub peer: String,
    pub module: String,
    pub path: String,
    pub start_unix_ms: u64,
    /// `unix_ms_at_drop - start_unix_ms`, saturating at zero
    /// so a clock skew between registration and drain doesn't
    /// produce a wraparound.
    pub duration_ms: u64,
    /// Total bytes the data plane reported for this transfer.
    /// Snapshotted from the per-row atomic at Drop time. Zero
    /// until c-1b wires the receive loop; field already lives
    /// on `TransferRecord.bytes` so future consumers don't
    /// see a shape change.
    pub bytes: u64,
    /// `true` if the handler reported success (Subscribe-era
    /// `TransferComplete`); `false` if it failed or the
    /// guard drained without a recorded outcome (panic,
    /// client cancellation before the handler reached the
    /// outcome-capture call).
    pub ok: bool,
    /// Empty when `ok == true`. Otherwise the handler's
    /// `Status::message()` for failures, or a short
    /// "cancelled before outcome recorded" marker when the
    /// guard drained without [`ActiveJobGuard::record_outcome`]
    /// being called.
    pub error_message: String,
}

/// One row's worth of progress data, returned by
/// [`ActiveJobs::snapshot_progress_samples`] for the c-4
/// ticker to fold into `TransferProgress` events.
#[derive(Debug, Clone)]
pub struct ProgressSample {
    pub transfer_id: String,
    pub bytes_completed: u64,
    /// Instantaneous rate over the tick window — `delta_bytes *
    /// 1000 / delta_ms`. Zero on the same-ms first tick.
    pub throughput_bps: u64,
}

/// In-memory registry, shared between the dispatch boundary and
/// future `GetState` reads.
#[derive(Clone)]
pub struct ActiveJobs {
    inner: Arc<Inner>,
}

/// Internal table row pairing the wire-shape `ActiveJob`
/// snapshot data with the per-transfer cancellation token.
/// Stored in a single locked map so:
///
/// 1. `snapshot()` and `cancel(id)` see a consistent view —
///    a row visible in `snapshot()` always has a cancellation
///    entry, and vice versa.
/// 2. `cancel(id)` can inspect the row's `kind` to decide
///    cancellable / unsupported atomically with the token
///    lookup, no parallel-map race.
///
/// Round-1 of m-jobs-1 used parallel `table` + `cancellations`
/// maps; the reviewer caught two races (snapshot-then-cancel
/// returning `false`, and `cancel` returning `true` for kinds
/// that ignore the token).
struct TableEntry {
    /// Snapshot fields. Cloned into the public `ActiveJob`
    /// shape at `snapshot()` time, with `bytes_completed`
    /// loaded from `bytes_counter`.
    job: ActiveJob,
    cancellation: CancellationToken,
    /// Per-row byte counter. Cloned (Arc bump) into every
    /// [`ByteProgressSink`] handed out by
    /// [`ActiveJobGuard::bytes_counter`]; loaded by
    /// `snapshot()` and by Drop when building the
    /// `TransferRecord`.
    bytes_counter: Arc<AtomicU64>,
    /// c-4: byte counter value observed by the most recent
    /// `snapshot_progress_samples` call. Used to compute
    /// per-tick byte deltas for the `TransferProgress.throughput_bps`
    /// field. Initialized to 0; updated atomically each tick.
    last_progress_bytes: AtomicU64,
    /// c-4: unix-ms timestamp of the most recent
    /// `snapshot_progress_samples` call against this row.
    /// Initialized to the row's `start_unix_ms`, so the first
    /// tick computes throughput over `(now - start)`.
    last_progress_unix_ms: AtomicU64,
    /// c-5b: per-row event ring. Pushed under the table lock
    /// by [`ActiveJobs::emit_event`]; cloned under the same
    /// lock by [`ActiveJobs::subscribe_with_ring`] for replay.
    /// Bounded to [`JOB_EVENT_RING_CAP`] — oldest entries
    /// drop on overflow.
    events_ring: VecDeque<DaemonEvent>,
}

struct Inner {
    table: Mutex<HashMap<String, TableEntry>>,
    /// Bounded ring of completed transfers, drained from
    /// `table` by [`ActiveJobGuard::Drop`]. Push at the back,
    /// trim from the front, so iteration order is
    /// oldest-first; callers reading `GetState.recent[]` will
    /// typically reverse for display.
    recent: Mutex<VecDeque<TransferRecord>>,
    /// Maximum number of entries kept in `recent`. Sized at
    /// construction so the `DEFAULT_RECENT_LIMIT` constant
    /// stays the only place the default is named.
    recent_limit: usize,
    /// Monotonic counter feeding [`mint_transfer_id`]. Keeps ids
    /// unique within a single millisecond when multiple
    /// transfers register at the same instant.
    counter: AtomicU64,
    /// rec-1: persistence signal. Empty by default — `ActiveJobs::new`
    /// and the test constructors leave persistence off, so they touch
    /// no disk. [`ActiveJobs::arm_persistence`] installs the sender
    /// (once) at daemon startup; after that, [`ActiveJobGuard::drop`]
    /// pings it whenever it appends to the ring, and the writer task
    /// rewrites `recents.jsonl`. A unit `()` signal (not the record)
    /// keeps the ring the single source of truth — the writer reads the
    /// current bounded ring rather than reconstructing it from a stream
    /// of deltas.
    persist_tx: OnceLock<mpsc::UnboundedSender<()>>,
}

impl ActiveJobs {
    /// Construct a registry with the default recent-runs
    /// ring depth ([`DEFAULT_RECENT_LIMIT`] = 50).
    pub fn new() -> Self {
        Self::with_recent_limit(DEFAULT_RECENT_LIMIT)
    }

    /// Construct a registry with a custom recent-runs ring
    /// depth. `limit == 0` is allowed and disables the ring
    /// (Drop still removes the active row; nothing is
    /// preserved). Will be reached by the future
    /// `GetState.GetStateRequest.recent_limit` plumbing.
    #[allow(dead_code)]
    pub fn with_recent_limit(limit: usize) -> Self {
        Self {
            inner: Arc::new(Inner {
                table: Mutex::new(HashMap::new()),
                recent: Mutex::new(VecDeque::with_capacity(limit)),
                recent_limit: limit,
                counter: AtomicU64::new(0),
                persist_tx: OnceLock::new(),
            }),
        }
    }

    /// Insert a row and return a guard that removes it on drop.
    ///
    /// For RPCs whose module + path are known at dispatch
    /// (`pull`, `delegated_pull`) the caller passes them
    /// directly. For streaming RPCs (`push`, `pull_sync`) the
    /// caller passes empty strings and the handler fills the
    /// row via [`ActiveJobGuard::set_endpoint`] once it has
    /// parsed the first stream frame.
    ///
    /// Sync because the table is `std::sync::Mutex`-guarded;
    /// callers in async dispatch handlers don't need to
    /// `.await` (the critical section is bounded by the size
    /// of the table, which is small).
    pub fn register(
        &self,
        kind: ActiveJobKind,
        peer: String,
        module: String,
        path: String,
    ) -> ActiveJobGuard {
        let transfer_id = mint_transfer_id(&self.inner.counter);
        let start_unix_ms = unix_ms_now();
        let row = ActiveJob {
            transfer_id: transfer_id.clone(),
            kind,
            peer,
            module,
            path,
            start_unix_ms,
            bytes_completed: 0,
        };
        let cancellation = CancellationToken::new();
        let bytes_counter = Arc::new(AtomicU64::new(0));
        let entry = TableEntry {
            job: row,
            cancellation: cancellation.clone(),
            bytes_counter: Arc::clone(&bytes_counter),
            last_progress_bytes: AtomicU64::new(0),
            last_progress_unix_ms: AtomicU64::new(start_unix_ms),
            events_ring: VecDeque::with_capacity(JOB_EVENT_RING_CAP),
        };
        self.inner
            .table
            .lock()
            .expect("active_jobs table poisoned")
            .insert(transfer_id.clone(), entry);
        ActiveJobGuard {
            inner: Arc::clone(&self.inner),
            transfer_id,
            start_unix_ms,
            outcome: Mutex::new(None),
            cancellation,
            bytes_counter,
        }
    }

    /// Try to cancel an active transfer by id. Returns a
    /// [`CancelOutcome`] distinguishing the three outcomes
    /// the upcoming `CancelJob` RPC needs to map onto gRPC
    /// status codes.
    ///
    /// Idempotent for `Cancelled`: firing an
    /// already-cancelled token is a no-op. The entry stays
    /// in the map until the guard drops; a second call
    /// against the same live id keeps returning `Cancelled`.
    ///
    /// `Unsupported` is returned synchronously when the row's
    /// kind does not honor cancellation
    /// ([`ActiveJobKind::supports_cancellation`]). The token
    /// is **not** fired in that case — handlers that don't
    /// race the token would silently keep running, and the
    /// caller would be lied to.
    #[allow(dead_code)]
    pub fn cancel(&self, transfer_id: &str) -> CancelOutcome {
        let guard = self.inner.table.lock().expect("active_jobs table poisoned");
        match guard.get(transfer_id) {
            None => CancelOutcome::NotFound,
            Some(entry) if !entry.job.kind.supports_cancellation() => CancelOutcome::Unsupported,
            Some(entry) => {
                entry.cancellation.cancel();
                CancelOutcome::Cancelled
            }
        }
    }

    /// Snapshot of every active row. Used by tests in this
    /// slice; will be used by `GetState.active[]` once the
    /// RPC handler lands in a later sub-slice.
    ///
    /// `bytes_completed` is loaded from the per-row atomic
    /// inside the lock so the snapshot reflects every report
    /// that landed before the snapshot acquired the lock.
    /// Reports that arrive concurrently (or after the lock
    /// is released) show up in the next snapshot.
    #[allow(dead_code)]
    pub fn snapshot(&self) -> Vec<ActiveJob> {
        self.inner
            .table
            .lock()
            .expect("active_jobs table poisoned")
            .values()
            .map(|e| {
                let mut job = e.job.clone();
                job.bytes_completed = e.bytes_counter.load(Ordering::Relaxed);
                job
            })
            .collect()
    }

    /// Push an event onto the per-row event ring (bounded by
    /// [`JOB_EVENT_RING_CAP`]) AND broadcast it on `events_tx`.
    /// Both happen under the table lock so c-5b's
    /// [`subscribe_with_ring`] doesn't see torn state:
    ///
    /// - If subscribe acquires the lock FIRST, it snapshots
    ///   the ring at its current state and registers a
    ///   broadcast Receiver. Any subsequent `emit_event`
    ///   adds to the ring (subscriber's snapshot is stale by
    ///   one entry) AND broadcasts (subscriber's Receiver
    ///   sees it on its next poll). Subscriber sees the event
    ///   exactly once via the broadcast.
    /// - If `emit_event` acquires the lock first, it pushes
    ///   to the ring and broadcasts. The broadcast send
    ///   completes while holding the lock so a not-yet-
    ///   subscribed client misses it via broadcast — but a
    ///   subsequent `subscribe_with_ring` snapshots the ring
    ///   (event present) and registers a Receiver (future
    ///   events only). Subscriber sees the event exactly once
    ///   via the replay.
    ///
    /// No-op if the transfer_id isn't in the table (row
    /// already drained or never registered) — the broadcast
    /// still fires either way so live subscribers without a
    /// specific filter still see it.
    pub fn emit_event(
        &self,
        events_tx: &broadcast::Sender<DaemonEvent>,
        transfer_id: &str,
        event: DaemonEvent,
    ) {
        let mut table = self.inner.table.lock().expect("active_jobs table poisoned");
        if let Some(entry) = table.get_mut(transfer_id) {
            if entry.events_ring.len() >= JOB_EVENT_RING_CAP {
                entry.events_ring.pop_front();
            }
            entry.events_ring.push_back(event.clone());
        }
        // Broadcast inside the lock — see ordering rationale
        // in the doc comment above. send is sync (in-memory
        // ring push, no I/O); critical section stays bounded.
        let _ = events_tx.send(event);
    }

    /// Register a broadcast Receiver AND snapshot the per-row
    /// event ring for `transfer_id_filter` atomically under
    /// the table lock. Returns:
    ///
    /// - `Receiver<DaemonEvent>` — live broadcast subscription
    ///   for the c-5a filtered forwarder.
    /// - `Vec<DaemonEvent>` — events to replay before live
    ///   forwarding starts. Empty unless `replay && !filter.is_empty()
    ///   && row exists`.
    ///
    /// Together with [`emit_event`]'s under-lock contract,
    /// guarantees no event is delivered to the subscriber
    /// twice (replay + broadcast) or zero times (missed
    /// during the subscribe race window).
    pub fn subscribe_with_ring(
        &self,
        events_tx: &broadcast::Sender<DaemonEvent>,
        transfer_id_filter: &str,
        replay: bool,
    ) -> (broadcast::Receiver<DaemonEvent>, Vec<DaemonEvent>) {
        let table = self.inner.table.lock().expect("active_jobs table poisoned");
        let rx = events_tx.subscribe();
        let events = if replay && !transfer_id_filter.is_empty() {
            table
                .get(transfer_id_filter)
                .map(|entry| entry.events_ring.iter().cloned().collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        (rx, events)
    }

    /// For each active row, sample the byte counter + throughput
    /// and invoke `emit` while holding the table lock. The lock
    /// is held across every `emit` call so the c-4 progress
    /// ticker can broadcast events WITHOUT racing the c-3
    /// terminal-event Drop path.
    ///
    /// Ordering invariant: [`ActiveJobGuard::Drop`] also takes
    /// this lock to remove the row. Holding the lock during
    /// emit guarantees that either:
    ///
    /// 1. The ticker took the lock first → all progress events
    ///    for currently-live rows fire BEFORE any Drop can
    ///    remove a row. The spawn closure's terminal event,
    ///    which is broadcast AFTER Drop (c-3 round 2), therefore
    ///    comes AFTER the corresponding progress events for the
    ///    same transfer_id.
    /// 2. Drop took the lock first → row already removed by the
    ///    time the ticker iterates → no progress event fires
    ///    for that transfer_id. The spawn closure's terminal
    ///    event has been (or will shortly be) broadcast.
    ///
    /// Either way, a `TransferProgress` for a given transfer_id
    /// cannot follow its `TransferComplete` / `TransferError`.
    ///
    /// `emit` must be cheap and non-blocking — it's called
    /// under a `std::sync::Mutex`. The `broadcast::Sender::send`
    /// the c-4 ticker uses is synchronous (pushes to a ring
    /// buffer, never awaits), so this is safe.
    ///
    /// `throughput_bps` is the instantaneous rate over the
    /// elapsed window — `(delta_bytes * 1000) / delta_ms`. A
    /// smoothed EWMA is a future C sub-slice once an operator
    /// signal justifies the extra state.
    #[cfg(test)]
    pub fn for_each_progress_sample<F: FnMut(ProgressSample)>(&self, mut emit: F) {
        let now_ms = unix_ms_now();
        let table = self.inner.table.lock().expect("active_jobs table poisoned");
        for entry in table.values() {
            let cur_bytes = entry.bytes_counter.load(Ordering::Relaxed);
            // swap returns the OLD value; cur_bytes/now_ms is the
            // new baseline for the next tick.
            let last_bytes = entry.last_progress_bytes.swap(cur_bytes, Ordering::Relaxed);
            let last_ms = entry.last_progress_unix_ms.swap(now_ms, Ordering::Relaxed);
            let delta_bytes = cur_bytes.saturating_sub(last_bytes);
            let delta_ms = now_ms.saturating_sub(last_ms);
            // delta_ms == 0 can happen on the same-millisecond
            // first tick (register and tick in the same ms). Skip
            // the divide; report 0 throughput; next tick will have
            // a real window.
            let throughput_bps = if delta_ms == 0 {
                0
            } else {
                ((delta_bytes as u128) * 1000 / (delta_ms as u128)) as u64
            };
            emit(ProgressSample {
                transfer_id: entry.job.transfer_id.clone(),
                bytes_completed: cur_bytes,
                throughput_bps,
            });
        }
    }

    /// c-5b: sibling of [`for_each_progress_sample`] that also
    /// pushes the built event onto the row's event ring AND
    /// broadcasts it via `events_tx`. All three (sample
    /// computation, ring push, broadcast) happen under the
    /// same table lock — so the ordering against
    /// [`subscribe_with_ring`] / [`emit_event`] / Drop holds.
    ///
    /// Returns the number of events emitted (one per active
    /// row). Used by the c-4 progress ticker.
    pub fn tick_progress_emit<F>(
        &self,
        events_tx: &broadcast::Sender<DaemonEvent>,
        mut build_event: F,
    ) -> usize
    where
        F: FnMut(&ProgressSample) -> DaemonEvent,
    {
        let now_ms = unix_ms_now();
        let mut table = self.inner.table.lock().expect("active_jobs table poisoned");
        let mut count: usize = 0;
        for entry in table.values_mut() {
            let cur_bytes = entry.bytes_counter.load(Ordering::Relaxed);
            let last_bytes = entry.last_progress_bytes.swap(cur_bytes, Ordering::Relaxed);
            let last_ms = entry.last_progress_unix_ms.swap(now_ms, Ordering::Relaxed);
            let delta_bytes = cur_bytes.saturating_sub(last_bytes);
            let delta_ms = now_ms.saturating_sub(last_ms);
            let throughput_bps = if delta_ms == 0 {
                0
            } else {
                ((delta_bytes as u128) * 1000 / (delta_ms as u128)) as u64
            };
            let sample = ProgressSample {
                transfer_id: entry.job.transfer_id.clone(),
                bytes_completed: cur_bytes,
                throughput_bps,
            };
            let event = build_event(&sample);
            if entry.events_ring.len() >= JOB_EVENT_RING_CAP {
                entry.events_ring.pop_front();
            }
            entry.events_ring.push_back(event.clone());
            let _ = events_tx.send(event);
            count += 1;
        }
        count
    }

    /// Convenience wrapper around [`for_each_progress_sample`]
    /// that collects the samples into a Vec. Used by tests that
    /// only want to inspect the sample shape without exercising
    /// the broadcast emit. Callers that broadcast from the
    /// returned Vec DO NOT preserve the terminal-event ordering
    /// invariant — the lock is released before any send fires.
    /// The c-4 ticker uses `for_each_progress_sample` directly
    /// for that reason.
    #[cfg(test)]
    pub fn snapshot_progress_samples(&self) -> Vec<ProgressSample> {
        let mut out = Vec::new();
        self.for_each_progress_sample(|s| out.push(s));
        out
    }

    /// Snapshot of the recent-runs ring, oldest first. Will
    /// be consumed by `GetState.recent[]` once the RPC
    /// handler lands.
    #[allow(dead_code)]
    pub fn recent(&self) -> Vec<TransferRecord> {
        self.inner
            .recent
            .lock()
            .expect("active_jobs recent poisoned")
            .iter()
            .cloned()
            .collect()
    }

    /// rec-2: clear the recent-runs ring and, if persistence is armed,
    /// trigger the writer to rewrite the on-disk store empty. Returns
    /// the number of entries removed.
    ///
    /// **Never touches the planner's `perf_local.jsonl`.** This only
    /// affects the recents ring and its dedicated `recents.jsonl`
    /// backing store — the predictor's historical telemetry is a
    /// separate store and is deliberately left intact (see module
    /// [`crate::recents_store`] and [`blit_core::perf_history`]).
    pub fn clear_recent(&self) -> usize {
        let cleared = {
            let mut ring = self.inner.recent.lock().unwrap_or_else(|e| e.into_inner());
            let n = ring.len();
            ring.clear();
            n
        };
        // Persist the cleared state: the writer reads the now-empty ring
        // and atomically rewrites recents.jsonl empty. Non-blocking, and
        // a no-op when persistence isn't armed (test/default tables).
        if let Some(tx) = self.inner.persist_tx.get() {
            let _ = tx.send(());
        }
        cleared
    }

    /// rec-1: hydrate the ring from the on-disk recents store and arm
    /// write-through persistence, using the default
    /// [`crate::recents_store::recents_path`]. Called once at daemon
    /// startup (`main`), before serving, so the first `GetState`
    /// already reflects pre-restart recents.
    ///
    /// Returns the [`RecentsWriter`] the caller must hand to
    /// [`spawn_recents_writer`]; until it runs, ring appends still
    /// queue persistence signals (the channel is unbounded), so none
    /// are lost in the startup window.
    pub fn arm_persistence(&self) -> eyre::Result<RecentsWriter> {
        Ok(self.arm_persistence_at(crate::recents_store::recents_path()?))
    }

    /// [`Self::arm_persistence`] against an explicit path. Lets tests
    /// drive hydration + write-through against a tempdir without
    /// touching the global config dir.
    pub fn arm_persistence_at(&self, path: std::path::PathBuf) -> RecentsWriter {
        // Hydrate the ring from disk. Startup-only, so the ring is
        // empty here; we still drain-then-extend (rather than assume
        // empty) so the method is correct if ever called post-startup.
        let loaded = crate::recents_store::load(&path, self.inner.recent_limit);
        {
            let mut ring = self.inner.recent.lock().unwrap_or_else(|e| e.into_inner());
            ring.clear();
            ring.extend(loaded);
        }
        let (tx, rx) = mpsc::unbounded_channel();
        // `set` fails only if persistence was already armed; the first
        // arm wins and a duplicate call is a no-op writer that will
        // never be pinged.
        let _ = self.inner.persist_tx.set(tx);
        RecentsWriter {
            active_jobs: self.clone(),
            rx,
            path,
        }
    }
}

/// rec-1: the write-through half of recents persistence, produced by
/// [`ActiveJobs::arm_persistence`]. Owns the persistence-signal
/// receiver and a handle back to the ring; [`spawn_recents_writer`]
/// drives its [`RecentsWriter::run`] loop on the runtime for the
/// daemon's lifetime (mirroring the progress ticker).
pub struct RecentsWriter {
    active_jobs: ActiveJobs,
    rx: mpsc::UnboundedReceiver<()>,
    path: std::path::PathBuf,
}

impl RecentsWriter {
    /// Drain persistence signals and rewrite the recents store. Each
    /// signal coalesces with any others already queued (a burst of
    /// completions collapses into one atomic rewrite of the current
    /// ring), then writes the bounded ring to disk. Returns when the
    /// sender is dropped (all `ActiveJobs` clones gone — daemon
    /// shutdown).
    pub async fn run(mut self) {
        while self.rx.recv().await.is_some() {
            // Coalesce: if several completions raced, one rewrite of
            // the (already-updated) ring covers them all.
            while self.rx.try_recv().is_ok() {}
            let records = self.active_jobs.recent();
            if let Err(e) = crate::recents_store::write_atomic(&self.path, &records) {
                eprintln!("[blitd] failed to persist recents to {:?}: {e}", self.path);
            }
        }
    }
}

/// rec-1: spawn the recents writer on the runtime. The handle is owned
/// for the daemon's lifetime; on process exit tokio aborts it (any
/// final unpersisted recents are immaterial). Mirrors
/// [`crate::service::spawn_progress_ticker`].
pub fn spawn_recents_writer(writer: RecentsWriter) -> tokio::task::JoinHandle<()> {
    tokio::spawn(writer.run())
}

impl Default for ActiveJobs {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard tying an `ActiveJob` row's lifetime to the
/// dispatcher's spawned task. Drop removes the row whether the
/// task completed, errored, or was cancelled — same posture as
/// the metrics active-transfers gauge — and pushes a
/// [`TransferRecord`] onto the registry's recent-runs ring.
///
/// Drop is **synchronous and deterministic**: after the guard
/// is dropped, the row is gone and the ring has been updated.
/// This is the contract `GetState.active[]` /
/// `GetState.recent[]` rely on.
pub struct ActiveJobGuard {
    inner: Arc<Inner>,
    transfer_id: String,
    /// Registration timestamp, captured at the same instant the
    /// row's `start_unix_ms` was stamped. Exposed via
    /// [`ActiveJobGuard::start_unix_ms`] so subscribers building
    /// `TransferStarted` events don't need to hit the table lock
    /// just to read this field.
    start_unix_ms: u64,
    /// Filled by [`record_outcome`] before Drop. If still
    /// `None` at Drop time the spawn task either panicked or
    /// was cancelled before reaching the outcome-capture call,
    /// and the ring entry records `ok=false` +
    /// `"cancelled before outcome recorded"`.
    outcome: Mutex<Option<RecordedOutcome>>,
    /// Per-transfer cancellation token. Cloned from the
    /// registry's `cancellations` map at register time so
    /// handlers can `.await` against it without re-acquiring
    /// the map lock. Fired by `ActiveJobs::cancel(id)`;
    /// handlers that opt in race the token against their
    /// transfer future.
    cancellation: CancellationToken,
    /// Same atomic the [`TableEntry::bytes_counter`] holds —
    /// kept here so Drop can read the final value without
    /// re-acquiring the table lock just to inspect it before
    /// removal. Cloned Arc, not a separate counter.
    bytes_counter: Arc<AtomicU64>,
}

/// Outcome handed to [`ActiveJobGuard::record_outcome`].
/// `error_message` is required when `ok == false`; empty
/// otherwise.
struct RecordedOutcome {
    ok: bool,
    error_message: String,
}

impl ActiveJobGuard {
    /// Stable id assigned to this transfer. Exposed so handlers
    /// that want to surface the id in their wire response (M-C
    /// `TransferStarted.transfer_id`, M-Jobs `CancelJob`) can
    /// read it. Currently only the tests in this module
    /// consume it; future slices will read it from the
    /// dispatch boundary.
    #[allow(dead_code)]
    pub fn transfer_id(&self) -> &str {
        &self.transfer_id
    }

    /// Unix-milliseconds timestamp captured at `register()` time.
    /// Exposed so the Subscribe dispatch site can build a
    /// `TransferStarted` event without re-stamping the clock
    /// (and so the start time on the event matches the one
    /// `GetState.active[].start_unix_ms` will surface for the
    /// same row).
    pub fn start_unix_ms(&self) -> u64 {
        self.start_unix_ms
    }

    /// Current value of the per-row byte counter. Read by the
    /// terminal-event builder (`TransferComplete.bytes`) so the
    /// emitted event carries the same total `TransferRecord.bytes`
    /// will freeze into the ring on Drop. Relaxed load matches
    /// the c-1a contract — readers only need eventual visibility.
    pub fn bytes_completed_load(&self) -> u64 {
        self.bytes_counter.load(Ordering::Relaxed)
    }

    /// Wall-clock duration since registration, in milliseconds.
    /// Used to populate `TransferComplete.duration_ms`. Saturates
    /// at zero on a backwards clock jump, same posture as the
    /// `TransferRecord.duration_ms` build at Drop.
    pub fn elapsed_ms(&self) -> u64 {
        unix_ms_now().saturating_sub(self.start_unix_ms)
    }

    /// Update the row's `module` and `path` fields. Used by
    /// streaming-RPC handlers (`handle_push_stream`,
    /// `handle_pull_sync_stream`) once they've parsed the
    /// first stream frame and know the transfer's endpoint.
    ///
    /// At dispatch the streaming RPCs register a row with
    /// empty strings for `module` / `path` because the
    /// `BlitService` doesn't see those fields synchronously
    /// (they arrive in `ClientPushRequest::Header` /
    /// `TransferOperationSpec` mid-stream). After this call,
    /// the row matches what `pull` / `delegated_pull` register
    /// at dispatch.
    ///
    /// No-op if the row has already been drained — handlers
    /// may parse the header right around when the client
    /// cancels, and we'd rather silently skip the update than
    /// re-insert a row that the dispatcher's spawned task has
    /// already cleaned up.
    pub fn set_endpoint(&self, module: String, path: String) {
        let mut table = self.inner.table.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = table.get_mut(&self.transfer_id) {
            entry.job.module = module;
            entry.job.path = path;
        }
    }

    /// Reference to the per-row cancellation token. Handlers
    /// that opt into daemon-side cancellation race against
    /// `cancellation_token().cancelled()` inside a `tokio::select!`;
    /// `ActiveJobs::cancel(id)` fires this token from outside
    /// (via the CancelJob RPC in m-jobs-2).
    pub fn cancellation_token(&self) -> &CancellationToken {
        &self.cancellation
    }

    /// Clonable handle to this transfer's byte counter. Handers
    /// pass it into the data-plane write loop; each chunk write
    /// calls [`ByteProgressSink::report`] with the bytes just
    /// written. Reports show up in `GetState.active[].bytes_completed`
    /// on the next snapshot, and in `GetState.recent[].bytes`
    /// once the guard drops.
    ///
    /// The sink wraps an `Arc` of the same atomic the table row
    /// holds; cloning is cheap and keeping a clone alive past
    /// Drop is harmless — reports after Drop just bump an
    /// orphaned atomic, no row to resurrect.
    #[allow(dead_code)]
    pub fn bytes_counter(&self) -> ByteProgressSink {
        ByteProgressSink::from_counter(Arc::clone(&self.bytes_counter))
    }

    /// Capture the transfer's outcome before Drop. Spawn
    /// closures in `service/core.rs` call this with the
    /// handler's `Result` translated to `(ok, error_message)`
    /// just before the guard goes out of scope.
    ///
    /// Last writer wins — if a closure happens to call it
    /// twice (e.g. one branch records success then a follow-up
    /// branch overrides) the most recent value is used. In
    /// practice each spawn closure calls it once.
    ///
    /// If never called, Drop records the entry with
    /// `ok=false` and a "cancelled before outcome recorded"
    /// error_message so the ring carries a placeholder rather
    /// than silently dropping the run.
    pub fn record_outcome(&self, ok: bool, error_message: Option<String>) {
        let mut cell = self.outcome.lock().unwrap_or_else(|e| e.into_inner());
        *cell = Some(RecordedOutcome {
            ok,
            error_message: error_message.unwrap_or_default(),
        });
    }
}

impl Drop for ActiveJobGuard {
    fn drop(&mut self) {
        // Synchronous remove-and-record. PoisonError still
        // hands us the inner guards via `into_inner`, so the
        // active row and the ring are updated even if a panic
        // poisoned a mutex on the way in. This matches the
        // rest of the codebase's stance on poisoning — surface
        // the failure, but don't leak state.
        //
        // Lock order: table → recent. Held sequentially (no
        // nested acquisitions). `cancel(id)` takes only the
        // table lock, so it can't deadlock against this Drop
        // path.
        let id = std::mem::take(&mut self.transfer_id);
        let outcome = {
            let mut cell = self.outcome.lock().unwrap_or_else(|e| e.into_inner());
            cell.take()
        };
        let entry = {
            let mut table = self.inner.table.lock().unwrap_or_else(|e| e.into_inner());
            table.remove(&id)
        };
        if let Some(entry) = entry {
            if self.inner.recent_limit > 0 {
                // Final byte count: load before the entry's
                // Arc<AtomicU64> goes out of scope. The
                // ActiveJobGuard's clone is still alive (we're
                // inside its Drop), but reading off the entry
                // is equivalent and keeps the lookup paired
                // with the row being drained.
                let bytes = entry.bytes_counter.load(Ordering::Relaxed);
                let record = build_record(entry.job, outcome, bytes);
                push_recent(&self.inner.recent, record, self.inner.recent_limit);
                // rec-1: nudge the persistence writer (if armed). Drop
                // is synchronous and on the runtime, so we must not do
                // file I/O here — an unbounded-channel send is
                // non-blocking and never awaits. `send` only errs if
                // the writer task is gone (daemon shutting down), which
                // we ignore: a recents write lost at shutdown is
                // immaterial.
                if let Some(tx) = self.inner.persist_tx.get() {
                    let _ = tx.send(());
                }
            }
        }
    }
}

fn build_record(row: ActiveJob, outcome: Option<RecordedOutcome>, bytes: u64) -> TransferRecord {
    let drop_unix_ms = unix_ms_now();
    let duration_ms = drop_unix_ms.saturating_sub(row.start_unix_ms);
    let (ok, error_message) = match outcome {
        Some(o) => (o.ok, o.error_message),
        None => (false, "cancelled before outcome recorded".to_string()),
    };
    TransferRecord {
        transfer_id: row.transfer_id,
        kind: row.kind,
        peer: row.peer,
        module: row.module,
        path: row.path,
        start_unix_ms: row.start_unix_ms,
        duration_ms,
        bytes,
        ok,
        error_message,
    }
}

fn push_recent(recent: &Mutex<VecDeque<TransferRecord>>, record: TransferRecord, limit: usize) {
    let mut buf = recent.lock().unwrap_or_else(|e| e.into_inner());
    buf.push_back(record);
    while buf.len() > limit {
        buf.pop_front();
    }
}

fn mint_transfer_id(counter: &AtomicU64) -> String {
    let n = counter.fetch_add(1, Ordering::Relaxed);
    let ms = unix_ms_now();
    // `t<unix-ms>-<n>` keeps ids short (~22 chars), sortable
    // by submission time, and unique within a daemon
    // instance. Daemon restart resets the counter; durability
    // across restart is deferred per §10 open questions.
    format!("t{ms}-{n}")
}

fn unix_ms_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::Barrier;

    #[tokio::test]
    async fn register_inserts_then_drop_removes() {
        let table = ActiveJobs::new();
        {
            let _guard = table.register(
                ActiveJobKind::Pull,
                "127.0.0.1:9000".to_string(),
                "mod-a".to_string(),
                "sub/dir".to_string(),
            );
            let snap = table.snapshot();
            assert_eq!(snap.len(), 1);
            assert_eq!(snap[0].kind, ActiveJobKind::Pull);
            assert_eq!(snap[0].peer, "127.0.0.1:9000");
            assert_eq!(snap[0].module, "mod-a");
            assert_eq!(snap[0].path, "sub/dir");
            assert!(snap[0].transfer_id.starts_with('t'));
            assert!(snap[0].start_unix_ms > 0);
        }
        // Drop is synchronous now; no need to yield.
        assert!(table.snapshot().is_empty());
    }

    #[tokio::test]
    async fn transfer_ids_unique_under_concurrent_registers() {
        // Deterministic concurrent-registration test: barrier
        // gates registration so the parent only inspects the
        // table once all N rows are live; a second barrier
        // gates drop so the parent observes the empty table
        // after every guard releases. No sleep-based timing.
        let n = 64;
        let table = ActiveJobs::new();
        let registered = Arc::new(Barrier::new(n + 1));
        let release = Arc::new(Barrier::new(n + 1));
        let mut handles = Vec::with_capacity(n);
        for _ in 0..n {
            let t = table.clone();
            let registered = Arc::clone(&registered);
            let release = Arc::clone(&release);
            handles.push(tokio::spawn(async move {
                let guard = t.register(
                    ActiveJobKind::DelegatedPull,
                    "peer".to_string(),
                    "mod".to_string(),
                    "/".to_string(),
                );
                let id = guard.transfer_id().to_string();
                // Signal "I'm registered" and block until the
                // parent says we may drop.
                registered.wait().await;
                release.wait().await;
                drop(guard);
                id
            }));
        }

        // Parent rendezvous with all spawned tasks at the
        // registration barrier. Every row is live when this
        // returns.
        registered.wait().await;
        let mid_snap = table.snapshot();
        assert_eq!(mid_snap.len(), n, "all rows should be live");

        // Release the spawned tasks; they each drop their
        // guard immediately after the second barrier.
        release.wait().await;

        // Await every spawn so its Drop has definitely run by
        // the time we re-snapshot.
        let mut ids = Vec::with_capacity(n);
        for h in handles {
            ids.push(h.await.unwrap());
        }
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n, "transfer ids must be unique");
        assert!(table.snapshot().is_empty());
    }

    #[tokio::test]
    async fn drop_removes_row_after_holder_releases_contended_lock() {
        // Deterministic test of Drop under contention. Uses
        // two `std::sync::mpsc` rendezvous channels so the
        // sequencing is explicit:
        //
        //   1. Holder thread acquires the registry mutex,
        //      sends `LOCKED` on `tx_locked`.
        //   2. Parent task receives `LOCKED` — the holder now
        //      definitively owns the lock.
        //   3. Parent spawns the dropper. The dropper's
        //      `drop(guard)` calls `lock()` on the same mutex
        //      and must wait — there is no other code path
        //      out of `ActiveJobGuard::Drop`.
        //   4. Parent sends `RELEASE` on `tx_release`; the
        //      holder thread completes its scope (releasing
        //      the mutex) and the dropper unblocks.
        //   5. Parent awaits both threads, then asserts the
        //      table is empty.
        //
        // The previous round's version asserted "dropper has
        // not finished while holder holds the lock" via an
        // `AtomicBool` + spin — racy when the holder hadn't
        // yet acquired the mutex by the time the dropper ran.
        // The reviewer flagged the race; this version drops
        // that assertion entirely because the `std::sync::Mutex`
        // semantics (Drop's `lock()` call cannot complete
        // until the holder releases) are structural, not
        // testable timing. The deterministic property under
        // test is the same one `GetState.active[]` will rely
        // on: after the guard is dropped, the row is gone.
        let table = ActiveJobs::new();
        let guard = table.register(
            ActiveJobKind::Pull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );

        let (tx_locked, rx_locked) = std::sync::mpsc::sync_channel::<()>(0);
        let (tx_release, rx_release) = std::sync::mpsc::sync_channel::<()>(0);

        let table_for_holder = table.clone();
        let holder = tokio::task::spawn_blocking(move || {
            let _held = table_for_holder
                .inner
                .table
                .lock()
                .expect("active_jobs table poisoned");
            tx_locked.send(()).expect("locked-send");
            rx_release.recv().expect("release-recv");
            // _held drops here — mutex releases when this
            // function returns.
        });

        // Wait until the holder definitely owns the mutex
        // before spawning the dropper.
        rx_locked.recv().expect("locked-recv");

        let dropper = tokio::task::spawn_blocking(move || {
            drop(guard);
        });

        // Holder is parked on `rx_release.recv()`; the
        // dropper is parked on `mutex.lock()`. Send the
        // release signal so the holder returns, dropping its
        // `_held` guard and unblocking the dropper.
        tx_release.send(()).expect("release-send");
        holder.await.expect("holder join");
        dropper.await.expect("dropper join");

        // The dropper's `Drop` has now run to completion under
        // genuine contention — and the row must be gone.
        assert!(table.snapshot().is_empty());
    }

    #[tokio::test]
    async fn set_endpoint_updates_row_in_place() {
        // Streaming-RPC dispatchers register with empty
        // module/path; the handler fills them in via
        // `set_endpoint` once the first stream frame parses.
        let table = ActiveJobs::new();
        let guard = table.register(
            ActiveJobKind::Push,
            "10.0.0.5:443".to_string(),
            String::new(),
            String::new(),
        );

        // Initial snapshot: empty module/path.
        let initial = table.snapshot();
        assert_eq!(initial.len(), 1);
        assert!(initial[0].module.is_empty());
        assert!(initial[0].path.is_empty());

        guard.set_endpoint("mod-streaming".to_string(), "sub/dir".to_string());

        // After set_endpoint: same row, populated fields.
        let updated = table.snapshot();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].transfer_id, initial[0].transfer_id);
        assert_eq!(updated[0].module, "mod-streaming");
        assert_eq!(updated[0].path, "sub/dir");
        // start_unix_ms is unchanged — set_endpoint doesn't
        // re-stamp the registration time.
        assert_eq!(updated[0].start_unix_ms, initial[0].start_unix_ms);
    }

    #[tokio::test]
    async fn set_endpoint_is_noop_after_guard_drops() {
        // Catches the race where a handler parses the first
        // frame just as the client cancels: by the time
        // `set_endpoint` fires, the row is already gone.
        // `set_endpoint` must NOT re-insert a stale row.
        let table = ActiveJobs::new();
        let guard = table.register(
            ActiveJobKind::PullSync,
            "p".to_string(),
            String::new(),
            String::new(),
        );
        let id_before_drop = guard.transfer_id().to_string();

        // Manually remove the row to simulate "drained while
        // the handler was still preparing the set_endpoint
        // call." This is the same path Drop takes.
        table.inner.table.lock().unwrap().remove(&id_before_drop);
        assert!(table.snapshot().is_empty());

        // The handler then calls set_endpoint. No row exists,
        // so the call must be a no-op — not a re-insert.
        guard.set_endpoint("mod".to_string(), "p".to_string());
        assert!(
            table.snapshot().is_empty(),
            "set_endpoint must not re-insert a drained row"
        );

        // Letting the guard's Drop run is also a no-op on the
        // already-empty table.
        drop(guard);
        assert!(table.snapshot().is_empty());
    }

    #[tokio::test]
    async fn drop_with_recorded_outcome_pushes_to_recent() {
        let table = ActiveJobs::new();
        {
            let guard = table.register(
                ActiveJobKind::Pull,
                "peer".to_string(),
                "mod".to_string(),
                "p".to_string(),
            );
            guard.record_outcome(true, None);
        }
        let recent = table.recent();
        assert_eq!(recent.len(), 1);
        let r = &recent[0];
        assert_eq!(r.kind, ActiveJobKind::Pull);
        assert_eq!(r.peer, "peer");
        assert_eq!(r.module, "mod");
        assert_eq!(r.path, "p");
        assert!(r.ok);
        assert!(r.error_message.is_empty());
        // duration_ms is `unix_ms - start_unix_ms`; can be 0
        // if the test ran in the same millisecond, but the
        // field must be present.
        let _ = r.duration_ms;
    }

    #[tokio::test]
    async fn drop_with_error_outcome_carries_message() {
        let table = ActiveJobs::new();
        {
            let guard = table.register(
                ActiveJobKind::Push,
                "p".to_string(),
                String::new(),
                String::new(),
            );
            guard.record_outcome(false, Some("module not found".to_string()));
        }
        let recent = table.recent();
        assert_eq!(recent.len(), 1);
        assert!(!recent[0].ok);
        assert_eq!(recent[0].error_message, "module not found");
    }

    #[tokio::test]
    async fn drop_without_recorded_outcome_marks_cancelled() {
        // If the spawn task panics or is cancelled before
        // reaching `record_outcome`, the ring should still
        // carry a placeholder rather than silently dropping
        // the run.
        let table = ActiveJobs::new();
        {
            let _guard = table.register(
                ActiveJobKind::DelegatedPull,
                "p".to_string(),
                "mod".to_string(),
                "p".to_string(),
            );
            // No record_outcome call.
        }
        let recent = table.recent();
        assert_eq!(recent.len(), 1);
        assert!(!recent[0].ok);
        assert_eq!(recent[0].error_message, "cancelled before outcome recorded");
    }

    #[tokio::test]
    async fn recent_ring_bounded_evicts_oldest() {
        let table = ActiveJobs::with_recent_limit(3);
        // Push 5 entries; only the last 3 should survive.
        for i in 0..5 {
            let guard = table.register(
                ActiveJobKind::Pull,
                format!("peer{i}"),
                "mod".to_string(),
                "p".to_string(),
            );
            guard.record_outcome(true, None);
        }
        let recent = table.recent();
        assert_eq!(recent.len(), 3);
        // Oldest-first ordering: the 3 survivors are peer2, peer3, peer4.
        assert_eq!(recent[0].peer, "peer2");
        assert_eq!(recent[1].peer, "peer3");
        assert_eq!(recent[2].peer, "peer4");
    }

    #[tokio::test]
    async fn recent_ring_zero_limit_disables_history() {
        let table = ActiveJobs::with_recent_limit(0);
        {
            let guard = table.register(
                ActiveJobKind::Pull,
                "p".to_string(),
                "m".to_string(),
                "p".to_string(),
            );
            guard.record_outcome(true, None);
        }
        // Active row drained, but no ring entry pushed.
        assert!(table.snapshot().is_empty());
        assert!(table.recent().is_empty());
    }

    /// rec-1: a persisted record for hydration tests. Fields are all
    /// public; `transfer_id` is a fixed marker since hydration replays
    /// whatever was on disk verbatim (no id minting).
    fn persisted_record(id: &str) -> TransferRecord {
        TransferRecord {
            transfer_id: id.to_string(),
            kind: ActiveJobKind::Pull,
            peer: "peer".to_string(),
            module: "mod".to_string(),
            path: "path".to_string(),
            start_unix_ms: 1,
            duration_ms: 2,
            bytes: 3,
            ok: true,
            error_message: String::new(),
        }
    }

    /// Poll the recents file until it holds at least `want` records or
    /// the bounded attempt budget runs out — the writer task flushes
    /// asynchronously after a completion signal.
    async fn poll_recents(path: &std::path::Path, want: usize) -> Vec<TransferRecord> {
        for _ in 0..100 {
            let loaded = crate::recents_store::load(path, DEFAULT_RECENT_LIMIT);
            if loaded.len() >= want {
                return loaded;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        crate::recents_store::load(path, DEFAULT_RECENT_LIMIT)
    }

    /// rec-1: arming persistence hydrates the in-memory ring from the
    /// on-disk store, oldest-first, so a restarted daemon's first
    /// `GetState.recent[]` reflects pre-restart runs.
    #[tokio::test]
    async fn arm_persistence_hydrates_ring_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recents.jsonl");
        crate::recents_store::write_atomic(&path, &[persisted_record("a"), persisted_record("b")])
            .unwrap();

        let table = ActiveJobs::with_recent_limit(DEFAULT_RECENT_LIMIT);
        assert!(table.recent().is_empty(), "ring starts empty pre-arm");
        let _writer = table.arm_persistence_at(path.clone());

        let ids: Vec<_> = table.recent().into_iter().map(|r| r.transfer_id).collect();
        assert_eq!(ids, ["a", "b"], "hydrated from disk, oldest-first");
    }

    /// rec-1: a completed transfer is written through to the recents
    /// store by the writer task, so it survives a restart.
    #[tokio::test]
    async fn completed_transfer_writes_through_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recents.jsonl");

        let table = ActiveJobs::with_recent_limit(DEFAULT_RECENT_LIMIT);
        let writer = table.arm_persistence_at(path.clone());
        let handle = tokio::spawn(writer.run());

        {
            let guard = table.register(
                ActiveJobKind::Pull,
                "wt-peer".to_string(),
                "mod".to_string(),
                "p".to_string(),
            );
            guard.record_outcome(true, None);
        } // drop → push_recent + persistence signal

        let loaded = poll_recents(&path, 1).await;
        assert_eq!(loaded.len(), 1, "completion flushed to disk");
        assert_eq!(loaded[0].peer, "wt-peer");
        assert!(loaded[0].ok);
        handle.abort();
    }

    /// rec-1: persistence is opt-in — a table that was never armed
    /// touches no disk on completion (the writer signal is a no-op when
    /// the `OnceLock` is empty). Guards against a regression that would
    /// make every `ActiveJobs` (including test/default ones) write.
    #[tokio::test]
    async fn unarmed_table_does_not_persist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recents.jsonl");
        let table = ActiveJobs::with_recent_limit(DEFAULT_RECENT_LIMIT);
        {
            let guard = table.register(
                ActiveJobKind::Pull,
                "p".to_string(),
                "mod".to_string(),
                "p".to_string(),
            );
            guard.record_outcome(true, None);
        }
        // Nothing armed → no writer → the store file is never created.
        assert!(!path.exists(), "unarmed ActiveJobs must not write recents");
        assert_eq!(
            table.recent().len(),
            1,
            "but the in-memory ring still works"
        );
    }

    /// Poll the recents store until it loads empty (or the budget runs
    /// out) — `clear_recent` empties the ring synchronously but the
    /// on-disk rewrite is flushed asynchronously by the writer task.
    async fn poll_recents_empty(path: &std::path::Path) -> bool {
        for _ in 0..100 {
            if crate::recents_store::load(path, DEFAULT_RECENT_LIMIT).is_empty() {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        crate::recents_store::load(path, DEFAULT_RECENT_LIMIT).is_empty()
    }

    /// rec-2: `clear_recent` empties the in-memory ring and the on-disk
    /// recents store, returning the count removed — and **never touches
    /// the planner's `perf_local.jsonl`**. The sibling perf-telemetry
    /// file (the predictor's training data) must be byte-for-byte intact
    /// after a clear; this is the core safety property of the feature.
    #[tokio::test]
    async fn clear_recent_empties_store_but_not_perf_local() {
        let dir = tempfile::tempdir().unwrap();
        let recents_path = dir.path().join("recents.jsonl");
        // Stand in for the planner's telemetry living in the same dir.
        let perf_local = dir.path().join("perf_local.jsonl");
        let perf_contents = "{\"telemetry\":\"planner training data\"}\n";
        std::fs::write(&perf_local, perf_contents).unwrap();

        let table = ActiveJobs::with_recent_limit(DEFAULT_RECENT_LIMIT);
        let writer = table.arm_persistence_at(recents_path.clone());
        let handle = tokio::spawn(writer.run());

        // One completed transfer → ring + store hold one entry.
        {
            let guard = table.register(
                ActiveJobKind::Pull,
                "p".to_string(),
                "mod".to_string(),
                "p".to_string(),
            );
            guard.record_outcome(true, None);
        }
        assert_eq!(poll_recents(&recents_path, 1).await.len(), 1);

        let cleared = table.clear_recent();
        assert_eq!(cleared, 1, "one entry removed");
        assert!(table.recent().is_empty(), "in-memory ring emptied at once");
        assert!(
            poll_recents_empty(&recents_path).await,
            "recents.jsonl rewritten empty by the writer"
        );

        // The critical assertion: planner telemetry is byte-for-byte
        // untouched by the clear.
        assert_eq!(
            std::fs::read_to_string(&perf_local).unwrap(),
            perf_contents,
            "clear_recent must never touch perf_local.jsonl"
        );
        handle.abort();
    }

    /// rec-2: `clear_recent` on an unarmed table still empties the ring
    /// (and is a no-op for persistence). Returns the count removed.
    #[tokio::test]
    async fn clear_recent_unarmed_empties_ring() {
        let table = ActiveJobs::with_recent_limit(DEFAULT_RECENT_LIMIT);
        for _ in 0..3 {
            let guard = table.register(
                ActiveJobKind::Pull,
                "p".to_string(),
                "mod".to_string(),
                "p".to_string(),
            );
            guard.record_outcome(true, None);
        }
        assert_eq!(table.recent().len(), 3);
        assert_eq!(table.clear_recent(), 3);
        assert!(table.recent().is_empty());
    }

    #[tokio::test]
    async fn cancel_fires_token_for_cancellable_kind() {
        let table = ActiveJobs::new();
        let guard = table.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "mod".to_string(),
            "/".to_string(),
        );
        let id = guard.transfer_id().to_string();
        let token = guard.cancellation_token().clone();
        assert!(!token.is_cancelled(), "fresh token must not be cancelled");

        assert_eq!(table.cancel(&id), CancelOutcome::Cancelled);
        assert!(token.is_cancelled(), "token must be observably cancelled");

        // Idempotent: a second cancel call still reports
        // Cancelled while the row is alive, even though the
        // token is already in the cancelled state.
        assert_eq!(table.cancel(&id), CancelOutcome::Cancelled);
    }

    #[tokio::test]
    async fn cancel_returns_unsupported_for_non_delegated_kinds() {
        // Push / pull / pull_sync register tokens for
        // consistent shape, but their handlers don't race the
        // token (CLI is in the byte path). `cancel` must
        // surface that policy as `Unsupported` rather than
        // firing the token and reporting `Cancelled`.
        let table = ActiveJobs::new();
        for kind in [
            ActiveJobKind::Push,
            ActiveJobKind::Pull,
            ActiveJobKind::PullSync,
        ] {
            let guard = table.register(kind, "p".to_string(), "m".to_string(), "/".to_string());
            let id = guard.transfer_id().to_string();
            let token = guard.cancellation_token().clone();
            assert_eq!(
                table.cancel(&id),
                CancelOutcome::Unsupported,
                "{} should not be cancellable today",
                kind.as_str()
            );
            assert!(
                !token.is_cancelled(),
                "{}: token must NOT have been fired for an unsupported kind",
                kind.as_str()
            );
            drop(guard);
        }
    }

    #[tokio::test]
    async fn cancel_returns_not_found_for_unknown_or_drained_id() {
        let table = ActiveJobs::new();
        assert_eq!(table.cancel("not-a-real-id"), CancelOutcome::NotFound);

        // After a guard drops, its id is no longer active.
        let id = {
            let guard = table.register(
                ActiveJobKind::DelegatedPull,
                "p".to_string(),
                "m".to_string(),
                "/".to_string(),
            );
            let id = guard.transfer_id().to_string();
            guard.record_outcome(true, None);
            drop(guard);
            id
        };
        assert_eq!(table.cancel(&id), CancelOutcome::NotFound);
    }

    #[tokio::test]
    async fn cancellation_token_wakes_awaiter() {
        // Handler-shape regression test: a future awaiting on
        // `guard.cancellation_token().cancelled()` must resolve
        // when `table.cancel(id)` is called from another task.
        let table = ActiveJobs::new();
        let guard = table.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        let id = guard.transfer_id().to_string();
        let token = guard.cancellation_token().clone();

        let waiter = tokio::spawn(async move {
            token.cancelled().await;
        });

        // Give the waiter a chance to actually park on the
        // cancelled() future before we fire it. The barrier
        // pattern from the concurrent-registers test is
        // overkill here; CancellationToken is well-behaved
        // and a yield is enough.
        tokio::task::yield_now().await;
        assert_eq!(table.cancel(&id), CancelOutcome::Cancelled);

        // The waiter resolves now that the token is cancelled.
        waiter.await.expect("waiter joined");
    }

    #[test]
    fn supports_cancellation_matches_dispatch_policy() {
        assert!(!ActiveJobKind::Push.supports_cancellation());
        assert!(!ActiveJobKind::Pull.supports_cancellation());
        assert!(!ActiveJobKind::PullSync.supports_cancellation());
        assert!(ActiveJobKind::DelegatedPull.supports_cancellation());
    }

    #[test]
    fn kind_strings_match_dispatch_site_names() {
        assert_eq!(ActiveJobKind::Push.as_str(), "push");
        assert_eq!(ActiveJobKind::Pull.as_str(), "pull");
        assert_eq!(ActiveJobKind::PullSync.as_str(), "pull_sync");
        assert_eq!(ActiveJobKind::DelegatedPull.as_str(), "delegated_pull");
    }

    #[tokio::test]
    async fn bytes_counter_starts_at_zero_and_reflects_reports() {
        let table = ActiveJobs::new();
        let guard = table.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        let snap = table.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].bytes_completed, 0);

        let sink = guard.bytes_counter();
        sink.report(1024);
        sink.report(2048);
        let snap = table.snapshot();
        assert_eq!(
            snap[0].bytes_completed, 3072,
            "snapshot must reflect both reports"
        );
    }

    #[tokio::test]
    async fn bytes_counter_clones_share_state() {
        // The data plane is welcome to clone the sink — every
        // clone hits the same atomic. A clone outliving the
        // guard's Drop is also fine: it just bumps an orphaned
        // counter, no row reappears.
        let table = ActiveJobs::new();
        let guard = table.register(
            ActiveJobKind::DelegatedPull,
            "p".to_string(),
            "m".to_string(),
            "/".to_string(),
        );
        let sink_a = guard.bytes_counter();
        let sink_b = sink_a.clone();
        let sink_c = guard.bytes_counter();
        sink_a.report(10);
        sink_b.report(20);
        sink_c.report(30);
        let snap = table.snapshot();
        assert_eq!(snap[0].bytes_completed, 60);
    }

    #[tokio::test]
    async fn drop_records_final_bytes_in_recent() {
        let table = ActiveJobs::new();
        {
            let guard = table.register(
                ActiveJobKind::DelegatedPull,
                "p".to_string(),
                "m".to_string(),
                "/".to_string(),
            );
            let sink = guard.bytes_counter();
            sink.report(5 * 1024 * 1024);
            guard.record_outcome(true, None);
        }
        let recent = table.recent();
        assert_eq!(recent.len(), 1);
        assert_eq!(
            recent[0].bytes,
            5 * 1024 * 1024,
            "recent record must carry final byte count"
        );
        assert!(recent[0].ok);
    }

    #[tokio::test]
    async fn report_after_drop_does_not_resurrect_row() {
        // A held sink whose guard has already dropped is a
        // benign no-op writer: the atomic is orphaned, the
        // table row is gone, and the next snapshot is still
        // empty.
        let table = ActiveJobs::new();
        let sink = {
            let guard = table.register(
                ActiveJobKind::DelegatedPull,
                "p".to_string(),
                "m".to_string(),
                "/".to_string(),
            );
            let sink = guard.bytes_counter();
            guard.record_outcome(true, None);
            sink
        };
        assert!(table.snapshot().is_empty(), "row drained on Drop");

        sink.report(999);
        assert!(
            table.snapshot().is_empty(),
            "post-Drop report must not re-insert"
        );
        // The TransferRecord captured at Drop reflects bytes
        // reported BEFORE the drop (zero here, since we
        // reported only after). The post-Drop report is lost
        // to consumers, which is the intended behavior — Drop
        // is the snapshot point for the ring entry.
        let recent = table.recent();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].bytes, 0);
    }
}
