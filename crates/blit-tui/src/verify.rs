//! F4 Verify form state. The Verify pane lets the operator
//! pick two local paths and run `blit_app::check::compare_trees`
//! to see how they differ — same code path as `blit check`.
//!
//! d-2-f4-verify scope: text-input form, run-on-Enter, result
//! rendering as match / diff / missing counts. Mode toggle
//! (size+mtime vs checksum) and remote endpoints are
//! deferred (matches the CLI's `blit check` semantics —
//! local paths only, per TUI_DESIGN §5.4).

use blit_app::check::CheckResult;
use std::time::Instant;

/// Which form field has the cursor. `None` means the
/// operator's keystrokes go through the pane's regular
/// action dispatcher (c/d/e for profile, r for refresh,
/// etc.). When `Source` or `Destination`, char input
/// edits the field instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyFocus {
    None,
    Source,
    Destination,
}

impl VerifyFocus {
    /// `true` when keystrokes should be interpreted as
    /// text edits rather than action keys.
    pub fn is_editing(self) -> bool {
        !matches!(self, VerifyFocus::None)
    }
}

#[derive(Debug)]
pub enum VerifyStatus {
    /// No run yet (or the form was just edited).
    Idle,
    /// `compare_trees` is running on a blocking task.
    Running,
    /// Last run succeeded; `result` carries the counts.
    Done {
        result: CheckResult,
        #[allow(dead_code)]
        finished_at: Instant,
    },
    /// Last run failed.
    Error { message: String },
}

#[derive(Debug)]
pub struct VerifyState {
    pub source: String,
    pub destination: String,
    focus: VerifyFocus,
    status: VerifyStatus,
    /// Generation counter for in-flight runs. Same
    /// pattern as `transfers_setup_gen` in AppState —
    /// the run task tags its reply with the gen and the
    /// apply arm drops mismatches.
    request_id: u64,
}

impl Default for VerifyState {
    fn default() -> Self {
        Self::new()
    }
}

impl VerifyState {
    pub fn new() -> Self {
        Self {
            source: String::new(),
            destination: String::new(),
            focus: VerifyFocus::None,
            status: VerifyStatus::Idle,
            request_id: 0,
        }
    }

    pub fn focus(&self) -> VerifyFocus {
        self.focus
    }

    pub fn status(&self) -> &VerifyStatus {
        &self.status
    }

    /// Cycle the focus: None → Source → Destination → None.
    /// Called by `Tab` on F4.
    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            VerifyFocus::None => VerifyFocus::Source,
            VerifyFocus::Source => VerifyFocus::Destination,
            VerifyFocus::Destination => VerifyFocus::None,
        };
    }

    /// Drop focus (Esc clears editing mode without quitting
    /// the TUI).
    pub fn clear_focus(&mut self) {
        self.focus = VerifyFocus::None;
    }

    /// Append a character to the focused field. No-op when
    /// focus is None.
    pub fn insert_char(&mut self, c: char) {
        match self.focus {
            VerifyFocus::Source => self.source.push(c),
            VerifyFocus::Destination => self.destination.push(c),
            VerifyFocus::None => {}
        }
        // Editing invalidates any prior run result.
        if matches!(
            self.status,
            VerifyStatus::Done { .. } | VerifyStatus::Error { .. }
        ) {
            self.status = VerifyStatus::Idle;
        }
    }

    /// Delete the last char from the focused field. No-op
    /// when focus is None or the field is empty.
    pub fn backspace(&mut self) {
        match self.focus {
            VerifyFocus::Source => {
                self.source.pop();
            }
            VerifyFocus::Destination => {
                self.destination.pop();
            }
            VerifyFocus::None => {}
        }
        if matches!(
            self.status,
            VerifyStatus::Done { .. } | VerifyStatus::Error { .. }
        ) {
            self.status = VerifyStatus::Idle;
        }
    }

    /// Begin a run. Bumps the generation, flips to
    /// `Running`, returns the new generation so the
    /// spawned task can tag its reply.
    pub fn begin_run(&mut self) -> u64 {
        self.request_id += 1;
        self.status = VerifyStatus::Running;
        self.request_id
    }

    /// Apply a result if the generation matches. Returns
    /// `true` on apply, `false` if the reply was stale.
    pub fn apply_result(&mut self, request_id: u64, result: CheckResult) -> bool {
        if request_id != self.request_id {
            return false;
        }
        self.status = VerifyStatus::Done {
            result,
            finished_at: Instant::now(),
        };
        true
    }

    /// Apply a run failure. Same generation gate.
    pub fn apply_error(&mut self, request_id: u64, message: String) -> bool {
        if request_id != self.request_id {
            return false;
        }
        self.status = VerifyStatus::Error { message };
        true
    }

    /// `true` when both fields are non-empty and we're
    /// not already running. Caller (Enter handler) uses
    /// this to decide whether kicking a run makes sense.
    pub fn can_run(&self) -> bool {
        !self.source.trim().is_empty()
            && !self.destination.trim().is_empty()
            && !matches!(self.status, VerifyStatus::Running)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_check_result() -> CheckResult {
        CheckResult {
            matching: 0,
            differing: Vec::new(),
            missing_on_src: Vec::new(),
            missing_on_dest: Vec::new(),
            errors: Vec::new(),
        }
    }

    #[test]
    fn new_state_starts_idle_with_no_focus() {
        let state = VerifyState::new();
        assert_eq!(state.focus(), VerifyFocus::None);
        assert!(matches!(state.status(), VerifyStatus::Idle));
        assert!(state.source.is_empty());
        assert!(state.destination.is_empty());
    }

    #[test]
    fn cycle_focus_walks_three_states_then_returns_to_none() {
        let mut state = VerifyState::new();
        state.cycle_focus();
        assert_eq!(state.focus(), VerifyFocus::Source);
        state.cycle_focus();
        assert_eq!(state.focus(), VerifyFocus::Destination);
        state.cycle_focus();
        assert_eq!(state.focus(), VerifyFocus::None);
    }

    #[test]
    fn focus_is_editing_only_in_field_states() {
        assert!(!VerifyFocus::None.is_editing());
        assert!(VerifyFocus::Source.is_editing());
        assert!(VerifyFocus::Destination.is_editing());
    }

    #[test]
    fn insert_char_targets_focused_field() {
        let mut state = VerifyState::new();
        // None-focused → ignored.
        state.insert_char('a');
        assert!(state.source.is_empty());

        state.cycle_focus(); // Source
        state.insert_char('/');
        state.insert_char('t');
        state.insert_char('m');
        state.insert_char('p');
        assert_eq!(state.source, "/tmp");
        assert!(state.destination.is_empty());

        state.cycle_focus(); // Destination
        state.insert_char('d');
        assert_eq!(state.destination, "d");
        assert_eq!(state.source, "/tmp");
    }

    #[test]
    fn backspace_pops_focused_field() {
        let mut state = VerifyState::new();
        state.cycle_focus(); // Source
        state.insert_char('a');
        state.insert_char('b');
        state.backspace();
        assert_eq!(state.source, "a");
        // Empty: backspace is a no-op.
        state.backspace();
        state.backspace();
        assert!(state.source.is_empty());
    }

    #[test]
    fn editing_invalidates_done_or_error_status() {
        let mut state = VerifyState::new();
        state.cycle_focus(); // Source
        state.insert_char('a');
        let gen = state.begin_run();
        assert!(state.apply_result(gen, empty_check_result()));
        assert!(matches!(state.status(), VerifyStatus::Done { .. }));
        // Edit invalidates the result so the user knows
        // the displayed counts are stale.
        state.insert_char('b');
        assert!(matches!(state.status(), VerifyStatus::Idle));
    }

    #[test]
    fn can_run_requires_both_fields_and_not_running() {
        let mut state = VerifyState::new();
        assert!(!state.can_run());
        state.cycle_focus(); // Source
        state.insert_char('a');
        assert!(!state.can_run()); // destination empty
        state.cycle_focus(); // Destination
        state.insert_char('b');
        assert!(state.can_run());
        // Whitespace-only doesn't count.
        state.source.clear();
        state.source.push_str("   ");
        assert!(!state.can_run());
    }

    #[test]
    fn apply_result_drops_stale_generation() {
        let mut state = VerifyState::new();
        state.cycle_focus();
        state.insert_char('a');
        let gen1 = state.begin_run();
        // Operator edits the form before the run finishes;
        // editing flips status back to Idle but
        // begin_run again would bump generation.
        let _gen2 = state.begin_run();
        // Old reply arrives — should be ignored.
        let applied = state.apply_result(gen1, empty_check_result());
        assert!(!applied);
        // State still Running (from gen2).
        assert!(matches!(state.status(), VerifyStatus::Running));
    }

    #[test]
    fn clear_focus_resets_to_none() {
        let mut state = VerifyState::new();
        state.cycle_focus();
        state.cycle_focus();
        state.clear_focus();
        assert_eq!(state.focus(), VerifyFocus::None);
    }
}
