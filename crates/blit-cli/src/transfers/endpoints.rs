//! Endpoint shim. The pure parsing / formatting / source-and-
//! destination-validation helpers moved to `blit_app::endpoints`
//! in Phase 5 A.0. This file keeps the clap-coupled gate
//! functions (which read `TransferArgs`) until they're reshaped
//! to take primitive inputs in a later A.0 commit.
//!
//! Re-exports preserve the call sites at
//! `crate::transfers::endpoints::{Endpoint, parse_transfer_endpoint,
//! format_remote_endpoint, ensure_remote_destination_supported,
//! ensure_remote_source_supported}` so this commit moves the code
//! without touching consumers.

use crate::cli::TransferArgs;
use eyre::{bail, Result};

pub use blit_app::endpoints::{
    ensure_remote_destination_supported, ensure_remote_source_supported, format_remote_endpoint,
    parse_transfer_endpoint, Endpoint,
};

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
