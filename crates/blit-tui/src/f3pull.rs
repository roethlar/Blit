//! F3 pull state — the TUI-owned lifecycle for a
//! remote→local pull initiated from the F3 cursor (d-35).
//!
//! Unlike the F2 transfers pane (which only *observes*
//! daemon-tracked jobs via the Subscribe stream), an F3
//! pull is driven by the TUI process itself: the daemon's
//! `PullSync` RPC streams bytes into the TUI's local
//! filesystem. So the lifecycle lives here, mirroring
//! F4's local-transfer `TransferState` shape rather than
//! the daemon-job model.
//!
//! Flow:
//!
//! 1. `p` on a selectable F3 row → [`F3PullState::begin`]
//!    captures the source [`RemoteEndpoint`] (derived by
//!    `browse::pull_source_endpoint`) and opens a
//!    destination-input prompt (`EnteringDest`).
//! 2. The operator types a local destination path; the
//!    input router feeds chars / Backspace into
//!    [`F3PullState::push_char`] / [`F3PullState::pop_char`].
//! 3. `Esc` → [`F3PullState::cancel`] (back to Idle).
//!    `Enter` with a non-empty dest →
//!    [`F3PullState::begin_run`] returns the launch
//!    params and transitions to `Running`.
//! 4. The pull task replies; [`F3PullState::apply_done`] /
//!    [`F3PullState::apply_error`] record the terminal
//!    state (guarded by `request_id` so a stale reply from
//!    a superseded run is dropped).

use blit_app::endpoints::Endpoint;
use blit_app::transfers::resolution::resolve_destination;
use blit_core::remote::endpoint::RemoteEndpoint;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Lifecycle of an F3 pull.
#[derive(Debug, Clone)]
pub enum F3PullStatus {
    /// No pull in progress.
    Idle,
    /// Operator is typing the local destination path.
    EnteringDest {
        /// Resolved remote source (host + port + module
        /// + rel_path) — moved into the launch on commit.
        source: RemoteEndpoint,
        /// The destination path the operator is typing.
        dest: String,
    },
    /// PullSync RPC in flight. d-37: `files` / `bytes` are
    /// live cumulative counters fed by the progress
    /// forwarder (0 until the first `Payload` /
    /// `FileComplete` event lands). d-39: `bytes_per_sec`
    /// is the average throughput (0 until ~1s elapsed, to
    /// dodge meaningless early-transfer spikes).
    Running {
        dest: String,
        request_id: u64,
        files: usize,
        bytes: u64,
        bytes_per_sec: u64,
    },
    /// Pull finished successfully. (The remote source is
    /// still shown in the Stats block's `Pull:` line, so
    /// the footer fragment only needs the outcome + dest.)
    /// d-38: `finished_at` drives the auto-hide TTL.
    Done {
        dest: String,
        files: usize,
        bytes: u64,
        finished_at: Instant,
    },
    /// Pull failed (validation or transport).
    Error {
        message: String,
        finished_at: Instant,
    },
}

/// Launch params handed from [`F3PullState::begin_run`] to
/// the dispatcher's spawn helper.
pub struct PullLaunch {
    pub source: RemoteEndpoint,
    pub dest_root: PathBuf,
    pub request_id: u64,
}

#[derive(Debug, Clone)]
pub struct F3PullState {
    status: F3PullStatus,
    /// Monotonic per-run id. Each `begin_run` bumps it and
    /// stamps the new value into `Running`; the reply arm
    /// drops replies whose id doesn't match the current
    /// run (same generation-guard pattern as the F2 cancel
    /// machine).
    request_seq: u64,
}

impl Default for F3PullState {
    fn default() -> Self {
        Self::new()
    }
}

impl F3PullState {
    pub fn new() -> Self {
        Self {
            status: F3PullStatus::Idle,
            request_seq: 0,
        }
    }

    pub fn status(&self) -> &F3PullStatus {
        &self.status
    }

    /// `true` while the destination prompt is open — the
    /// input router consults this to route keystrokes to
    /// the prompt instead of the normal F3 dispatcher.
    pub fn is_entering_dest(&self) -> bool {
        matches!(self.status, F3PullStatus::EnteringDest { .. })
    }

    /// `true` while a pull RPC is in flight. The dispatcher
    /// blocks a second `p` while running.
    pub fn is_running(&self) -> bool {
        matches!(self.status, F3PullStatus::Running { .. })
    }

    /// Open the destination prompt for `source`. No-op if a
    /// pull is already entering-dest or running (the
    /// dispatcher gates on that, but be defensive).
    pub fn begin(&mut self, source: RemoteEndpoint) {
        if self.is_entering_dest() || self.is_running() {
            return;
        }
        self.status = F3PullStatus::EnteringDest {
            source,
            dest: String::new(),
        };
    }

    /// Append a char to the destination path (no-op unless
    /// entering-dest).
    pub fn push_char(&mut self, c: char) {
        if let F3PullStatus::EnteringDest { dest, .. } = &mut self.status {
            dest.push(c);
        }
    }

    /// Drop the last char from the destination path. Returns
    /// true if a char was popped.
    pub fn pop_char(&mut self) -> bool {
        if let F3PullStatus::EnteringDest { dest, .. } = &mut self.status {
            dest.pop().is_some()
        } else {
            false
        }
    }

    /// Abort the prompt (Esc) — back to Idle.
    pub fn cancel(&mut self) {
        if self.is_entering_dest() {
            self.status = F3PullStatus::Idle;
        }
    }

    /// Commit the prompt (Enter). Returns the launch params
    /// and transitions to `Running`. Returns `None` (and
    /// stays in `EnteringDest`) when the dest is empty —
    /// there's nothing to pull into.
    pub fn begin_run(&mut self) -> Option<PullLaunch> {
        // Pull the payload out of EnteringDest without
        // cloning the endpoint.
        let (source, dest) = match std::mem::replace(&mut self.status, F3PullStatus::Idle) {
            F3PullStatus::EnteringDest { source, dest } => (source, dest),
            // Not entering-dest — restore and bail. (The
            // `mem::replace` set Idle; for non-EnteringDest
            // states we want to keep what was there, but
            // the only caller guards with is_entering_dest
            // so this branch is unreachable in practice.)
            other => {
                self.status = other;
                return None;
            }
        };
        if dest.trim().is_empty() {
            // Restore the prompt so the operator can keep
            // typing.
            self.status = F3PullStatus::EnteringDest { source, dest };
            return None;
        }
        let raw_dest = dest.trim().to_string();
        // d-35 round 2: apply the same rsync-style
        // destination resolution the CLI runs before a
        // pull (`resolve_destination`). `run_pull_sync`
        // treats `dest_root` as already-resolved — a
        // single-file pull expects the final FILE path,
        // and a directory pull into an existing local dir
        // must nest under the source basename rather than
        // merge. Skipping this (round 1) made
        // "pull file into existing dir" try to create the
        // dir itself as the output file, and
        // "pull dir into existing dir" merge contents.
        let raw_source = source.display();
        let resolved = resolve_destination(
            &raw_source,
            &raw_dest,
            &Endpoint::Remote(source.clone()),
            Endpoint::Local(PathBuf::from(&raw_dest)),
        );
        let dest_root = match resolved {
            Endpoint::Local(p) => p,
            // resolve_destination preserves the dst
            // variant; a Local dst can't become Remote.
            Endpoint::Remote(_) => PathBuf::from(&raw_dest),
        };
        self.request_seq += 1;
        let request_id = self.request_seq;
        self.status = F3PullStatus::Running {
            dest: raw_dest,
            request_id,
            files: 0,
            bytes: 0,
            bytes_per_sec: 0,
        };
        Some(PullLaunch {
            source,
            dest_root,
            request_id,
        })
    }

    /// d-37 / d-39: apply a live progress snapshot. Updates
    /// the `Running` counters + throughput in place;
    /// generation-guarded so a snapshot from a superseded
    /// run is dropped.
    pub fn apply_progress(
        &mut self,
        request_id: u64,
        files: usize,
        bytes: u64,
        bytes_per_sec: u64,
    ) {
        if let F3PullStatus::Running {
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

    /// Apply a successful pull reply. Dropped if `request_id`
    /// doesn't match the current `Running` run. Returns true
    /// if applied. `at` stamps the d-38 auto-hide deadline.
    pub fn apply_done(&mut self, request_id: u64, files: usize, bytes: u64, at: Instant) -> bool {
        match &self.status {
            F3PullStatus::Running {
                request_id: rid,
                dest,
                ..
            } if *rid == request_id => {
                self.status = F3PullStatus::Done {
                    dest: dest.clone(),
                    files,
                    bytes,
                    finished_at: at,
                };
                true
            }
            _ => false,
        }
    }

    /// Apply a failed pull reply. Same generation guard.
    pub fn apply_error(&mut self, request_id: u64, message: String, at: Instant) -> bool {
        match &self.status {
            F3PullStatus::Running {
                request_id: rid, ..
            } if *rid == request_id => {
                self.status = F3PullStatus::Error {
                    message,
                    finished_at: at,
                };
                true
            }
            _ => false,
        }
    }

    /// d-38: `true` while a terminal (Done / Error)
    /// fragment is showing. The event loop ticks while
    /// this holds so the fragment auto-hides on schedule.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            F3PullStatus::Done { .. } | F3PullStatus::Error { .. }
        )
    }

    /// d-38: clear a Done / Error fragment back to Idle
    /// once it's been on screen for `ttl`. Mirrors the
    /// d-23 cancel-status auto-hide, but at the state
    /// level (the reload-banner pattern) rather than
    /// renderer-side, so `is_terminal` flips false once
    /// expired and the loop stops ticking for it.
    pub fn clear_terminal_if_expired(&mut self, now: Instant, ttl: Duration) {
        let finished_at = match &self.status {
            F3PullStatus::Done { finished_at, .. } => *finished_at,
            F3PullStatus::Error { finished_at, .. } => *finished_at,
            _ => return,
        };
        if now.saturating_duration_since(finished_at) >= ttl {
            self.status = F3PullStatus::Idle;
        }
    }

    /// d-38: how long a Done / Error fragment stays on
    /// screen before auto-hiding. Fixed (not yet
    /// config-tunable) — long enough to read the outcome.
    pub const TERMINAL_TTL: Duration = Duration::from_secs(5);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(raw: &str) -> RemoteEndpoint {
        RemoteEndpoint::parse(raw).expect("endpoint")
    }

    #[test]
    fn new_is_idle() {
        let s = F3PullState::new();
        assert!(matches!(s.status(), F3PullStatus::Idle));
        assert!(!s.is_entering_dest());
        assert!(!s.is_running());
    }

    #[test]
    fn begin_opens_dest_prompt_with_empty_dest() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/photos/2024"));
        assert!(s.is_entering_dest());
        match s.status() {
            F3PullStatus::EnteringDest { source, dest } => {
                assert_eq!(source.display(), "nas:/photos/2024");
                assert!(dest.is_empty());
            }
            other => panic!("expected EnteringDest, got {other:?}"),
        }
    }

    #[test]
    fn push_and_pop_edit_dest() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/m/"));
        s.push_char('/');
        s.push_char('t');
        s.push_char('m');
        s.push_char('p');
        assert!(s.pop_char());
        match s.status() {
            F3PullStatus::EnteringDest { dest, .. } => assert_eq!(dest, "/tm"),
            other => panic!("expected EnteringDest, got {other:?}"),
        }
    }

    #[test]
    fn pop_on_empty_dest_returns_false() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/m/"));
        assert!(!s.pop_char());
    }

    #[test]
    fn cancel_returns_to_idle() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/m/"));
        s.push_char('x');
        s.cancel();
        assert!(matches!(s.status(), F3PullStatus::Idle));
    }

    #[test]
    fn begin_run_with_empty_dest_keeps_prompt() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/m/"));
        // No dest typed.
        assert!(s.begin_run().is_none());
        assert!(s.is_entering_dest(), "prompt stays open on empty dest");
    }

    #[test]
    fn begin_run_with_whitespace_dest_keeps_prompt() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/m/"));
        s.push_char(' ');
        s.push_char(' ');
        assert!(s.begin_run().is_none());
        assert!(s.is_entering_dest());
    }

    #[test]
    fn begin_run_launches_and_transitions_to_running() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/photos/2024"));
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        let launch = s.begin_run().expect("launch");
        assert_eq!(launch.dest_root, PathBuf::from("/tmp/out"));
        assert_eq!(launch.request_id, 1);
        // Source endpoint carried through.
        assert_eq!(launch.source.display(), "nas:/photos/2024");
        assert!(s.is_running());
    }

    #[test]
    fn begin_run_starts_with_zero_progress() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/m/x"));
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        s.begin_run().expect("launch");
        match s.status() {
            F3PullStatus::Running { files, bytes, .. } => {
                assert_eq!(*files, 0);
                assert_eq!(*bytes, 0);
            }
            other => panic!("expected Running, got {other:?}"),
        }
    }

    #[test]
    fn apply_progress_updates_running_counters() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/m/x"));
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        let launch = s.begin_run().expect("launch");
        s.apply_progress(launch.request_id, 3, 4096, 2048);
        match s.status() {
            F3PullStatus::Running {
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
    }

    #[test]
    fn apply_progress_drops_stale_request() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/m/x"));
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        let launch = s.begin_run().expect("launch");
        s.apply_progress(launch.request_id + 99, 5, 5, 5);
        match s.status() {
            F3PullStatus::Running {
                files,
                bytes,
                bytes_per_sec,
                ..
            } => {
                assert_eq!(*files, 0, "stale progress must not apply");
                assert_eq!(*bytes, 0);
                assert_eq!(*bytes_per_sec, 0);
            }
            other => panic!("expected Running, got {other:?}"),
        }
    }

    #[test]
    fn apply_progress_noop_when_not_running() {
        let mut s = F3PullState::new();
        // Idle — apply_progress must be a harmless no-op.
        s.apply_progress(1, 9, 9, 9);
        assert!(matches!(s.status(), F3PullStatus::Idle));
    }

    #[test]
    fn begin_run_trims_dest() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/m/"));
        for c in "  /tmp/out  ".chars() {
            s.push_char(c);
        }
        let launch = s.begin_run().expect("launch");
        assert_eq!(launch.dest_root, PathBuf::from("/tmp/out"));
    }

    #[test]
    fn apply_done_records_terminal_state() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/photos/x"));
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        let launch = s.begin_run().expect("launch");
        let applied = s.apply_done(launch.request_id, 12, 4096, Instant::now());
        assert!(applied);
        match s.status() {
            F3PullStatus::Done {
                files, bytes, dest, ..
            } => {
                assert_eq!(*files, 12);
                assert_eq!(*bytes, 4096);
                assert_eq!(dest, "/tmp/out");
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn apply_error_records_message() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/photos/x"));
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        let launch = s.begin_run().expect("launch");
        assert!(s.apply_error(
            launch.request_id,
            "connection refused".to_string(),
            Instant::now()
        ));
        match s.status() {
            F3PullStatus::Error { message, .. } => assert_eq!(message, "connection refused"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn stale_reply_is_dropped() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/photos/x"));
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        let launch = s.begin_run().expect("launch");
        // A reply for a different (older) request id must
        // not clobber the current Running state.
        assert!(!s.apply_done(launch.request_id + 99, 1, 1, Instant::now()));
        assert!(s.is_running());
    }

    #[test]
    fn request_ids_increment_across_runs() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/m/x"));
        for c in "/a".chars() {
            s.push_char(c);
        }
        let first = s.begin_run().expect("first launch");
        s.apply_done(first.request_id, 0, 0, Instant::now());
        // Second run.
        s.begin(endpoint("nas:/m/y"));
        for c in "/b".chars() {
            s.push_char(c);
        }
        let second = s.begin_run().expect("second launch");
        assert!(
            second.request_id > first.request_id,
            "run ids must be monotonic so stale replies are dropped"
        );
    }

    #[test]
    fn begin_is_noop_while_running() {
        let mut s = F3PullState::new();
        s.begin(endpoint("nas:/m/x"));
        for c in "/a".chars() {
            s.push_char(c);
        }
        s.begin_run().expect("launch");
        assert!(s.is_running());
        // A second begin while running must not reset.
        s.begin(endpoint("nas:/m/y"));
        assert!(s.is_running());
    }

    // d-35 round 2: rsync-style destination resolution.
    // `run_pull_sync` treats `dest_root` as the already
    // resolved target, so begin_run must apply the same
    // `resolve_destination` semantics the CLI does.

    fn launch_for(source: &str, dest: &str) -> PathBuf {
        let mut s = F3PullState::new();
        s.begin(endpoint(source));
        for c in dest.chars() {
            s.push_char(c);
        }
        s.begin_run().expect("launch").dest_root
    }

    /// Non-existent, non-trailing-slash dest → used as-is
    /// (the operator named an exact target path / rename).
    #[test]
    fn resolve_non_container_dest_used_as_is() {
        let dest_root = launch_for("nas:/photos/2024", "/tmp/blit-no-such-dir-xyz/out");
        assert_eq!(dest_root, PathBuf::from("/tmp/blit-no-such-dir-xyz/out"));
    }

    /// Trailing-slash dest is a container even when it
    /// doesn't exist → nest under the source basename.
    #[test]
    fn resolve_trailing_slash_dest_nests_under_basename() {
        // Dir source `2024` into container `/tmp/x/` →
        // `/tmp/x/2024`.
        let dest_root = launch_for("nas:/photos/2024", "/tmp/blit-no-such-dir-xyz/");
        assert_eq!(dest_root, PathBuf::from("/tmp/blit-no-such-dir-xyz/2024"));
    }

    /// File source into a trailing-slash container →
    /// `<dir>/<filename>` (the final file path
    /// `run_pull_sync` expects for a single-file pull).
    #[test]
    fn resolve_file_into_container_appends_filename() {
        let dest_root = launch_for("nas:/docs/readme.txt", "/tmp/blit-no-such-dir-xyz/");
        assert_eq!(
            dest_root,
            PathBuf::from("/tmp/blit-no-such-dir-xyz/readme.txt")
        );
    }

    /// An EXISTING local directory is a container too —
    /// dir source nests under its basename rather than
    /// merging into the dir.
    #[test]
    fn resolve_existing_dir_dest_nests_under_basename() {
        let tmp = tempfile::tempdir().expect("tmp");
        let dest = tmp.path().to_string_lossy().to_string();
        let dest_root = launch_for("nas:/photos/2024", &dest);
        assert_eq!(dest_root, tmp.path().join("2024"));
    }

    /// File source into an existing dir → `<dir>/<file>`.
    #[test]
    fn resolve_file_into_existing_dir_appends_filename() {
        let tmp = tempfile::tempdir().expect("tmp");
        let dest = tmp.path().to_string_lossy().to_string();
        let dest_root = launch_for("nas:/docs/readme.txt", &dest);
        assert_eq!(dest_root, tmp.path().join("readme.txt"));
    }

    // d-38: terminal-fragment auto-hide TTL.

    fn launched(state: &mut F3PullState, source: &str) -> u64 {
        state.begin(endpoint(source));
        for c in "/tmp/out".chars() {
            state.push_char(c);
        }
        state.begin_run().expect("launch").request_id
    }

    #[test]
    fn done_is_terminal_running_and_idle_are_not() {
        let mut s = F3PullState::new();
        assert!(!s.is_terminal(), "Idle is not terminal");
        let rid = launched(&mut s, "nas:/m/x");
        assert!(!s.is_terminal(), "Running is not terminal");
        s.apply_done(rid, 1, 1, Instant::now());
        assert!(s.is_terminal(), "Done is terminal");
    }

    #[test]
    fn error_is_terminal() {
        let mut s = F3PullState::new();
        let rid = launched(&mut s, "nas:/m/x");
        s.apply_error(rid, "boom".to_string(), Instant::now());
        assert!(s.is_terminal());
    }

    #[test]
    fn clear_terminal_hides_done_after_ttl() {
        let mut s = F3PullState::new();
        let rid = launched(&mut s, "nas:/m/x");
        let finished = Instant::now();
        s.apply_done(rid, 5, 500, finished);
        // Within TTL → still showing.
        s.clear_terminal_if_expired(finished, F3PullState::TERMINAL_TTL);
        assert!(s.is_terminal(), "within TTL the fragment stays");
        // Past TTL → cleared to Idle.
        s.clear_terminal_if_expired(
            finished + F3PullState::TERMINAL_TTL + Duration::from_millis(1),
            F3PullState::TERMINAL_TTL,
        );
        assert!(matches!(s.status(), F3PullStatus::Idle));
    }

    #[test]
    fn clear_terminal_at_exact_boundary_hides() {
        let mut s = F3PullState::new();
        let rid = launched(&mut s, "nas:/m/x");
        let finished = Instant::now();
        s.apply_error(rid, "boom".to_string(), finished);
        // `>=` boundary: exactly TTL elapsed → hidden.
        s.clear_terminal_if_expired(
            finished + F3PullState::TERMINAL_TTL,
            F3PullState::TERMINAL_TTL,
        );
        assert!(matches!(s.status(), F3PullStatus::Idle));
    }

    #[test]
    fn clear_terminal_is_noop_on_running_and_idle() {
        let mut s = F3PullState::new();
        // Idle — no-op.
        s.clear_terminal_if_expired(
            Instant::now() + Duration::from_secs(3600),
            F3PullState::TERMINAL_TTL,
        );
        assert!(matches!(s.status(), F3PullStatus::Idle));
        // Running — a long-running pull must never be
        // cleared by the TTL sweep.
        launched(&mut s, "nas:/m/x");
        s.clear_terminal_if_expired(
            Instant::now() + Duration::from_secs(3600),
            F3PullState::TERMINAL_TTL,
        );
        assert!(s.is_running(), "Running is immune to the terminal TTL");
    }
}
