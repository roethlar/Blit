//! Local→local transfer: the orchestration call.
//!
//! First sub-slice of the transfers track. Moves only the
//! `spawn_blocking` lift + the orchestrator invocation + the
//! error-context wrapping out of `crates/blit-cli/src/transfers
//! /local.rs`. Everything else (clap-arg → `LocalMirrorOptions`
//! translation, indicatif spinner, summary print) stays in the
//! CLI for now; those will move when `transfers/mod.rs` lands
//! (it owns `build_filter` etc., which the options builder
//! depends on).
//!
//! The CLI's `build_local_options` still reads `TransferArgs`
//! directly, so this function accepts the already-built
//! `LocalMirrorOptions` struct from blit-core. The TUI's future
//! local-transfer trigger will build its own `LocalMirrorOptions`
//! from a TUI input modal and call this function — same shape,
//! no clap coupling.

use blit_core::orchestrator::{LocalMirrorOptions, LocalMirrorSummary, TransferOrchestrator};
use eyre::{Context, Result};
use std::path::Path;

pub use blit_core::orchestrator::TransferOutcome;

/// Run a local→local copy / mirror to completion. Wraps the
/// blocking orchestrator call in `spawn_blocking` so async
/// callers don't stall the runtime. Returns the summary
/// verbatim; caller handles presentation (spinner clear,
/// stdout / JSON / TUI render).
///
/// `mirror` is taken from `options.mirror`; the parameter exists
/// for the error-message wording ("failed to mirror" vs "failed
/// to copy") which was inline in the pre-A.0 CLI version.
pub async fn run(
    src: &Path,
    dst: &Path,
    options: LocalMirrorOptions,
) -> Result<LocalMirrorSummary> {
    let src = src.to_path_buf();
    let dst = dst.to_path_buf();
    let mirror = options.mirror;
    tokio::task::spawn_blocking(move || {
        TransferOrchestrator::new()
            .execute_local_mirror(&src, &dst, options)
            .with_context(|| {
                format!(
                    "failed to {} from {} to {}",
                    if mirror { "mirror" } else { "copy" },
                    src.display(),
                    dst.display()
                )
            })
    })
    .await?
}
