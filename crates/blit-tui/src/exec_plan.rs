//! audit-7d8: pure transfer-execution builders extracted from `main.rs`
//! (behavior-preserving — verbatim move, no logic change). These turn a
//! `PullKind` + endpoints into the `blit_app` execution/option structs the
//! spawn tasks hand to the transfer runners. All pure (no async, no
//! AppState, no I/O); the spawn_* tasks call them.

use crate::f3pull;
use blit_core::remote::endpoint::RemoteEndpoint;

/// d-55 R2 / d-57: build the `PullSyncOptions` for an F3 pull.
///
/// `mirror_mode` MUST live here, on the options — the wire
/// `TransferOperationSpec` is built from `options`
/// (`RemotePullClient::build_spec_from_options`), so it's
/// `options.mirror_mode` that tells the daemon to compute the
/// delete list. The execution-level `PullSyncExecution.mirror_mode`
/// is only the receive-side `track_paths` flag; setting it alone
/// (d-55 round 1) left the daemon emitting `MirrorMode::Off`, so
/// `apply_pull_mirror_purge` had no paths to delete and the
/// "mirror" silently behaved like a plain pull. The CLI sets the
/// options field (`blit-cli/src/transfers/remote.rs`); we match it.
///
/// d-57: a `Move` sets `require_complete_scan` so the daemon
/// refuses a partial source scan — mirroring the CLI's move guard
/// (`run_remote_pull_transfer_deferred(.., true)`). Deleting the
/// remote source after an incomplete copy would lose the files
/// that were skipped.
pub(crate) fn f3_pull_options(kind: f3pull::PullKind) -> blit_core::remote::pull::PullSyncOptions {
    use f3pull::PullKind;
    blit_core::remote::pull::PullSyncOptions {
        mirror_mode: kind == PullKind::Mirror,
        require_complete_scan: kind == PullKind::Move,
        ..blit_core::remote::pull::PullSyncOptions::default()
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
    use blit_app::endpoints::Endpoint;
    use blit_app::transfers::remote::PushExecution;
    use blit_core::generated::MirrorMode;
    let mirror = kind == f3pull::PullKind::Mirror;
    let remote_label = remote.display();
    PushExecution {
        source: Endpoint::Local(local_source),
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
        resume: false,
        resume_block_size: 0,
        remote_label,
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
        // The TUI doesn't surface a `--relay-via-cli` toggle yet, so
        // don't suggest it in transport-error hints.
        relay_fallback_suggestable: false,
        dst_label,
        detach: false,
    }
}
