//! Endpoint shim. All endpoint parsing / formatting /
//! validation logic — including the remote-transfer support
//! gates — lives in `blit_app::endpoints`. The two functions
//! defined here are paper-thin clap-arg adapters that map
//! `&TransferArgs` → primitive booleans before delegating.
//!
//! Re-exports preserve the call sites at
//! `crate::transfers::endpoints::{Endpoint, parse_transfer_endpoint,
//! format_remote_endpoint, ensure_remote_destination_supported,
//! ensure_remote_source_supported}` so the CLI doesn't need
//! import changes after the gate move.

use crate::cli::TransferArgs;
use eyre::Result;

pub use blit_app::endpoints::{
    ensure_remote_destination_supported, ensure_remote_source_supported, format_remote_endpoint,
    parse_transfer_endpoint, Endpoint,
};

/// Clap-shaped wrapper for the pull gate.
/// Library counterpart: [`blit_app::endpoints::ensure_remote_pull_supported`].
pub fn ensure_remote_pull_supported(args: &TransferArgs) -> Result<()> {
    blit_app::endpoints::ensure_remote_pull_supported(args.dry_run, args.workers.is_some())
}

/// Clap-shaped wrapper for the push gate.
/// Library counterpart: [`blit_app::endpoints::ensure_remote_push_supported`].
pub fn ensure_remote_push_supported(args: &TransferArgs) -> Result<()> {
    blit_app::endpoints::ensure_remote_push_supported(
        args.dry_run,
        args.workers.is_some(),
        args.checksum,
    )
}
