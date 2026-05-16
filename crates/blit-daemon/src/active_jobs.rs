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
//! - Byte-level progress (`bytes_completed` / `bytes_total`
//!   on active rows; `bytes` / `files` on records) —
//!   milestone C extends rows from the write-loop
//!   instrumentation.
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

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_util::sync::CancellationToken;

/// Default depth of the recent-runs ring buffer. Mirrors the
/// `GetStateRequest.recent_limit = 0 → 50` default the design
/// doc (§6.3) calls for. Carried as a constant here so b-3
/// can land before the proto types.
pub const DEFAULT_RECENT_LIMIT: usize = 50;

/// What kind of transfer a row represents. Mirrors the
/// dispatch sites in `service/core.rs`. When milestone C
/// introduces the `TransferStarted.Kind` wire enum, the
/// conversion will live in the GetState handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
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

/// In-memory registry, shared between the dispatch boundary and
/// future `GetState` reads.
#[derive(Clone)]
pub struct ActiveJobs {
    inner: Arc<Inner>,
}

struct Inner {
    table: Mutex<HashMap<String, ActiveJob>>,
    /// Per-row cancellation tokens, keyed by transfer_id.
    /// Mutated in lockstep with `table`: register inserts
    /// both, Drop removes both. `cancel(id)` looks up the
    /// token and fires it; handlers race against it.
    ///
    /// Kept as a parallel map rather than embedded in
    /// `ActiveJob` because `ActiveJob` is the snapshot row
    /// returned over the wire and the token isn't a
    /// user-visible field.
    cancellations: Mutex<HashMap<String, CancellationToken>>,
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
                cancellations: Mutex::new(HashMap::new()),
                recent: Mutex::new(VecDeque::with_capacity(limit)),
                recent_limit: limit,
                counter: AtomicU64::new(0),
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
        };
        let cancellation = CancellationToken::new();
        self.inner
            .table
            .lock()
            .expect("active_jobs table poisoned")
            .insert(transfer_id.clone(), row);
        self.inner
            .cancellations
            .lock()
            .expect("active_jobs cancellations poisoned")
            .insert(transfer_id.clone(), cancellation.clone());
        ActiveJobGuard {
            inner: Arc::clone(&self.inner),
            transfer_id,
            outcome: Mutex::new(None),
            cancellation,
        }
    }

    /// Fire the cancellation token of an active row, if it
    /// exists. Returns `true` if a matching row was found and
    /// its token was fired (whether or not the handler was
    /// listening yet); `false` if the transfer_id wasn't
    /// active. `CancelJob` (m-jobs-2) calls this from the
    /// gRPC handler.
    ///
    /// Idempotent: firing an already-cancelled token is a
    /// no-op. The token stays in the map until the guard
    /// drops; a second call against the same id while the
    /// transfer is still draining returns `true` again.
    #[allow(dead_code)]
    pub fn cancel(&self, transfer_id: &str) -> bool {
        let guard = self
            .inner
            .cancellations
            .lock()
            .expect("active_jobs cancellations poisoned");
        match guard.get(transfer_id) {
            Some(token) => {
                token.cancel();
                true
            }
            None => false,
        }
    }

    /// Snapshot of every active row. Used by tests in this
    /// slice; will be used by `GetState.active[]` once the
    /// RPC handler lands in a later sub-slice.
    #[allow(dead_code)]
    pub fn snapshot(&self) -> Vec<ActiveJob> {
        self.inner
            .table
            .lock()
            .expect("active_jobs table poisoned")
            .values()
            .cloned()
            .collect()
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
        if let Some(row) = table.get_mut(&self.transfer_id) {
            row.module = module;
            row.path = path;
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
        // active row, the cancellations map, and the ring are
        // all updated even if a panic poisoned a mutex on the
        // way in. This matches the rest of the codebase's
        // stance on poisoning — surface the failure, but
        // don't leak state.
        //
        // Lock order: table → cancellations → recent. Held
        // sequentially (no nested acquisitions). `cancel(id)`
        // takes only the cancellations lock, so it can't
        // deadlock against this Drop path.
        let id = std::mem::take(&mut self.transfer_id);
        let outcome = {
            let mut cell = self.outcome.lock().unwrap_or_else(|e| e.into_inner());
            cell.take()
        };
        let row = {
            let mut table = self.inner.table.lock().unwrap_or_else(|e| e.into_inner());
            table.remove(&id)
        };
        {
            let mut cancellations = self
                .inner
                .cancellations
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            cancellations.remove(&id);
        }
        if let Some(row) = row {
            if self.inner.recent_limit > 0 {
                let record = build_record(row, outcome);
                push_recent(&self.inner.recent, record, self.inner.recent_limit);
            }
        }
    }
}

fn build_record(row: ActiveJob, outcome: Option<RecordedOutcome>) -> TransferRecord {
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

    #[tokio::test]
    async fn cancel_fires_token_for_known_transfer_id() {
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

        let fired = table.cancel(&id);
        assert!(fired, "cancel must return true for an active row");
        assert!(token.is_cancelled(), "token must be observably cancelled");

        // Idempotent: a second cancel call returns true while
        // the row is still alive, even though the token is
        // already cancelled.
        assert!(table.cancel(&id));
    }

    #[tokio::test]
    async fn cancel_returns_false_for_unknown_transfer_id() {
        let table = ActiveJobs::new();
        assert!(!table.cancel("not-a-real-id"));

        // After a guard drops, its id should no longer cancel.
        let id = {
            let guard = table.register(
                ActiveJobKind::Pull,
                "p".to_string(),
                "m".to_string(),
                "/".to_string(),
            );
            let id = guard.transfer_id().to_string();
            guard.record_outcome(true, None);
            drop(guard);
            id
        };
        assert!(
            !table.cancel(&id),
            "cancel must return false for a drained row"
        );
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
        assert!(table.cancel(&id));

        // The waiter resolves now that the token is cancelled.
        waiter.await.expect("waiter joined");
    }

    #[test]
    fn kind_strings_match_dispatch_site_names() {
        assert_eq!(ActiveJobKind::Push.as_str(), "push");
        assert_eq!(ActiveJobKind::Pull.as_str(), "pull");
        assert_eq!(ActiveJobKind::PullSync.as_str(), "pull_sync");
        assert_eq!(ActiveJobKind::DelegatedPull.as_str(), "delegated_pull");
    }
}
