# bridge-2-prometheus-http: long-running /metrics HTTP server

**Severity**: Feature (Milestone E — optional Prometheus bridge, step 2)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `eade7dd`

## What

Second slice of the Prometheus bridge. bridge-1 delivered the
`format_metrics` formatter + a print-once CLI; bridge-2 adds the
long-running HTTP exporter Prometheus actually scrapes. `--listen
<addr>` runs the server; without it the bridge-1 print-once behavior is
unchanged.

## Approach

- **Pull model**: each `GET /metrics` triggers a fresh `GetState` query,
  so the metrics are as fresh as the scrape — no background poll loop or
  cached staleness.
- **Hand-rolled on `tokio` TCP**, no axum/hyper. A single read-only
  endpoint doesn't justify a web framework, and the gRPC client
  (`blit-app`) is the only heavy dep we want. `tokio` (already a dep)
  provides `TcpListener` + async I/O.
- `server::serve(addr, remote, recent_limit)`: bind, accept loop, one
  task per connection. A single `accept` error logs and continues
  (doesn't kill the exporter).
- `handle_conn`: read the request head (bounded — `MAX_REQUEST_HEAD` =
  16 KiB, tolerant of segmented arrival), route via `request_target`.
  `GET /metrics` → scrape → `200 text/plain; version=0.0.4`; anything
  else → `404`. `Connection: close`, one response per connection
  (Prometheus opens a fresh connection per scrape).
- **Failed scrape → `200` with `blit_daemon_up 0`** (`metrics::
  down_metrics`), so the target registers as up-but-down rather than a
  scrape error. `down_metrics` shares `UP_HELP` with `format_metrics` so
  the `blit_daemon_up` HELP line is identical across scrapes (Prometheus
  warns otherwise).

## Files changed

- `crates/blit-prometheus-bridge/src/server.rs` (new): the HTTP server +
  pure helpers (`request_target`, `http_response`).
- `crates/blit-prometheus-bridge/src/metrics.rs`: `down_metrics()` +
  shared `UP_HELP` const.
- `crates/blit-prometheus-bridge/src/main.rs`: `--listen:
  Option<SocketAddr>` selects server vs. print-once; `mod server`.

## Tests

Bridge crate now 9 unit tests (+4 over bridge-1's 5):
- `request_target_extracts_get_path_only` — GET-only routing; non-GET
  and malformed lines → `None`.
- `http_response_sets_content_length_and_close` — status line,
  `Content-Length`, `Connection: close`, exact body after the blank
  line.
- `http_response_content_length_counts_bytes_not_chars` — multibyte body
  → byte length, not char count.
- `down_metrics_reports_up_zero` — `blit_daemon_up 0`, no version label.

The accept loop + live scrape are integration (need a running daemon).

## Scope

- Read-only `GET /metrics` only; `Connection: close` (no keep-alive) —
  fine for Prometheus, which opens a fresh connection per scrape.
- No TLS / auth — same operator-network trust model as the rest of the
  design (§5.2 release plan, §9). Bind to `127.0.0.1:<port>` for a
  local-only exporter.
- Operation counters are still omitted (bridge-1 round-2 decision): they
  await a wire signal distinguishing metrics-disabled zeros from real
  zeros. See [[feedback-getstate-counters-zero]].

## Round 2 (commit `f7ff757`)

**Reopen finding (Medium):** idle/slow clients parked connection tasks
forever. The 16 KiB `MAX_REQUEST_HEAD` bounded buffered *bytes*, not
*time* — a client could connect and send nothing, or trickle bytes below
the cap without ever ending the headers, holding a task + socket
indefinitely. That contradicted the slowloris-guard comment and is risky
because `--listen` accepts any `SocketAddr`, including non-loopback.

**Fix:**
- `REQUEST_HEAD_TIMEOUT` (5s) wraps the head read in
  `tokio::time::timeout`. A Prometheus scrape sends its head in one
  segment immediately, so 5s is generous.
- `read_request_head` now returns `Result<Option<String>>`: `Ok(None)`
  means the deadline elapsed → `handle_conn` replies `408 Request
  Timeout` and releases the connection. The inner (untimed) read loop is
  `read_head_bytes`.

**Test (+1, bridge crate now 10):**
`idle_client_released_by_read_timeout` — binds a real listener, connects
an idle peer that sends nothing, and asserts `read_request_head`
(100 ms test timeout) returns `Ok(None)` promptly, wrapped in a 2 s
outer guard so a broken timeout fails loudly instead of hanging.

## Reviewer comments

(empty — pending round-2 grade)
