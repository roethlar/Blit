//! `list-modules` — fetch the module list exported by a remote
//! daemon.
//!
//! Moved from `crates/blit-cli/src/list_modules.rs` in A.0. The
//! CLI wraps this with its `--json` / text formatters and with
//! the smart-dispatch `blit list <bare-host>` path that also lands
//! here.

use blit_core::generated::blit_client::BlitClient;
use blit_core::generated::ListModulesRequest;
use blit_core::remote::endpoint::RemoteEndpoint;
use eyre::{Context, Result};
use serde::Serialize;

/// One row of the module list. Direct shape of the wire response
/// but decoupled from the prost-generated `ModuleInfo` type.
#[derive(Debug, Clone, Serialize)]
pub struct Module {
    pub name: String,
    pub path: String,
    pub read_only: bool,
}

/// Issue the `ListModules` RPC against `remote` and return the
/// modules. Caller handles formatting.
pub async fn query(remote: &RemoteEndpoint) -> Result<Vec<Module>> {
    let uri = remote.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to {}", uri))?;

    let response = client
        .list_modules(ListModulesRequest {})
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
        .into_inner();

    Ok(response
        .modules
        .into_iter()
        .map(|m| Module {
            name: m.name,
            path: m.path,
            read_only: m.read_only,
        })
        .collect())
}
