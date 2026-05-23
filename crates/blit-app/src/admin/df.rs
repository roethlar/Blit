//! `df` — filesystem stats for a remote module.
//!
//! Moved from `crates/blit-cli/src/df.rs` as part of A.0. The CLI
//! keeps the human / JSON formatting; this module just speaks gRPC
//! and returns the structured response.

use blit_core::generated::FilesystemStatsRequest;
use blit_core::remote::RemoteEndpoint;
use eyre::Result;
use serde::Serialize;

/// Filesystem usage for a remote module. Deserialized as-is by the
/// CLI's `--json` path and rendered as a three-line block by the
/// CLI's text path. The TUI will consume this directly for the
/// F1 daemon-detail pane.
#[derive(Debug, Clone, Serialize)]
pub struct FilesystemStats {
    pub module: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
}

/// Issue the `FilesystemStats` RPC against `remote` for `module`.
/// Caller handles formatting; presenter-agnostic.
pub async fn query(remote: &RemoteEndpoint, module: String) -> Result<FilesystemStats> {
    let uri = remote.control_plane_uri();
    let mut client = crate::client::connect_with_timeout(uri.clone()).await?;

    let response = client
        .filesystem_stats(FilesystemStatsRequest { module })
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
        .into_inner();

    Ok(FilesystemStats {
        module: response.module,
        total_bytes: response.total_bytes,
        used_bytes: response.used_bytes,
        free_bytes: response.free_bytes,
    })
}
