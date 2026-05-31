# audit-5b2-bridge-server-lifecycle: graceful shutdown + concurrency bound

**Severity**: Robustness
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `05f77ec`
**Parent finding**: `audit-5-bridge-robustness` (items 3 + 4). With
audit-5a (items 1-2) and audit-5b1 (items 5-6), this **completes
audit-5**.

## What

The Prometheus-bridge `serve()` loop accepted connections forever with
no shutdown path and spawned an unbounded handler task per connection.

## Approach (both in `serve()`)

- **Graceful shutdown (item 3):** the accept loop races
  `listener.accept()` against `tokio::signal::ctrl_c()` in a `biased`
  `select!`. On SIGINT it stops accepting and returns `Ok(())`, so the
  process exits cleanly — in-flight handlers (holding their own permits)
  run to completion as their tasks finish, rather than being hard-killed
  mid-scrape.
- **Concurrency bound (item 4):** a `tokio::sync::Semaphore`
  (`MAX_CONCURRENT_SCRAPES = 64`). A permit is acquired **before**
  `accept()`, so under a scrape flood excess connections back-pressure
  into the OS accept backlog instead of spawning unbounded handler tasks
  (each of which fires a `GetState` RPC at the daemon). The permit moves
  into the spawned task and is dropped on completion, freeing the slot.
  An accept error drops the permit (frees the slot) and continues.

## Files changed

- `crates/blit-prometheus-bridge/src/server.rs`: `MAX_CONCURRENT_SCRAPES`;
  `serve()` loop reworked (ctrl_c select + semaphore).

## Tests

No new unit test. `serve()`'s loop is signal/integration-shaped — it runs
until SIGINT and binds a real listener, so it isn't cleanly unit-testable
without a server-lifecycle/signal harness (a larger addition). The
per-connection logic it drives (`handle_conn`, `write_all_within`,
`scrape_body`, `build_listener`) is covered by existing unit tests; the
loop changes are compile-checked and use standard tokio patterns
(`biased select!` on `ctrl_c`, `Semaphore::acquire_owned` before accept).

## Scope

Completes audit-5 (Prometheus-bridge robustness). Remaining audit
backlog: audit-6 (test gaps), audit-7b/c/d/e (code health); plus audit-1c
(transfer stall-timeout — design filed, awaiting owner scope) and the
`--retry`/`--wait` follow-up feature.

## Reviewer comments

(empty — pending review)
