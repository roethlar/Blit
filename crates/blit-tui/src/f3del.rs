//! F3 delete state — purge the F3 cursor row OR the d-49 marked
//! set (d-45 / d-50, TUI_DESIGN §5.3 `D: delete`).
//!
//! Destructive, so it's gated behind a confirm prompt: `D` opens
//! `delete <label>? y/N` (the label is a single path spec, or
//! "N item(s)" for a batch), `y`/`Y` fires one `Purge` RPC over
//! all `rel_paths`, `n`/`N`/`Esc` aborts. Targets are **frozen**
//! into the `Confirming` state at prompt-open time — changing the
//! cursor/selection afterward can't change what gets deleted (the
//! d-30 batch-cancel freezing lesson). All targets share one
//! module (they come from a single F3 view), so one `Purge`
//! suffices.
//!
//! Safety:
//! - The dispatcher refuses module roots / empty rel-paths (you
//!   can't nuke a whole module from the TUI — mirrors `blit rm`).
//! - Read-only modules are gated client-side (d-46) and enforced
//!   server-side as a backstop (the daemon rejects `Purge`).
//!
//! Outcome rendering: a **single**-row delete is path-gated (the
//! d-41 du pattern — `gate_path = Some(spec)`, hides once the
//! cursor leaves it). A **batch** carries `gate_path = None` and
//! shows its outcome until the next action (its rows are gone
//! after the post-delete refresh anyway). Replies are
//! generation-guarded by `request_id`.

use blit_core::remote::endpoint::RemoteEndpoint;

/// Lifecycle of an F3 delete (single cursor row OR a d-49 batch).
///
/// `label` is the human-readable target shown in the prompt /
/// outcome — the canonical path for a single delete, or
/// "N items" for a batch. `gate_path` carries the d-45 outcome
/// gating: `Some(spec)` for a single delete (Done/Error hide once
/// the cursor leaves that path); `None` for a batch (the rows are
/// gone after the post-delete refresh, so the outcome simply
/// shows until the next action).
#[derive(Debug, Clone)]
pub enum F3DelStatus {
    /// No delete in progress.
    Idle,
    /// Confirm prompt open. `module_endpoint` + `rel_paths` are
    /// frozen at open time (all targets share one module).
    Confirming {
        module_endpoint: RemoteEndpoint,
        rel_paths: Vec<String>,
        label: String,
        gate_path: Option<String>,
    },
    /// `Purge` RPC in flight.
    Deleting {
        request_id: u64,
        label: String,
        gate_path: Option<String>,
    },
    /// Purge succeeded; `files_deleted` is the daemon's count.
    Done {
        label: String,
        files_deleted: u64,
        gate_path: Option<String>,
    },
    /// Purge failed.
    Error {
        label: String,
        message: String,
        gate_path: Option<String>,
    },
}

/// Launch params handed from [`F3DelState::confirm`] to the spawn
/// helper. All `rel_paths` are deleted under `module_endpoint`'s
/// module in one `Purge`.
pub struct DelLaunch {
    pub module_endpoint: RemoteEndpoint,
    pub rel_paths: Vec<String>,
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

    /// Open the confirm prompt to delete `rel_paths` under
    /// `module_endpoint`'s module. `label` is shown in the prompt
    /// / outcome; `gate_path` is `Some(spec)` for a single-row
    /// delete (outcome gated on the cursor staying there) or
    /// `None` for a batch. No-op if already confirming/deleting,
    /// or if `rel_paths` is empty.
    pub fn begin(
        &mut self,
        module_endpoint: RemoteEndpoint,
        rel_paths: Vec<String>,
        label: String,
        gate_path: Option<String>,
    ) {
        if self.is_confirming() || self.is_deleting() || rel_paths.is_empty() {
            return;
        }
        self.status = F3DelStatus::Confirming {
            module_endpoint,
            rel_paths,
            label,
            gate_path,
        };
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
        let (module_endpoint, rel_paths, label, gate_path) =
            match std::mem::replace(&mut self.status, F3DelStatus::Idle) {
                F3DelStatus::Confirming {
                    module_endpoint,
                    rel_paths,
                    label,
                    gate_path,
                } => (module_endpoint, rel_paths, label, gate_path),
                other => {
                    self.status = other;
                    return None;
                }
            };
        self.request_seq += 1;
        let request_id = self.request_seq;
        self.status = F3DelStatus::Deleting {
            request_id,
            label,
            gate_path,
        };
        Some(DelLaunch {
            module_endpoint,
            rel_paths,
            request_id,
        })
    }

    /// Apply a successful Purge reply. Generation-guarded — a
    /// superseded delete's reply is dropped.
    pub fn apply_done(&mut self, request_id: u64, files_deleted: u64) -> bool {
        match &self.status {
            F3DelStatus::Deleting {
                request_id: rid,
                label,
                gate_path,
            } if *rid == request_id => {
                self.status = F3DelStatus::Done {
                    label: label.clone(),
                    files_deleted,
                    gate_path: gate_path.clone(),
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
                label,
                gate_path,
            } if *rid == request_id => {
                self.status = F3DelStatus::Error {
                    label: label.clone(),
                    message,
                    gate_path: gate_path.clone(),
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

    /// Single-row delete helper: one rel-path, gated on the spec.
    fn begin_single(s: &mut F3DelState, spec: &str, rel: &str) {
        s.begin(
            endpoint(spec),
            vec![rel.to_string()],
            spec.to_string(),
            Some(spec.to_string()),
        );
    }

    #[test]
    fn begin_opens_confirm_with_frozen_target() {
        let mut s = F3DelState::new();
        begin_single(&mut s, "nas:/home/old.txt", "old.txt");
        match s.status() {
            F3DelStatus::Confirming {
                label,
                rel_paths,
                gate_path,
                ..
            } => {
                assert_eq!(label, "nas:/home/old.txt");
                assert_eq!(rel_paths, &vec!["old.txt".to_string()]);
                assert_eq!(gate_path.as_deref(), Some("nas:/home/old.txt"));
            }
            other => panic!("expected Confirming, got {other:?}"),
        }
        assert!(s.is_confirming());
    }

    #[test]
    fn begin_with_empty_rel_paths_is_noop() {
        let mut s = F3DelState::new();
        s.begin(endpoint("nas:/m/a"), vec![], "0 items".to_string(), None);
        assert!(matches!(s.status(), F3DelStatus::Idle));
    }

    #[test]
    fn batch_confirm_carries_all_rel_paths_and_no_gate() {
        let mut s = F3DelState::new();
        s.begin(
            endpoint("nas:/home/"),
            vec![
                "a.txt".to_string(),
                "b.txt".to_string(),
                "c.txt".to_string(),
            ],
            "3 item(s)".to_string(),
            None,
        );
        let launch = s.confirm().expect("confirm");
        assert_eq!(launch.rel_paths.len(), 3);
        match s.status() {
            F3DelStatus::Deleting {
                label, gate_path, ..
            } => {
                assert_eq!(label, "3 item(s)");
                assert!(gate_path.is_none(), "batch outcome is not path-gated");
            }
            other => panic!("expected Deleting, got {other:?}"),
        }
    }

    #[test]
    fn cancel_returns_to_idle() {
        let mut s = F3DelState::new();
        begin_single(&mut s, "nas:/home/old.txt", "old.txt");
        s.cancel();
        assert!(matches!(s.status(), F3DelStatus::Idle));
    }

    #[test]
    fn confirm_transitions_to_deleting_and_returns_launch() {
        let mut s = F3DelState::new();
        begin_single(&mut s, "nas:/home/old.txt", "old.txt");
        let launch = s.confirm().expect("confirm yields a launch");
        assert_eq!(launch.rel_paths, vec!["old.txt".to_string()]);
        match s.status() {
            F3DelStatus::Deleting {
                request_id, label, ..
            } => {
                assert_eq!(*request_id, launch.request_id);
                assert_eq!(label, "nas:/home/old.txt");
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
        begin_single(&mut s, "nas:/m/a", "a");
        let _ = s.confirm();
        // Now Deleting — a second begin must not reopen a prompt.
        begin_single(&mut s, "nas:/m/b", "b");
        assert!(s.is_deleting());
    }

    #[test]
    fn apply_done_updates_deleting() {
        let mut s = F3DelState::new();
        begin_single(&mut s, "nas:/m/a", "a");
        let launch = s.confirm().unwrap();
        assert!(s.apply_done(launch.request_id, 3));
        match s.status() {
            F3DelStatus::Done {
                label,
                files_deleted,
                ..
            } => {
                assert_eq!(label, "nas:/m/a");
                assert_eq!(*files_deleted, 3);
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn apply_error_updates_deleting() {
        let mut s = F3DelState::new();
        begin_single(&mut s, "nas:/m/a", "a");
        let launch = s.confirm().unwrap();
        assert!(s.apply_error(launch.request_id, "read-only module".to_string()));
        match s.status() {
            F3DelStatus::Error { label, message, .. } => {
                assert_eq!(label, "nas:/m/a");
                assert_eq!(message, "read-only module");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn apply_done_drops_stale_request() {
        let mut s = F3DelState::new();
        begin_single(&mut s, "nas:/m/a", "a");
        let launch = s.confirm().unwrap();
        assert!(!s.apply_done(launch.request_id + 99, 1));
        assert!(s.is_deleting());
    }
}
