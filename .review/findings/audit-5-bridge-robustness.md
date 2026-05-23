# audit-5-bridge-robustness: Prometheus bridge robustness fixes

**Severity**: Robustness
**Status**: Open
**Branch**: (none yet)

## What

Ground-up audit of the `blit-prometheus-bridge` crate (4 source files)
identified several robustness issues:

1. **`crates/blit-prometheus-bridge/src/main.rs:55-56`** — One-shot mode
   (`--remote` without `--listen`) calls `jobs::query().await` without a
   `tokio::time::timeout` wrapper. Bridge-2 wraps the identical call in
   `SCRAPE_TIMEOUT` (8s), but the one-shot path inherits the OS TCP
   connect timeout (60-127s). `blit-prometheus-bridge --remote dead-host:9031`
   hangs for minutes, problematic for cron/textfile-collector usage.

2. **`crates/blit-prometheus-bridge/src/metrics.rs:120-131`** —
   `escape_label()` escapes `\`, `"`, and `\n` but not `\r`. The Prometheus
   exposition format requires `\r` → `\\r` escaping. Latent bug: current
   label values don't contain `\r` (version comes from `CARGO_PKG_VERSION`),
   but any future label with `\r` produces non-compliant output.

3. **`crates/blit-prometheus-bridge/src/server.rs:61-76`** — No graceful
   shutdown mechanism (no signal handler, no shutdown channel).
   SIGINT/SIGTERM kills in-flight scrapes abruptly.

4. **`crates/blit-prometheus-bridge/src/server.rs:71-75`** — No concurrency
   bound on accepted connections. Each scrape spawns a new task without
   semaphore/limit. Thundering herd of scrapes (misconfigured Prometheus)
   could overwhelm daemon with concurrent `GetState` RPCs.

5. **`crates/blit-prometheus-bridge/src/server.rs:115`** — Response write has
   no timeout (`write_all` + `flush`). Request read has `REQUEST_HEAD_TIMEOUT`
   (5s) but write side is unbounded.

6. **`crates/blit-prometheus-bridge/src/server.rs:54`** — `TcpListener::bind`
   without `SO_REUSEADDR`. Quick restart cycles may fail with "Address already
   in use" on platforms where tokio doesn't apply SO_REUSEADDR by default.

## Approach

1. Add `SCRAPE_TIMEOUT` wrapper to one-shot path matching bridge-2's approach
2. Add `\r` to the escape table in `escape_label()`
3. Add `tokio::signal::ctrl_c()` handler for graceful shutdown
4. Add `tokio::sync::Semaphore` with reasonable limit (32-64) on concurrent handlers
5. Add write timeout (e.g. `TOKIO_WRITE_TIMEOUT`) on response body
6. Set `SO_REUSEADDR` on the TCP listener socket

## Files changed

TBD by coder. Primarily `metrics.rs`, `server.rs`, `main.rs`.

## Tests

- Unit test: `escape_label` handles `\r` correctly
- Unit test: timeout fires on one-shot path
- Integration test: HTTP server with graceful shutdown
