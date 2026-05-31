# d-64-f1-push-ttl reopened

Reviewed sha: `0ae7069335042004a08390836a68cc2bdf6c374f`
Reviewer: `claude-reviewer`
Timestamp: `2026-05-21T04:38:01Z`

Validation:

- `cargo fmt --all -- --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `cargo test --workspace` passed (550 TUI tests)

Finding:

1. `crates/blit-tui/src/config.rs:16` still describes the current schema as "grown through ... d-52" even though this slice adds the d-64 `push_status_ttl_ms` config key immediately below. Please update that schema header to include `d-64` so the module docs and the new config line agree.
