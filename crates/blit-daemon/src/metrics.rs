//! Internal-only counters for daemon RPC dispatch.
//!
//! Counters increment at the gRPC handler boundary in `service/core.rs`.
//! Off by default; the daemon's `--metrics` flag turns collection on so
//! atomic adds in the hot path are skipped entirely when not needed.
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
    pub fn inc_active(&self) {
        if self.enabled {
            self.active_transfers.fetch_add(1, Relaxed);
        }
    }

    #[inline]
    pub fn dec_active(&self) {
        if self.enabled {
            self.active_transfers.fetch_sub(1, Relaxed);
        }
    }

    #[inline]
    pub fn inc_error(&self) {
        if self.enabled {
            self.transfer_errors.fetch_add(1, Relaxed);
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
        m.inc_active();
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
        m.inc_active();
        m.dec_active();
        m.inc_error();
        assert_eq!(m.push_operations.load(Relaxed), 1);
        assert_eq!(m.pull_operations.load(Relaxed), 2);
        assert_eq!(m.active_transfers.load(Relaxed), 0);
        assert_eq!(m.transfer_errors.load(Relaxed), 1);
    }
}
