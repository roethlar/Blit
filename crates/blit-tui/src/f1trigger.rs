//! F1 trigger-transfer modal state (d-58).
//!
//! TUI_DESIGN §5.1's F1 detail block advertises
//! `[t] trigger transfer`. Pressing `t` on a daemon row opens a
//! two-field modal — a remote **source** spec (prefilled to the
//! selected daemon's `host:port:/`) and a local **destination**
//! path — then runs the transfer.
//!
//! This first slice covers the remote→local **pull** only: on
//! commit the modal hands the parsed source + dest to the
//! verified F3 pull machine (`F3PullState::start_pull`) and the
//! dispatcher jumps to F3 so the operator watches the pull in its
//! existing footer. So there's no new execution path, reply
//! channel, or progress UI here — just field collection. Push,
//! mirror, and remote→remote (delegated) triggers are follow-ups.
//!
//! Flow:
//! 1. `t` on a daemon row → [`F1TriggerState::begin`] (source
//!    prefilled, focus on the dest field).
//! 2. The operator edits either field; `Tab` toggles focus.
//! 3. `Esc` → [`F1TriggerState::cancel`]. `Enter` →
//!    [`F1TriggerState::take`] yields `(source, dest)` for the
//!    dispatcher to parse + launch (or `None` if either is blank).

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
        };
    }

    /// `Tab` — toggle which field is being edited.
    pub fn toggle_focus(&mut self) {
        if let F1TriggerStatus::Editing { focus, .. } = &mut self.status {
            *focus = match focus {
                TriggerField::Source => TriggerField::Dest,
                TriggerField::Dest => TriggerField::Source,
            };
        }
    }

    /// Append a char to the focused field.
    pub fn push_char(&mut self, c: char) {
        if let F1TriggerStatus::Editing {
            source,
            dest,
            focus,
        } = &mut self.status
        {
            match focus {
                TriggerField::Source => source.push(c),
                TriggerField::Dest => dest.push(c),
            }
        }
    }

    /// Drop the last char from the focused field. Returns true if
    /// a char was popped.
    pub fn pop_char(&mut self) -> bool {
        if let F1TriggerStatus::Editing {
            source,
            dest,
            focus,
        } = &mut self.status
        {
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

    /// `Enter` — close the modal and yield the trimmed
    /// `(source, dest)` for the dispatcher to parse + launch.
    /// Returns `None` (and stays open) when either field is blank
    /// — there's nothing to launch yet.
    pub fn take(&mut self) -> Option<(String, String)> {
        let (source, dest) = match &self.status {
            F1TriggerStatus::Editing { source, dest, .. } => {
                (source.trim().to_string(), dest.trim().to_string())
            }
            F1TriggerStatus::Idle => return None,
        };
        if source.is_empty() || dest.is_empty() {
            return None;
        }
        self.status = F1TriggerStatus::Idle;
        Some((source, dest))
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
            } => {
                assert_eq!(source, "nas:9031:/");
                assert!(dest.is_empty());
                assert_eq!(*focus, TriggerField::Dest, "dest is the field to fill");
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
    fn take_yields_trimmed_pair_and_closes() {
        let mut s = F1TriggerState::new();
        s.begin("  nas:9031:/home/docs  ".to_string());
        for c in "  /tmp/out  ".chars() {
            s.push_char(c);
        }
        let (source, dest) = s.take().expect("both fields set");
        assert_eq!(source, "nas:9031:/home/docs");
        assert_eq!(dest, "/tmp/out");
        assert!(matches!(s.status(), F1TriggerStatus::Idle), "take closes");
    }

    #[test]
    fn take_keeps_modal_open_when_a_field_is_blank() {
        let mut s = F1TriggerState::new();
        s.begin("nas:9031:/home".to_string());
        // No dest typed.
        assert!(s.take().is_none());
        assert!(s.is_editing(), "blank dest keeps the modal open");
        // Blank source (clear it) also blocks.
        s.toggle_focus();
        while s.pop_char() {}
        for c in "/tmp/out".chars() {
            // refill dest after toggling back
            let _ = c;
        }
        // Refill dest on its field.
        s.toggle_focus();
        for c in "/tmp/out".chars() {
            s.push_char(c);
        }
        // Source is now blank → still blocked.
        assert!(s.take().is_none());
        assert!(s.is_editing());
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
