Reviewed sha: `f7ff757dee22e220684e66c7736db66fe80f70ab`

Verdict: reopened

Validation run:

- `cargo fmt --all -- --check`: passed
- `cargo clippy --workspace --all-targets -- -D warnings`: passed
- `cargo test --workspace`: passed
- `cargo test -p blit-prometheus-bridge`: passed, 10 tests

Findings:

1. Medium: the daemon scrape itself is still unbounded, so hung daemon calls can park `/metrics` handlers indefinitely.

   The round-2 timeout closes the idle-client request-head hole, but once `GET /metrics` is parsed, `handle_conn` awaits `jobs::query(remote, recent_limit)` at `crates/blit-prometheus-bridge/src/server.rs:94` with no deadline. `jobs::query` opens a tonic channel and awaits `GetState` with no timeout in `crates/blit-app/src/admin/jobs.rs:22` through `crates/blit-app/src/admin/jobs.rs:33`. If the daemon endpoint accepts a connection but stalls, or the RPC never completes, the HTTP request task remains stuck until the lower transport eventually gives up. That also violates the advertised bridge behavior from the finding doc: a failed scrape should return `200` with `blit_daemon_up 0`, but a hung scrape returns nothing until Prometheus times out the target as a scrape error.

   Put a scrape deadline around the `jobs::query` call in the bridge server and route elapsed deadlines through `metrics::down_metrics()` the same way other query errors are handled. Please add a regression test that uses a pending scrape future or a fake/stalled endpoint to prove `/metrics` returns `blit_daemon_up 0` within the configured scrape timeout instead of hanging.
