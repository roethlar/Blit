//! `scan` — mDNS discovery of blit daemons on the LAN.
//!
//! Moved from `crates/blit-cli/src/scan.rs` in A.0. The TUI's F1
//! Daemons screen consumes the same `MdnsDiscoveredService` shape
//! the CLI does; the CLI keeps its JSON / text formatters.
//!
//! `MdnsDiscoveredService` is re-exported from `blit-core` rather
//! than wrapped, because it already exposes the §3.2 TXT
//! enrichment accessors (`modules()`, `module_count()`,
//! `delegation_enabled()`) the CLI consumes.

use eyre::{Context, Result};
use std::time::Duration;

pub use blit_core::mdns::MdnsDiscoveredService;

/// Block-on-tokio wrapper around `blit_core::mdns::discover`.
/// Discovery is synchronous (the mdns-sd crate exposes a sync
/// API); the wrapper spawns the blocking call onto a worker
/// thread so async callers don't stall the runtime.
pub async fn discover(wait: Duration) -> Result<Vec<MdnsDiscoveredService>> {
    tokio::task::spawn_blocking(move || blit_core::mdns::discover(wait))
        .await
        .context("mDNS discovery task panicked")?
}
