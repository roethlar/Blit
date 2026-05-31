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

/// d-55/d-57: which flavor of remote→local pull this is.
/// `Copy` (`p`) just receives. `Mirror` (`m`) receives then
/// purges local files absent from the source. `Move` (`v`)
/// receives then deletes the *remote* source. Both `Mirror`
/// and `Move` are destructive and route through the
/// [`F3PullStatus::Confirm`] gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullKind {
    Copy,
    Mirror,
    Move,
}

impl PullKind {
    /// Destructive kinds delete something (local extraneous for
    /// mirror, the remote source for move) and so must pass the
    /// confirm gate before launching.
    pub fn is_destructive(self) -> bool {
        matches!(self, PullKind::Mirror | PullKind::Move)
    }

    /// Footer verb in the EnteringDest / Running / Done forms
    /// (e.g. `("mirror", "mirroring", "mirrored")`).
    pub fn verbs(self) -> (&'static str, &'static str, &'static str) {
        match self {
            PullKind::Copy => ("pull", "pulling", "pulled"),
            PullKind::Mirror => ("mirror", "mirroring", "mirrored"),
            PullKind::Move => ("move", "moving", "moved"),
        }
    }
}

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
        /// d-55/d-57: copy (`p`) / mirror (`m`) / move (`v`).
        /// A destructive kind routes commit through
        /// [`F3PullStatus::Confirm`] instead of launching directly.
        kind: PullKind,
    },
    /// d-55/d-57: destructive confirm. A mirror deletes local
    /// files absent from the source; a move deletes the remote
    /// source after the receive. Either way it's gated behind an
    /// explicit y/N before any deletion. `dest` is the
    /// operator-typed string (for the prompt); `dest_root` is
    /// the already-resolved target handed to the launch on
    /// confirm.
    Confirm {
        source: RemoteEndpoint,
        dest: String,
        dest_root: PathBuf,
        kind: PullKind,
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
        /// d-55/d-57: drives the footer verb.
        kind: PullKind,
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
        /// d-55/d-57: drives the footer verb.
        kind: PullKind,
        /// d-56: files deleted by the destructive phase — the
        /// mirror purge count, or the move's remote-source
        /// removal count (0 for a plain pull). Surfaced in the
        /// Done footer.
        deleted: u64,
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
    /// d-55/d-57: copy / mirror / move — the spawn helper keys
    /// `mirror_mode`, `require_complete_scan`, and the post-pull
    /// purge / remote-source delete off this.
    pub kind: PullKind,
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

    /// d-55/d-57: `true` while the destructive confirm
    /// (mirror or move) is open — the input router treats it as
    /// a modal (y/N).
    pub fn is_confirming_destructive(&self) -> bool {
        matches!(self.status, F3PullStatus::Confirm { .. })
    }

    /// `true` when any stage of a pull is in progress (prompt,
    /// confirm, or running) — the dispatcher gates a new
    /// `p`/`m`/`v` on this so a second op can't stack.
    fn is_busy(&self) -> bool {
        self.is_entering_dest() || self.is_running() || self.is_confirming_destructive()
    }

    /// Open the destination prompt for `source` (copy). No-op
    /// if an op is already in progress (the dispatcher gates on
    /// that, but be defensive).
    pub fn begin(&mut self, source: RemoteEndpoint) {
        self.begin_kind(source, PullKind::Copy);
    }

    /// d-55: open the destination prompt for `source` in mirror
    /// mode. Same prompt as [`begin`], but on commit it routes
    /// through the destructive [`F3PullStatus::Confirm`] gate.
    pub fn begin_mirror(&mut self, source: RemoteEndpoint) {
        self.begin_kind(source, PullKind::Mirror);
    }

    /// d-57: open the destination prompt for `source` in move
    /// mode (receive then delete the remote source). Destructive,
    /// so commit routes through the [`F3PullStatus::Confirm`] gate.
    pub fn begin_move(&mut self, source: RemoteEndpoint) {
        self.begin_kind(source, PullKind::Move);
    }

    fn begin_kind(&mut self, source: RemoteEndpoint, kind: PullKind) {
        if self.is_busy() {
            return;
        }
        self.status = F3PullStatus::EnteringDest {
            source,
            dest: String::new(),
            kind,
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

    /// d-53: the destination the operator has typed so far,
    /// while the prompt is open. Used by the batch-copy
    /// coordinator to capture the dest once before `begin_run`
    /// consumes it, so it can be reused for the queued sources.
    pub fn entering_dest(&self) -> Option<&str> {
        match &self.status {
            F3PullStatus::EnteringDest { dest, .. } => Some(dest.as_str()),
            _ => None,
        }
    }

    /// d-53: start a pull for `source` into `raw_dest` directly,
    /// bypassing the prompt — drives queued batch-copy sources
    /// after the operator entered the dest once. No-op (`None`)
    /// if a pull is already entering-dest/running or `raw_dest`
    /// is blank.
    pub fn start_pull(&mut self, source: RemoteEndpoint, raw_dest: String) -> Option<PullLaunch> {
        if self.is_busy() {
            return None;
        }
        self.launch(source, raw_dest, PullKind::Copy)
    }

    /// Commit the prompt (Enter). Returns the launch params
    /// and transitions to `Running`. Returns `None` (and
    /// stays in `EnteringDest`) when the dest is empty —
    /// there's nothing to pull into.
    pub fn begin_run(&mut self) -> Option<PullLaunch> {
        // Pull the payload out of EnteringDest without
        // cloning the endpoint.
        let (source, dest, kind) = match std::mem::replace(&mut self.status, F3PullStatus::Idle) {
            F3PullStatus::EnteringDest { source, dest, kind } => (source, dest, kind),
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
        // Empty dest → restore the prompt so the operator can
        // keep typing (applies to copy / mirror / move).
        if dest.trim().is_empty() {
            self.status = F3PullStatus::EnteringDest { source, dest, kind };
            return None;
        }
        if kind.is_destructive() {
            // d-55/d-57: a mirror deletes local extraneous files;
            // a move deletes the remote source. Both route through
            // an explicit confirm rather than launching. Resolve
            // the dest now so the confirm prompt and the eventual
            // launch agree on the target.
            let raw_dest = dest.trim().to_string();
            let dest_root = Self::resolve_dest(&source, &raw_dest);
            self.status = F3PullStatus::Confirm {
                source,
                dest: raw_dest,
                dest_root,
                kind,
            };
            None
        } else {
            self.launch(source, dest, PullKind::Copy)
        }
    }

    /// d-55/d-57: confirm the pending destructive op (y on the
    /// [`F3PullStatus::Confirm`] prompt) — bumps the run id and
    /// transitions to `Running`, handing back the launch params
    /// (carrying the kind so the spawn helper runs the right
    /// destructive phase). No-op (`None`) unless a confirm is open.
    pub fn confirm_destructive(&mut self) -> Option<PullLaunch> {
        let (source, dest, dest_root, kind) =
            match std::mem::replace(&mut self.status, F3PullStatus::Idle) {
                F3PullStatus::Confirm {
                    source,
                    dest,
                    dest_root,
                    kind,
                } => (source, dest, dest_root, kind),
                other => {
                    self.status = other;
                    return None;
                }
            };
        self.request_seq += 1;
        let request_id = self.request_seq;
        self.status = F3PullStatus::Running {
            dest,
            request_id,
            files: 0,
            bytes: 0,
            bytes_per_sec: 0,
            kind,
        };
        Some(PullLaunch {
            source,
            dest_root,
            request_id,
            kind,
        })
    }

    /// d-55/d-57: abort the pending destructive confirm (n / Esc)
    /// — back to Idle. No-op unless a confirm is open.
    pub fn cancel_destructive(&mut self) {
        if self.is_confirming_destructive() {
            self.status = F3PullStatus::Idle;
        }
    }

    /// d-55: rsync-style destination resolution, shared by the
    /// copy launch and the destructive-confirm gate.
    /// `run_pull_sync` treats `dest_root` as already-resolved
    /// (see [`launch`]), so both paths must apply identical
    /// semantics or a mirror would purge the wrong directory.
    fn resolve_dest(source: &RemoteEndpoint, raw_dest: &str) -> PathBuf {
        let raw_source = source.display();
        let resolved = resolve_destination(
            &raw_source,
            raw_dest,
            &Endpoint::Remote(source.clone()),
            Endpoint::Local(PathBuf::from(raw_dest)),
        );
        match resolved {
            Endpoint::Local(p) => p,
            // resolve_destination preserves the dst variant; a
            // Local dst can't become Remote.
            Endpoint::Remote(_) => PathBuf::from(raw_dest),
        }
    }

    /// d-53: shared core of `begin_run` / `start_pull` — resolve
    /// the destination and transition to `Running`. Returns
    /// `None` (no state change) when `raw_dest_in` is blank.
    fn launch(
        &mut self,
        source: RemoteEndpoint,
        raw_dest_in: String,
        kind: PullKind,
    ) -> Option<PullLaunch> {
        if raw_dest_in.trim().is_empty() {
            return None;
        }
        let raw_dest = raw_dest_in.trim().to_string();
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
        let dest_root = Self::resolve_dest(&source, &raw_dest);
        self.request_seq += 1;
        let request_id = self.request_seq;
        self.status = F3PullStatus::Running {
            dest: raw_dest,
            request_id,
            files: 0,
            bytes: 0,
            bytes_per_sec: 0,
            kind,
        };
        Some(PullLaunch {
            source,
            dest_root,
            request_id,
            kind,
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
    /// d-56: `deleted` is the mirror purge's `files_deleted`
    /// (0 for a plain pull), shown in the Done footer.
    pub fn apply_done(
        &mut self,
        request_id: u64,
        files: usize,
        bytes: u64,
        deleted: u64,
        at: Instant,
    ) -> bool {
        match &self.status {
            F3PullStatus::Running {
                request_id: rid,
                dest,
                kind,
                ..
            } if *rid == request_id => {
                self.status = F3PullStatus::Done {
                    dest: dest.clone(),
                    files,
                    bytes,
                    finished_at: at,
                    kind: *kind,
                    deleted,
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

    /// d-40 round 2: wall-clock remaining before the d-38
    /// auto-hide fires on a Done / Error fragment. Mirrors
    /// `main::cancel_status_remaining_ttl` so the event
    /// loop can collapse its sleep budget to
    /// `min(live_tick_interval, remaining)` — otherwise a
    /// short `pull_status_ttl_ms` (floor 250ms) could
    /// linger up to a full `live_tick.interval_ms` (ceiling
    /// 5s) before the next wake clears it.
    ///
    /// `None` when no terminal fragment is showing, or when
    /// it has already expired (the next clear handles it;
    /// no further wakeup is owed).
    pub fn terminal_remaining(&self, now: Instant, ttl: Duration) -> Option<Duration> {
        let finished_at = match &self.status {
            F3PullStatus::Done { finished_at, .. } => *finished_at,
            F3PullStatus::Error { finished_at, .. } => *finished_at,
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

    /// Representative fixed TTL for the auto-hide tests.
    /// `clear_terminal_if_expired` takes the TTL as a
    /// parameter (d-40 sources it from config), so the
    /// tests just need any concrete duration.
    const TEST_TTL: Duration = Duration::from_secs(5);

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
            F3PullStatus::EnteringDest { source, dest, kind } => {
                assert_eq!(source.display(), "nas:/photos/2024");
                assert!(dest.is_empty());
                assert_eq!(*kind, PullKind::Copy, "begin opens a copy prompt");
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
        let applied = s.apply_done(launch.request_id, 12, 4096, 0, Instant::now());
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
        assert!(!s.apply_done(launch.request_id + 99, 1, 1, 0, Instant::now()));
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
        s.apply_done(first.request_id, 0, 0, 0, Instant::now());
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
    fn entering_dest_reports_typed_dest() {
        let mut s = F3PullState::new();
        assert_eq!(s.entering_dest(), None, "no prompt → None");
        s.begin(endpoint("nas:/m/x"));
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        assert_eq!(s.entering_dest(), Some("/tmp/out"));
    }

    #[test]
    fn start_pull_launches_directly_without_prompt() {
        let mut s = F3PullState::new();
        // d-53: queued batch source goes straight to Running.
        let launch = s
            .start_pull(endpoint("nas:/m/y"), "/tmp/out".to_string())
            .expect("direct launch");
        assert!(s.is_running());
        assert_eq!(launch.request_id, 1);
    }

    /// d-53 R2 (reviewer reopen): with a container (trailing-
    /// slash) destination, two batch sources resolve to DISTINCT
    /// nested paths under their own basenames — they can't
    /// collide on the same target. (The Enter handler forces the
    /// trailing slash for batches; see `needs_container_slash`.)
    #[test]
    fn container_dest_nests_each_source_distinctly() {
        let mut a = F3PullState::new();
        let la = a
            .start_pull(endpoint("nas:/m/a.txt"), "/tmp/blit-d53-out/".to_string())
            .expect("a launch");
        let mut b = F3PullState::new();
        let lb = b
            .start_pull(endpoint("nas:/m/b.txt"), "/tmp/blit-d53-out/".to_string())
            .expect("b launch");
        assert_ne!(
            la.dest_root, lb.dest_root,
            "a container dest must nest each source under its own basename"
        );
        assert!(la.dest_root.ends_with("a.txt"), "got {:?}", la.dest_root);
        assert!(lb.dest_root.ends_with("b.txt"), "got {:?}", lb.dest_root);
    }

    #[test]
    fn start_pull_is_noop_when_busy_or_blank() {
        let mut s = F3PullState::new();
        // Blank dest → None, stays Idle.
        assert!(s
            .start_pull(endpoint("nas:/m/y"), "  ".to_string())
            .is_none());
        assert!(matches!(s.status(), F3PullStatus::Idle));
        // Already running → None.
        let _ = s.start_pull(endpoint("nas:/m/y"), "/tmp/out".to_string());
        assert!(s.is_running());
        assert!(s
            .start_pull(endpoint("nas:/m/z"), "/tmp/out".to_string())
            .is_none());
    }

    #[test]
    fn done_is_terminal_running_and_idle_are_not() {
        let mut s = F3PullState::new();
        assert!(!s.is_terminal(), "Idle is not terminal");
        let rid = launched(&mut s, "nas:/m/x");
        assert!(!s.is_terminal(), "Running is not terminal");
        s.apply_done(rid, 1, 1, 0, Instant::now());
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
        s.apply_done(rid, 5, 500, 0, finished);
        // Within TTL → still showing.
        s.clear_terminal_if_expired(finished, TEST_TTL);
        assert!(s.is_terminal(), "within TTL the fragment stays");
        // Past TTL → cleared to Idle.
        s.clear_terminal_if_expired(finished + TEST_TTL + Duration::from_millis(1), TEST_TTL);
        assert!(matches!(s.status(), F3PullStatus::Idle));
    }

    #[test]
    fn clear_terminal_at_exact_boundary_hides() {
        let mut s = F3PullState::new();
        let rid = launched(&mut s, "nas:/m/x");
        let finished = Instant::now();
        s.apply_error(rid, "boom".to_string(), finished);
        // `>=` boundary: exactly TTL elapsed → hidden.
        s.clear_terminal_if_expired(finished + TEST_TTL, TEST_TTL);
        assert!(matches!(s.status(), F3PullStatus::Idle));
    }

    #[test]
    fn clear_terminal_is_noop_on_running_and_idle() {
        let mut s = F3PullState::new();
        // Idle — no-op.
        s.clear_terminal_if_expired(Instant::now() + Duration::from_secs(3600), TEST_TTL);
        assert!(matches!(s.status(), F3PullStatus::Idle));
        // Running — a long-running pull must never be
        // cleared by the TTL sweep.
        launched(&mut s, "nas:/m/x");
        s.clear_terminal_if_expired(Instant::now() + Duration::from_secs(3600), TEST_TTL);
        assert!(s.is_running(), "Running is immune to the terminal TTL");
    }

    // d-40 R2: `terminal_remaining` feeds the event loop's
    // sleep budget so a short pull TTL isn't delayed by a
    // long live tick (reviewer reopen).

    #[test]
    fn terminal_remaining_some_within_ttl_none_after() {
        let mut s = F3PullState::new();
        let rid = launched(&mut s, "nas:/m/x");
        let finished = Instant::now();
        s.apply_done(rid, 5, 500, 0, finished);
        // Partway through the window → remaining shrinks.
        let elapsed = Duration::from_millis(100);
        assert_eq!(
            s.terminal_remaining(finished + elapsed, TEST_TTL),
            Some(TEST_TTL - elapsed)
        );
        // At/after the boundary → None (the clear handles it).
        assert!(s
            .terminal_remaining(finished + TEST_TTL, TEST_TTL)
            .is_none());
        assert!(s
            .terminal_remaining(finished + TEST_TTL + Duration::from_millis(1), TEST_TTL)
            .is_none());
    }

    #[test]
    fn terminal_remaining_none_on_idle_and_running() {
        let mut s = F3PullState::new();
        // Idle → no deadline.
        assert!(s.terminal_remaining(Instant::now(), TEST_TTL).is_none());
        // Running → no terminal deadline (only Done/Error).
        launched(&mut s, "nas:/m/x");
        assert!(s.terminal_remaining(Instant::now(), TEST_TTL).is_none());
    }

    #[test]
    fn terminal_remaining_some_for_error_fragment() {
        let mut s = F3PullState::new();
        let rid = launched(&mut s, "nas:/m/x");
        let finished = Instant::now();
        s.apply_error(rid, "boom".to_string(), finished);
        assert_eq!(
            s.terminal_remaining(finished, TEST_TTL),
            Some(TEST_TTL),
            "fresh error fragment has the full TTL remaining"
        );
    }

    // d-55/d-57: destructive flow — begin_mirror / begin_move →
    // Confirm → confirm_destructive, the gate that distinguishes
    // `m` / `v` from `p`.

    #[test]
    fn begin_mirror_opens_prompt_in_mirror_kind() {
        let mut s = F3PullState::new();
        s.begin_mirror(endpoint("nas:/photos/2024"));
        assert!(s.is_entering_dest(), "mirror reuses the dest prompt");
        match s.status() {
            F3PullStatus::EnteringDest { kind, .. } => {
                assert_eq!(*kind, PullKind::Mirror);
            }
            other => panic!("expected EnteringDest, got {other:?}"),
        }
    }

    #[test]
    fn begin_move_opens_prompt_in_move_kind() {
        let mut s = F3PullState::new();
        s.begin_move(endpoint("nas:/photos/2024"));
        assert!(s.is_entering_dest(), "move reuses the dest prompt");
        match s.status() {
            F3PullStatus::EnteringDest { kind, .. } => {
                assert_eq!(*kind, PullKind::Move);
            }
            other => panic!("expected EnteringDest, got {other:?}"),
        }
    }

    #[test]
    fn begin_run_on_destructive_routes_to_confirm_not_running() {
        for begin in [
            F3PullState::begin_mirror as fn(&mut F3PullState, RemoteEndpoint),
            F3PullState::begin_move,
        ] {
            let mut s = F3PullState::new();
            begin(&mut s, endpoint("nas:/photos/2024"));
            for c in "/tmp/out".chars() {
                s.push_char(c);
            }
            // Enter on a destructive op does NOT launch — it opens
            // the confirm gate.
            assert!(
                s.begin_run().is_none(),
                "destructive Enter defers the launch to confirm"
            );
            assert!(s.is_confirming_destructive());
            assert!(!s.is_running(), "no PullSync until the operator confirms");
        }
    }

    #[test]
    fn begin_run_on_empty_destructive_dest_keeps_prompt() {
        let mut s = F3PullState::new();
        s.begin_mirror(endpoint("nas:/m/"));
        // No dest typed.
        assert!(s.begin_run().is_none());
        assert!(
            s.is_entering_dest(),
            "empty destructive dest keeps the prompt, doesn't confirm"
        );
        assert!(!s.is_confirming_destructive());
    }

    #[test]
    fn confirm_destructive_launches_with_mirror_kind() {
        let mut s = F3PullState::new();
        s.begin_mirror(endpoint("nas:/photos/2024"));
        for c in "/tmp/blit-no-such-dir-xyz/out".chars() {
            s.push_char(c);
        }
        s.begin_run();
        assert!(s.is_confirming_destructive());
        let launch = s
            .confirm_destructive()
            .expect("confirm launches the mirror");
        assert_eq!(launch.kind, PullKind::Mirror, "the launch carries the kind");
        assert!(s.is_running());
        match s.status() {
            F3PullStatus::Running { kind, .. } => assert_eq!(*kind, PullKind::Mirror),
            other => panic!("expected Running, got {other:?}"),
        }
    }

    #[test]
    fn confirm_destructive_launches_with_move_kind() {
        let mut s = F3PullState::new();
        s.begin_move(endpoint("nas:/photos/2024"));
        for c in "/tmp/blit-no-such-dir-xyz/out".chars() {
            s.push_char(c);
        }
        s.begin_run();
        let launch = s.confirm_destructive().expect("confirm launches the move");
        assert_eq!(
            launch.kind,
            PullKind::Move,
            "the launch carries the move kind"
        );
        match s.status() {
            F3PullStatus::Running { kind, .. } => assert_eq!(*kind, PullKind::Move),
            other => panic!("expected Running, got {other:?}"),
        }
    }

    #[test]
    fn cancel_destructive_returns_to_idle() {
        let mut s = F3PullState::new();
        s.begin_move(endpoint("nas:/m/x"));
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        s.begin_run();
        assert!(s.is_confirming_destructive());
        s.cancel_destructive();
        assert!(matches!(s.status(), F3PullStatus::Idle));
    }

    #[test]
    fn confirm_destructive_is_noop_when_not_confirming() {
        let mut s = F3PullState::new();
        // Idle → nothing to confirm.
        assert!(s.confirm_destructive().is_none());
        assert!(matches!(s.status(), F3PullStatus::Idle));
        // While entering-dest (not yet committed) → also a no-op.
        s.begin_mirror(endpoint("nas:/m/x"));
        assert!(s.confirm_destructive().is_none());
        assert!(s.is_entering_dest());
    }

    /// The destructive confirm must resolve the dest with the
    /// SAME rsync-style semantics as a copy — a mirror that purged
    /// (or a move that received into) the wrong directory would be
    /// a data-loss bug.
    #[test]
    fn destructive_confirm_resolves_dest_like_copy() {
        // Copy resolution: dir source into a trailing-slash
        // container nests under the basename.
        let copy_dest = launch_for("nas:/photos/2024", "/tmp/blit-no-such-dir-xyz/");

        let mut s = F3PullState::new();
        s.begin_mirror(endpoint("nas:/photos/2024"));
        for c in "/tmp/blit-no-such-dir-xyz/".chars() {
            s.push_char(c);
        }
        s.begin_run();
        let launch = s.confirm_destructive().expect("confirm");
        assert_eq!(
            launch.dest_root, copy_dest,
            "destructive op resolves the target identically to a copy"
        );
    }

    #[test]
    fn done_carries_kind_and_deleted_count() {
        let mut s = F3PullState::new();
        s.begin_mirror(endpoint("nas:/m/x"));
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        s.begin_run();
        let launch = s.confirm_destructive().expect("confirm");
        // d-56: the purge deleted 4 local files.
        s.apply_done(launch.request_id, 3, 300, 4, Instant::now());
        match s.status() {
            F3PullStatus::Done { kind, deleted, .. } => {
                assert_eq!(*kind, PullKind::Mirror, "Done remembers the kind");
                assert_eq!(*deleted, 4, "Done carries the delete count");
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn begin_is_noop_while_confirming_destructive() {
        let mut s = F3PullState::new();
        s.begin_mirror(endpoint("nas:/m/x"));
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        s.begin_run();
        assert!(s.is_confirming_destructive());
        // A `p` (or `m`/`v`) while a confirm is open must not reset.
        s.begin(endpoint("nas:/m/y"));
        assert!(
            s.is_confirming_destructive(),
            "confirm is modal vs a new op"
        );
    }

    #[test]
    fn pull_kind_verbs_and_destructive() {
        assert!(!PullKind::Copy.is_destructive());
        assert!(PullKind::Mirror.is_destructive());
        assert!(PullKind::Move.is_destructive());
        assert_eq!(PullKind::Move.verbs(), ("move", "moving", "moved"));
    }
}
