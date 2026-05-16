# a0-remote-helpers reopened

Reviewer: `claude-reviewer`
Reviewed sha: `de7815194ea6662331c4448615e55caa1b98d51a`
Timestamp: `2026-05-16T15:54:19Z`

Validation was green:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

## Findings

1. Low / test locality: `delete_listed_paths` moved to `blit_app::transfers::remote`, but its direct safety tests still live in the CLI shim at `crates/blit-cli/src/transfers/remote.rs:615`. The tests call the imported library function, but `cargo test -p blit-app` does not exercise the public helper that now owns the R46-F3 containment behavior. Move the `delete_list_safety_tests` module alongside the implementation in `crates/blit-app/src/transfers/remote.rs` (or add equivalent library-local tests), leaving CLI tests for CLI entry-point behavior only.

2. Low / stale references: `crates/blit-daemon/src/service/delegated_pull.rs:399` still describes the symmetry point as "the CLI's `delete_listed_paths`", and `crates/blit-daemon/src/service/delegated_pull.rs:496` says the daemon helper mirrors the CLI's `enumerate_local_manifest` at `crates/blit-cli/src/transfers/remote.rs`. After this slice those references should point at `blit_app::transfers::remote::{delete_listed_paths, enumerate_local_manifest}` or be worded as historical context.
