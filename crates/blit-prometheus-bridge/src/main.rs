//! `blit-prometheus-bridge` — scrape a blit daemon's `GetState`
//! and emit Prometheus metrics. A SEPARATE binary by design
//! (`TUI_DESIGN.md` §9 / Milestone E): the daemon never speaks
//! HTTP/Prometheus itself; this bridge translates its gRPC
//! `GetState` snapshot into the Prometheus text format.
//!
//! Two modes:
//! - **one-shot** (bridge-1, default): query once, print the metrics
//!   to stdout, exit. Usable by a `curl`-free smoke test or a
//!   node_exporter `textfile` collector.
//! - **`--listen <addr>`** (bridge-2): run a long-lived HTTP server
//!   that serves `GET /metrics`, scraping the daemon afresh per
//!   request (pull model). Both modes share `metrics::format_metrics`.

mod metrics;
mod server;

use blit_app::admin::jobs;
use blit_core::remote::endpoint::RemoteEndpoint;
use clap::Parser;
use eyre::{Context, Result};
use std::net::SocketAddr;

#[derive(Parser)]
#[command(
    name = "blit-prometheus-bridge",
    about = "Scrape a blit daemon's GetState and expose Prometheus metrics."
)]
struct Args {
    /// Daemon to scrape (`host` or `host:port`).
    #[arg(long)]
    remote: String,
    /// recent-runs ring depth to request — bounds
    /// `blit_recent_transfers`.
    #[arg(long, default_value_t = 50)]
    recent_limit: u32,
    /// Serve `GET /metrics` on this address (e.g. `127.0.0.1:9119`)
    /// instead of printing once and exiting. Each scrape queries the
    /// daemon afresh.
    #[arg(long)]
    listen: Option<SocketAddr>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let remote = RemoteEndpoint::parse(&args.remote)
        .with_context(|| format!("parsing remote endpoint '{}'", args.remote))?;

    if let Some(addr) = args.listen {
        // bridge-2: long-running HTTP exporter.
        server::serve(addr, remote, args.recent_limit).await
    } else {
        // bridge-1: one-shot print.
        let state = jobs::query(&remote, args.recent_limit)
            .await
            .with_context(|| format!("querying GetState from {}", remote.display()))?;
        print!("{}", metrics::format_metrics(&state));
        Ok(())
    }
}
