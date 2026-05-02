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
            // Check if this is the "use forward slashes" error - propagate it
            let err_msg = err.to_string();
            if err_msg.contains("forward slashes") {
                return Err(err);
            }
            // Check for remote-like patterns that failed parsing
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

/// Common gate shared by every remote-touching path.
fn ensure_remote_common(args: &TransferArgs) -> Result<()> {
    if args.dry_run {
        bail!("--dry-run is not supported for remote transfers");
    }
    if args.workers.is_some() {
        bail!("--workers limiter is not supported for remote transfers");
    }
    Ok(())
}

/// Gate for **remote-source / local-destination** pulls. Allows
/// `--checksum`: the pull-sync handshake negotiates checksum
/// support with the daemon and bails at the ack if the daemon has
/// `--no-server-checksums`. Closes R15-F1 of
/// `docs/reviews/followup_review_2026-05-02.md`: the previous
/// blanket `--checksum` rejection made the F11 ack-mismatch error
/// path unreachable from the CLI.
pub fn ensure_remote_pull_supported(args: &TransferArgs) -> Result<()> {
    ensure_remote_common(args)
}

/// Gate for **local-source / remote-destination** pushes and
/// **remote-remote** relays. The push protocol has no per-transfer
/// capability negotiation yet, so `--checksum` is rejected here
/// rather than silently degrading. Symmetric pull-side support
/// arrived through the F11 ack negotiation; push needs its own
/// equivalent before this gate can lift.
pub fn ensure_remote_push_supported(args: &TransferArgs) -> Result<()> {
    ensure_remote_common(args)?;
    if args.checksum {
        bail!(
            "--checksum is not supported for remote-destination transfers \
             (push protocol has no checksum capability negotiation today)"
        );
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
