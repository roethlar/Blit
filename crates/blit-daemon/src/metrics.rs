//! Prometheus-compatible metrics endpoint for the daemon.
//!
//! Exposes transfer counters on a lightweight HTTP server. Only active
//! when `--metrics-addr` is passed on the command line.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Arc;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};

/// Atomic transfer counters shared between the gRPC service and the
/// metrics HTTP endpoint.
#[derive(Debug, Default)]
pub struct TransferMetrics {
    pub bytes_transferred: AtomicU64,
    pub files_transferred: AtomicU64,
    pub active_transfers: AtomicU64,
    pub transfer_errors: AtomicU64,
    pub push_operations: AtomicU64,
    pub pull_operations: AtomicU64,
    pub purge_operations: AtomicU64,
}

impl TransferMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn render_prometheus(&self) -> String {
        let mut out = String::with_capacity(1024);

        out.push_str("# HELP blitd_bytes_transferred_total Total bytes transferred.\n");
        out.push_str("# TYPE blitd_bytes_transferred_total counter\n");
        out.push_str(&format!(
            "blitd_bytes_transferred_total {}\n",
            self.bytes_transferred.load(Relaxed)
        ));

        out.push_str("# HELP blitd_files_transferred_total Total files transferred.\n");
        out.push_str("# TYPE blitd_files_transferred_total counter\n");
        out.push_str(&format!(
            "blitd_files_transferred_total {}\n",
            self.files_transferred.load(Relaxed)
        ));

        out.push_str("# HELP blitd_active_transfers Currently active transfer streams.\n");
        out.push_str("# TYPE blitd_active_transfers gauge\n");
        out.push_str(&format!(
            "blitd_active_transfers {}\n",
            self.active_transfers.load(Relaxed)
        ));

        out.push_str("# HELP blitd_transfer_errors_total Total transfer errors.\n");
        out.push_str("# TYPE blitd_transfer_errors_total counter\n");
        out.push_str(&format!(
            "blitd_transfer_errors_total {}\n",
            self.transfer_errors.load(Relaxed)
        ));

        out.push_str("# HELP blitd_push_operations_total Total push operations.\n");
        out.push_str("# TYPE blitd_push_operations_total counter\n");
        out.push_str(&format!(
            "blitd_push_operations_total {}\n",
            self.push_operations.load(Relaxed)
        ));

        out.push_str("# HELP blitd_pull_operations_total Total pull operations.\n");
        out.push_str("# TYPE blitd_pull_operations_total counter\n");
        out.push_str(&format!(
            "blitd_pull_operations_total {}\n",
            self.pull_operations.load(Relaxed)
        ));

        out.push_str("# HELP blitd_purge_operations_total Total purge operations.\n");
        out.push_str("# TYPE blitd_purge_operations_total counter\n");
        out.push_str(&format!(
            "blitd_purge_operations_total {}\n",
            self.purge_operations.load(Relaxed)
        ));

        out
    }
}

async fn handle_request(
    req: Request<Body>,
    metrics: Arc<TransferMetrics>,
) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            let body = metrics.render_prometheus();
            Ok(Response::builder()
                .header("content-type", "text/plain; version=0.0.4; charset=utf-8")
                .body(Body::from(body))
                .unwrap())
        }
        (&Method::GET, "/health") => Ok(Response::new(Body::from("ok\n"))),
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("not found\n"))
            .unwrap()),
    }
}

/// Spawn the metrics HTTP server on a background task. Returns
/// immediately; the server runs until the runtime shuts down.
pub fn spawn_metrics_server(addr: SocketAddr, metrics: Arc<TransferMetrics>) {
    tokio::spawn(async move {
        let make_svc = make_service_fn(move |_| {
            let metrics = Arc::clone(&metrics);
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    handle_request(req, Arc::clone(&metrics))
                }))
            }
        });

        match Server::try_bind(&addr) {
            Ok(builder) => {
                eprintln!("[info] metrics server listening on http://{}/metrics", addr);
                if let Err(e) = builder.serve(make_svc).await {
                    eprintln!("[warn] metrics server error: {e}");
                }
            }
            Err(e) => {
                eprintln!("[warn] failed to bind metrics server on {addr}: {e}");
            }
        }
    });
}
