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
use blit_core::generated::DaemonState;
use blit_core::remote::endpoint::RemoteEndpoint;
use clap::Parser;
use eyre::{Context, Result};
use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;

/// audit-5: bound the one-shot `GetState` query. Pre-fix the one-shot
/// path inherited the OS TCP connect timeout (60-127s) against a dead
/// host, so `blit-prometheus-bridge --remote dead:9031` hung for
/// minutes — bad for cron / node_exporter textfile-collector use. Match
/// the server path's `SCRAPE_TIMEOUT` (8s, below Prometheus's 10s
/// default). Unlike the server path (which emits `down_metrics` on
/// timeout), the one-shot path keeps its fail-loudly semantics: a
/// timeout is a hard error with a non-zero exit a cron wrapper can
/// detect.
const ONESHOT_TIMEOUT: Duration = Duration::from_secs(8);

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
    // w5-1: without a backend every log::warn!/error! in blit-core is
    // silently discarded. Stderr, warn level,
    // `blit-prometheus-bridge: <level>: <msg>`.
    blit_core::stderr_log::init("blit-prometheus-bridge");
    let args = Args::parse();
    let remote = RemoteEndpoint::parse(&args.remote)
        .with_context(|| format!("parsing remote endpoint '{}'", args.remote))?;

    if let Some(addr) = args.listen {
        // bridge-2: long-running HTTP exporter.
        server::serve(addr, remote, args.recent_limit).await
    } else {
        // bridge-1: one-shot print.
        let state = query_within(jobs::query(&remote, args.recent_limit), ONESHOT_TIMEOUT)
            .await
            .with_context(|| format!("querying GetState from {}", remote.display()))?;
        print!("{}", metrics::format_metrics(&state));
        Ok(())
    }
}

/// audit-5: run a `GetState` query future under `timeout`, mapping an
/// elapsed deadline to an error (the caller adds endpoint context). Kept
/// generic over the future so it's unit-testable with a synthetic
/// `pending()` future without standing up a daemon — the same approach
/// the server path's `scrape_body` uses.
async fn query_within<F>(query: F, timeout: Duration) -> Result<DaemonState>
where
    F: Future<Output = Result<DaemonState>>,
{
    match tokio::time::timeout(timeout, query).await {
        Ok(result) => result,
        Err(_elapsed) => Err(eyre::eyre!("timed out after {timeout:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn query_within_times_out_on_a_stalled_query() {
        // A query that never resolves must surface a timeout error
        // rather than hang. A short real timeout against a `pending`
        // future is deterministic (it can only ever fire the timeout).
        let pending = std::future::pending::<Result<DaemonState>>();
        let err = query_within(pending, Duration::from_millis(10))
            .await
            .expect_err("a stalled query must time out");
        assert!(err.to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn query_within_passes_through_a_prompt_result() {
        let ok = query_within(async { Ok(DaemonState::default()) }, Duration::from_secs(8))
            .await
            .expect("a prompt success passes through");
        assert_eq!(ok, DaemonState::default());
        // And a prompt error propagates (not swallowed).
        let err = query_within(
            async { Err(eyre::eyre!("connection refused")) },
            Duration::from_secs(8),
        )
        .await
        .expect_err("a prompt error propagates");
        assert!(err.to_string().contains("connection refused"));
    }
}
