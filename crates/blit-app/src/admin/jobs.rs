//! `jobs list` — fetch the in-flight + recent-transfer snapshot
//! from a remote daemon's `GetState` RPC.
//!
//! Used by `blit jobs list <remote>` (sub-slice b-5) and, in
//! the future, the TUI's F1/F2 panes (A.1). Returns the raw
//! wire `DaemonState` plus typed helpers; the CLI/TUI layer
//! does its own formatting.

use blit_core::generated::blit_client::BlitClient;
use blit_core::generated::{CancelJobRequest, DaemonState, GetStateRequest};
use blit_core::remote::endpoint::RemoteEndpoint;
use eyre::{Context, Result};
use tonic::Code;

/// Issue the `GetState` RPC against `remote`. `recent_limit = 0`
/// asks the daemon for its default ring depth (50 today).
/// Larger non-zero values truncate the response server-side;
/// values larger than the ring return everything the ring
/// holds, no error.
pub async fn query(remote: &RemoteEndpoint, recent_limit: u32) -> Result<DaemonState> {
    let uri = remote.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to {}", uri))?;

    let response = client
        .get_state(GetStateRequest { recent_limit })
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?;

    Ok(response.into_inner())
}

/// Outcome of a `CancelJob` RPC. The wire surface encodes
/// cancel / not-found / unsupported via gRPC status codes;
/// this enum is the typed view CLI / TUI consumers match on
/// without re-deriving status semantics each time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelJobOutcome {
    /// The daemon fired the cancellation token. The handler
    /// will tear down on its next `.await` resolve.
    Cancelled { transfer_id: String },
    /// No active transfer matched the requested id.
    NotFound { transfer_id: String },
    /// The transfer exists but its kind doesn't honor
    /// cancellation from another client (push / pull /
    /// pull_sync — CLI is in the byte path).
    Unsupported {
        transfer_id: String,
        message: String,
    },
}

/// Issue the `CancelJob` RPC against `remote`. Errors only on
/// transport failures or an unexpected status; outcomes that
/// are part of the contract (NotFound, FailedPrecondition,
/// Ok) get mapped onto [`CancelJobOutcome`] for the caller to
/// render.
pub async fn cancel(remote: &RemoteEndpoint, transfer_id: &str) -> Result<CancelJobOutcome> {
    let uri = remote.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to {}", uri))?;

    let result = client
        .cancel_job(CancelJobRequest {
            transfer_id: transfer_id.to_string(),
        })
        .await;

    match result {
        Ok(response) => {
            let body = response.into_inner();
            // The daemon echoes the transfer_id back; fall
            // back to the request id if the server returned
            // empty (shouldn't happen with the current
            // implementation, but defensive).
            let id = if body.transfer_id.is_empty() {
                transfer_id.to_string()
            } else {
                body.transfer_id
            };
            Ok(CancelJobOutcome::Cancelled { transfer_id: id })
        }
        Err(status) => match status.code() {
            Code::NotFound => Ok(CancelJobOutcome::NotFound {
                transfer_id: transfer_id.to_string(),
            }),
            Code::FailedPrecondition => Ok(CancelJobOutcome::Unsupported {
                transfer_id: transfer_id.to_string(),
                message: status.message().to_string(),
            }),
            _ => Err(eyre::eyre!(
                "CancelJob failed ({}): {}",
                status.code(),
                status.message()
            )),
        },
    }
}

/// Single-poll snapshot for the watch path. Either we found
/// the row alive in `GetState.active[]`, or it had already
/// drained into `recent[]`, or it isn't in either (transfer
/// completed before the ring rotated it out, or never
/// existed).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum WatchSnapshot {
    /// Transfer is in flight. Embeds the active row.
    Active(blit_core::generated::ActiveTransfer),
    /// Transfer has completed (success or failure). Embeds
    /// the record from the ring.
    Finished(blit_core::generated::TransferRecord),
    /// No row found in either table. The transfer either
    /// never existed or rotated out of the recent ring.
    NotFound,
}

/// Look the transfer up in a freshly fetched daemon state.
/// Pure helper used by the watch loop; carved out so unit
/// tests can drive it with synthetic `DaemonState` values
/// without standing up a tonic server.
///
/// `recent[]` precedence intentional: if the same id somehow
/// appears in both (it shouldn't — Drop removes-then-pushes
/// atomically under the table lock), we report Finished
/// rather than Active because the row's terminal record is
/// the authoritative one.
pub fn watch_snapshot(state: &DaemonState, transfer_id: &str) -> WatchSnapshot {
    for r in state.recent.iter().rev() {
        if r.transfer_id == transfer_id {
            return WatchSnapshot::Finished(r.clone());
        }
    }
    for a in &state.active {
        if a.transfer_id == transfer_id {
            return WatchSnapshot::Active(a.clone());
        }
    }
    WatchSnapshot::NotFound
}

/// Human-readable label for a `TransferKind` proto enum value.
/// Stable across releases — the CLI formatter and any TUI both
/// render to this string. Unknown values (from a forward-version
/// daemon emitting a kind we don't know yet) render as
/// `"unknown"` so the row stays visible.
pub fn kind_label(kind: i32) -> &'static str {
    use blit_core::generated::TransferKind;
    match TransferKind::try_from(kind) {
        Ok(TransferKind::Push) => "push",
        Ok(TransferKind::Pull) => "pull",
        Ok(TransferKind::PullSync) => "pull_sync",
        Ok(TransferKind::DelegatedPull) => "delegated_pull",
        Ok(TransferKind::Unspecified) | Err(_) => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_core::generated::TransferKind;

    #[test]
    fn kind_label_maps_known_variants() {
        assert_eq!(kind_label(TransferKind::Push as i32), "push");
        assert_eq!(kind_label(TransferKind::Pull as i32), "pull");
        assert_eq!(kind_label(TransferKind::PullSync as i32), "pull_sync");
        assert_eq!(
            kind_label(TransferKind::DelegatedPull as i32),
            "delegated_pull"
        );
    }

    #[test]
    fn kind_label_unknown_or_unspecified_is_safe() {
        assert_eq!(kind_label(TransferKind::Unspecified as i32), "unknown");
        // A value the enum doesn't know about (forward-version
        // daemon) shouldn't panic and shouldn't be silently
        // miscategorised.
        assert_eq!(kind_label(999), "unknown");
    }

    fn empty_state() -> DaemonState {
        DaemonState::default()
    }

    fn active_row(id: &str) -> blit_core::generated::ActiveTransfer {
        blit_core::generated::ActiveTransfer {
            transfer_id: id.to_string(),
            kind: TransferKind::DelegatedPull as i32,
            peer: "p".to_string(),
            module: "m".to_string(),
            path: "/".to_string(),
            start_unix_ms: 1,
            bytes_completed: 0,
            bytes_total: 0,
        }
    }

    fn recent_row(id: &str, ok: bool, err: &str) -> blit_core::generated::TransferRecord {
        blit_core::generated::TransferRecord {
            transfer_id: id.to_string(),
            kind: TransferKind::DelegatedPull as i32,
            peer: "p".to_string(),
            module: "m".to_string(),
            path: "/".to_string(),
            start_unix_ms: 1,
            duration_ms: 100,
            bytes: 0,
            files: 0,
            ok,
            error_message: err.to_string(),
        }
    }

    #[test]
    fn watch_snapshot_finds_active_row() {
        let mut state = empty_state();
        state.active.push(active_row("t-1"));
        match watch_snapshot(&state, "t-1") {
            WatchSnapshot::Active(a) => assert_eq!(a.transfer_id, "t-1"),
            other => panic!("expected Active, got {other:?}"),
        }
    }

    #[test]
    fn watch_snapshot_finds_finished_row() {
        let mut state = empty_state();
        state.recent.push(recent_row("t-2", true, ""));
        match watch_snapshot(&state, "t-2") {
            WatchSnapshot::Finished(r) => {
                assert_eq!(r.transfer_id, "t-2");
                assert!(r.ok);
            }
            other => panic!("expected Finished, got {other:?}"),
        }
    }

    #[test]
    fn watch_snapshot_not_found_when_absent_from_both() {
        let state = empty_state();
        assert!(matches!(
            watch_snapshot(&state, "t-nope"),
            WatchSnapshot::NotFound
        ));
    }

    #[test]
    fn watch_snapshot_prefers_finished_when_both_present() {
        // If a Drop-then-push race somehow leaves the same
        // id in both tables, the terminal record wins —
        // that's the authoritative outcome.
        let mut state = empty_state();
        state.active.push(active_row("t-3"));
        state
            .recent
            .push(recent_row("t-3", false, "handler failed"));
        match watch_snapshot(&state, "t-3") {
            WatchSnapshot::Finished(r) => {
                assert!(!r.ok);
                assert_eq!(r.error_message, "handler failed");
            }
            other => panic!("expected Finished, got {other:?}"),
        }
    }
}
