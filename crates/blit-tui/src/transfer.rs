//! F4 local transfer state. Operator types a Source +
//! Destination into the Verify form, then triggers a
//! copy (`C`) or mirror (`M`) — reuses the same form
//! fields so a workflow goes:
//!
//! 1. Tab → enter form
//! 2. Type source path → Tab → type destination
//! 3. Esc → leave editing
//! 4. `C` to copy, `M` to mirror
//!
//! d-4 R2: `M` doesn't run the mirror directly. It first
//! transitions to [`TransferStatus::ConfirmingMirror`] and
//! waits for `y` (run) or `n` (cancel) — same destructive-
//! operation guard the CLI applies in
//! `crates/blit-cli/src/transfers/mod.rs:181`. Mirror can
//! delete extraneous files at the destination, so a single
//! keystroke shouldn't be enough.
//!
//! The actual transfer runs via
//! `blit_app::transfers::local::run`, identical to the
//! CLI's `blit copy` / `blit mirror` code path.

use blit_core::orchestrator::LocalMirrorSummary;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferKind {
    Copy,
    Mirror,
}

impl TransferKind {
    pub fn label(self) -> &'static str {
        match self {
            TransferKind::Copy => "copy",
            TransferKind::Mirror => "mirror",
        }
    }
}

#[derive(Debug)]
pub enum TransferStatus {
    Idle,
    /// `M` was pressed but the mirror hasn't started.
    /// Waiting for `y` (run) or `n` (cancel). Mirror can
    /// delete files at the destination, so requiring an
    /// explicit confirm matches the CLI's
    /// `confirm_destructive_operation` prompt
    /// (`crates/blit-cli/src/transfers/mod.rs:181`).
    ConfirmingMirror,
    Running {
        kind: TransferKind,
    },
    Done {
        kind: TransferKind,
        summary: Box<LocalMirrorSummary>,
        #[allow(dead_code)]
        finished_at: Instant,
    },
    Error {
        kind: TransferKind,
        message: String,
    },
}

#[derive(Debug)]
pub struct TransferState {
    status: TransferStatus,
    request_id: u64,
}

impl Default for TransferState {
    fn default() -> Self {
        Self::new()
    }
}

impl TransferState {
    pub fn new() -> Self {
        Self {
            status: TransferStatus::Idle,
            request_id: 0,
        }
    }

    pub fn status(&self) -> &TransferStatus {
        &self.status
    }

    /// `true` while a transfer is in flight; callers use
    /// this to block a second trigger until the first
    /// completes (same in-flight guard pattern as the F2
    /// setup task).
    pub fn is_running(&self) -> bool {
        matches!(self.status, TransferStatus::Running { .. })
    }

    /// `1` while the F4 local transfer is in flight,
    /// `0` otherwise. Folded into the tab-strip "active"
    /// count so the operator sees F4-initiated transfers
    /// alongside daemon-stream transfers (e-2 round 2).
    pub fn count_active(&self) -> usize {
        usize::from(self.is_running())
    }

    /// `1` once the F4 local transfer has reached a
    /// terminal state (Done or Error), `0` otherwise. The
    /// F4 surface only retains one slot, so the count
    /// flips back to 0 the next time `M` / `C` kicks a
    /// fresh run (which transitions to `Running` /
    /// `ConfirmingMirror`).
    pub fn count_recent(&self) -> usize {
        usize::from(matches!(
            self.status,
            TransferStatus::Done { .. } | TransferStatus::Error { .. }
        ))
    }

    /// `true` while a mirror confirmation prompt is open.
    /// Used by the F4 dispatcher to route `y`/`n` to the
    /// confirm/cancel arms instead of the no-op default.
    pub fn is_confirming_mirror(&self) -> bool {
        matches!(self.status, TransferStatus::ConfirmingMirror)
    }

    /// `true` when a fresh trigger should be blocked
    /// (running OR awaiting confirmation). `can_start_transfer`
    /// in `main.rs` uses this to refuse a second `M` while
    /// the first confirmation is still on screen.
    pub fn is_busy(&self) -> bool {
        self.is_running() || self.is_confirming_mirror()
    }

    pub fn begin(&mut self, kind: TransferKind) -> u64 {
        self.request_id += 1;
        self.status = TransferStatus::Running { kind };
        self.request_id
    }

    /// Open the destructive-operation confirmation prompt
    /// for a mirror. No request_id bump — the actual run
    /// happens later via [`Self::begin`] if the operator
    /// confirms.
    pub fn begin_confirm_mirror(&mut self) {
        self.status = TransferStatus::ConfirmingMirror;
    }

    /// Drop a pending mirror confirmation back to Idle.
    /// Called when the operator presses `n`/Esc, or when
    /// they edit the Verify form (which would change the
    /// effective paths underneath the confirm prompt).
    /// Returns `true` if a confirmation was actually
    /// dismissed — lets callers know whether to surface a
    /// "cancelled" message.
    pub fn cancel_confirm(&mut self) -> bool {
        if matches!(self.status, TransferStatus::ConfirmingMirror) {
            self.status = TransferStatus::Idle;
            true
        } else {
            false
        }
    }

    /// Synchronously record a validation error (bad source
    /// path, remote endpoint, etc.) without spinning up a
    /// background task. Bumps the request_id so any prior
    /// in-flight reply drops harmlessly.
    pub fn note_validation_error(&mut self, kind: TransferKind, message: String) -> u64 {
        self.request_id += 1;
        self.status = TransferStatus::Error { kind, message };
        self.request_id
    }

    pub fn apply_done(
        &mut self,
        request_id: u64,
        kind: TransferKind,
        summary: LocalMirrorSummary,
    ) -> bool {
        if request_id != self.request_id {
            return false;
        }
        self.status = TransferStatus::Done {
            kind,
            summary: Box::new(summary),
            finished_at: Instant::now(),
        };
        true
    }

    pub fn apply_error(&mut self, request_id: u64, kind: TransferKind, message: String) -> bool {
        if request_id != self.request_id {
            return false;
        }
        self.status = TransferStatus::Error { kind, message };
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_summary() -> LocalMirrorSummary {
        LocalMirrorSummary::default()
    }

    #[test]
    fn new_is_idle_and_not_running() {
        let state = TransferState::new();
        assert!(matches!(state.status(), TransferStatus::Idle));
        assert!(!state.is_running());
    }

    #[test]
    fn begin_marks_running_with_kind() {
        let mut state = TransferState::new();
        let id = state.begin(TransferKind::Copy);
        assert_eq!(id, 1);
        assert!(state.is_running());
        match state.status() {
            TransferStatus::Running { kind } => assert_eq!(*kind, TransferKind::Copy),
            other => panic!("expected Running, got {other:?}"),
        }
    }

    #[test]
    fn apply_done_writes_when_current() {
        let mut state = TransferState::new();
        let id = state.begin(TransferKind::Mirror);
        let applied = state.apply_done(id, TransferKind::Mirror, empty_summary());
        assert!(applied);
        match state.status() {
            TransferStatus::Done { kind, .. } => assert_eq!(*kind, TransferKind::Mirror),
            other => panic!("expected Done, got {other:?}"),
        }
        assert!(!state.is_running());
    }

    #[test]
    fn apply_done_drops_stale_generation() {
        let mut state = TransferState::new();
        let id1 = state.begin(TransferKind::Copy);
        let _id2 = state.begin(TransferKind::Mirror);
        let applied = state.apply_done(id1, TransferKind::Copy, empty_summary());
        assert!(!applied);
        // Still Running with the second begin's kind.
        match state.status() {
            TransferStatus::Running { kind } => assert_eq!(*kind, TransferKind::Mirror),
            other => panic!("expected Running, got {other:?}"),
        }
    }

    #[test]
    fn apply_error_records_message() {
        let mut state = TransferState::new();
        let id = state.begin(TransferKind::Copy);
        let applied = state.apply_error(id, TransferKind::Copy, "perm denied".to_string());
        assert!(applied);
        match state.status() {
            TransferStatus::Error { kind, message } => {
                assert_eq!(*kind, TransferKind::Copy);
                assert_eq!(message, "perm denied");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn transfer_kind_label() {
        assert_eq!(TransferKind::Copy.label(), "copy");
        assert_eq!(TransferKind::Mirror.label(), "mirror");
    }

    // d-4 R2: mirror confirmation guard.

    #[test]
    fn begin_confirm_mirror_idles_to_confirming() {
        let mut state = TransferState::new();
        state.begin_confirm_mirror();
        assert!(state.is_confirming_mirror());
        assert!(state.is_busy());
        // Not Running — the actual transfer hasn't started yet.
        assert!(!state.is_running());
    }

    #[test]
    fn cancel_confirm_returns_to_idle() {
        let mut state = TransferState::new();
        state.begin_confirm_mirror();
        let dismissed = state.cancel_confirm();
        assert!(dismissed);
        assert!(matches!(state.status(), TransferStatus::Idle));
        assert!(!state.is_busy());
    }

    #[test]
    fn cancel_confirm_no_op_outside_confirming() {
        let mut state = TransferState::new();
        let dismissed = state.cancel_confirm();
        assert!(!dismissed);
        // begin() puts us in Running; cancel must not pull
        // the rug out from under an actual transfer.
        state.begin(TransferKind::Copy);
        let dismissed = state.cancel_confirm();
        assert!(!dismissed);
        assert!(state.is_running());
    }

    #[test]
    fn confirm_then_begin_transitions_to_running() {
        let mut state = TransferState::new();
        state.begin_confirm_mirror();
        let id = state.begin(TransferKind::Mirror);
        assert_eq!(id, 1, "first begin bumps gen from 0");
        assert!(state.is_running());
    }

    #[test]
    fn note_validation_error_bumps_gen_and_writes_error() {
        let mut state = TransferState::new();
        let id = state.note_validation_error(TransferKind::Mirror, "bad path".to_string());
        assert_eq!(id, 1);
        match state.status() {
            TransferStatus::Error { kind, message } => {
                assert_eq!(*kind, TransferKind::Mirror);
                assert_eq!(message, "bad path");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn note_validation_error_drops_stale_running_reply() {
        let mut state = TransferState::new();
        let stale_id = state.begin(TransferKind::Copy);
        // A new attempt fails validation synchronously.
        state.note_validation_error(TransferKind::Copy, "remote not supported".to_string());
        // The original async run's reply lands too late.
        let applied = state.apply_done(stale_id, TransferKind::Copy, empty_summary());
        assert!(!applied, "stale reply must drop now that gen has advanced");
        // Error banner stays visible.
        assert!(matches!(state.status(), TransferStatus::Error { .. }));
    }

    #[test]
    fn is_busy_covers_running_and_confirming() {
        let mut state = TransferState::new();
        assert!(!state.is_busy());
        state.begin_confirm_mirror();
        assert!(state.is_busy());
        state.cancel_confirm();
        assert!(!state.is_busy());
        state.begin(TransferKind::Copy);
        assert!(state.is_busy());
    }

    // e-2 round 2: tab-strip counts include F4 local transfers.

    #[test]
    fn count_active_is_one_while_running_zero_otherwise() {
        let mut state = TransferState::new();
        assert_eq!(state.count_active(), 0, "Idle → 0 active");
        state.begin_confirm_mirror();
        assert_eq!(state.count_active(), 0, "ConfirmingMirror → 0 active");
        state.cancel_confirm();
        state.begin(TransferKind::Copy);
        assert_eq!(state.count_active(), 1, "Running → 1 active");
    }

    #[test]
    fn count_recent_is_one_after_terminal_state() {
        let mut state = TransferState::new();
        assert_eq!(state.count_recent(), 0, "Idle → 0 recent");
        let id = state.begin(TransferKind::Copy);
        assert_eq!(state.count_recent(), 0, "Running → 0 recent");
        state.apply_done(id, TransferKind::Copy, empty_summary());
        assert_eq!(state.count_recent(), 1, "Done → 1 recent");
        // A new attempt erases the previous "recent" by
        // transitioning back to Running. The counter
        // models "currently visible terminal state in F4,"
        // not a history ring.
        state.begin(TransferKind::Mirror);
        assert_eq!(state.count_recent(), 0, "back to Running → 0 recent");
    }

    #[test]
    fn count_recent_counts_errors() {
        let mut state = TransferState::new();
        let id = state.begin(TransferKind::Copy);
        state.apply_error(id, TransferKind::Copy, "boom".to_string());
        assert_eq!(state.count_recent(), 1, "Error → 1 recent");
    }
}
