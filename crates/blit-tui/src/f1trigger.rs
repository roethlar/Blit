//! F1 trigger-transfer modal state (d-58).
//!
//! TUI_DESIGN §5.1's F1 detail block advertises
//! `[t] trigger transfer`. Pressing `t` on a daemon row opens a
//! two-field modal — a remote **source** spec (prefilled to the
//! selected daemon's `host:port:/`) and a local **destination**
//! path — then runs the transfer.
//!
//! On commit the modal hands the parsed source + dest to the
//! verified F3 pull machine and the dispatcher jumps to F3 so the
//! operator watches the transfer in its existing footer — there's
//! no new execution path, reply channel, or progress UI here, just
//! field collection. d-59/d-60 add a copy/mirror/move kind cycle
//! (TUI_DESIGN §1 "copy / mirror / move … between any two
//! endpoints"); mirror and move route through F3's destructive
//! confirm gate. Push and remote→remote (delegated) triggers are
//! follow-ups.
//!
//! Flow:
//! 1. `t` on a daemon row → [`F1TriggerState::begin`] (source
//!    prefilled, focus on the dest field, copy kind).
//! 2. The operator edits either field; `Tab` toggles focus;
//!    Up/Down ([`F1TriggerState::cycle_kind`]) cycles
//!    copy → mirror → move.
//! 3. `Esc` → [`F1TriggerState::cancel`]. `Enter` →
//!    [`F1TriggerState::peek`] reads `(source, dest, kind)` WITHOUT
//!    closing (or `None` if either field is blank). The dispatcher
//!    validates + launches, then either [`F1TriggerState::close`]s
//!    on success or records [`F1TriggerState::set_error`] (keeping
//!    the modal open with an inline message) on a validation
//!    failure. d-62: edits clear a pending error.

use crate::f3pull::PullKind;

/// Which field the modal's keystrokes currently edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerField {
    Source,
    Dest,
}

#[derive(Debug, Clone)]
pub enum F1TriggerStatus {
    /// Modal closed.
    Idle,
    /// Modal open, collecting the two endpoints.
    Editing {
        source: String,
        dest: String,
        focus: TriggerField,
        /// d-59/d-60: transfer kind — copy / mirror / move.
        /// TUI_DESIGN §1 "copy / mirror / move … between any two
        /// endpoints". Cycled with the Up/Down arrows; mirror and
        /// move route through F3's destructive confirm after commit.
        kind: PullKind,
        /// d-62: a validation error from the last commit attempt
        /// (e.g. an unparseable endpoint). Shown in the prompt;
        /// cleared on the next edit. `None` = no error yet.
        error: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct F1TriggerState {
    status: F1TriggerStatus,
}

impl Default for F1TriggerState {
    fn default() -> Self {
        Self::new()
    }
}

impl F1TriggerState {
    pub fn new() -> Self {
        Self {
            status: F1TriggerStatus::Idle,
        }
    }

    pub fn status(&self) -> &F1TriggerStatus {
        &self.status
    }

    /// `true` while the modal is open — the input router consults
    /// this to route keystrokes to the modal instead of the F1
    /// dispatcher.
    pub fn is_editing(&self) -> bool {
        matches!(self.status, F1TriggerStatus::Editing { .. })
    }

    /// Open the modal with `source` prefilled (the selected
    /// daemon's `host:port:/`) and the cursor on the dest field —
    /// the source is usually almost right, the dest is always
    /// empty. No-op if already open.
    pub fn begin(&mut self, source: String) {
        if self.is_editing() {
            return;
        }
        self.status = F1TriggerStatus::Editing {
            source,
            dest: String::new(),
            focus: TriggerField::Dest,
            kind: PullKind::Copy,
            error: None,
        };
    }

    /// `Tab` — toggle which field is being edited.
    pub fn toggle_focus(&mut self) {
        if let F1TriggerStatus::Editing { focus, error, .. } = &mut self.status {
            *focus = match focus {
                TriggerField::Source => TriggerField::Dest,
                TriggerField::Dest => TriggerField::Source,
            };
            *error = None;
        }
    }

    /// d-59/d-60: Up/Down — cycle the transfer kind. `forward`
    /// (Up) goes Copy → Mirror → Move → Copy; `!forward` (Down)
    /// reverses.
    pub fn cycle_kind(&mut self, forward: bool) {
        if let F1TriggerStatus::Editing { kind, error, .. } = &mut self.status {
            *kind = match (*kind, forward) {
                (PullKind::Copy, true) => PullKind::Mirror,
                (PullKind::Mirror, true) => PullKind::Move,
                (PullKind::Move, true) => PullKind::Copy,
                (PullKind::Copy, false) => PullKind::Move,
                (PullKind::Mirror, false) => PullKind::Copy,
                (PullKind::Move, false) => PullKind::Mirror,
            };
            *error = None;
        }
    }

    /// Append a char to the focused field. d-62: clears any
    /// pending validation error (the operator is fixing the input).
    pub fn push_char(&mut self, c: char) {
        if let F1TriggerStatus::Editing {
            source,
            dest,
            focus,
            error,
            ..
        } = &mut self.status
        {
            match focus {
                TriggerField::Source => source.push(c),
                TriggerField::Dest => dest.push(c),
            }
            *error = None;
        }
    }

    /// Drop the last char from the focused field. Returns true if
    /// a char was popped.
    pub fn pop_char(&mut self) -> bool {
        if let F1TriggerStatus::Editing {
            source,
            dest,
            focus,
            error,
            ..
        } = &mut self.status
        {
            *error = None;
            match focus {
                TriggerField::Source => source.pop().is_some(),
                TriggerField::Dest => dest.pop().is_some(),
            }
        } else {
            false
        }
    }

    /// `Esc` — close the modal.
    pub fn cancel(&mut self) {
        self.status = F1TriggerStatus::Idle;
    }

    /// d-62: read the trimmed `(source, dest, kind)` WITHOUT
    /// closing. `None` when either field is blank (nothing to
    /// launch yet — the modal stays open silently). The dispatcher
    /// validates the endpoints and then either [`close`]s (on a
    /// successful launch) or records a [`set_error`] (keeping the
    /// modal open for the operator to fix the typo).
    pub fn peek(&self) -> Option<(String, String, PullKind)> {
        match &self.status {
            F1TriggerStatus::Editing {
                source, dest, kind, ..
            } => {
                let source = source.trim().to_string();
                let dest = dest.trim().to_string();
                if source.is_empty() || dest.is_empty() {
                    None
                } else {
                    Some((source, dest, *kind))
                }
            }
            F1TriggerStatus::Idle => None,
        }
    }

    /// d-62: close the modal after a successful launch.
    pub fn close(&mut self) {
        self.status = F1TriggerStatus::Idle;
    }

    /// d-62: record a validation error and keep the modal open so
    /// the operator can correct the input.
    pub fn set_error(&mut self, message: impl Into<String>) {
        if let F1TriggerStatus::Editing { error, .. } = &mut self.status {
            *error = Some(message.into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_idle() {
        let s = F1TriggerState::new();
        assert!(matches!(s.status(), F1TriggerStatus::Idle));
        assert!(!s.is_editing());
    }

    #[test]
    fn begin_prefills_source_and_focuses_dest() {
        let mut s = F1TriggerState::new();
        s.begin("nas:9031:/".to_string());
        assert!(s.is_editing());
        match s.status() {
            F1TriggerStatus::Editing {
                source,
                dest,
                focus,
                kind,
                ..
            } => {
                assert_eq!(source, "nas:9031:/");
                assert!(dest.is_empty());
                assert_eq!(*focus, TriggerField::Dest, "dest is the field to fill");
                assert_eq!(*kind, PullKind::Copy, "trigger starts in copy mode");
            }
            other => panic!("expected Editing, got {other:?}"),
        }
    }

    #[test]
    fn typing_edits_focused_field_and_tab_toggles() {
        let mut s = F1TriggerState::new();
        s.begin("nas:9031:/".to_string());
        // Focus starts on dest.
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        // Tab → source; append a module path.
        s.toggle_focus();
        for c in "home/docs".chars() {
            s.push_char(c);
        }
        match s.status() {
            F1TriggerStatus::Editing { source, dest, .. } => {
                assert_eq!(source, "nas:9031:/home/docs");
                assert_eq!(dest, "/tmp/out");
            }
            other => panic!("expected Editing, got {other:?}"),
        }
    }

    #[test]
    fn pop_char_edits_focused_field() {
        let mut s = F1TriggerState::new();
        s.begin("nas:9031:/".to_string());
        for c in "/tmpx".chars() {
            s.push_char(c);
        }
        assert!(s.pop_char());
        // On source (toggle), popping the prefill works too.
        s.toggle_focus();
        assert!(s.pop_char());
        match s.status() {
            F1TriggerStatus::Editing { source, dest, .. } => {
                assert_eq!(dest, "/tmp");
                assert_eq!(source, "nas:9031:");
            }
            other => panic!("expected Editing, got {other:?}"),
        }
    }

    #[test]
    fn cancel_closes_modal() {
        let mut s = F1TriggerState::new();
        s.begin("nas:9031:/".to_string());
        s.cancel();
        assert!(matches!(s.status(), F1TriggerStatus::Idle));
    }

    #[test]
    fn peek_yields_trimmed_pair_without_closing() {
        let mut s = F1TriggerState::new();
        s.begin("  nas:9031:/home/docs  ".to_string());
        for c in "  /tmp/out  ".chars() {
            s.push_char(c);
        }
        let (source, dest, kind) = s.peek().expect("both fields set");
        assert_eq!(source, "nas:9031:/home/docs");
        assert_eq!(dest, "/tmp/out");
        assert_eq!(kind, PullKind::Copy, "default mode is copy");
        assert!(s.is_editing(), "peek does NOT close — the caller does");
        // `close` is the explicit terminator.
        s.close();
        assert!(matches!(s.status(), F1TriggerStatus::Idle));
    }

    #[test]
    fn cycle_kind_advances_copy_mirror_move_and_peek_reports_it() {
        let mut s = F1TriggerState::new();
        s.begin("nas:9031:/home".to_string());
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        // Up cycles Copy → Mirror → Move → Copy.
        s.cycle_kind(true);
        assert!(
            matches!(
                s.status(),
                F1TriggerStatus::Editing {
                    kind: PullKind::Mirror,
                    ..
                }
            ),
            "Up: copy → mirror"
        );
        s.cycle_kind(true);
        match s.status() {
            F1TriggerStatus::Editing { kind, .. } => assert_eq!(*kind, PullKind::Move),
            other => panic!("expected Editing, got {other:?}"),
        }
        // Down reverses Move → Mirror.
        s.cycle_kind(false);
        let (_, _, kind) = s.peek().expect("set");
        assert_eq!(kind, PullKind::Mirror, "peek reports the cycled kind");
    }

    #[test]
    fn peek_is_none_when_a_field_is_blank() {
        let mut s = F1TriggerState::new();
        s.begin("nas:9031:/home".to_string());
        // No dest typed.
        assert!(s.peek().is_none());
        assert!(s.is_editing(), "blank dest keeps the modal open");
        // Type a dest, then blank the source → still None.
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        s.toggle_focus();
        while s.pop_char() {}
        assert!(s.peek().is_none(), "blank source → None");
        assert!(s.is_editing());
    }

    #[test]
    fn set_error_shows_and_edits_clear_it() {
        let mut s = F1TriggerState::new();
        s.begin("nas:9031:/home".to_string());
        s.set_error("invalid source: nas:9031:/home");
        match s.status() {
            F1TriggerStatus::Editing { error, .. } => {
                assert_eq!(error.as_deref(), Some("invalid source: nas:9031:/home"));
            }
            other => panic!("expected Editing, got {other:?}"),
        }
        // Any edit clears the error so stale feedback doesn't linger.
        s.push_char('x');
        match s.status() {
            F1TriggerStatus::Editing { error, .. } => assert!(error.is_none()),
            other => panic!("expected Editing, got {other:?}"),
        }
    }

    #[test]
    fn begin_is_noop_while_open() {
        let mut s = F1TriggerState::new();
        s.begin("nas:9031:/".to_string());
        for c in "/tmp".chars() {
            s.push_char(c);
        }
        // A second begin must not wipe the in-progress dest.
        s.begin("other:9031:/".to_string());
        match s.status() {
            F1TriggerStatus::Editing { source, dest, .. } => {
                assert_eq!(source, "nas:9031:/");
                assert_eq!(dest, "/tmp");
            }
            other => panic!("expected Editing, got {other:?}"),
        }
    }
}
