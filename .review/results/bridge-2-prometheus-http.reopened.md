Reviewed sha: `eade7dde1527e387bc63b42253012cd46a1f7859`

Verdict: reopened

Validation run:

- `cargo fmt --all -- --check`: passed
- `cargo clippy --workspace --all-targets -- -D warnings`: passed
- `cargo test --workspace`: passed
- `cargo test -p blit-prometheus-bridge`: passed, 9 tests

Findings:

1. Medium: idle or slow clients can park connection tasks forever.

   `crates/blit-prometheus-bridge/src/server.rs:52` spawns one task per accepted connection, and `read_request_head` then waits on `stream.read(&mut chunk).await` at `crates/blit-prometheus-bridge/src/server.rs:100` with no timeout. The 16 KiB cap at `crates/blit-prometheus-bridge/src/server.rs:105` only bounds buffered bytes after reads happen; it does not bound time. A client can connect and send nothing, or trickle bytes below the cap without ever ending headers, and each connection keeps a task and socket open indefinitely. This contradicts the comment at `crates/blit-prometheus-bridge/src/server.rs:23` that frames the cap as guarding against slowloris-style clients, and it is risky because `--listen` accepts any `SocketAddr`, including non-loopback binds.

   Add an explicit request-head read timeout, for example by wrapping `read_request_head` in `tokio::time::timeout` from `handle_conn`, or by timing out each read loop. Return a closed connection or a small `408 Request Timeout` response on timeout. Please add a regression test that connects to the server/helper and proves an idle client is released within the configured timeout.
