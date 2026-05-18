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

    pub fn begin(&mut self, kind: TransferKind) -> u64 {
        self.request_id += 1;
        self.status = TransferStatus::Running { kind };
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
}
