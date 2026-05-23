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
use blit_core::generated::DaemonState;
use blit_core::remote::endpoint::RemoteEndpoint;
use eyre::{Context, Result};
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::sync::Semaphore;

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

/// Deadline for the daemon `GetState` scrape itself. `jobs::query` has
/// no internal timeout, so a daemon that accepts the connection but
/// stalls would otherwise park the `/metrics` handler indefinitely. Kept
/// below Prometheus's default `scrape_timeout` (10s) so the bridge
/// answers with `blit_daemon_up 0` *before* Prometheus gives up and
/// records a scrape error instead.
const SCRAPE_TIMEOUT: Duration = Duration::from_secs(8);

/// audit-5: deadline for writing the HTTP response. The request *head*
/// read is already bounded (`REQUEST_HEAD_TIMEOUT`), but the write side
/// was not — a client that stops reading (full socket buffer) would
/// otherwise park the handler task on `write_all`/`flush` indefinitely.
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);

/// audit-5: cap on concurrently-handled scrape connections. A
/// misconfigured / runaway Prometheus (or a deliberate flood) opening
/// many simultaneous scrapes would otherwise spawn an unbounded number
/// of handler tasks, each firing a `GetState` RPC at the daemon. The
/// accept loop acquires a permit before accepting, so excess
/// connections queue in the OS backlog (back-pressure) rather than
/// piling up as tasks. 64 is generous — Prometheus opens one scrape per
/// interval per server.
const MAX_CONCURRENT_SCRAPES: usize = 64;

/// audit-5: build the listener with `SO_REUSEADDR` so a quick
/// restart can rebind the port while a previous socket lingers in
/// `TIME_WAIT`, instead of failing with "address already in use".
/// (`TcpListener::bind` does not set it on all platforms.)
async fn build_listener(addr: SocketAddr) -> Result<TcpListener> {
    let socket = if addr.is_ipv4() {
        TcpSocket::new_v4()
    } else {
        TcpSocket::new_v6()
    }
    .with_context(|| format!("creating listener socket for {addr}"))?;
    socket
        .set_reuseaddr(true)
        .with_context(|| format!("setting SO_REUSEADDR for {addr}"))?;
    socket
        .bind(addr)
        .with_context(|| format!("binding {addr}"))?;
    // Backlog: generous for a scrape endpoint — Prometheus opens one
    // short-lived connection per scrape interval.
    socket
        .listen(1024)
        .with_context(|| format!("listening on {addr}"))
}

/// Serve `/metrics` until the process is killed. Binds `addr`, then
/// handles each connection on its own task.
pub(crate) async fn serve(
    addr: SocketAddr,
    remote: RemoteEndpoint,
    recent_limit: u32,
) -> Result<()> {
    let listener = build_listener(addr).await?;
    eprintln!(
        "blit-prometheus-bridge: serving http://{addr}/metrics (scraping {})",
        remote.display()
    );
    // audit-5: bound concurrent handlers. Acquiring a permit BEFORE
    // accepting back-pressures into the OS accept backlog under a scrape
    // flood instead of spawning unbounded tasks.
    let limiter = Arc::new(Semaphore::new(MAX_CONCURRENT_SCRAPES));
    loop {
        // The semaphore is never closed, so acquire only errs on a bug;
        // treat that as fatal-stop rather than spin.
        let Ok(permit) = Arc::clone(&limiter).acquire_owned().await else {
            break;
        };
        // audit-5: graceful shutdown — stop accepting on SIGINT/Ctrl-C
        // so the process can exit cleanly (in-flight handlers, holding
        // their own permits, run to completion as their tasks finish).
        let accepted = tokio::select! {
            biased;
            _ = tokio::signal::ctrl_c() => {
                eprintln!("blit-prometheus-bridge: shutdown signal received; no longer accepting");
                break;
            }
            res = listener.accept() => res,
        };
        let (stream, _peer) = match accepted {
            Ok(v) => v,
            Err(err) => {
                // A single accept failure shouldn't kill the exporter.
                // Dropping `permit` here frees the slot.
                eprintln!("blit-prometheus-bridge: accept error: {err}");
                continue;
            }
        };
        let remote = remote.clone();
        tokio::spawn(async move {
            // Hold the permit for the handler's lifetime; dropped on
            // completion, freeing the slot for the next connection.
            let _permit = permit;
            if let Err(err) = handle_conn(stream, &remote, recent_limit).await {
                eprintln!("blit-prometheus-bridge: connection error: {err}");
            }
        });
    }
    Ok(())
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
            // audit-5b1 round 2: bound this write too. A client that
            // already stopped reading (the very reason we hit the head
            // timeout) must not now park the task on the 408 write —
            // best-effort, errors ignored since we're closing anyway.
            let _ = write_all_within(&mut stream, resp.as_bytes(), WRITE_TIMEOUT).await;
            return Ok(());
        }
    };
    let request_line = head.lines().next().unwrap_or("");

    let response = if request_target(request_line) == Some("/metrics") {
        // Pull model: scrape the daemon for THIS request, under a
        // deadline so a hung daemon can't park the handler.
        let body = scrape_body(jobs::query(remote, recent_limit), SCRAPE_TIMEOUT).await;
        http_response("200 OK", "text/plain; version=0.0.4; charset=utf-8", &body)
    } else {
        http_response(
            "404 Not Found",
            "text/plain; charset=utf-8",
            "not found — try GET /metrics\n",
        )
    };

    // audit-5: bound the response write so a client that stops reading
    // can't park this handler forever.
    write_all_within(&mut stream, response.as_bytes(), WRITE_TIMEOUT).await?;
    Ok(())
}

/// Write `bytes` then flush, under `timeout`. Generic over the writer so
/// it's unit-testable with an in-memory pipe + a `pending` reader (no
/// real socket needed). An elapsed deadline is an error, not a silent
/// truncation.
async fn write_all_within<W>(writer: &mut W, bytes: &[u8], timeout: Duration) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    let write = async {
        writer.write_all(bytes).await?;
        writer.flush().await?;
        Ok::<(), std::io::Error>(())
    };
    match tokio::time::timeout(timeout, write).await {
        Ok(result) => result.context("writing HTTP response"),
        Err(_elapsed) => Err(eyre::eyre!(
            "writing HTTP response timed out after {timeout:?}"
        )),
    }
}

/// Run a GetState `scrape` under `timeout` and return the metrics body.
/// A timed-out OR failed scrape both yield `down_metrics()`
/// (`blit_daemon_up 0`), so a hung or unreachable daemon still produces
/// a prompt `200` response rather than parking the handler until
/// Prometheus times the target out. Generic over the scrape future so
/// it's testable with a pending/failed future (no live daemon needed).
async fn scrape_body<F>(scrape: F, timeout: Duration) -> String
where
    F: Future<Output = Result<DaemonState>>,
{
    match tokio::time::timeout(timeout, scrape).await {
        Ok(Ok(state)) => metrics::format_metrics(&state),
        Ok(Err(err)) => {
            eprintln!("blit-prometheus-bridge: scrape failed: {err:#}");
            metrics::down_metrics()
        }
        Err(_elapsed) => {
            eprintln!("blit-prometheus-bridge: scrape timed out after {timeout:?}");
            metrics::down_metrics()
        }
    }
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

    /// audit-5: the listener binds via the SO_REUSEADDR path and is
    /// usable. (Binding on :0 then reading back local_addr exercises
    /// build_listener end to end.)
    #[tokio::test]
    async fn build_listener_binds_with_reuseaddr() {
        let listener = build_listener("127.0.0.1:0".parse().unwrap())
            .await
            .expect("build_listener should bind");
        let addr = listener.local_addr().unwrap();
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
        assert_ne!(addr.port(), 0, "an ephemeral port was assigned");
    }

    /// audit-6c: end-to-end HTTP integration. A real client speaks HTTP
    /// to `handle_conn` over a loopback socket and reads the full
    /// response. The scrape target is unreachable (127.0.0.1:1 →
    /// ECONNREFUSED), so the scrape fails fast and the body is the
    /// "daemon down" metrics — exercising the whole path (accept → read
    /// request head → route /metrics → scrape → format → bounded write)
    /// without standing up a daemon.
    async fn http_roundtrip(remote_raw: &str, request: &str) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = build_listener("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let remote = RemoteEndpoint::parse(remote_raw).unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            handle_conn(stream, &remote, 0).await.unwrap();
        });
        let mut client = TcpStream::connect(addr).await.unwrap();
        client.write_all(request.as_bytes()).await.unwrap();
        let mut resp = String::new();
        client.read_to_string(&mut resp).await.unwrap();
        server.await.unwrap();
        resp
    }

    #[tokio::test]
    async fn metrics_endpoint_serves_http_200_with_down_metrics_when_daemon_unreachable() {
        let resp = http_roundtrip("127.0.0.1:1", "GET /metrics HTTP/1.1\r\nHost: x\r\n\r\n").await;
        assert!(
            resp.starts_with("HTTP/1.1 200 OK"),
            "expected 200 status line, got: {resp:?}"
        );
        assert!(
            resp.contains("text/plain; version=0.0.4"),
            "expected prometheus content-type, got: {resp}"
        );
        assert!(
            resp.contains("Content-Length:"),
            "framed response must carry Content-Length, got: {resp}"
        );
        // Unreachable daemon → down metrics (blit_daemon_up 0).
        assert!(
            resp.contains("blit_daemon_up 0"),
            "expected down metrics body, got: {resp}"
        );
    }

    #[tokio::test]
    async fn unknown_path_serves_http_404() {
        // Path != /metrics: scrape is never attempted, so the (unreachable)
        // remote is irrelevant — the route is decided before any scrape.
        let resp = http_roundtrip(
            "127.0.0.1:1",
            "GET /favicon.ico HTTP/1.1\r\nHost: x\r\n\r\n",
        )
        .await;
        assert!(
            resp.starts_with("HTTP/1.1 404 Not Found"),
            "expected 404 status line, got: {resp:?}"
        );
        assert!(
            resp.contains("try GET /metrics"),
            "expected the 404 hint body, got: {resp}"
        );
    }

    /// audit-5: a write to a peer that never reads must surface a
    /// timeout, not park the handler forever. A 1-byte duplex whose
    /// other half is dropped-but-unread blocks `write_all` of a larger
    /// payload, so a short deadline can only fire the timeout.
    #[tokio::test]
    async fn write_all_within_times_out_when_peer_never_reads() {
        let (mut server_side, _client_side) = tokio::io::duplex(1);
        // _client_side is never read from; its buffer fills at 1 byte.
        let big = vec![b'x'; 64 * 1024];
        let err = write_all_within(&mut server_side, &big, Duration::from_millis(20))
            .await
            .expect_err("a non-reading peer must time out");
        assert!(err.to_string().contains("timed out"), "got: {err}");
    }

    /// audit-5: the happy path writes all bytes and flushes within the
    /// deadline.
    #[tokio::test]
    async fn write_all_within_succeeds_when_peer_reads() {
        let (mut server_side, mut client_side) = tokio::io::duplex(1024);
        let payload = b"HTTP/1.1 200 OK\r\n\r\nbody";
        let reader = tokio::spawn(async move {
            let mut buf = Vec::new();
            client_side.read_to_end(&mut buf).await.unwrap();
            buf
        });
        write_all_within(&mut server_side, payload, Duration::from_secs(5))
            .await
            .expect("write should succeed");
        drop(server_side); // EOF so read_to_end returns
        assert_eq!(reader.await.unwrap(), payload);
    }

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

    /// Regression for the bridge-2 round-2 reopen: a daemon that accepts
    /// the connection but never answers GetState must not park the
    /// handler — the scrape deadline routes to `blit_daemon_up 0`.
    #[tokio::test]
    async fn hung_scrape_returns_up_zero_within_timeout() {
        let started = Instant::now();
        let outcome = tokio::time::timeout(
            Duration::from_secs(2),
            // A scrape future that never resolves (stalled daemon).
            scrape_body(
                std::future::pending::<Result<DaemonState>>(),
                Duration::from_millis(100),
            ),
        )
        .await
        .expect("scrape_body must return on its own, not hit the 2s guard");
        assert!(
            outcome.contains("blit_daemon_up 0"),
            "a hung scrape should report the daemon down: {outcome}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "the handler should be released promptly"
        );
    }

    #[tokio::test]
    async fn failed_scrape_returns_up_zero() {
        let body = scrape_body(
            async { Err(eyre::eyre!("connection refused")) },
            Duration::from_secs(1),
        )
        .await;
        assert!(body.contains("blit_daemon_up 0"), "{body}");
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
