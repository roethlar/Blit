//! F4 Profile state — a thin wrapper around
//! `blit_app::profile::ProfileReport` plus a fetch-status
//! indicator. Backed by a local file (`perf_local.jsonl`)
//! and a predictor state file; no RPC.
//!
//! The pane also drives the design's lifecycle hotkeys:
//! [d] disable / [e] enable toggle perf-history recording,
//! and [c] clear wipes the on-disk log behind a y/N confirm
//! (d-66 — `confirming_clear`, since the wipe is permanent).

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
    /// d-66: `true` while a `[c] clear` keystroke is awaiting
    /// y/N confirmation. Clearing the history log is permanent
    /// (the records can't be recovered), so — like every other
    /// destructive TUI action (F2 cancel, F3 delete, F3/F1
    /// mirror·move) — it confirms before firing rather than
    /// wiping on a single keystroke.
    confirming_clear: bool,
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
            confirming_clear: false,
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

    /// d-66: arm the destructive-clear confirm. The next
    /// `y` runs the clear; `n`/`Esc` cancels.
    pub fn begin_clear_confirm(&mut self) {
        self.confirming_clear = true;
    }

    /// d-66: `true` while awaiting y/N for `[c] clear`.
    pub fn is_confirming_clear(&self) -> bool {
        self.confirming_clear
    }

    /// d-66: drop the pending clear confirm without clearing.
    pub fn cancel_clear_confirm(&mut self) {
        self.confirming_clear = false;
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
    fn clear_confirm_lifecycle() {
        let mut state = ProfileState::new();
        assert!(!state.is_confirming_clear(), "starts disarmed");
        state.begin_clear_confirm();
        assert!(state.is_confirming_clear(), "armed after begin");
        state.cancel_clear_confirm();
        assert!(!state.is_confirming_clear(), "disarmed after cancel");
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
