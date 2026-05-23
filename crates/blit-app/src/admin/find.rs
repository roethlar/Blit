//! `find` — pattern-matched file/directory search across a remote
//! subtree (server-streaming RPC).
//!
//! Moved from `crates/blit-cli/src/find.rs` in A.0. Same callback
//! pattern as `du::stream` so a TUI can forward entries to its
//! event loop while the CLI prints inline.

use blit_core::generated::FindRequest;
use blit_core::remote::RemoteEndpoint;
use eyre::Result;
use serde::Serialize;

/// Input parameters for a `find` call. Mirrors the wire shape but
/// avoids leaking the prost-generated type into callers' API.
#[derive(Debug, Clone)]
pub struct FindParams {
    pub module: String,
    pub start_path: String,
    pub pattern: String,
    pub case_sensitive: bool,
    pub include_files: bool,
    pub include_directories: bool,
    pub max_results: u32,
}

/// One row from the streamed find response.
#[derive(Debug, Clone, Serialize)]
pub struct FindEntry {
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime_seconds: i64,
}

/// Stream find results from `remote`, invoking `on_entry` per
/// match. Caller handles formatting / collection.
pub async fn stream<F>(remote: &RemoteEndpoint, params: FindParams, mut on_entry: F) -> Result<()>
where
    F: FnMut(FindEntry) -> Result<()>,
{
    let uri = remote.control_plane_uri();
    let mut client = crate::client::connect_with_timeout(uri.clone()).await?;

    let mut stream = client
        .find(FindRequest {
            module: params.module,
            start_path: params.start_path,
            pattern: params.pattern,
            case_sensitive: params.case_sensitive,
            include_files: params.include_files,
            include_directories: params.include_directories,
            max_results: params.max_results,
        })
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
        .into_inner();

    while let Some(entry) = stream
        .message()
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
    {
        on_entry(FindEntry {
            path: entry.relative_path,
            is_dir: entry.is_dir,
            size: entry.size,
            mtime_seconds: entry.mtime_seconds,
        })?;
    }

    Ok(())
}
