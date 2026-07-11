//! Clap-arg adapter wrappers for the remote-transfer support
//! gates. The substantive logic lives in `blit_app::endpoints`.
//! Every other endpoint helper (`Endpoint`,
//! `parse_transfer_endpoint`, `format_remote_endpoint`,
//! `ensure_remote_destination_supported`,
//! `ensure_remote_source_supported`) is imported from
//! `blit_app::endpoints` directly at each call site rather than
//! re-exported through this module.

use crate::cli::TransferArgs;
use eyre::Result;

/// Clap-shaped wrapper for the pull gate.
/// Library counterpart: [`blit_app::endpoints::ensure_remote_pull_supported`].
pub fn ensure_remote_pull_supported(args: &TransferArgs) -> Result<()> {
    blit_app::endpoints::ensure_remote_pull_supported(args.dry_run, args.workers.is_some())
}

/// Clap-shaped wrapper for the push gate. `--checksum` passes since
/// otp-10b-2 (the session's Checksum compare is role-agnostic).
/// Library counterpart: [`blit_app::endpoints::ensure_remote_push_supported`].
pub fn ensure_remote_push_supported(args: &TransferArgs) -> Result<()> {
    blit_app::endpoints::ensure_remote_push_supported(args.dry_run, args.workers.is_some())
}
