//! F4 Diagnostics dump state. Operator presses `s`
//! (snapshot) on F4 to write a JSON dump of the current
//! Source/Destination pair to disk. The dump reuses
//! `blit_app::diagnostics::dump::endpoint_snapshot` so the
//! file shape matches the CLI's `blit diagnostics dump
//! --json` output verbatim.
//!
//! Mnemonic: `s` was picked over `d` because `d` is taken
//! by ProfileDisable. The TUI_DESIGN listing `[d] dump`
//! conflicts with `[d] disable` on the same screen — this
//! slice resolves the conflict by binding the dump on a
//! different key.

use std::path::PathBuf;

#[derive(Debug)]
pub enum DiagnosticsStatus {
    Idle,
    Running,
    Done { path: PathBuf },
    Error { message: String },
}

#[derive(Debug)]
pub struct DiagnosticsState {
    status: DiagnosticsStatus,
    request_id: u64,
}

impl Default for DiagnosticsState {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticsState {
    pub fn new() -> Self {
        Self {
            status: DiagnosticsStatus::Idle,
            request_id: 0,
        }
    }

    pub fn status(&self) -> &DiagnosticsStatus {
        &self.status
    }

    /// Begin a dump. Bumps the generation, flips to
    /// Running, returns the new generation for the spawned
    /// task to tag its reply with.
    pub fn begin_dump(&mut self) -> u64 {
        self.request_id += 1;
        self.status = DiagnosticsStatus::Running;
        self.request_id
    }

    /// Apply a successful dump. Returns false on stale
    /// generation (operator hit `s` again before the first
    /// completed).
    pub fn apply_done(&mut self, request_id: u64, path: PathBuf) -> bool {
        if request_id != self.request_id {
            return false;
        }
        self.status = DiagnosticsStatus::Done { path };
        true
    }

    /// Apply a failure with the same generation gate.
    pub fn apply_error(&mut self, request_id: u64, message: String) -> bool {
        if request_id != self.request_id {
            return false;
        }
        self.status = DiagnosticsStatus::Error { message };
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_idle() {
        let state = DiagnosticsState::new();
        assert!(matches!(state.status(), DiagnosticsStatus::Idle));
    }

    #[test]
    fn begin_dump_increments_request_id() {
        let mut state = DiagnosticsState::new();
        let id1 = state.begin_dump();
        assert_eq!(id1, 1);
        assert!(matches!(state.status(), DiagnosticsStatus::Running));
        let id2 = state.begin_dump();
        assert_eq!(id2, 2);
    }

    #[test]
    fn apply_done_writes_path_when_current() {
        let mut state = DiagnosticsState::new();
        let id = state.begin_dump();
        let applied = state.apply_done(id, PathBuf::from("/tmp/dump.json"));
        assert!(applied);
        match state.status() {
            DiagnosticsStatus::Done { path, .. } => {
                assert_eq!(path, &PathBuf::from("/tmp/dump.json"));
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn apply_done_drops_stale_generation() {
        let mut state = DiagnosticsState::new();
        let id1 = state.begin_dump();
        // Operator hits `s` again; bump generation.
        let _id2 = state.begin_dump();
        // Older reply arrives — should be ignored.
        let applied = state.apply_done(id1, PathBuf::from("/tmp/stale.json"));
        assert!(!applied);
        assert!(matches!(state.status(), DiagnosticsStatus::Running));
    }

    #[test]
    fn apply_error_records_message() {
        let mut state = DiagnosticsState::new();
        let id = state.begin_dump();
        let applied = state.apply_error(id, "permission denied".to_string());
        assert!(applied);
        match state.status() {
            DiagnosticsStatus::Error { message } => {
                assert_eq!(message, "permission denied");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }
}
