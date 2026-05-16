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
}
