//! F1 push state — the TUI-owned lifecycle for a local→remote
//! push triggered from the F1 trigger modal (d-61).
//!
//! The F1 trigger (d-58…d-60) handles remote→local transfers by
//! delegating to the F3 pull machine. The opposite direction —
//! pushing a *local* path to a remote daemon — has no F3
//! equivalent (F3 is a remote browser), so its lifecycle lives
//! here, mirroring the F3 pull/delete shape: the dispatcher
//! spawns `run_remote_push` on a task, the task replies, and the
//! event loop applies the terminal state (generation-guarded by
//! `request_id` so a stale reply from a superseded run is
//! dropped). There's no live byte progress in this first slice —
//! just Running → Done / Error, shown in the F1 footer.
//!
//! Scope: **copy** push only. Mirror push (server-side
//! delete-extraneous) and move push (delete the local source
//! after) are follow-ups — they need their own confirm gates.

/// Lifecycle of an F1 push.
#[derive(Debug, Clone)]
pub enum F1PushStatus {
    /// No push in progress / shown.
    Idle,
    /// Push RPC in flight. `label` is the remote destination
    /// shown in the footer.
    Running { request_id: u64, label: String },
    /// Push finished — files + bytes sent, dest label. (No
    /// auto-hide TTL in this slice; the terminal status persists
    /// until the next push begins — a follow-up could add one.)
    Done {
        files: u64,
        bytes: u64,
        label: String,
    },
    /// Push failed (validation or transport).
    Error { message: String },
}

#[derive(Debug, Clone)]
pub struct F1PushState {
    status: F1PushStatus,
    /// Monotonic per-run id — each `begin` bumps it and stamps the
    /// new value into `Running`; the reply arm drops replies whose
    /// id doesn't match (same generation-guard as the F3 machines).
    request_seq: u64,
}

impl Default for F1PushState {
    fn default() -> Self {
        Self::new()
    }
}

impl F1PushState {
    pub fn new() -> Self {
        Self {
            status: F1PushStatus::Idle,
            request_seq: 0,
        }
    }

    pub fn status(&self) -> &F1PushStatus {
        &self.status
    }

    /// `true` while a push RPC is in flight. The dispatcher blocks
    /// a second push while running.
    pub fn is_running(&self) -> bool {
        matches!(self.status, F1PushStatus::Running { .. })
    }

    /// Begin a push to `label`. Bumps the run id, transitions to
    /// `Running`, and returns the new `request_id` for the spawned
    /// task. No-op (`None`) if a push is already running.
    pub fn begin(&mut self, label: String) -> Option<u64> {
        if self.is_running() {
            return None;
        }
        self.request_seq += 1;
        let request_id = self.request_seq;
        self.status = F1PushStatus::Running { request_id, label };
        Some(request_id)
    }

    /// Apply a successful push reply. Dropped (returns false) if
    /// `request_id` doesn't match the current `Running` run.
    pub fn apply_done(&mut self, request_id: u64, files: u64, bytes: u64) -> bool {
        match &self.status {
            F1PushStatus::Running {
                request_id: rid,
                label,
            } if *rid == request_id => {
                self.status = F1PushStatus::Done {
                    files,
                    bytes,
                    label: label.clone(),
                };
                true
            }
            _ => false,
        }
    }

    /// Apply a failed push reply. Same generation guard.
    pub fn apply_error(&mut self, request_id: u64, message: String) -> bool {
        match &self.status {
            F1PushStatus::Running {
                request_id: rid, ..
            } if *rid == request_id => {
                self.status = F1PushStatus::Error { message };
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_idle() {
        let s = F1PushState::new();
        assert!(matches!(s.status(), F1PushStatus::Idle));
        assert!(!s.is_running());
    }

    #[test]
    fn begin_transitions_to_running_with_label() {
        let mut s = F1PushState::new();
        let rid = s.begin("nas:9031".to_string()).expect("started");
        assert_eq!(rid, 1);
        assert!(s.is_running());
        match s.status() {
            F1PushStatus::Running { request_id, label } => {
                assert_eq!(*request_id, 1);
                assert_eq!(label, "nas:9031");
            }
            other => panic!("expected Running, got {other:?}"),
        }
    }

    #[test]
    fn begin_is_noop_while_running() {
        let mut s = F1PushState::new();
        s.begin("a".to_string()).expect("first");
        assert!(s.begin("b".to_string()).is_none(), "second push blocked");
        assert!(s.is_running());
    }

    #[test]
    fn apply_done_records_terminal_state() {
        let mut s = F1PushState::new();
        let rid = s.begin("nas:9031".to_string()).expect("started");
        assert!(s.apply_done(rid, 12, 4096));
        match s.status() {
            F1PushStatus::Done {
                files,
                bytes,
                label,
                ..
            } => {
                assert_eq!(*files, 12);
                assert_eq!(*bytes, 4096);
                assert_eq!(label, "nas:9031");
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn apply_error_records_message() {
        let mut s = F1PushState::new();
        let rid = s.begin("nas:9031".to_string()).expect("started");
        assert!(s.apply_error(rid, "connection refused".to_string()));
        match s.status() {
            F1PushStatus::Error { message, .. } => assert_eq!(message, "connection refused"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn stale_reply_is_dropped() {
        let mut s = F1PushState::new();
        let rid = s.begin("nas:9031".to_string()).expect("started");
        // A reply for a superseded id must not clobber Running.
        assert!(!s.apply_done(rid + 99, 1, 1));
        assert!(s.is_running());
        assert!(!s.apply_error(rid + 99, "x".to_string()));
        assert!(s.is_running());
    }

    #[test]
    fn run_ids_increment_so_a_new_push_supersedes() {
        let mut s = F1PushState::new();
        let first = s.begin("a".to_string()).expect("first");
        s.apply_done(first, 0, 0);
        let second = s.begin("b".to_string()).expect("second");
        assert!(second > first, "ids monotonic → stale replies dropped");
    }
}
