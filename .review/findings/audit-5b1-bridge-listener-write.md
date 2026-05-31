# audit-5b1-bridge-listener-write: SO_REUSEADDR listener + response write timeout

**Severity**: Robustness
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `a7db3a5`
**Parent finding**: `audit-5-bridge-robustness` (items 5 + 6). audit-5a
covered items 1-2 (one-shot timeout, `\r` escaping); **audit-5b2** will
cover items 3-4 (graceful `ctrl_c` shutdown + a `Semaphore` concurrency
bound on the accept loop).

## What

Two Prometheus-bridge server tweaks from audit-5:

- **SO_REUSEADDR (item 6):** `TcpListener::bind` doesn't set
  `SO_REUSEADDR` on all platforms, so a quick restart could fail with
  "address already in use" while a prior socket lingers in `TIME_WAIT`.
- **Response write timeout (item 5):** the request-*head* read is
  bounded (`REQUEST_HEAD_TIMEOUT`), but the response *write* was not — a
  client that stops reading (full socket buffer) parks the handler task
  on `write_all`/`flush` indefinitely.

## Approach

- `build_listener(addr)` constructs the listener via `TcpSocket`
  (v4/v6 by addr), `set_reuseaddr(true)`, `bind`, `listen(1024)`.
  `serve()` calls it instead of `TcpListener::bind`.
- `write_all_within(writer, bytes, WRITE_TIMEOUT=10s)` wraps
  `write_all` + `flush` in `tokio::time::timeout`; elapsed → error (not
  silent truncation). Generic over `W: AsyncWriteExt + Unpin` so it's
  unit-testable with an in-memory pipe. `handle_conn`'s final write
  routes through it.

## Files changed

- `crates/blit-prometheus-bridge/src/server.rs`: `WRITE_TIMEOUT`,
  `build_listener`, `write_all_within`; `serve()` + `handle_conn`
  rewired; tests.

## Tests

`blit-prometheus-bridge` 18 (+3):

- `build_listener_binds_with_reuseaddr` — binds on `:0` via the
  SO_REUSEADDR path; an ephemeral port is assigned.
- `write_all_within_times_out_when_peer_never_reads` — a 1-byte
  `tokio::io::duplex` with an unread peer blocks `write_all` of a 64 KiB
  payload, so a 20 ms deadline can only fire the timeout (deterministic).
- `write_all_within_succeeds_when_peer_reads` — happy path writes +
  flushes all bytes within the deadline.

## Scope / next

audit-5b2: graceful shutdown (race `tokio::signal::ctrl_c()` against the
accept loop) + a `tokio::sync::Semaphore` bound (~64 permits) so a scrape
storm can't spawn unbounded handler tasks. That completes audit-5.

## Round 2 (commit `28e9956`)

**Reopen finding:** the request-head-timeout branch still wrote its
`408 Request Timeout` response via raw `stream.write_all` — an unbounded
write on exactly the path the finding targets (a client that stopped
reading is *why* the head timed out, so its 408 write can park the task
just as easily).

**Fix:** route the 408 write through `write_all_within(..., WRITE_TIMEOUT)`
too (best-effort — errors ignored since the connection is closing). Now
every response-write path in `handle_conn` is bounded.

## Reviewer comments

(empty — pending round-2 grade)
