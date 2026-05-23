//! `du` — disk usage for a remote subtree (server-streaming RPC).
//!
//! Moved from `crates/blit-cli/src/du.rs` in A.0. The streaming
//! shape is preserved via a callback so a TUI can plug an event-
//! channel forwarder while the CLI prints inline. Caller handles
//! all formatting.

use blit_core::generated::DiskUsageRequest;
use blit_core::remote::RemoteEndpoint;
use eyre::Result;
use serde::Serialize;

/// One row of the streamed disk-usage report. JSON field names
/// match the pre-A.0 CLI `--json` output exactly (`bytes` /
/// `files` / `dirs`); A.0 is no-behavior-change.
#[derive(Debug, Clone, Serialize)]
pub struct DiskUsageEntry {
    pub path: String,
    pub bytes: u64,
    pub files: u64,
    pub dirs: u64,
}

/// Stream disk-usage entries from `remote`, invoking `on_entry`
/// per row. The closure decides what to do with each entry; the
/// CLI's text mode prints, the CLI's JSON mode collects into a
/// vec, the TUI forwards to an event channel.
pub async fn stream<F>(
    remote: &RemoteEndpoint,
    module: String,
    start_path: String,
    max_depth: u32,
    mut on_entry: F,
) -> Result<()>
where
    F: FnMut(DiskUsageEntry) -> Result<()>,
{
    let uri = remote.control_plane_uri();
    let mut client = crate::client::connect_with_timeout(uri.clone()).await?;

    let mut stream = client
        .disk_usage(DiskUsageRequest {
            module,
            start_path,
            max_depth,
        })
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
        .into_inner();

    while let Some(entry) = stream
        .message()
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
    {
        on_entry(DiskUsageEntry {
            path: entry.relative_path,
            bytes: entry.byte_total,
            files: entry.file_count,
            dirs: entry.dir_count,
        })?;
    }

    Ok(())
}
