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
//! On-screen output (§3.1 / D5): when `--metrics` is on, the daemon
//! emits a one-line summary to stderr at each RPC completion — like
//! `rclone --stats` or rsync's `--stats`, but per-transfer from the
//! daemon side. Operator running the daemon under systemd / in a
//! foreground terminal gets visible feedback without needing the TUI.

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Arc;
use std::time::Duration;

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

    /// §3.1 / D5: emit a one-line summary to stderr at RPC completion
    /// when `--metrics` is enabled. Operator-facing visibility for
    /// daemon foreground / systemd journal use. No-op when metrics
    /// are disabled — atomics aren't even loaded.
    ///
    /// Format is intentionally compact (one line, structured key=val
    /// pairs) so a log aggregator can parse it without regex
    /// gymnastics. The format string is exposed via
    /// `format_completion_line` for unit tests.
    pub fn log_completion(&self, op_kind: &str, duration: Duration, ok: bool) {
        if !self.enabled {
            return;
        }
        let line = self.format_completion_line(op_kind, duration, ok);
        eprintln!("{line}");
    }

    /// Extracted formatter so the format contract is unit-testable
    /// without capturing stderr. Reads atomics with the same `Relaxed`
    /// ordering the inc_* writers use — counters are observability,
    /// not synchronization.
    pub fn format_completion_line(&self, op_kind: &str, duration: Duration, ok: bool) -> String {
        let push = self.push_operations.load(Relaxed);
        let pull = self.pull_operations.load(Relaxed);
        let purge = self.purge_operations.load(Relaxed);
        let active = self.active_transfers.load(Relaxed);
        let errors = self.transfer_errors.load(Relaxed);
        let status = if ok { "ok" } else { "err" };
        format!(
            "[metrics] {op_kind} {status} in {duration:.2?} \
             (push_ops={push} pull_ops={pull} purge_ops={purge} \
             active={active} errors={errors})"
        )
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

    /// §3.1 / D5: format_completion_line must include the op kind,
    /// status, duration, and every cumulative counter. The format
    /// is intentionally compact one-line key=value pairs so a log
    /// aggregator can parse without regex gymnastics.
    #[test]
    fn completion_line_format_is_stable() {
        let m = TransferMetrics::enabled();
        m.inc_push();
        m.inc_pull();
        m.inc_pull();
        m.inc_purge();
        m.inc_error();
        let line = m.format_completion_line("push", Duration::from_millis(1234), true);
        assert!(line.starts_with("[metrics] push ok in "), "{line}");
        assert!(line.contains("push_ops=1"), "{line}");
        assert!(line.contains("pull_ops=2"), "{line}");
        assert!(line.contains("purge_ops=1"), "{line}");
        assert!(line.contains("active=0"), "{line}");
        assert!(line.contains("errors=1"), "{line}");
    }

    /// §3.1 / D5: failing handlers tag the line `err` so an operator
    /// `grep err` over the daemon's stderr surfaces problem RPCs
    /// without parsing the counter block.
    #[test]
    fn completion_line_marks_errors_explicitly() {
        let m = TransferMetrics::enabled();
        let line = m.format_completion_line("pull_sync", Duration::from_secs(0), false);
        assert!(line.contains("pull_sync err"), "{line}");
    }

    /// `--metrics` off is the default; `log_completion` and the
    /// formatter must skip atomics entirely. Disabled-state line is
    /// never expected to be emitted (gated by `enabled` in
    /// `log_completion`); the formatter still runs in test for
    /// shape parity but reads zeros across the board.
    #[test]
    fn disabled_completion_line_reads_zero_counters() {
        let m = TransferMetrics::disabled();
        // Pretend something tried to push — disabled state must drop the inc.
        m.inc_push();
        m.inc_error();
        let line = m.format_completion_line("push", Duration::from_millis(10), true);
        assert!(
            line.contains("push_ops=0") && line.contains("errors=0"),
            "{line}"
        );
    }

    /// §3.1 followup: the active-transfer gauge in the completion
    /// line must reflect state AFTER the just-finished RPC is
    /// removed. Call-site contract is "drop the ActiveGuard before
    /// `log_completion`". This test asserts the formatter's
    /// behavior matches the contract: with the guard dropped,
    /// `active` reads 0 for a single completed transfer (and N-1
    /// for N concurrent transfers where one just finished).
    #[test]
    fn completion_line_reflects_active_after_guard_drop() {
        let m = TransferMetrics::enabled();
        // Simulate one in-flight + one finishing.
        let _other = Arc::clone(&m).enter_transfer();
        let finishing = Arc::clone(&m).enter_transfer();
        assert_eq!(m.active_transfers.load(Relaxed), 2);
        // Call-site contract: drop the guard before logging.
        drop(finishing);
        let line = m.format_completion_line("push", Duration::from_millis(1), true);
        assert!(
            line.contains("active=1"),
            "expected active=1 (one transfer still in flight); got {line}"
        );
    }
}
