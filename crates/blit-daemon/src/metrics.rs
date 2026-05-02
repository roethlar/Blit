//! Internal-only counters for daemon RPC dispatch.
//!
//! Counters increment at the gRPC handler boundary in `service/core.rs`.
//! Off by default; the daemon's `--metrics` flag turns collection on so
//! atomic adds in the hot path are skipped entirely when not needed.
//!
//! Semantics — `inc_*` counters are **attempts**, incremented at the
//! dispatch boundary regardless of whether the handler succeeds. The
//! separate `transfer_errors` counter increments when a handler returns
//! `Err`. `active_transfers` is a gauge tracked via `ActiveGuard`, an
//! RAII handle that decrements on `Drop` so panics or task cancellation
//! can't leak the gauge (F5 of `docs/reviews/codebase_review_2026-05-01.md`).
//!
//! No exposure mechanism (no HTTP, no RPC) yet. Counters exist so that
//! a future GUI/TUI gRPC `GetState`-style RPC can read them — design in
//! `docs/plan/TUI_DESIGN.md`. Until then this is internal scaffolding.

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct TransferMetrics {
    enabled: bool,
    pub push_operations: AtomicU64,
    pub pull_operations: AtomicU64,
    pub purge_operations: AtomicU64,
    pub active_transfers: AtomicU64,
    pub transfer_errors: AtomicU64,
}

impl TransferMetrics {
    /// Disabled collector. All `inc_*` calls are no-ops; atomics never
    /// touched. Default state — daemons without `--metrics` get this.
    pub fn disabled() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Enabled collector. `inc_*` and `dec_*` perform atomic ops.
    pub fn enabled() -> Arc<Self> {
        Arc::new(Self {
            enabled: true,
            ..Self::default()
        })
    }

    #[allow(dead_code)] // Read by future GUI/TUI exposure RPC.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[inline]
    pub fn inc_push(&self) {
        if self.enabled {
            self.push_operations.fetch_add(1, Relaxed);
        }
    }

    #[inline]
    pub fn inc_pull(&self) {
        if self.enabled {
            self.pull_operations.fetch_add(1, Relaxed);
        }
    }

    #[inline]
    pub fn inc_purge(&self) {
        if self.enabled {
            self.purge_operations.fetch_add(1, Relaxed);
        }
    }

    #[inline]
    pub fn inc_error(&self) {
        if self.enabled {
            self.transfer_errors.fetch_add(1, Relaxed);
        }
    }

    /// Increment the `active_transfers` gauge and return a guard that
    /// decrements on `Drop`. Move the guard into the spawned handler
    /// task so that panics, cancellation, and normal completion all
    /// release the gauge. This closes F5 from
    /// `docs/reviews/codebase_review_2026-05-01.md`: the previous
    /// inc/dec-around-await pattern leaked the gauge whenever the
    /// handler future was dropped before its `dec_active()` line ran.
    pub fn enter_transfer(self: Arc<Self>) -> ActiveGuard {
        if self.enabled {
            self.active_transfers.fetch_add(1, Relaxed);
        }
        ActiveGuard {
            metrics: Some(self),
        }
    }
}

/// RAII guard for `active_transfers`. Decrements on `Drop` so that
/// panic, cancellation, and normal completion of the host task all
/// release the gauge.
pub struct ActiveGuard {
    metrics: Option<Arc<TransferMetrics>>,
}

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        if let Some(metrics) = self.metrics.take() {
            if metrics.enabled {
                metrics.active_transfers.fetch_sub(1, Relaxed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_skips_increments() {
        let m = TransferMetrics::disabled();
        m.inc_push();
        m.inc_pull();
        let _g = Arc::clone(&m).enter_transfer();
        assert_eq!(m.push_operations.load(Relaxed), 0);
        assert_eq!(m.pull_operations.load(Relaxed), 0);
        assert_eq!(m.active_transfers.load(Relaxed), 0);
    }

    #[test]
    fn enabled_records_increments() {
        let m = TransferMetrics::enabled();
        m.inc_push();
        m.inc_pull();
        m.inc_pull();
        let g = Arc::clone(&m).enter_transfer();
        assert_eq!(m.active_transfers.load(Relaxed), 1);
        drop(g);
        m.inc_error();
        assert_eq!(m.push_operations.load(Relaxed), 1);
        assert_eq!(m.pull_operations.load(Relaxed), 2);
        assert_eq!(m.active_transfers.load(Relaxed), 0);
        assert_eq!(m.transfer_errors.load(Relaxed), 1);
    }

    #[test]
    fn active_guard_decrements_on_drop() {
        // RAII: dropping the guard decrements the gauge regardless
        // of whether the host scope completed normally.
        let m = TransferMetrics::enabled();
        {
            let _g = Arc::clone(&m).enter_transfer();
            assert_eq!(m.active_transfers.load(Relaxed), 1);
        }
        assert_eq!(m.active_transfers.load(Relaxed), 0);
    }

    #[test]
    fn active_guard_decrements_on_panic() {
        // The whole point of the RAII guard: a panic between
        // enter_transfer() and drop must still release the gauge.
        let m = TransferMetrics::enabled();
        let m_for_panic = Arc::clone(&m);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _g = m_for_panic.enter_transfer();
            assert_eq!(
                _g.metrics.as_ref().unwrap().active_transfers.load(Relaxed),
                1
            );
            panic!("simulated handler panic");
        }));
        assert!(result.is_err(), "panic should propagate");
        assert_eq!(
            m.active_transfers.load(Relaxed),
            0,
            "guard must release gauge even when the host scope panics"
        );
    }

    #[test]
    fn active_guard_handles_concurrent_transfers() {
        // Multiple guards stacked correctly.
        let m = TransferMetrics::enabled();
        let g1 = Arc::clone(&m).enter_transfer();
        let g2 = Arc::clone(&m).enter_transfer();
        let g3 = Arc::clone(&m).enter_transfer();
        assert_eq!(m.active_transfers.load(Relaxed), 3);
        drop(g2);
        assert_eq!(m.active_transfers.load(Relaxed), 2);
        drop(g1);
        drop(g3);
        assert_eq!(m.active_transfers.load(Relaxed), 0);
    }
}
