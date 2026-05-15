//! `rm` — purge a path on a remote daemon.
//!
//! Moved from `crates/blit-cli/src/rm.rs` in A.0. The interactive
//! confirmation prompt and JSON / text formatters stay in `blit-cli`
//! (presentation). The CLI's `move` verb also consumes the
//! `delete_remote_path` helper for its source-side removal step.

use blit_core::generated::blit_client::BlitClient;
use blit_core::generated::PurgeRequest;
use blit_core::remote::endpoint::{RemoteEndpoint, RemotePath};
use eyre::{bail, Context, Result};
use std::path::PathBuf;

/// Issue a `Purge` RPC against `remote` for the given `module` and
/// `paths_to_delete`. Returns the count the daemon reports.
/// Caller handles confirmation prompting and presentation.
pub async fn purge(
    remote: &RemoteEndpoint,
    module: String,
    paths_to_delete: Vec<String>,
) -> Result<u64> {
    let uri = remote.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to {}", uri))?;

    let response = client
        .purge(PurgeRequest {
            module,
            paths_to_delete,
        })
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
        .into_inner();

    Ok(response.files_deleted)
}

/// Convenience wrapper around [`purge`] that derives the module
/// from the `RemoteEndpoint` and accepts a single relative path.
/// Used by `blit move` for its source-side removal step.
pub async fn delete_remote_path(remote: &RemoteEndpoint, rel_path: &str) -> Result<u64> {
    let (module, _) = extract_module_and_path(remote)?;
    purge(remote, module, vec![rel_path.to_string()]).await
}

/// Pull the (module, rel_path) pair out of a `RemoteEndpoint`,
/// rejecting bare-host (`Discovery`) variants with an rm-specific
/// error message. Pub so the CLI's interactive `rm` handler can
/// share the same validation.
pub fn extract_module_and_path(remote: &RemoteEndpoint) -> Result<(String, PathBuf)> {
    match &remote.path {
        RemotePath::Module { module, rel_path } => Ok((module.clone(), rel_path.clone())),
        RemotePath::Root { rel_path } => Ok((String::new(), rel_path.clone())),
        RemotePath::Discovery => {
            bail!("remote removal requires module syntax (e.g., server:/module/path)")
        }
    }
}
