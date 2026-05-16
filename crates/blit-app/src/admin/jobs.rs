//! `jobs list` — fetch the in-flight + recent-transfer snapshot
//! from a remote daemon's `GetState` RPC.
//!
//! Used by `blit jobs list <remote>` (sub-slice b-5) and, in
//! the future, the TUI's F1/F2 panes (A.1). Returns the raw
//! wire `DaemonState` plus typed helpers; the CLI/TUI layer
//! does its own formatting.

use blit_core::generated::blit_client::BlitClient;
use blit_core::generated::{DaemonState, GetStateRequest};
use blit_core::remote::endpoint::RemoteEndpoint;
use eyre::{Context, Result};

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
