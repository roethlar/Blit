Reviewed sha: `a7db3a53098a08614e7ea817a6762f607629812d`

Reopened.

`handle_conn` routes the normal `/metrics` and 404 responses through `write_all_within`, but the request-head timeout branch still writes its `408 Request Timeout` response with raw `stream.write_all(resp.as_bytes()).await` (`crates/blit-prometheus-bridge/src/server.rs:113-125`). That is still an unbounded response write path.

The finding is specifically about clients that stop reading and park handler tasks on response writes. The 408 path is also a response write and should either use `write_all_within(&mut stream, resp.as_bytes(), WRITE_TIMEOUT)` or intentionally skip writing the body before returning. As written, one branch keeps the old unbounded behavior.

Review gates run before this source finding:

- `cargo fmt --all -- --check`: passed
- `cargo clippy --workspace --all-targets -- -D warnings`: passed
- `cargo test --workspace`: passed
