use crate::cli::TransferArgs;
use eyre::{bail, Result};
use std::path::PathBuf;

use blit_core::remote::{RemoteEndpoint, RemotePath};

#[derive(Debug, Clone)]
pub enum Endpoint {
    Local(PathBuf),
    Remote(RemoteEndpoint),
}

pub fn parse_transfer_endpoint(input: &str) -> Result<Endpoint> {
    match RemoteEndpoint::parse(input) {
        Ok(endpoint) => Ok(Endpoint::Remote(endpoint)),
        Err(err) => {
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

pub fn ensure_remote_transfer_supported(args: &TransferArgs) -> Result<()> {
    if args.dry_run {
        bail!("--dry-run is not supported for remote transfers");
    }
    if args.checksum {
        bail!("--checksum is not supported for remote transfers");
    }
    if args.workers.is_some() {
        bail!("--workers limiter is not supported for remote transfers");
    }
    Ok(())
}

pub fn ensure_remote_destination_supported(remote: &RemoteEndpoint) -> Result<()> {
    match &remote.path {
        RemotePath::Module { .. } | RemotePath::Root { .. } => Ok(()),
        RemotePath::Discovery => {
            bail!(
                "remote destination must include a module or root (e.g., server:/module/ or server://path)"
            )
        }
    }
}

pub fn ensure_remote_source_supported(remote: &RemoteEndpoint) -> Result<()> {
    match remote.path {
        RemotePath::Module { .. } | RemotePath::Root { .. } => Ok(()),
        RemotePath::Discovery => {
            bail!(
                "remote source must include a module or root (e.g., server:/module/ or server://path)"
            )
        }
    }
}
