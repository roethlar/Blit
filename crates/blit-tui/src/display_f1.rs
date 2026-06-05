//! audit-7d3: F1 state→display mapping helpers extracted from `main.rs`
//! (behavior-preserving — verbatim move, no logic change). Pure functions
//! bridging the F1 trigger modal and F1 push state machines to the
//! renderer-facing `screens::f1::*` structs, keeping the screens layer out
//! of the `f1trigger`/`f1push` module internals.

use crate::{f1push, f1trigger, f3pull, screens};

/// d-58/d-59: bridge the F1 trigger modal to the renderer-facing
/// `TriggerPrompt`. `None` when the modal is closed.
pub(crate) fn f1_trigger_prompt(
    state: &f1trigger::F1TriggerState,
) -> Option<screens::f1::TriggerPrompt> {
    use f1trigger::{F1TriggerStatus, TriggerField};
    match state.status() {
        F1TriggerStatus::Idle => None,
        F1TriggerStatus::Editing {
            source,
            dest,
            focus,
            kind,
            error,
            confirming,
        } => Some(screens::f1::TriggerPrompt {
            source: source.clone(),
            dest: dest.clone(),
            source_focused: *focus == TriggerField::Source,
            // imperative verb ("copy"/"mirror"/"move"); pull's
            // verb triple uses "pull" for Copy, so spell "copy"
            // here to match the design's launcher vocabulary.
            mode: match kind {
                f3pull::PullKind::Copy => "copy",
                f3pull::PullKind::Mirror => "mirror",
                f3pull::PullKind::Move => "move",
            },
            // Mirror + move delete something → flag red.
            destructive: kind.is_destructive(),
            // d-62: inline validation error from the last commit.
            error: error.clone(),
            // d-65/d-71: a destructive transfer awaiting y/N confirm.
            // The detail spells out what gets deleted. Move's victim
            // depends on direction: a local→remote push move deletes
            // the LOCAL source; a remote→remote delegated move deletes
            // the REMOTE source — classify the source string to say
            // which.
            confirm_detail: confirming.then(|| match kind {
                f3pull::PullKind::Mirror => "deletes extraneous at dest",
                f3pull::PullKind::Move => move_delete_target_phrase(source),
                f3pull::PullKind::Copy => "",
            }),
        }),
    }
}

/// audit-h11: classify a move source for the F1 confirm-detail
/// renderer. Returns the human-facing phrase describing what the
/// confirmed move would delete.
///
/// Per `feedback_endpoint_parse_err` the 4-bucket classification
/// of endpoints is module/root=remote, bare-discovery & local=local,
/// **Err must reject**. The original implementation folded `Err(_)`
/// into a catch-all that printed "deletes the local source" —
/// silently lying about the deletion target when the source string
/// failed to parse.
///
/// Invariant: by the time this helper runs, `plan_f1_trigger` at
/// `main.rs:3588` has already rejected unparseable sources via
/// `TriggerOutcome::Rejected`. The state machine never reaches the
/// confirming state with an `Err` source. The `Err` arm here is a
/// future-proof guard against a refactor that loosens that gate —
/// `debug_assert!(false, …)` panics loudly in debug builds; release
/// builds degrade to a non-lying "(parse error)" phrase rather than
/// misclassifying the deletion target.
fn move_delete_target_phrase(source: &str) -> &'static str {
    use blit_app::endpoints::{parse_transfer_endpoint, Endpoint};
    match parse_transfer_endpoint(source) {
        Ok(Endpoint::Remote(_)) => "deletes the remote source",
        Ok(Endpoint::Local(_)) => "deletes the local source",
        Err(_) => {
            debug_assert!(
                false,
                "audit-h11: confirm_detail reached with unparseable \
                 source '{source}' — plan_f1_trigger at main.rs:3588 \
                 should have rejected before this state",
            );
            "deletes the source (parse error — refusing to classify)"
        }
    }
}

/// d-61: bridge the F1 push state to the renderer-facing
/// `PushStatusDisplay`. `None` when Idle (the discovery footer
/// shows instead).
pub(crate) fn f1_push_status(
    state: &f1push::F1PushState,
) -> Option<screens::f1::PushStatusDisplay> {
    use f1push::F1PushStatus;
    use screens::f1::PushStatusDisplay;
    match state.status() {
        F1PushStatus::Idle => None,
        F1PushStatus::Running {
            label,
            files,
            bytes,
            bytes_per_sec,
            kind,
            delegated,
            ..
        } => Some(PushStatusDisplay::Running {
            label: label.clone(),
            files: *files,
            bytes: *bytes,
            bytes_per_sec: *bytes_per_sec,
            // d-65/d-68: present-participle verb for the kind
            // (or "delegating" for a remote→remote delegated copy).
            verb: push_present_verb(*kind, *delegated),
        }),
        F1PushStatus::Done {
            files,
            bytes,
            label,
            kind,
            delegated,
            ..
        } => Some(PushStatusDisplay::Done {
            files: *files,
            bytes: *bytes,
            label: label.clone(),
            // d-65/d-68: past-tense verb for the kind.
            verb: push_past_verb(*kind, *delegated),
        }),
        F1PushStatus::Error {
            message,
            kind,
            delegated,
            ..
        } => Some(PushStatusDisplay::Error {
            message: message.clone(),
            verb: push_past_verb(*kind, *delegated),
        }),
    }
}

/// d-65: push footer verbs by kind. Copy reads as "push" (not
/// "pull") since this is the local→remote direction. d-68/d-70: a
/// remote→remote delegated copy reads "delegating/delegated" (the
/// CLI host isn't in the byte path, so neither push nor pull fits);
/// a delegated mirror reads "mirroring/mirrored" — the destructive
/// dest-purge is the salient thing and the footer label shows the
/// remote dest, so the delegated context stays clear.
fn push_present_verb(kind: f3pull::PullKind, delegated: bool) -> &'static str {
    match (delegated, kind) {
        (true, f3pull::PullKind::Copy) => "delegating",
        (_, f3pull::PullKind::Mirror) => "mirroring",
        (_, f3pull::PullKind::Move) => "moving",
        (false, f3pull::PullKind::Copy) => "pushing",
    }
}

fn push_past_verb(kind: f3pull::PullKind, delegated: bool) -> &'static str {
    match (delegated, kind) {
        (true, f3pull::PullKind::Copy) => "delegated",
        (_, f3pull::PullKind::Mirror) => "mirrored",
        (_, f3pull::PullKind::Move) => "moved",
        (false, f3pull::PullKind::Copy) => "pushed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// audit-h11: a remote `host:/module/` source classifies as
    /// remote (the move would delete the remote source).
    #[test]
    fn move_delete_target_phrase_classifies_remote_source() {
        assert_eq!(
            move_delete_target_phrase("host-a:/m/"),
            "deletes the remote source"
        );
    }

    /// audit-h11: a remote `host://root-path` source classifies
    /// as remote.
    #[test]
    fn move_delete_target_phrase_classifies_root_remote_source() {
        assert_eq!(
            move_delete_target_phrase("host-a://root/sub"),
            "deletes the remote source"
        );
    }

    /// audit-h11: a plain filesystem path classifies as local.
    #[test]
    fn move_delete_target_phrase_classifies_local_source() {
        assert_eq!(
            move_delete_target_phrase("/Users/x/src"),
            "deletes the local source"
        );
    }

    /// audit-h11: pinning the load-bearing property — the helper
    /// returns DIFFERENT phrases for Local vs Remote (no silent
    /// fold to "local source"). Pre-fix, `_ => "deletes the local
    /// source"` meant a remote source that failed to parse, or any
    /// future Endpoint variant added without updating this match,
    /// would have been misclassified as local.
    #[test]
    fn move_delete_target_phrase_local_and_remote_differ() {
        let local = move_delete_target_phrase("/tmp/src");
        let remote = move_delete_target_phrase("host-a:/m/");
        assert_ne!(
            local, remote,
            "audit-h11: local and remote sources must produce \
             distinct phrases — a fold would have made these \
             identical and silently lied about a remote-source move"
        );
        assert!(local.contains("local"));
        assert!(remote.contains("remote"));
    }
}
