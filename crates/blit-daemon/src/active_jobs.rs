//! In-memory registry of in-flight transfers on the daemon.
//!
//! Milestone B of `docs/plan/TUI_DESIGN.md` §6.3: the always-on
//! `ActiveJobs` table that `GetState.active[]` will read from
//! once the RPC lands in a later sub-slice. Populated at the
//! dispatch boundary in `service/core.rs`; rows are drained on
//! RPC completion via the RAII guard returned by [`register`].
//!
//! Scope of this slice (`b-1-active-jobs`):
//!
//! - Table struct + `ActiveJob` row + `ActiveJobKind`.
//! - `register(kind, peer, module, path) -> ActiveJobGuard`
//!   inserts a row and returns a guard whose `Drop` removes it
//!   synchronously.
//! - `snapshot()` for tests (and the future `GetState`).
//! - Wiring at the two RPC dispatch sites where module + path
//!   are known synchronously: `pull` and `delegated_pull`.
//!
//! Out of scope (next sub-slice `b-2`):
//!
//! - Streaming RPCs (`push`, `pull_sync`) — their module + path
//!   arrive in the first stream frame, not in the request
//!   metadata. Filling those rows needs a handler-side
//!   `guard.set_endpoint(...)` update path; deferred so the
//!   register/drain plumbing can be reviewed independently
//!   from the streaming-init plumbing.
//! - Recent-runs ring buffer (drains-out side).
//! - `GetState` RPC reading from this table.
//! - `CancelJob` plumbing (M-Jobs adds the `CancellationToken`
//!   field on each row).
//! - Byte-level progress (`bytes_completed`/`bytes_total`) —
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

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// What kind of transfer a row represents. Mirrors the
/// dispatch sites in `service/core.rs`. When milestone C
/// introduces the `TransferStarted.Kind` wire enum, the
/// conversion will live in the GetState handler.
///
/// `Push` and `PullSync` variants are defined here but not
/// yet constructed at any dispatch site — those are
/// streaming RPCs whose module + path arrive in the first
/// stream frame, and the guard update path needed to fill
/// the row asynchronously lands in b-2. The variants exist
/// now so the table's wire shape doesn't change between
/// slices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
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

/// In-memory registry, shared between the dispatch boundary and
/// future `GetState` reads.
#[derive(Clone)]
pub struct ActiveJobs {
    inner: Arc<Inner>,
}

struct Inner {
    table: Mutex<HashMap<String, ActiveJob>>,
    /// Monotonic counter feeding [`mint_transfer_id`]. Keeps ids
    /// unique within a single millisecond when multiple
    /// transfers register at the same instant.
    counter: AtomicU64,
}

impl ActiveJobs {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                table: Mutex::new(HashMap::new()),
                counter: AtomicU64::new(0),
            }),
        }
    }

    /// Insert a row and return a guard that removes it on drop.
    ///
    /// Module + path are eagerly required because the
    /// dispatch sites this slice wires (`pull`,
    /// `delegated_pull`) have them synchronously available.
    /// Streaming RPCs (`push`, `pull_sync`) — where module
    /// arrives in the first stream frame — are deferred to
    /// b-2, which will add a guard update API.
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
        self.inner
            .table
            .lock()
            .expect("active_jobs table poisoned")
            .insert(transfer_id.clone(), row);
        ActiveJobGuard {
            inner: Arc::clone(&self.inner),
            transfer_id,
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
}

impl Default for ActiveJobs {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard tying an `ActiveJob` row's lifetime to the
/// dispatcher's spawned task. Drop removes the row whether the
/// task completed, errored, or was cancelled — same posture as
/// the metrics active-transfers gauge.
///
/// Drop is **synchronous and deterministic**: after the guard
/// is dropped, the row is gone. This is the contract
/// `GetState.active[]` relies on.
pub struct ActiveJobGuard {
    inner: Arc<Inner>,
    transfer_id: String,
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
}

impl Drop for ActiveJobGuard {
    fn drop(&mut self) {
        // Synchronous removal. PoisonError still hands us the
        // inner guard via `into_inner`, so the row is drained
        // even if a panic poisoned the mutex on the way in.
        // This matches the rest of the codebase's stance on
        // poisoning — surface the failure, but don't leak
        // state.
        let id = std::mem::take(&mut self.transfer_id);
        let mut table = self.inner.table.lock().unwrap_or_else(|e| e.into_inner());
        table.remove(&id);
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

    #[test]
    fn kind_strings_match_dispatch_site_names() {
        assert_eq!(ActiveJobKind::Push.as_str(), "push");
        assert_eq!(ActiveJobKind::Pull.as_str(), "pull");
        assert_eq!(ActiveJobKind::PullSync.as_str(), "pull_sync");
        assert_eq!(ActiveJobKind::DelegatedPull.as_str(), "delegated_pull");
    }
}
