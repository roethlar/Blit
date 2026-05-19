# d-30-batch-cancel reopened

Commit reviewed: `aaa9c42e9663749884c44a9c6df52fdf04d4c4c8`

Validation:

- `cargo fmt --all -- --check`: passed
- `cargo clippy --workspace --all-targets -- -D warnings`: passed
- `cargo test --workspace`: passed

## Findings

1. `crates/blit-tui/src/main.rs:237`, `crates/blit-tui/src/main.rs:1070`, `crates/blit-tui/src/main.rs:1919` - Batch confirmation can cancel a different set of transfers than the operator confirmed. `ConfirmingBatch` stores only `count`, and the `y` path later calls `spawn_batch_cancels(&app.transfers, ...)`, which snapshots `active_rows()` at confirm time. The event loop continues applying Subscribe events while the prompt is open (`app.transfers.apply_event(...)`), so the active set can change between `X` and `y`. Repro: A/B are active, operator presses `X` and sees `cancel 2 transfers? y/N`; A/B complete and C/D start before the operator presses `y`; the `y` path cancels C/D. The confirmation prompt needs to freeze the target transfer IDs at prompt creation, then spawn cancels for exactly those IDs.
