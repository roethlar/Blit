//! F4 Verify form state. The Verify pane lets the operator
//! pick two local paths and run `blit_app::check::compare_trees`
//! to see how they differ — same code path as `blit check`.
//!
//! d-2-f4-verify scope: text-input form, run-on-Enter, result
//! rendering as match / diff / missing counts.
//!
//! d-6 adds the mode toggle: `H` flips between size+mtime
//! (the default — fast, what rsync does without `--checksum`)
//! and checksum mode (slower, but catches content edits that
//! preserve mtime). Remote endpoints stay deferred (matches
//! the CLI's `blit check` semantics — local paths only, per
//! TUI_DESIGN §5.4).

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
    Running {
        /// d-8: captured at `begin_run()`. Done preserves
        /// it so the result banner can show duration.
        started_at: Instant,
    },
    /// Last run succeeded; `result` carries the counts.
    Done {
        result: CheckResult,
        started_at: Instant,
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
    /// d-6: `false` (default) means size+mtime compare —
    /// fast, matches rsync's default. `true` flips to
    /// per-file checksum, which catches content edits
    /// that preserve mtime (timestamps copied from a
    /// snapshot, manual `touch` after edit, etc.). Toggled
    /// by `H` from F4.
    use_checksum: bool,
    /// d-7: `false` (default) means two-way compare —
    /// reports `missing-on-src` AND `missing-on-dst`.
    /// `true` flips to one-way — skips the dst-walk so the
    /// operator only sees what's missing on dst (matches
    /// `blit check --one-way`). Useful for "did the src
    /// reach dst?" without caring about extras at dst.
    /// Toggled by `O` from F4.
    one_way: bool,
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
            use_checksum: false,
            one_way: false,
        }
    }

    /// `true` if the next compare_trees run will use
    /// per-file checksum, `false` for size+mtime.
    pub fn use_checksum(&self) -> bool {
        self.use_checksum
    }

    /// `true` if the next compare_trees run will skip
    /// `missing-on-src` (one-way mode). `false` (default)
    /// reports both directions.
    pub fn one_way(&self) -> bool {
        self.one_way
    }

    /// Flip the checksum mode and invalidate any in-flight
    /// or completed run. The mode change is meaningful
    /// only at the next `compare_trees` call, so a Done
    /// banner from the OLD mode would be misleading after
    /// the flip — collapse to Idle and bump the generation
    /// (same shape as `invalidate_run` for edits).
    pub fn toggle_checksum(&mut self) {
        self.use_checksum = !self.use_checksum;
        self.invalidate_run();
    }

    /// Flip the one-way / two-way mode. Same invalidation
    /// contract as [`Self::toggle_checksum`]: a Done
    /// banner from a two-way run would be misleading after
    /// flipping to one-way (the missing-on-src count would
    /// have meant something different).
    pub fn toggle_one_way(&mut self) {
        self.one_way = !self.one_way;
        self.invalidate_run();
    }

    pub fn focus(&self) -> VerifyFocus {
        self.focus
    }

    pub fn status(&self) -> &VerifyStatus {
        &self.status
    }

    /// `true` while a `compare_trees` call is in flight.
    /// Symmetric to `TransferState::is_running` — used by
    /// the d-9 live-tick gate so the event loop only burns
    /// a wakeup timer while something is actually running.
    pub fn is_running(&self) -> bool {
        matches!(self.status, VerifyStatus::Running { .. })
    }

    /// d-18: wipe the currently focused field's text in
    /// one keystroke (terminal "kill-line" convention,
    /// bound to Ctrl-U in the F4 dispatcher). Same
    /// invalidation contract as character edits — a
    /// pending or completed run for the prior text gets
    /// dropped. No-op when focus is None (Ctrl-U falls
    /// through to the regular dispatcher then). Returns
    /// `true` if a field was actually mutated.
    pub fn clear_focused_field(&mut self) -> bool {
        let mutated = match self.focus {
            VerifyFocus::Source => {
                let had_content = !self.source.is_empty();
                self.source.clear();
                had_content
            }
            VerifyFocus::Destination => {
                let had_content = !self.destination.is_empty();
                self.destination.clear();
                had_content
            }
            VerifyFocus::None => false,
        };
        if mutated {
            self.invalidate_run();
        }
        mutated
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
    /// focus is None. Edits invalidate any prior result
    /// AND drop any in-flight run via `invalidate_run`
    /// (d-2 round-2 fix: the operator may type while a
    /// `compare_trees` call is still blocking; the reply
    /// must NOT land against the now-edited paths).
    pub fn insert_char(&mut self, c: char) {
        let mutated = match self.focus {
            VerifyFocus::Source => {
                self.source.push(c);
                true
            }
            VerifyFocus::Destination => {
                self.destination.push(c);
                true
            }
            VerifyFocus::None => false,
        };
        if mutated {
            self.invalidate_run();
        }
    }

    /// Delete the last char from the focused field. Same
    /// invalidation contract as `insert_char`.
    pub fn backspace(&mut self) {
        let mutated = match self.focus {
            VerifyFocus::Source => self.source.pop().is_some(),
            VerifyFocus::Destination => self.destination.pop().is_some(),
            VerifyFocus::None => false,
        };
        if mutated {
            self.invalidate_run();
        }
    }

    /// Drop any in-flight run result. Bumps the generation
    /// so a still-pending compare_trees task's reply gets
    /// dropped on arrival, AND collapses Done/Error to
    /// Idle so the displayed counts can't outlive the
    /// fields they were computed against.
    fn invalidate_run(&mut self) {
        // Bumping the request_id is what actually makes a
        // future `apply_result` / `apply_error` for the
        // old run return false. Generation matches the
        // pattern used by `begin_run`.
        self.request_id += 1;
        // Collapse user-visible status. Running → Idle
        // tells the renderer to stop showing
        // "running compare_trees..." for a run whose
        // result will now be ignored.
        if matches!(
            self.status,
            VerifyStatus::Done { .. } | VerifyStatus::Error { .. } | VerifyStatus::Running { .. }
        ) {
            self.status = VerifyStatus::Idle;
        }
    }

    /// Begin a run. Bumps the generation, flips to
    /// `Running`, returns the new generation so the
    /// spawned task can tag its reply.
    pub fn begin_run(&mut self) -> u64 {
        self.request_id += 1;
        self.status = VerifyStatus::Running {
            started_at: Instant::now(),
        };
        self.request_id
    }

    /// Apply a result if the generation matches. Returns
    /// `true` on apply, `false` if the reply was stale.
    pub fn apply_result(&mut self, request_id: u64, result: CheckResult) -> bool {
        if request_id != self.request_id {
            return false;
        }
        // d-8: preserve `started_at` from the in-flight
        // Running variant so the Done banner can show
        // total duration. Defensive fallback as in
        // `transfer::apply_done` for the same reason.
        let started_at = match self.status {
            VerifyStatus::Running { started_at } => started_at,
            _ => Instant::now(),
        };
        self.status = VerifyStatus::Done {
            result,
            started_at,
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
            && !matches!(self.status, VerifyStatus::Running { .. })
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

    /// d-2 round-2 regression: typing into the form while
    /// a `compare_trees` call is still running MUST drop
    /// the in-flight reply on arrival. Without this guard
    /// the operator types `/tmp/a` + `/tmp/b` + Enter, then
    /// immediately keeps typing — the blocking task's old
    /// reply lands and labels the new paths with the old
    /// comparison.
    #[test]
    fn edit_during_running_drops_in_flight_reply() {
        let mut state = VerifyState::new();
        state.cycle_focus(); // Source
        state.insert_char('a');
        state.cycle_focus(); // Destination
        state.insert_char('b');
        let gen = state.begin_run();
        assert!(matches!(state.status(), VerifyStatus::Running { .. }));

        // Operator keeps typing while the run is in flight.
        state.insert_char('c');
        // Status collapses back to Idle: we're no longer
        // really running the gen the worker is computing.
        assert!(matches!(state.status(), VerifyStatus::Idle));

        // The in-flight reply lands — apply_result must
        // refuse to write it, because the generation
        // moved on.
        let applied = state.apply_result(gen, empty_check_result());
        assert!(!applied);
        assert!(matches!(state.status(), VerifyStatus::Idle));

        // Same for an Err reply from the same generation.
        let applied_err = state.apply_error(gen, "boom".to_string());
        assert!(!applied_err);
    }

    /// Backspace during Running has the same guard as
    /// `insert_char`.
    #[test]
    fn backspace_during_running_drops_in_flight_reply() {
        let mut state = VerifyState::new();
        state.cycle_focus(); // Source
        state.insert_char('a');
        state.cycle_focus(); // Destination
        state.insert_char('b');
        let gen = state.begin_run();
        // Cycle through None back to Source so the next
        // backspace mutates a non-empty field.
        state.cycle_focus(); // None
        state.cycle_focus(); // Source
        state.backspace();
        assert!(matches!(state.status(), VerifyStatus::Idle));
        let applied = state.apply_result(gen, empty_check_result());
        assert!(!applied);
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
        assert!(matches!(state.status(), VerifyStatus::Running { .. }));
    }

    #[test]
    fn clear_focus_resets_to_none() {
        let mut state = VerifyState::new();
        state.cycle_focus();
        state.cycle_focus();
        state.clear_focus();
        assert_eq!(state.focus(), VerifyFocus::None);
    }

    // d-6: checksum mode toggle.

    #[test]
    fn new_state_uses_size_mtime_compare() {
        let state = VerifyState::new();
        assert!(
            !state.use_checksum(),
            "default mode matches rsync default (no --checksum)"
        );
    }

    #[test]
    fn toggle_checksum_flips_the_flag() {
        let mut state = VerifyState::new();
        state.toggle_checksum();
        assert!(state.use_checksum(), "first toggle → checksum mode");
        state.toggle_checksum();
        assert!(!state.use_checksum(), "second toggle → back to size+mtime");
    }

    /// Flipping mode must invalidate any prior result.
    /// A Done from the OLD mode would mislead the operator
    /// after the flip, because the displayed counts no
    /// longer correspond to the mode shown in the header.
    #[test]
    fn toggle_checksum_invalidates_done_result() {
        let mut state = VerifyState::new();
        state.source = "/tmp/a".to_string();
        state.destination = "/tmp/b".to_string();
        let gen = state.begin_run();
        state.apply_result(gen, empty_check_result());
        assert!(matches!(state.status(), VerifyStatus::Done { .. }));
        state.toggle_checksum();
        assert!(matches!(state.status(), VerifyStatus::Idle));
    }

    /// Flipping mode while a compare_trees run is in
    /// flight must drop the eventual reply — its counts
    /// belong to the prior mode.
    #[test]
    fn toggle_checksum_drops_in_flight_reply() {
        let mut state = VerifyState::new();
        state.source = "/tmp/a".to_string();
        state.destination = "/tmp/b".to_string();
        let gen = state.begin_run();
        state.toggle_checksum();
        let applied = state.apply_result(gen, empty_check_result());
        assert!(
            !applied,
            "reply tagged with the pre-toggle gen must be dropped"
        );
        // Status collapsed back to Idle, not stuck on
        // Running with a result that'll never arrive.
        assert!(matches!(state.status(), VerifyStatus::Idle));
    }

    // d-7: one-way toggle.

    #[test]
    fn new_state_uses_two_way_compare() {
        let state = VerifyState::new();
        assert!(
            !state.one_way(),
            "default matches `blit check` default (two-way)"
        );
    }

    #[test]
    fn toggle_one_way_flips_the_flag() {
        let mut state = VerifyState::new();
        state.toggle_one_way();
        assert!(state.one_way(), "first toggle → one-way");
        state.toggle_one_way();
        assert!(!state.one_way(), "second toggle → back to two-way");
    }

    /// Same invalidation contract as the checksum toggle —
    /// a Done banner from a two-way run would mean something
    /// different after the flip (the missing-on-src count
    /// would now correspond to "skipped" not "discovered").
    #[test]
    fn toggle_one_way_invalidates_done_result() {
        let mut state = VerifyState::new();
        state.source = "/tmp/a".to_string();
        state.destination = "/tmp/b".to_string();
        let gen = state.begin_run();
        state.apply_result(gen, empty_check_result());
        assert!(matches!(state.status(), VerifyStatus::Done { .. }));
        state.toggle_one_way();
        assert!(matches!(state.status(), VerifyStatus::Idle));
    }

    /// The two toggles are independent. Flipping one
    /// doesn't touch the other.
    #[test]
    fn checksum_and_one_way_toggles_are_independent() {
        let mut state = VerifyState::new();
        state.toggle_checksum();
        assert!(state.use_checksum());
        assert!(!state.one_way(), "one_way unchanged");
        state.toggle_one_way();
        assert!(state.use_checksum(), "checksum unchanged");
        assert!(state.one_way());
    }

    // d-8: started_at preserved across begin_run → apply_result.

    // d-18: Ctrl-U clears the focused field.

    #[test]
    fn clear_focused_field_clears_source_when_source_focused() {
        let mut state = VerifyState::new();
        state.cycle_focus(); // → Source
        state.insert_char('a');
        state.insert_char('b');
        assert_eq!(state.source, "ab");
        let mutated = state.clear_focused_field();
        assert!(mutated);
        assert_eq!(state.source, "");
        // Destination untouched.
        assert_eq!(state.destination, "");
    }

    #[test]
    fn clear_focused_field_clears_destination_when_destination_focused() {
        let mut state = VerifyState::new();
        state.cycle_focus(); // Source
        state.insert_char('x');
        state.cycle_focus(); // Destination
        state.insert_char('y');
        let mutated = state.clear_focused_field();
        assert!(mutated);
        assert_eq!(state.destination, "");
        // Source preserved — clear is per-field.
        assert_eq!(state.source, "x");
    }

    #[test]
    fn clear_focused_field_noop_when_no_focus() {
        let mut state = VerifyState::new();
        assert_eq!(state.focus(), VerifyFocus::None);
        let mutated = state.clear_focused_field();
        assert!(!mutated);
    }

    #[test]
    fn clear_focused_field_returns_false_for_already_empty_field() {
        let mut state = VerifyState::new();
        state.cycle_focus(); // Source — empty
        let mutated = state.clear_focused_field();
        assert!(
            !mutated,
            "clearing an empty field is a no-op (don't bump request_id needlessly)"
        );
    }

    #[test]
    fn clear_focused_field_invalidates_pending_run() {
        let mut state = VerifyState::new();
        state.source = "src".to_string();
        state.destination = "dst".to_string();
        let id = state.begin_run();
        state.cycle_focus(); // Source
        let mutated = state.clear_focused_field();
        assert!(mutated);
        // Stale Run reply must drop on arrival.
        let applied = state.apply_result(id, empty_check_result());
        assert!(!applied, "stale reply must drop after field clear");
    }

    #[test]
    fn apply_result_preserves_started_at_from_running() {
        let mut state = VerifyState::new();
        state.source = "/tmp/a".to_string();
        state.destination = "/tmp/b".to_string();
        let id = state.begin_run();
        let started = match state.status() {
            VerifyStatus::Running { started_at } => *started_at,
            other => panic!("expected Running after begin_run, got {other:?}"),
        };
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert!(state.apply_result(id, empty_check_result()));
        match state.status() {
            VerifyStatus::Done {
                started_at,
                finished_at,
                ..
            } => {
                assert_eq!(*started_at, started);
                assert!(finished_at >= started_at);
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }
}
