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
                f3pull::PullKind::Move => {
                    use blit_app::endpoints::{parse_transfer_endpoint, Endpoint};
                    match parse_transfer_endpoint(source) {
                        Ok(Endpoint::Remote(_)) => "deletes the remote source",
                        _ => "deletes the local source",
                    }
                }
                f3pull::PullKind::Copy => "",
            }),
        }),
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
