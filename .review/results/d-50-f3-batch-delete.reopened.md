# d-50-f3-batch-delete reopened

Reviewed commit: `1349cfb50d89b38f531b3ce22a10f102b38281e5`
Reviewed at: `2026-05-20T20:47:41Z`
Reviewer: `reviewer`

Validation:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

Findings:

1. Batch delete Done/Error footer can persist indefinitely after the operation.

   Batch deletes set `gate_path = None`, and `f3_del_to_display` treats that as always visible at `crates/blit-tui/src/main.rs:2409` through `crates/blit-tui/src/main.rs:2423`. The comments in `crates/blit-tui/src/main.rs:2406` and `crates/blit-tui/src/f3del.rs:23` say the batch outcome shows until the next action, but no action clears `F3DelStatus::Done` or `F3DelStatus::Error`. After a successful batch delete, pressing Down, changing filters, refreshing, or navigating elsewhere in F3 still renders `deleted N item(s)` because `None => true` bypasses the path gate.

   Please add a real terminal-outcome lifecycle for batch delete: either clear batch Done/Error on the next relevant F3 action, or use the same TTL/live-tick pattern as d-38. Add a regression test that applies a batch Done/Error and then simulates an ordinary subsequent F3 action or TTL expiry, verifying the stale footer no longer renders.
