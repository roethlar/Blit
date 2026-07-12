//! Local→local transfer: the one app-layer chokepoint (CLI + TUI).
//!
//! Since otp-11 this rides the unified `TransferSession` over the
//! in-process transport (`blit_core::transfer_session::run_local_session`,
//! `docs/plan/OTP11_LOCAL_SESSION.md`): both roles in this process, the
//! same choreography as every remote session, bytes applied through the
//! local write strategy (clonefile / block-clone / copy_file_range where
//! available). The signature is unchanged — callers keep building
//! `LocalMirrorOptions` and consuming `LocalMirrorSummary`.
//!
//! The CLI's `build_local_options` still reads `TransferArgs`
//! directly, so this function accepts the already-built
//! `LocalMirrorOptions` struct from blit-core; the TUI builds its own
//! `LocalMirrorOptions` from its Verify form — same shape, no clap
//! coupling.

use blit_core::transfer_session::{LocalMirrorOptions, LocalMirrorSummary};
use eyre::{Context, Result};
use std::path::Path;

pub use blit_core::transfer_session::TransferOutcome;

/// Run a local→local copy / mirror to completion as one transfer
/// session. Returns the summary verbatim; caller handles presentation
/// (spinner clear, stdout / JSON / TUI render).
///
/// `options.mirror` decides copy vs mirror semantics inside the
/// session and also drives the error-message wording
/// ("failed to mirror …" vs "failed to copy …") when the underlying
/// call fails — matching the pre-A.0 CLI version of this site.
pub async fn run(
    src: &Path,
    dst: &Path,
    options: LocalMirrorOptions,
) -> Result<LocalMirrorSummary> {
    let mirror = options.mirror;
    blit_core::transfer_session::run_local_session(src, dst, options)
        .await
        .with_context(|| {
            format!(
                "failed to {} from {} to {}",
                if mirror { "mirror" } else { "copy" },
                src.display(),
                dst.display()
            )
        })
}
