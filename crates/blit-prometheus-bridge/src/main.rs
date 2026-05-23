//! `blit-prometheus-bridge` — scrape a blit daemon's `GetState`
//! and emit Prometheus metrics. A SEPARATE binary by design
//! (`TUI_DESIGN.md` §9 / Milestone E): the daemon never speaks
//! HTTP/Prometheus itself; this bridge translates its gRPC
//! `GetState` snapshot into the Prometheus text format.
//!
//! Slice 1 (bridge-1): one-shot — query once, print the metrics
//! to stdout, exit. This is the unit a `curl`-free smoke test or
//! a `textfile` collector can already use. A later slice adds the
//! long-running HTTP `/metrics` server + scrape loop on top of the
//! same `metrics::format_metrics` formatter.

mod metrics;

use blit_app::admin::jobs;
use blit_core::remote::endpoint::RemoteEndpoint;
use clap::Parser;
use eyre::{Context, Result};

#[derive(Parser)]
#[command(
    name = "blit-prometheus-bridge",
    about = "Scrape a blit daemon's GetState and print Prometheus metrics."
)]
struct Args {
    /// Daemon to scrape (`host` or `host:port`).
    #[arg(long)]
    remote: String,
    /// recent-runs ring depth to request — bounds
    /// `blit_recent_transfers`.
    #[arg(long, default_value_t = 50)]
    recent_limit: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let remote = RemoteEndpoint::parse(&args.remote)
        .with_context(|| format!("parsing remote endpoint '{}'", args.remote))?;
    let state = jobs::query(&remote, args.recent_limit)
        .await
        .with_context(|| format!("querying GetState from {}", remote.display()))?;
    print!("{}", metrics::format_metrics(&state));
    Ok(())
}
