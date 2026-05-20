//! F3 delete state — purge a remote path from the F3 cursor
//! (d-45, TUI_DESIGN §5.3 `D: delete`).
//!
//! Destructive, so it's gated behind a confirm prompt: `D` opens
//! `delete <path>? y/N`, `y`/`Y` fires the `Purge` RPC, `n`/`N`/
//! `Esc` aborts. The target (resolved `RemoteEndpoint` + its
//! display spec) is **frozen** into the `Confirming` state at
//! prompt-open time — moving the cursor afterward can't change
//! what gets deleted (the d-30 batch-cancel freezing lesson).
//!
//! Safety:
//! - The dispatcher refuses to open the prompt for a module root
//!   or empty rel-path (you can't nuke a whole module from the
//!   TUI — mirrors `blit rm`'s guard).
//! - Read-only modules are enforced **server-side**: the daemon
//!   rejects `Purge` on a read-only module and the error surfaces
//!   in the footer. (Client-side key-disable is a known gap — the
//!   daemon is the authority, so this is safe, just less polished
//!   than the design's "disable the key".)
//!
//! Outcome rendering is **path-gated** (the d-41 du pattern): the
//! `Done`/`Error` fragment only shows while the cursor is still on
//! the deleted path, so a stale outcome never lingers against the
//! wrong row — no TTL machinery needed. Replies are
//! generation-guarded by `request_id`.

use blit_core::remote::endpoint::RemoteEndpoint;

/// Lifecycle of an F3 delete.
#[derive(Debug, Clone)]
pub enum F3DelStatus {
    /// No delete in progress.
    Idle,
    /// Confirm prompt open. `endpoint` + `path` are frozen at
    /// open time; `path` is the canonical display spec (shown in
    /// the prompt and used for outcome gating).
    Confirming {
        endpoint: RemoteEndpoint,
        path: String,
    },
    /// `Purge` RPC in flight for `path`.
    Deleting { request_id: u64, path: String },
    /// Purge succeeded; `files_deleted` is the daemon's count.
    Done { path: String, files_deleted: u64 },
    /// Purge failed.
    Error { path: String, message: String },
}

/// Launch params handed from [`F3DelState::confirm`] to the spawn
/// helper. The frozen `path` lives on the `Deleting` status (for
/// the footer + outcome gating); the spawn only needs the
/// endpoint and the generation id.
pub struct DelLaunch {
    pub endpoint: RemoteEndpoint,
    pub request_id: u64,
}

#[derive(Debug, Clone)]
pub struct F3DelState {
    status: F3DelStatus,
    /// Monotonic per-delete id (generation guard, same pattern as
    /// the F3 pull / du machines).
    request_seq: u64,
}

impl Default for F3DelState {
    fn default() -> Self {
        Self::new()
    }
}

impl F3DelState {
    pub fn new() -> Self {
        Self {
            status: F3DelStatus::Idle,
            request_seq: 0,
        }
    }

    pub fn status(&self) -> &F3DelStatus {
        &self.status
    }

    /// `true` while the confirm prompt is open — the input router
    /// consults this to route `y`/`n`/`Esc` to the delete API
    /// instead of the normal F3 dispatch.
    pub fn is_confirming(&self) -> bool {
        matches!(self.status, F3DelStatus::Confirming { .. })
    }

    /// `true` while a Purge RPC is in flight (the dispatcher
    /// blocks a second `D` while deleting).
    pub fn is_deleting(&self) -> bool {
        matches!(self.status, F3DelStatus::Deleting { .. })
    }

    /// Open the confirm prompt for `endpoint` (display spec
    /// `path`). No-op if a delete is already confirming or
    /// deleting.
    pub fn begin(&mut self, endpoint: RemoteEndpoint, path: String) {
        if self.is_confirming() || self.is_deleting() {
            return;
        }
        self.status = F3DelStatus::Confirming { endpoint, path };
    }

    /// Abort the confirm prompt (`n`/`Esc`) — back to Idle. No-op
    /// unless confirming.
    pub fn cancel(&mut self) {
        if self.is_confirming() {
            self.status = F3DelStatus::Idle;
        }
    }

    /// Commit the confirm prompt (`y`). Returns the launch params
    /// and transitions to `Deleting`. Returns `None` (no state
    /// change) unless currently confirming.
    pub fn confirm(&mut self) -> Option<DelLaunch> {
        let (endpoint, path) = match std::mem::replace(&mut self.status, F3DelStatus::Idle) {
            F3DelStatus::Confirming { endpoint, path } => (endpoint, path),
            other => {
                self.status = other;
                return None;
            }
        };
        self.request_seq += 1;
        let request_id = self.request_seq;
        self.status = F3DelStatus::Deleting { request_id, path };
        Some(DelLaunch {
            endpoint,
            request_id,
        })
    }

    /// Apply a successful Purge reply. Generation-guarded — a
    /// superseded delete's reply is dropped.
    pub fn apply_done(&mut self, request_id: u64, files_deleted: u64) -> bool {
        match &self.status {
            F3DelStatus::Deleting {
                request_id: rid,
                path,
            } if *rid == request_id => {
                self.status = F3DelStatus::Done {
                    path: path.clone(),
                    files_deleted,
                };
                true
            }
            _ => false,
        }
    }

    /// Apply a failed Purge reply. Same generation guard.
    pub fn apply_error(&mut self, request_id: u64, message: String) -> bool {
        match &self.status {
            F3DelStatus::Deleting {
                request_id: rid,
                path,
            } if *rid == request_id => {
                self.status = F3DelStatus::Error {
                    path: path.clone(),
                    message,
                };
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(raw: &str) -> RemoteEndpoint {
        RemoteEndpoint::parse(raw).expect("endpoint")
    }

    #[test]
    fn new_is_idle() {
        assert!(matches!(F3DelState::new().status(), F3DelStatus::Idle));
    }

    #[test]
    fn begin_opens_confirm_with_frozen_target() {
        let mut s = F3DelState::new();
        s.begin(
            endpoint("nas:/home/old.txt"),
            "nas:/home/old.txt".to_string(),
        );
        match s.status() {
            F3DelStatus::Confirming { path, .. } => assert_eq!(path, "nas:/home/old.txt"),
            other => panic!("expected Confirming, got {other:?}"),
        }
        assert!(s.is_confirming());
    }

    #[test]
    fn cancel_returns_to_idle() {
        let mut s = F3DelState::new();
        s.begin(
            endpoint("nas:/home/old.txt"),
            "nas:/home/old.txt".to_string(),
        );
        s.cancel();
        assert!(matches!(s.status(), F3DelStatus::Idle));
    }

    #[test]
    fn confirm_transitions_to_deleting_and_returns_launch() {
        let mut s = F3DelState::new();
        s.begin(
            endpoint("nas:/home/old.txt"),
            "nas:/home/old.txt".to_string(),
        );
        let launch = s.confirm().expect("confirm yields a launch");
        match s.status() {
            F3DelStatus::Deleting { request_id, path } => {
                assert_eq!(*request_id, launch.request_id);
                assert_eq!(path, "nas:/home/old.txt");
            }
            other => panic!("expected Deleting, got {other:?}"),
        }
    }

    #[test]
    fn confirm_is_none_when_not_confirming() {
        let mut s = F3DelState::new();
        assert!(s.confirm().is_none());
        assert!(matches!(s.status(), F3DelStatus::Idle));
    }

    #[test]
    fn begin_is_noop_while_deleting() {
        let mut s = F3DelState::new();
        s.begin(endpoint("nas:/m/a"), "nas:/a".to_string());
        let _ = s.confirm();
        // Now Deleting — a second begin must not reopen a prompt.
        s.begin(endpoint("nas:/m/b"), "nas:/b".to_string());
        assert!(s.is_deleting());
    }

    #[test]
    fn apply_done_updates_deleting() {
        let mut s = F3DelState::new();
        s.begin(endpoint("nas:/m/a"), "nas:/a".to_string());
        let launch = s.confirm().unwrap();
        assert!(s.apply_done(launch.request_id, 3));
        match s.status() {
            F3DelStatus::Done {
                path,
                files_deleted,
            } => {
                assert_eq!(path, "nas:/a");
                assert_eq!(*files_deleted, 3);
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn apply_error_updates_deleting() {
        let mut s = F3DelState::new();
        s.begin(endpoint("nas:/m/a"), "nas:/a".to_string());
        let launch = s.confirm().unwrap();
        assert!(s.apply_error(launch.request_id, "read-only module".to_string()));
        match s.status() {
            F3DelStatus::Error { path, message } => {
                assert_eq!(path, "nas:/a");
                assert_eq!(message, "read-only module");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn apply_done_drops_stale_request() {
        let mut s = F3DelState::new();
        s.begin(endpoint("nas:/m/a"), "nas:/a".to_string());
        let launch = s.confirm().unwrap();
        assert!(!s.apply_done(launch.request_id + 99, 1));
        assert!(s.is_deleting());
    }
}
