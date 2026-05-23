//! F1 push state — the TUI-owned lifecycle for a local→remote
//! push triggered from the F1 trigger modal (d-61).
//!
//! The F1 trigger (d-58…d-60) handles remote→local transfers by
//! delegating to the F3 pull machine. The opposite direction —
//! pushing a *local* path to a remote daemon — has no F3
//! equivalent (F3 is a remote browser), so its lifecycle lives
//! here, mirroring the F3 pull/delete shape: the dispatcher
//! spawns `run_remote_push` on a task and the event loop applies
//! the terminal state (generation-guarded by `request_id` so a
//! stale reply from a superseded run is dropped). d-63: a progress
//! forwarder feeds live `files` / `bytes` / `bytes_per_sec`
//! counters into `Running` (via [`F1PushState::apply_progress`])
//! while the push runs; the authoritative totals still ride the
//! terminal reply. The whole lifecycle —
//! Running (with live counters) → Done / Error — shows in the F1
//! footer.
//!
//! d-65: the push `kind` (copy / mirror / move) drives the footer
//! verb. Mirror (server-side delete-extraneous at the dest) and
//! move (delete the local source after a successful push) are
//! destructive, so the trigger gates them behind a y/N confirm
//! before calling `begin`; the state machine itself is
//! kind-agnostic beyond the verb.
//!
//! d-68: this lifecycle is also reused for a remote→remote
//! *delegated* copy triggered from F1 (the destination daemon
//! pulls from the source daemon). It enters via `begin_delegated`
//! and carries `delegated: true` so the footer reads
//! "delegating / delegated" rather than "pushing / pushed" — the
//! CLI host is not in the byte path. Delegated copy ships without
//! live byte progress for now (the daemon reports via the pull
//! data-plane, not the push path); the terminal summary still
//! shows. Mirror/move and detached (F2-visible) delegation are
//! follow-ups.

use crate::f3pull::PullKind;
use std::time::{Duration, Instant};

/// Lifecycle of an F1 push.
#[derive(Debug, Clone)]
pub enum F1PushStatus {
    /// No push in progress / shown.
    Idle,
    /// Push RPC in flight. `label` is the remote destination
    /// shown in the footer. d-63: `files` / `bytes` are live
    /// cumulative counters fed by the progress forwarder (0 until
    /// the first event); `bytes_per_sec` is the average throughput
    /// (0 until ~1s elapsed, matching the F3 pull footer). d-65:
    /// `kind` (copy / mirror / move) drives the footer verb.
    Running {
        request_id: u64,
        label: String,
        files: u64,
        bytes: u64,
        bytes_per_sec: u64,
        kind: PullKind,
        /// d-68: `true` for a remote→remote delegated copy (drives
        /// the "delegating/delegated" verb instead of push verbs).
        delegated: bool,
    },
    /// Push finished — files + bytes sent, dest label. d-64:
    /// `finished_at` drives the auto-hide TTL.
    Done {
        files: u64,
        bytes: u64,
        label: String,
        finished_at: Instant,
        kind: PullKind,
        delegated: bool,
    },
    /// Push failed (validation or transport). d-64: `finished_at`
    /// drives the auto-hide TTL.
    Error {
        message: String,
        finished_at: Instant,
        kind: PullKind,
        delegated: bool,
    },
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

    /// Begin a push to `label` of the given `kind`. Bumps the run
    /// id, transitions to `Running`, and returns the new
    /// `request_id` for the spawned task. No-op (`None`) if a push
    /// is already running.
    pub fn begin(&mut self, label: String, kind: PullKind) -> Option<u64> {
        self.begin_inner(label, kind, false)
    }

    /// d-68: begin a remote→remote *delegated* copy (the destination
    /// daemon pulls from the source). Same lifecycle as a push, but
    /// `delegated: true` swaps the footer verb. Copy-only for now, so
    /// no `kind` parameter.
    pub fn begin_delegated(&mut self, label: String) -> Option<u64> {
        self.begin_inner(label, PullKind::Copy, true)
    }

    fn begin_inner(&mut self, label: String, kind: PullKind, delegated: bool) -> Option<u64> {
        if self.is_running() {
            return None;
        }
        self.request_seq += 1;
        let request_id = self.request_seq;
        self.status = F1PushStatus::Running {
            request_id,
            label,
            files: 0,
            bytes: 0,
            bytes_per_sec: 0,
            kind,
            delegated,
        };
        Some(request_id)
    }

    /// d-63: apply a live progress snapshot. Updates the `Running`
    /// counters in place; generation-guarded so a snapshot from a
    /// superseded run is dropped.
    pub fn apply_progress(&mut self, request_id: u64, files: u64, bytes: u64, bytes_per_sec: u64) {
        if let F1PushStatus::Running {
            request_id: rid,
            files: f,
            bytes: b,
            bytes_per_sec: bps,
            ..
        } = &mut self.status
        {
            if *rid == request_id {
                *f = files;
                *b = bytes;
                *bps = bytes_per_sec;
            }
        }
    }

    /// Apply a successful push reply. Dropped (returns false) if
    /// `request_id` doesn't match the current `Running` run. `at`
    /// stamps the d-64 auto-hide deadline.
    pub fn apply_done(&mut self, request_id: u64, files: u64, bytes: u64, at: Instant) -> bool {
        match &self.status {
            F1PushStatus::Running {
                request_id: rid,
                label,
                kind,
                delegated,
                ..
            } if *rid == request_id => {
                self.status = F1PushStatus::Done {
                    files,
                    bytes,
                    label: label.clone(),
                    finished_at: at,
                    kind: *kind,
                    delegated: *delegated,
                };
                true
            }
            _ => false,
        }
    }

    /// Apply a failed push reply. Same generation guard.
    pub fn apply_error(&mut self, request_id: u64, message: String, at: Instant) -> bool {
        match &self.status {
            F1PushStatus::Running {
                request_id: rid,
                kind,
                delegated,
                ..
            } if *rid == request_id => {
                self.status = F1PushStatus::Error {
                    message,
                    finished_at: at,
                    kind: *kind,
                    delegated: *delegated,
                };
                true
            }
            _ => false,
        }
    }

    /// d-64: `true` while a terminal (Done / Error) fragment is
    /// showing. The event loop ticks while this holds so the
    /// fragment auto-hides on schedule.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            F1PushStatus::Done { .. } | F1PushStatus::Error { .. }
        )
    }

    /// d-64: clear a Done / Error fragment back to Idle once it's
    /// been on screen for `ttl` (mirrors the F3 pull/delete
    /// auto-hide, at the state level).
    pub fn clear_terminal_if_expired(&mut self, now: Instant, ttl: Duration) {
        let finished_at = match &self.status {
            F1PushStatus::Done { finished_at, .. } => *finished_at,
            F1PushStatus::Error { finished_at, .. } => *finished_at,
            _ => return,
        };
        if now.saturating_duration_since(finished_at) >= ttl {
            self.status = F1PushStatus::Idle;
        }
    }

    /// d-64: wall-clock remaining before the auto-hide fires on a
    /// terminal fragment, so the loop can collapse its sleep
    /// budget. `None` when no terminal is showing or it already
    /// expired (mirrors `F3PullState::terminal_remaining`).
    pub fn terminal_remaining(&self, now: Instant, ttl: Duration) -> Option<Duration> {
        let finished_at = match &self.status {
            F1PushStatus::Done { finished_at, .. } => *finished_at,
            F1PushStatus::Error { finished_at, .. } => *finished_at,
            _ => return None,
        };
        let elapsed = now.saturating_duration_since(finished_at);
        if elapsed >= ttl {
            None
        } else {
            Some(ttl - elapsed)
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
        let rid = s
            .begin("nas:9031".to_string(), PullKind::Copy)
            .expect("started");
        assert_eq!(rid, 1);
        assert!(s.is_running());
        match s.status() {
            F1PushStatus::Running {
                request_id,
                label,
                files,
                bytes,
                ..
            } => {
                assert_eq!(*request_id, 1);
                assert_eq!(label, "nas:9031");
                assert_eq!(*files, 0, "starts with zero progress");
                assert_eq!(*bytes, 0);
            }
            other => panic!("expected Running, got {other:?}"),
        }
    }

    #[test]
    fn apply_progress_updates_running_counters() {
        let mut s = F1PushState::new();
        let rid = s
            .begin("nas:9031".to_string(), PullKind::Copy)
            .expect("started");
        s.apply_progress(rid, 3, 4096, 2048);
        match s.status() {
            F1PushStatus::Running {
                files,
                bytes,
                bytes_per_sec,
                ..
            } => {
                assert_eq!(*files, 3);
                assert_eq!(*bytes, 4096);
                assert_eq!(*bytes_per_sec, 2048);
            }
            other => panic!("expected Running, got {other:?}"),
        }
        // Stale progress (superseded run) is dropped.
        s.apply_progress(rid + 99, 9, 9, 9);
        match s.status() {
            F1PushStatus::Running { files, .. } => assert_eq!(*files, 3, "stale dropped"),
            other => panic!("expected Running, got {other:?}"),
        }
    }

    #[test]
    fn begin_is_noop_while_running() {
        let mut s = F1PushState::new();
        s.begin("a".to_string(), PullKind::Copy).expect("first");
        assert!(
            s.begin("b".to_string(), PullKind::Copy).is_none(),
            "second push blocked"
        );
        assert!(s.is_running());
    }

    #[test]
    fn apply_done_records_terminal_state() {
        let mut s = F1PushState::new();
        let rid = s
            .begin("nas:9031".to_string(), PullKind::Copy)
            .expect("started");
        assert!(s.apply_done(rid, 12, 4096, Instant::now()));
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
    fn begin_delegated_marks_running_and_flows_to_done() {
        let mut s = F1PushState::new();
        let rid = s
            .begin_delegated("skippy:/b/".to_string())
            .expect("started");
        match s.status() {
            F1PushStatus::Running {
                delegated, kind, ..
            } => {
                assert!(*delegated, "delegated run flagged");
                assert_eq!(*kind, PullKind::Copy, "delegated is copy-only");
            }
            other => panic!("expected Running, got {other:?}"),
        }
        assert!(s.apply_done(rid, 7, 700, Instant::now()));
        match s.status() {
            F1PushStatus::Done { delegated, .. } => {
                assert!(*delegated, "delegated flag carries to Done")
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn begin_push_is_not_delegated() {
        let mut s = F1PushState::new();
        s.begin("nas:9031".to_string(), PullKind::Copy).expect("p");
        match s.status() {
            F1PushStatus::Running { delegated, .. } => assert!(!*delegated),
            other => panic!("expected Running, got {other:?}"),
        }
    }

    #[test]
    fn apply_error_records_message() {
        let mut s = F1PushState::new();
        let rid = s
            .begin("nas:9031".to_string(), PullKind::Copy)
            .expect("started");
        assert!(s.apply_error(rid, "connection refused".to_string(), Instant::now()));
        match s.status() {
            F1PushStatus::Error { message, .. } => assert_eq!(message, "connection refused"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn stale_reply_is_dropped() {
        let mut s = F1PushState::new();
        let rid = s
            .begin("nas:9031".to_string(), PullKind::Copy)
            .expect("started");
        // A reply for a superseded id must not clobber Running.
        assert!(!s.apply_done(rid + 99, 1, 1, Instant::now()));
        assert!(s.is_running());
        assert!(!s.apply_error(rid + 99, "x".to_string(), Instant::now()));
        assert!(s.is_running());
    }

    #[test]
    fn run_ids_increment_so_a_new_push_supersedes() {
        let mut s = F1PushState::new();
        let first = s.begin("a".to_string(), PullKind::Copy).expect("first");
        s.apply_done(first, 0, 0, Instant::now());
        let second = s.begin("b".to_string(), PullKind::Copy).expect("second");
        assert!(second > first, "ids monotonic → stale replies dropped");
    }

    // d-64: terminal auto-hide TTL.

    const TEST_TTL: Duration = Duration::from_secs(5);

    #[test]
    fn done_and_error_are_terminal_running_and_idle_are_not() {
        let mut s = F1PushState::new();
        assert!(!s.is_terminal(), "Idle not terminal");
        let rid = s
            .begin("nas:9031".to_string(), PullKind::Copy)
            .expect("started");
        assert!(!s.is_terminal(), "Running not terminal");
        s.apply_done(rid, 1, 1, Instant::now());
        assert!(s.is_terminal(), "Done is terminal");
    }

    #[test]
    fn clear_terminal_hides_done_after_ttl() {
        let mut s = F1PushState::new();
        let rid = s
            .begin("nas:9031".to_string(), PullKind::Copy)
            .expect("started");
        let finished = Instant::now();
        s.apply_done(rid, 5, 500, finished);
        // Within TTL → still showing.
        s.clear_terminal_if_expired(finished, TEST_TTL);
        assert!(s.is_terminal());
        // Past TTL → cleared.
        s.clear_terminal_if_expired(finished + TEST_TTL + Duration::from_millis(1), TEST_TTL);
        assert!(matches!(s.status(), F1PushStatus::Idle));
    }

    #[test]
    fn clear_terminal_is_noop_on_running() {
        let mut s = F1PushState::new();
        s.begin("nas:9031".to_string(), PullKind::Copy)
            .expect("started");
        // A long-running push must never be cleared by the sweep.
        s.clear_terminal_if_expired(Instant::now() + Duration::from_secs(3600), TEST_TTL);
        assert!(s.is_running(), "Running is immune to the terminal TTL");
    }

    #[test]
    fn terminal_remaining_some_within_none_after_and_on_running() {
        let mut s = F1PushState::new();
        assert!(
            s.terminal_remaining(Instant::now(), TEST_TTL).is_none(),
            "Idle"
        );
        let rid = s
            .begin("nas:9031".to_string(), PullKind::Copy)
            .expect("started");
        assert!(
            s.terminal_remaining(Instant::now(), TEST_TTL).is_none(),
            "Running has no terminal deadline"
        );
        let finished = Instant::now();
        s.apply_error(rid, "boom".to_string(), finished);
        let elapsed = Duration::from_millis(100);
        assert_eq!(
            s.terminal_remaining(finished + elapsed, TEST_TTL),
            Some(TEST_TTL - elapsed)
        );
        assert!(s
            .terminal_remaining(finished + TEST_TTL, TEST_TTL)
            .is_none());
    }
}
