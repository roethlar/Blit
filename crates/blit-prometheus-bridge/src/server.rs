//! Minimal HTTP `/metrics` server for the Prometheus bridge
//! (bridge-2). Pull model: each scrape triggers a fresh `GetState`
//! query, so there's no background poll loop or cached staleness —
//! the metrics are as fresh as the moment Prometheus asked.
//!
//! Hand-rolled on `tokio` TCP rather than pulling in axum/hyper: a
//! single read-only `GET /metrics` endpoint doesn't justify a web
//! framework, and the daemon-side gRPC is the only heavy dep we want.
//! Scope: serves `GET /metrics` (anything else → 404), one response
//! per connection with `Connection: close` (no keep-alive — Prometheus
//! opens a fresh connection per scrape anyway). On a failed scrape it
//! still returns `200` with `blit_daemon_up 0` so the target registers
//! as up-but-down rather than a scrape error.

use crate::metrics;
use blit_app::admin::jobs;
use blit_core::remote::endpoint::RemoteEndpoint;
use eyre::{Context, Result};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Cap on how much of the request head we'll buffer before giving up
/// looking for the end of headers — bounds buffered bytes per
/// connection. Prometheus request heads are well under 1 KiB.
const MAX_REQUEST_HEAD: usize = 16 * 1024;

/// Wall-clock deadline for receiving the full request head. Bounds the
/// *time* a connection can hold a task (the byte cap alone doesn't: a
/// client can connect and send nothing, or trickle bytes forever). This
/// is the actual slowloris guard, and matters because `--listen` accepts
/// any `SocketAddr`, including non-loopback binds. A Prometheus scrape
/// sends its head in one segment immediately, so 5s is generous.
const REQUEST_HEAD_TIMEOUT: Duration = Duration::from_secs(5);

/// Serve `/metrics` until the process is killed. Binds `addr`, then
/// handles each connection on its own task.
pub(crate) async fn serve(
    addr: SocketAddr,
    remote: RemoteEndpoint,
    recent_limit: u32,
) -> Result<()> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    eprintln!(
        "blit-prometheus-bridge: serving http://{addr}/metrics (scraping {})",
        remote.display()
    );
    loop {
        let (stream, _peer) = match listener.accept().await {
            Ok(v) => v,
            Err(err) => {
                // A single accept failure shouldn't kill the exporter.
                eprintln!("blit-prometheus-bridge: accept error: {err}");
                continue;
            }
        };
        let remote = remote.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_conn(stream, &remote, recent_limit).await {
                eprintln!("blit-prometheus-bridge: connection error: {err}");
            }
        });
    }
}

/// Read one request, route it, write one response, close.
async fn handle_conn(
    mut stream: TcpStream,
    remote: &RemoteEndpoint,
    recent_limit: u32,
) -> Result<()> {
    let head = match read_request_head(&mut stream, REQUEST_HEAD_TIMEOUT).await? {
        Some(head) => head,
        None => {
            // Slow/idle client never finished its head within the
            // deadline — release the task + socket rather than parking
            // it forever.
            let resp = http_response(
                "408 Request Timeout",
                "text/plain; charset=utf-8",
                "request timeout\n",
            );
            let _ = stream.write_all(resp.as_bytes()).await;
            return Ok(());
        }
    };
    let request_line = head.lines().next().unwrap_or("");

    let response = if request_target(request_line) == Some("/metrics") {
        // Pull model: scrape the daemon for THIS request.
        let body = match jobs::query(remote, recent_limit).await {
            Ok(state) => metrics::format_metrics(&state),
            Err(err) => {
                eprintln!("blit-prometheus-bridge: scrape failed: {err:#}");
                metrics::down_metrics()
            }
        };
        http_response("200 OK", "text/plain; version=0.0.4; charset=utf-8", &body)
    } else {
        http_response(
            "404 Not Found",
            "text/plain; charset=utf-8",
            "not found — try GET /metrics\n",
        )
    };

    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

/// Read the request head within `timeout`. Returns `Ok(None)` when the
/// client didn't finish its head in time (slow/idle — the caller should
/// release the connection), `Ok(Some(head))` otherwise. Wrapping the
/// read in `tokio::time::timeout` is what actually bounds how long a
/// connection can hold a task; [`MAX_REQUEST_HEAD`] only bounds bytes.
async fn read_request_head(stream: &mut TcpStream, timeout: Duration) -> Result<Option<String>> {
    match tokio::time::timeout(timeout, read_head_bytes(stream)).await {
        Ok(result) => result.map(Some),
        Err(_elapsed) => Ok(None),
    }
}

/// Read up to the blank line that ends the headers, bounded by
/// [`MAX_REQUEST_HEAD`]. We only need the request line, but reading to
/// end-of-headers tolerates a request that arrives across multiple TCP
/// segments. No timeout here — the caller wraps it.
async fn read_head_bytes(stream: &mut TcpStream) -> Result<String> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            break; // client closed
        }
        data.extend_from_slice(&chunk[..n]);
        if data.windows(4).any(|w| w == b"\r\n\r\n") || data.len() >= MAX_REQUEST_HEAD {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&data).into_owned())
}

/// Parse a request line like `GET /metrics HTTP/1.1`, returning the
/// target path for a `GET` request only (the bridge is read-only). Any
/// other method, or a malformed line, returns `None`.
fn request_target(request_line: &str) -> Option<&str> {
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?;
    let target = parts.next()?;
    if method == "GET" {
        Some(target)
    } else {
        None
    }
}

/// Build a complete HTTP/1.1 response with an accurate `Content-Length`
/// and `Connection: close` (one response per connection).
fn http_response(status: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// Regression for the bridge-2 round-1 reopen: a client that
    /// connects but never finishes its request head must be released by
    /// the read timeout, not park the task forever. Drives the real
    /// `read_request_head` against a live socket with an idle peer.
    #[tokio::test]
    async fn idle_client_released_by_read_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        // Connect but send nothing; hold the client open so the server
        // side sees neither data nor EOF.
        let _client = TcpStream::connect(addr).await.unwrap();
        let (mut server_stream, _) = listener.accept().await.unwrap();

        let started = Instant::now();
        // Outer guard: if the timeout doesn't work, fail loudly rather
        // than hang the test runner.
        let outcome = tokio::time::timeout(
            Duration::from_secs(2),
            read_request_head(&mut server_stream, Duration::from_millis(100)),
        )
        .await
        .expect("read_request_head must return on its own, not hit the 2s guard");

        assert!(
            matches!(outcome, Ok(None)),
            "an idle client should time out to Ok(None), got {outcome:?}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "the idle client should be released promptly"
        );
        drop(_client);
    }

    #[test]
    fn request_target_extracts_get_path_only() {
        assert_eq!(request_target("GET /metrics HTTP/1.1"), Some("/metrics"));
        assert_eq!(request_target("GET / HTTP/1.1"), Some("/"));
        // Non-GET methods are not served.
        assert_eq!(request_target("POST /metrics HTTP/1.1"), None);
        assert_eq!(request_target("HEAD /metrics HTTP/1.1"), None);
        // Malformed lines.
        assert_eq!(request_target(""), None);
        assert_eq!(request_target("GET"), None);
    }

    #[test]
    fn http_response_sets_content_length_and_close() {
        let resp = http_response("200 OK", "text/plain", "hello\n");
        assert!(resp.starts_with("HTTP/1.1 200 OK\r\n"), "{resp}");
        assert!(resp.contains("Content-Length: 6\r\n"), "{resp}");
        assert!(resp.contains("Connection: close\r\n"), "{resp}");
        // Blank line then the exact body.
        assert!(resp.ends_with("\r\n\r\nhello\n"), "{resp}");
    }

    #[test]
    fn http_response_content_length_counts_bytes_not_chars() {
        // Multibyte body: Content-Length must be the byte length.
        let body = "café\n"; // 'é' is 2 bytes in UTF-8 → 6 bytes total
        let resp = http_response("200 OK", "text/plain", body);
        assert!(resp.contains("Content-Length: 6\r\n"), "{resp}");
    }
}
