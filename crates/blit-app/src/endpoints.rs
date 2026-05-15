//! Endpoint parsing for transfer sources / destinations.
//!
//! Moved from `crates/blit-cli/src/transfers/endpoints.rs` as part
//! of the Phase 5 A.0 extraction. The clap-coupled gate functions
//! (which read `TransferArgs` fields) stay in `blit-cli` for now;
//! they'll move once their inputs are reshaped to primitives.

use blit_core::remote::{RemoteEndpoint, RemotePath};
use eyre::{bail, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Endpoint {
    Local(PathBuf),
    Remote(RemoteEndpoint),
}

/// Parse a CLI / TUI source-or-destination input into an `Endpoint`.
/// Recognizes `host:/module/path` and `host://path` shapes as remote;
/// anything else is taken as a local filesystem path. Forward-slash
/// errors propagate so callers can show the user a clean diagnostic
/// instead of silently treating a misformatted remote as a local
/// path.
pub fn parse_transfer_endpoint(input: &str) -> Result<Endpoint> {
    match RemoteEndpoint::parse(input) {
        Ok(endpoint) => Ok(Endpoint::Remote(endpoint)),
        Err(err) => {
            let err_msg = err.to_string();
            if err_msg.contains("forward slashes") {
                return Err(err);
            }
            // Anything that looks like a remote URI (scheme or
            // `host:/path`) must parse as remote; treating a typo'd
            // remote as a local path silently was a long-standing
            // footgun.
            if input.contains("://") || input.contains(":/") {
                Err(err)
            } else {
                Ok(Endpoint::Local(PathBuf::from(input)))
            }
        }
    }
}

pub fn format_remote_endpoint(remote: &RemoteEndpoint) -> String {
    remote.display()
}

/// Reject a `RemoteEndpoint` whose `path` is `Discovery` (a bare
/// host without module / root). Used as the destination-side gate.
pub fn ensure_remote_destination_supported(remote: &RemoteEndpoint) -> Result<()> {
    match &remote.path {
        RemotePath::Module { .. } | RemotePath::Root { .. } => Ok(()),
        RemotePath::Discovery => bail!(
            "remote destination must include a module or root (e.g., server:/module/ or server://path)"
        ),
    }
}

/// Source-side counterpart of [`ensure_remote_destination_supported`].
pub fn ensure_remote_source_supported(remote: &RemoteEndpoint) -> Result<()> {
    match remote.path {
        RemotePath::Module { .. } | RemotePath::Root { .. } => Ok(()),
        RemotePath::Discovery => bail!(
            "remote source must include a module or root (e.g., server:/module/ or server://path)"
        ),
    }
}
