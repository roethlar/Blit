//! audit-7d2: F3 status→display mapping helpers extracted from `main.rs`
//! (behavior-preserving — verbatim move, no logic change). Pure functions
//! that bridge the F3 pull / du / delete state machines to the
//! renderer-facing `screens::f3::*Display` structs, keeping the screens
//! layer out of the `f3pull`/`f3du`/`f3del` module internals.

use crate::{f3del, f3du, f3pull, screens};

/// d-35: bridge the F3 pull state machine to the
/// renderer-facing `F3PullDisplay` (lives in
/// `screens/f3.rs` so the screens layer doesn't reach
/// into the `f3pull` module's internals).
pub(crate) fn f3_pull_to_display(status: &f3pull::F3PullStatus) -> screens::f3::F3PullDisplay {
    use f3pull::F3PullStatus;
    use screens::f3::F3PullDisplay;
    match status {
        F3PullStatus::Idle => F3PullDisplay::Hidden,
        F3PullStatus::EnteringDest { dest, kind, .. } => F3PullDisplay::EnteringDest {
            dest: dest.clone(),
            // imperative verb ("pull"/"mirror"/"move")
            verb: kind.verbs().0,
        },
        // d-55/d-57: destructive confirm (y/N). The detail spells
        // out what gets removed so the operator knows the stakes.
        F3PullStatus::Confirm { dest, kind, .. } => F3PullDisplay::Confirm {
            dest: dest.clone(),
            verb: kind.verbs().0,
            detail: confirm_detail(*kind),
        },
        F3PullStatus::Running {
            dest,
            files,
            bytes,
            bytes_per_sec,
            kind,
            ..
        } => F3PullDisplay::Running {
            dest: dest.clone(),
            files: *files,
            bytes: *bytes,
            bytes_per_sec: *bytes_per_sec,
            // present participle ("pulling"/"mirroring"/"moving")
            verb: kind.verbs().1,
        },
        F3PullStatus::Done {
            files,
            bytes,
            dest,
            kind,
            deleted,
            ..
        } => F3PullDisplay::Done {
            files: *files,
            bytes: *bytes,
            dest: dest.clone(),
            // past tense ("pulled"/"mirrored"/"moved")
            verb: kind.verbs().2,
            deleted: *deleted,
        },
        F3PullStatus::Error { message, .. } => F3PullDisplay::Error {
            message: message.clone(),
        },
    }
}

/// d-55/d-57: the destructive-confirm detail line for each kind —
/// what the operator is about to lose. `Copy` is non-destructive
/// and never reaches the confirm gate.
fn confirm_detail(kind: f3pull::PullKind) -> &'static str {
    use f3pull::PullKind;
    match kind {
        PullKind::Mirror => "deletes extraneous",
        PullKind::Move => "deletes the remote source",
        PullKind::Copy => "",
    }
}

/// d-41: bridge the F3 du state machine to the renderer-facing
/// `F3DuDisplay`. This is where the path-match gating lives:
/// the result only shows while the cursor is still on the path
/// the du was computed for (`current_path`, the cursor's
/// canonical spec). A `Running`/`Done`/`Error` for any other
/// path renders as `Hidden`, so an outdated subtree total never
/// appears against the wrong row.
pub(crate) fn f3_du_to_display(
    status: &f3du::F3DuStatus,
    current_path: Option<&str>,
) -> screens::f3::F3DuDisplay {
    use f3du::F3DuStatus;
    use screens::f3::F3DuDisplay;
    let matches = |path: &str| current_path == Some(path);
    match status {
        F3DuStatus::Idle => F3DuDisplay::Hidden,
        F3DuStatus::Running { path, .. } if matches(path) => F3DuDisplay::Running,
        F3DuStatus::Done {
            path, bytes, files, ..
        } if matches(path) => F3DuDisplay::Done {
            bytes: *bytes,
            files: *files,
        },
        F3DuStatus::Error { path, message } if matches(path) => F3DuDisplay::Error {
            message: message.clone(),
        },
        _ => F3DuDisplay::Hidden,
    }
}

/// d-45: bridge the F3 delete state to the renderer-facing
/// `F3DelDisplay`. `Confirming` / `Deleting` always show (an
/// active operation); `Done` / `Error` are path-gated like the
/// du display — a stale outcome hides once the cursor leaves the
/// deleted path.
pub(crate) fn f3_del_to_display(
    status: &f3del::F3DelStatus,
    current_path: Option<&str>,
) -> screens::f3::F3DelDisplay {
    use f3del::F3DelStatus;
    use screens::f3::F3DelDisplay;
    // d-50: a single-row delete carries `gate_path = Some(spec)`
    // and hides its outcome once the cursor leaves that path
    // (the d-45 behavior). A batch carries `None` and shows the
    // outcome until the next action (its rows are gone after the
    // post-delete refresh anyway).
    let gated = |gate: &Option<String>| match gate {
        Some(p) => current_path == Some(p.as_str()),
        None => true,
    };
    match status {
        F3DelStatus::Idle => F3DelDisplay::Hidden,
        F3DelStatus::Confirming { label, .. } => F3DelDisplay::Confirming {
            label: label.clone(),
        },
        F3DelStatus::Deleting { .. } => F3DelDisplay::Deleting,
        F3DelStatus::Done {
            label,
            files_deleted,
            gate_path,
            ..
        } if gated(gate_path) => F3DelDisplay::Done {
            label: label.clone(),
            files_deleted: *files_deleted,
        },
        F3DelStatus::Error {
            label,
            message,
            gate_path,
            ..
        } if gated(gate_path) => F3DelDisplay::Error {
            message: message.clone(),
        },
        _ => F3DelDisplay::Hidden,
    }
}
