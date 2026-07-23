//! audit-7d8: pure transfer-execution builders extracted from `main.rs`
//! (behavior-preserving — verbatim move, no logic change). These turn a
//! `PullKind` + endpoints into the `blit_app` execution/option structs the
//! spawn tasks hand to the transfer runners. All pure (no async, no
//! AppState, no I/O); the spawn_* tasks call them.

use crate::f3pull;
use blit_core::remote::endpoint::RemoteEndpoint;

/// d-55 R2 / d-57: build the [`DelegatedSpecOptions`] for a DELEGATED
/// F1 remote→remote transfer (the wire `TransferOperationSpec` is
/// built from these via `delegated_spec_from_options` — the builder
/// relocated to `operation_spec` when the old pull driver was deleted
/// at otp-10c-2).
///
/// The name is historical: F3 pulls ride the unified session since
/// otp-10b-2; only [`build_delegated_execution`] consumes this.
///
/// `mirror_mode` here tells the daemon to compute the delete list;
/// `require_complete_scan` on a `Move` makes the source daemon refuse
/// a partial scan (deleting the remote source after an incomplete
/// copy would lose the skipped files) — though delegated move is
/// rejected upstream, the mapping stays safe.
pub(crate) fn f3_pull_options(
    kind: f3pull::PullKind,
) -> blit_core::remote::transfer::operation_spec::DelegatedSpecOptions {
    use blit_core::remote::transfer::operation_spec::DelegatedSpecOptions;
    use f3pull::PullKind;
    DelegatedSpecOptions {
        mirror_mode: kind == PullKind::Mirror,
        require_complete_scan: kind == PullKind::Move,
        // codex otp-10b-2 F2: a move deletes the remote source after
        // the transfer, so its wire compare must never be
        // metadata-shaped — ignore_times maps to IGNORE_TIMES with top
        // precedence in the spec builder (delegated move is rejected
        // upstream in the TUI today; this keeps the mapping safe if
        // that gate ever loosens).
        ignore_times: kind == PullKind::Move,
        ..DelegatedSpecOptions::default()
    }
}

/// otp-10b-2: build the `PullExecution` for an F3 pull on the unified
/// transfer session — the pull mirror of [`build_f1_push_execution`],
/// unit-pinnable for the same reason (d-65 R2).
///
/// Mirror enables the session's one delete rule (this DESTINATION
/// diffs the complete source manifest against its tree at SourceDone
/// and deletes extraneous local entries — no post-pull purge step);
/// `FilteredSubset` matches the CLI pull's `--delete-scope` default,
/// and with no filter UI on the trigger it behaves as a whole-tree
/// scope. Mirror needs no scan gate here: the session refuses an
/// incomplete-scan mirror on its own. A `Move` sets
/// `require_complete_scan` (the remote source is deleted after the
/// pull — d-57) and maps through `move_comparison_mode` (transfer
/// unconditionally; codex otp-10a F1 mirrored on pull).
pub(crate) fn build_f3_pull_execution(
    remote: RemoteEndpoint,
    dest_root: std::path::PathBuf,
    kind: f3pull::PullKind,
) -> blit_app::transfers::remote::PullExecution {
    use blit_app::transfers::compare::{comparison_mode, move_comparison_mode, CompareFlags};
    use blit_app::transfers::remote::PullExecution;
    use blit_core::generated::MirrorMode;
    let mirror = kind == f3pull::PullKind::Mirror;
    let remote_label = remote.display();
    // No compare toggles on the F3 trigger — the default flags map to
    // SizeMtime (copy/mirror) / IgnoreTimes (move), through the same
    // one mapping the CLI verbs use.
    let compare_mode = if kind == f3pull::PullKind::Move {
        move_comparison_mode(CompareFlags::default())
    } else {
        comparison_mode(CompareFlags::default())
    };
    PullExecution {
        remote,
        dest_root,
        // No filter UI on the F3 trigger — the session scans everything.
        filter: None,
        mirror_mode: mirror,
        mirror_kind: if mirror {
            MirrorMode::FilteredSubset
        } else {
            MirrorMode::Off
        },
        force_grpc: false,
        trace_data_plane: false,
        require_complete_scan: kind == f3pull::PullKind::Move,
        drop_windows_metadata: false,
        resume: false,
        resume_block_size: 0,
        compare_mode,
        ignore_existing: false,
        remote_label,
        lifecycle_trace: Default::default(),
    }
}

/// d-61: spawn a local→remote COPY push for an F1 trigger.
/// d-65 R2: build the `PushExecution` for an F1 trigger push.
/// Extracted from `spawn_f1_push` so the mirror-safety options are
/// unit-pinnable (the reviewer flagged the inline construction as
/// untested). Mirror sets `mirror_mode` + `MirrorMode::All` — the
/// daemon deletes destination entries absent from the source — AND
/// `require_complete_scan`, so a partial local enumeration can never
/// drive that purge (an under-scanned source would otherwise make
/// valid remote files look extraneous). This matches the CLI's
/// `require_complete_scan: mirror_mode` in
/// `crates/blit-cli/src/transfers/remote.rs`. Copy/move push never
/// delete at the dest, so they leave both off — an incomplete scan
/// there only under-copies, which is safe and retryable.
pub(crate) fn build_f1_push_execution(
    local_source: std::path::PathBuf,
    remote: RemoteEndpoint,
    kind: f3pull::PullKind,
) -> blit_app::transfers::remote::PushExecution {
    use blit_app::transfers::compare::{comparison_mode, move_comparison_mode, CompareFlags};
    use blit_app::transfers::remote::PushExecution;
    use blit_core::generated::MirrorMode;
    let mirror = kind == f3pull::PullKind::Mirror;
    let remote_label = remote.display();
    PushExecution {
        source: local_source,
        remote,
        // No filter UI on the F1 trigger — the session scans everything.
        filter: None,
        mirror_mode: mirror,
        mirror_kind: if mirror {
            MirrorMode::All
        } else {
            MirrorMode::Off
        },
        force_grpc: false,
        trace_data_plane: false,
        require_complete_scan: mirror,
        drop_windows_metadata: false,
        resume: false,
        resume_block_size: 0,
        // codex otp-10a F1 via the ONE mapping (codex otp-10b-2 F6):
        // a move deletes the local source after the push, so it maps
        // through `move_comparison_mode` (transfer unconditionally —
        // the TUI exposes no compare toggles, so the flags are all
        // default). Copy/mirror map through the copy mapping
        // (SizeMtime default).
        compare_mode: if kind == f3pull::PullKind::Move {
            move_comparison_mode(CompareFlags::default())
        } else {
            comparison_mode(CompareFlags::default())
        },
        // No ignore-existing toggle on the F1 trigger.
        ignore_existing: false,
        remote_label,
        lifecycle_trace: Default::default(),
    }
}

/// d-70: build the `DelegatedPullExecution` for an F1 remote→remote
/// transfer. Extracted from `spawn_f1_delegated_pull` so the
/// mirror option is unit-pinnable (cf. the d-65 push builder). The
/// options come from `f3_pull_options(kind)`: copy → no flags;
/// mirror → `mirror_mode` on, `require_complete_scan` OFF. The OFF is
/// deliberate and matches the CLI's delegated path
/// (`crates/blit-cli/src/transfers/mod.rs` passes
/// `require_complete_scan = false` for delegated copy/mirror) — in a
/// delegated transfer the *daemons* enumerate, not this client, so
/// the d-65 client-side partial-scan guard doesn't apply. (Move,
/// which the CLI scans-completely for, is rejected upstream.) Always
/// attached (`detach: false`); detached/F2-visible delegation is a
/// follow-up.
pub(crate) fn build_delegated_execution(
    src: RemoteEndpoint,
    dst: RemoteEndpoint,
    kind: f3pull::PullKind,
) -> blit_app::transfers::remote::DelegatedPullExecution {
    let dst_label = dst.display();
    blit_app::transfers::remote::DelegatedPullExecution {
        src,
        dst,
        options: f3_pull_options(kind),
        trace_data_plane: false,
        dst_label,
        detach: false,
        lifecycle_trace: Default::default(),
    }
}
