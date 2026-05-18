# d-21-f2-active-cursor review

Reviewed sha: `bb957c31e812635e36ceaca13126b0b066d54b67`

## Findings

1. Medium - Active-row selection can silently retarget a different transfer.

   `crates/blit-tui/src/state.rs:110` stores the cursor as an index into `active_rows()`, and `selected_active_index()` at `crates/blit-tui/src/state.rs:303` only checks whether that index is still within `self.active.len()`. When a selected transfer completes or errors via the removal paths at `crates/blit-tui/src/state.rs:233` and `crates/blit-tui/src/state.rs:255`, the index is left untouched. If the selected row was in the middle of the table, the same index now highlights the next transfer. If the selected row was the only active transfer, `selected_active` remains `Some(0)` while the list is empty, then a later `TransferStarted` makes `selected_active_index()` return `Some(0)` again and highlights a new unrelated transfer without operator re-anchoring.

   That contradicts the handoff contract that the cursor "falls off" when the underlying transfer terminates and matters for the planned `K` cancel slice: the next action could target a transfer the operator did not explicitly select. Please track selection by `transfer_id` (and derive the display index from the sorted rows) or clear/reconcile `selected_active` whenever the selected row disappears. Add regression coverage for at least "selected middle row completes" and "single selected row completes, later new row starts".

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed (240 tests).
- `cargo test --workspace` passed.
