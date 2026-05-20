# d-47-f1-browse-nav reopened

Reviewed commit: `065fd95f6dfc055dd5cfa0e9bcc4af0d6c973b7a`
Reviewed at: `2026-05-20T19:58:07Z`
Reviewer: `reviewer`

Validation:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

Findings:

1. F1 Enter on the Local row is not a no-op.

   The d-47 finding says the Local row is a no-op because F3 is a remote browser. The implementation at `crates/blit-tui/src/main.rs:1261` resolves the selected row through `DaemonsState::endpoint_for_row` and retargets whenever it returns `Some`. That helper explicitly returns `127.0.0.1:9031` for Local rows at `crates/blit-tui/src/daemons.rs:324`, so pressing Enter on Local jumps to F3 and browses the loopback daemon instead of doing nothing.

   Please gate out `row.is_local()` before calling `retarget_browse` (or revise the feature contract and tests if Local daemon browsing is now intended). Add coverage for Enter/Descend on the Local row specifically; the current retarget test only exercises a normal remote endpoint helper path.
