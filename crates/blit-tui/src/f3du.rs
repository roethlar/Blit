//! F3 du state — disk-usage subtree total for the F3 cursor
//! (d-41, TUI_DESIGN §5.3 "subtree: X across N files").
//!
//! Pressing `u` ("usage") on a browsable F3 row streams the
//! daemon's `DiskUsage` RPC (`max_depth = 0` → a single root
//! aggregate entry) and shows the subtree byte/file total in the
//! Stats block.
//!
//! The result is bound to the path it was computed for. The
//! renderer only surfaces it while the cursor still sits on that
//! path (the bridge compares `path` against the cursor's
//! canonical spec), so moving the cursor hides a now-stale figure
//! without any timer — a `Done` result for `home/photos` simply
//! stops rendering once the operator navigates elsewhere, and
//! reappears if they come back.
//!
//! Like the F3 pull machine, replies are generation-guarded by
//! `request_id`: pressing `u` again (or on a different row)
//! supersedes an in-flight query, and the stale reply is dropped.

/// Lifecycle of an F3 du query. Every non-`Idle` variant carries
/// the `path` (the cursor's canonical spec) it pertains to, so
/// the renderer can gate on cursor position.
#[derive(Debug, Clone)]
pub enum F3DuStatus {
    /// No du computed / showing.
    Idle,
    /// DiskUsage RPC in flight for `path`.
    Running { request_id: u64, path: String },
    /// Subtree total for `path`.
    Done {
        path: String,
        bytes: u64,
        files: u64,
    },
    /// Query failed for `path`.
    Error { path: String, message: String },
}

#[derive(Debug, Clone)]
pub struct F3DuState {
    status: F3DuStatus,
    /// Monotonic per-query id (same generation-guard pattern as
    /// the F3 pull machine). Each `begin` bumps it.
    request_seq: u64,
}

impl Default for F3DuState {
    fn default() -> Self {
        Self::new()
    }
}

impl F3DuState {
    pub fn new() -> Self {
        Self {
            status: F3DuStatus::Idle,
            request_seq: 0,
        }
    }

    pub fn status(&self) -> &F3DuStatus {
        &self.status
    }

    /// Start a du query for `path` (the cursor's canonical spec).
    /// Bumps the generation, transitions to `Running`, and
    /// returns the new `request_id` for the spawn helper to stamp
    /// onto its reply. Always supersedes any prior query.
    pub fn begin(&mut self, path: String) -> u64 {
        self.request_seq += 1;
        let id = self.request_seq;
        self.status = F3DuStatus::Running {
            request_id: id,
            path,
        };
        id
    }

    /// Apply a successful du reply. Ignored (returns false) when
    /// the id doesn't match the current run — a superseded
    /// query's reply must not overwrite a newer one.
    pub fn apply_done(&mut self, request_id: u64, bytes: u64, files: u64) -> bool {
        match &self.status {
            F3DuStatus::Running {
                request_id: rid,
                path,
            } if *rid == request_id => {
                self.status = F3DuStatus::Done {
                    path: path.clone(),
                    bytes,
                    files,
                };
                true
            }
            _ => false,
        }
    }

    /// Apply a failed du reply. Same generation guard.
    pub fn apply_error(&mut self, request_id: u64, message: String) -> bool {
        match &self.status {
            F3DuStatus::Running {
                request_id: rid,
                path,
            } if *rid == request_id => {
                self.status = F3DuStatus::Error {
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

    #[test]
    fn new_is_idle() {
        let s = F3DuState::new();
        assert!(matches!(s.status(), F3DuStatus::Idle));
    }

    #[test]
    fn begin_transitions_to_running_with_path() {
        let mut s = F3DuState::new();
        let id = s.begin("nas:/home/photos".to_string());
        match s.status() {
            F3DuStatus::Running { request_id, path } => {
                assert_eq!(*request_id, id);
                assert_eq!(path, "nas:/home/photos");
            }
            other => panic!("expected Running, got {other:?}"),
        }
    }

    #[test]
    fn begin_bumps_generation_each_call() {
        let mut s = F3DuState::new();
        let a = s.begin("nas:/a".to_string());
        let b = s.begin("nas:/b".to_string());
        assert!(b > a, "each begin must supersede the prior id");
    }

    #[test]
    fn apply_done_updates_running_and_preserves_path() {
        let mut s = F3DuState::new();
        let id = s.begin("nas:/home/photos".to_string());
        assert!(s.apply_done(id, 14_680_064, 8_442));
        match s.status() {
            F3DuStatus::Done { path, bytes, files } => {
                assert_eq!(path, "nas:/home/photos");
                assert_eq!(*bytes, 14_680_064);
                assert_eq!(*files, 8_442);
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn apply_done_drops_stale_request() {
        let mut s = F3DuState::new();
        let id = s.begin("nas:/home/photos".to_string());
        // A reply for a superseded query must not apply.
        assert!(!s.apply_done(id + 99, 1, 1));
        assert!(matches!(s.status(), F3DuStatus::Running { .. }));
    }

    #[test]
    fn apply_error_updates_running() {
        let mut s = F3DuState::new();
        let id = s.begin("nas:/home/photos".to_string());
        assert!(s.apply_error(id, "boom".to_string()));
        match s.status() {
            F3DuStatus::Error { path, message } => {
                assert_eq!(path, "nas:/home/photos");
                assert_eq!(message, "boom");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn apply_on_idle_is_noop() {
        let mut s = F3DuState::new();
        assert!(!s.apply_done(1, 1, 1));
        assert!(!s.apply_error(1, "x".to_string()));
        assert!(matches!(s.status(), F3DuStatus::Idle));
    }

    #[test]
    fn second_begin_supersedes_running() {
        let mut s = F3DuState::new();
        let first = s.begin("nas:/a".to_string());
        let second = s.begin("nas:/b".to_string());
        // The first query's reply is now stale.
        assert!(!s.apply_done(first, 1, 1));
        // The second query's reply applies.
        assert!(s.apply_done(second, 2, 2));
        assert!(matches!(s.status(), F3DuStatus::Done { .. }));
    }
}
