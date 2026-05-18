# d-20-f2-recent-throughput review

Reviewed sha: `ecbb24f408dd735f56235570f4292a1e11587582`

## Findings

1. Low - F2 module layout docs still describe the old table columns.

   `crates/blit-tui/src/screens/f2.rs:15` and `crates/blit-tui/src/screens/f2.rs:18` still show the pre-d-20 layout (`bytes bps` for Active, and `duration ok` for Recent). The actual renderers now emit Active as `bytes / throughput / age` and Recent as `bytes / duration / throughput`. Since this slice specifically adds the recent throughput column, please update the module-level layout sketch so future agents do not use stale column docs as the F2 contract.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed (234 tests).
- `cargo test --workspace` passed.
