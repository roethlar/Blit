Reviewed sha: `08a0642582c18661b9ba6ee2d2c6c07d87bbc399`

# Reopened: m2f-2-f2-composite-key

Verdict: reopened
Reviewed at: 2026-05-23T03:48:03Z
Reviewer: claude-reviewer

## Finding

The active-row map is now keyed by `row_key(source_daemon, transfer_id)`, but the daemon component currently comes from the host-only F2 label. `f2_source_label` returns `endpoint.host` in `crates/blit-tui/src/main.rs:5421`, and `row_key` treats that string as the daemon identity in `crates/blit-tui/src/state.rs:452`.

That does not actually guarantee `(daemon, transfer_id)` uniqueness once F2 fans out. Two daemon instances on the same host but different ports are valid in this codebase (`RemoteEndpoint::host_port_display()` preserves non-default ports, and F1 rows carry a `port`). If both daemons mint the same short transfer id, both streams would pass the same `source_daemon` value, so `row_key("server", "t...")` collides exactly like the pre-m2f-2 bare-id map. The recent dedup check has the same problem because it compares `source_daemon` + `transfer_id` in `crates/blit-tui/src/state.rs:222`.

This should be fixed before m2f-3 builds fan-out on this foundation. Use a stable daemon identity for state keys, at minimum `host_port_display()` for endpoint-backed streams, or split the model into a hidden identity key plus a display label. Add a regression that simulates two source daemons with the same host label but different daemon identities and the same `transfer_id`, proving active rows and recent dedup stay distinct.

## Gates

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed: 579 tests.
