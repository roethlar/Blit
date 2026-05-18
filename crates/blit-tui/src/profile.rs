//! F4 Profile state — a thin wrapper around
//! `blit_app::profile::ProfileReport` plus a fetch-status
//! indicator. Backed by a local file (`perf_local.jsonl`)
//! and a predictor state file; no RPC.
//!
//! Atomic scope for a1-5: read-only display. The design
//! also calls for [c] clear / [d] disable / [e] enable
//! hotkeys; those are stateful and land in a future slice
//! alongside the Verify and Diagnostics F4 sub-blocks.

use blit_app::profile::ProfileReport;
use std::time::Instant;

/// Fetch status for the local profile read. Mirrors the
/// shape used in F1 / F3 so the operator's eye finds the
/// same banner conventions.
#[derive(Debug, Clone)]
pub enum ProfileFetchStatus {
    /// Initial state — no read attempted yet.
    Idle,
    /// File-read in flight (we use `tokio::task::spawn_blocking`
    /// because `profile::query` is sync).
    Pending,
    /// Last read returned `Ok`. `fetched_at` lets the
    /// renderer show "as of Xs ago".
    Loaded { fetched_at: Instant },
    /// Last read failed; carry the message for diagnostics.
    Error { message: String },
}

#[derive(Debug, Clone)]
pub struct ProfileState {
    report: Option<ProfileReport>,
    status: ProfileFetchStatus,
    /// Monotonically increasing generation, bumped on each
    /// `begin_fetch`. Same dedup pattern as `BrowseState` —
    /// stale replies from a prior fetch are ignored.
    pending_request_id: u64,
}

impl Default for ProfileState {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileState {
    pub fn new() -> Self {
        Self {
            report: None,
            status: ProfileFetchStatus::Idle,
            pending_request_id: 0,
        }
    }

    pub fn report(&self) -> Option<&ProfileReport> {
        self.report.as_ref()
    }

    pub fn status(&self) -> &ProfileFetchStatus {
        &self.status
    }

    /// Bump the request id, flip to Pending, return the id.
    pub fn begin_fetch(&mut self) -> u64 {
        self.pending_request_id += 1;
        self.status = ProfileFetchStatus::Pending;
        self.pending_request_id
    }

    /// True iff the request_id is current.
    pub fn is_current_request(&self, request_id: u64) -> bool {
        request_id == self.pending_request_id
    }

    /// Apply a freshly-read report. Caller has already
    /// dropped stale generations via `is_current_request`.
    pub fn apply_report(&mut self, report: ProfileReport, fetched_at: Instant) {
        self.report = Some(report);
        self.status = ProfileFetchStatus::Loaded { fetched_at };
    }

    /// Note a read failure. Keeps the previous `report`
    /// visible (operator still sees the last good snapshot)
    /// and surfaces the error in the footer.
    pub fn note_fetch_error(&mut self, message: String) {
        self.status = ProfileFetchStatus::Error { message };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_app::profile::ProfileReport;

    fn empty_report() -> ProfileReport {
        ProfileReport {
            enabled: true,
            records: Vec::new(),
            predictor_path: None,
            predictor: None,
        }
    }

    #[test]
    fn new_starts_idle() {
        let state = ProfileState::new();
        assert!(matches!(state.status(), ProfileFetchStatus::Idle));
        assert!(state.report().is_none());
    }

    #[test]
    fn begin_fetch_increments_request_id() {
        let mut state = ProfileState::new();
        let id1 = state.begin_fetch();
        assert_eq!(id1, 1);
        assert!(matches!(state.status(), ProfileFetchStatus::Pending));
        assert!(state.is_current_request(1));
        let id2 = state.begin_fetch();
        assert_eq!(id2, 2);
        assert!(!state.is_current_request(1));
        assert!(state.is_current_request(2));
    }

    #[test]
    fn apply_report_sets_loaded_and_stores_report() {
        let mut state = ProfileState::new();
        state.begin_fetch();
        state.apply_report(empty_report(), Instant::now());
        assert!(matches!(state.status(), ProfileFetchStatus::Loaded { .. }));
        assert!(state.report().is_some());
    }

    #[test]
    fn note_fetch_error_preserves_prior_report() {
        let mut state = ProfileState::new();
        state.begin_fetch();
        state.apply_report(empty_report(), Instant::now());
        state.note_fetch_error("permission denied".to_string());
        match state.status() {
            ProfileFetchStatus::Error { message } => {
                assert_eq!(message, "permission denied");
            }
            other => panic!("expected Error, got {other:?}"),
        }
        // Prior report still visible.
        assert!(state.report().is_some());
    }
}
