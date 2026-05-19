# d-26-f3-filter reopened

Reviewed sha: `47355b438fbd2d9ca91b0c0804f1ddb8a6e5e847`

Validation:
- `cargo fmt --all -- --check`: passed
- `cargo clippy --workspace --all-targets -- -D warnings`: passed
- `cargo test --workspace`: passed

## Findings

1. Medium — zero-match filters leave a hidden row actionable.

   `push_filter_char` falls back to `selected = 0` when the filter matches no rows, and `visible_indices()` correctly returns an empty visible table. But `selected_row()` and `descend()` still read `rows[selected]`, and the F3 dispatcher calls `app.browse.descend()` without checking whether the selected row is visible. The result is that `/zz` + Enter + Enter can show an empty table and then descend into hidden raw row 0. `render_stats` also reports that hidden row as selected while displaying `0/N entries`, which makes the state contradictory.

   Relevant lines:
   - `crates/blit-tui/src/browse.rs:155` — `selected_row()` returns the raw selected row.
   - `crates/blit-tui/src/browse.rs:277` — `descend()` reads the raw selected row without a filter-visibility guard.
   - `crates/blit-tui/src/browse.rs:399` — no-match filter changes fall back to raw row 0.
   - `crates/blit-tui/src/screens/f3.rs:140` — stats display the raw selected row.
   - `crates/blit-tui/src/main.rs:1001` — F3 Enter/Right/l descends unconditionally.

   Please make the zero-match state non-actionable and non-selected from the UI's perspective, then pin it with a regression test. Either expose a filter-aware selected row/action helper, or make navigation/action methods reject the selected row when it does not match the active filter.
