//! Prometheus metrics endpoint for the daemon.
//!
//! Counters live on `BlitService` and are incremented at the gRPC dispatch
//! boundary (one place per RPC) — that's the natural chokepoint for
//! "did this operation happen?" without reaching into the transfer
//! pipeline. Bytes/files-transferred counters can be added later when the
//! sink layer naturally feeds them; v1 is per-RPC operation counters plus
//! the active-transfers gauge and error counter.
//!
//! The HTTP server is hand-rolled (single GET endpoint, fixed response)
//! to avoid an additional framework dependency. Prometheus scrapers send
//! a minimal GET request; we accept any path under `/metrics` and any
//! path under `/health` for liveness checks.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Atomic counters shared between the gRPC service and the metrics endpoint.
/// All increments happen at the dispatch boundary in `service/core.rs`.
#[derive(Debug, Default)]
pub struct TransferMetrics {
    pub push_operations: AtomicU64,
    pub pull_operations: AtomicU64,
    pub purge_operations: AtomicU64,
    pub active_transfers: AtomicU64,
    pub transfer_errors: AtomicU64,
}

impl TransferMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn render_prometheus(&self) -> String {
        let mut out = String::with_capacity(1024);
        let row = |out: &mut String, name: &str, help: &str, kind: &str, val: u64| {
            out.push_str(&format!("# HELP {name} {help}\n"));
            out.push_str(&format!("# TYPE {name} {kind}\n"));
            out.push_str(&format!("{name} {val}\n"));
        };
        row(
            &mut out,
            "blitd_push_operations_total",
            "Total push RPC invocations.",
            "counter",
            self.push_operations.load(Relaxed),
        );
        row(
            &mut out,
            "blitd_pull_operations_total",
            "Total pull RPC invocations (Pull + PullSync).",
            "counter",
            self.pull_operations.load(Relaxed),
        );
        row(
            &mut out,
            "blitd_purge_operations_total",
            "Total purge RPC invocations.",
            "counter",
            self.purge_operations.load(Relaxed),
        );
        row(
            &mut out,
            "blitd_active_transfers",
            "Currently active transfer streams (push + pull).",
            "gauge",
            self.active_transfers.load(Relaxed),
        );
        row(
            &mut out,
            "blitd_transfer_errors_total",
            "Total transfer RPC failures.",
            "counter",
            self.transfer_errors.load(Relaxed),
        );
        out
    }
}

/// Spawn a tiny HTTP/1.1 server for `/metrics` and `/health`. Runs until
/// the runtime shuts down.
pub fn spawn_metrics_server(addr: SocketAddr, metrics: Arc<TransferMetrics>) {
    tokio::spawn(async move {
        let listener = match TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(err) => {
                eprintln!("[warn] failed to bind metrics server on {addr}: {err}");
                return;
            }
        };
        eprintln!(
            "[info] metrics endpoint listening on http://{}/metrics",
            addr
        );
        loop {
            match listener.accept().await {
                Ok((stream, _peer)) => {
                    let metrics = Arc::clone(&metrics);
                    tokio::spawn(handle_connection(stream, metrics));
                }
                Err(err) => {
                    eprintln!("[warn] metrics accept error: {err}");
                }
            }
        }
    });
}

async fn handle_connection(mut stream: tokio::net::TcpStream, metrics: Arc<TransferMetrics>) {
    // Read enough of the request to identify the path. Prometheus's GET is
    // small; we only care about the first line. Drop the rest.
    let mut buf = [0u8; 1024];
    let n = match stream.read(&mut buf).await {
        Ok(n) => n,
        Err(_) => return,
    };
    let request_line = std::str::from_utf8(&buf[..n])
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("");
    let path = request_line.split_whitespace().nth(1).unwrap_or("/");

    let (status, content_type, body) = if path.starts_with("/metrics") {
        (
            "200 OK",
            "text/plain; version=0.0.4; charset=utf-8",
            metrics.render_prometheus(),
        )
    } else if path.starts_with("/health") {
        ("200 OK", "text/plain; charset=utf-8", "ok\n".to_string())
    } else {
        (
            "404 Not Found",
            "text/plain; charset=utf-8",
            "not found\n".to_string(),
        )
    };

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_all_metrics() {
        let m = TransferMetrics::new();
        m.push_operations.fetch_add(3, Relaxed);
        m.pull_operations.fetch_add(7, Relaxed);
        m.active_transfers.fetch_add(2, Relaxed);
        let body = m.render_prometheus();
        assert!(body.contains("blitd_push_operations_total 3"));
        assert!(body.contains("blitd_pull_operations_total 7"));
        assert!(body.contains("blitd_active_transfers 2"));
        assert!(body.contains("# TYPE blitd_push_operations_total counter"));
        assert!(body.contains("# TYPE blitd_active_transfers gauge"));
    }

    #[test]
    fn render_zero_state() {
        let m = TransferMetrics::new();
        let body = m.render_prometheus();
        assert!(body.contains("blitd_push_operations_total 0"));
        assert!(body.contains("blitd_transfer_errors_total 0"));
    }
}
