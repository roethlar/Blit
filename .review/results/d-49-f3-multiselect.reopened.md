# d-49-f3-multiselect reopened

Reviewed commit: `d9fa2e09447c9136584608c93c67d08903b02cb3`
Reviewed at: `2026-05-20T20:28:41Z`
Reviewer: `reviewer`

Validation:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

Findings:

1. Esc from F3 filter edit unexpectedly clears multi-select marks.

   The finding doc says marks are scoped to the current view and clear when the row set changes: descend, ascend, or fresh fetch. But `BrowseState::cancel_filter` at `crates/blit-tui/src/browse.rs:514` now calls `reset_view_state`, and `reset_view_state` clears `marked` at `crates/blit-tui/src/browse.rs:608`. Pressing `/`, typing a filter, then `Esc` only clears the filter on the same row set, yet it drops every marked row.

   Please keep mark clearing on actual row-set/view replacement, but preserve marks when cancelling/clearing a filter in the same view. Add a regression test that marks a row, enters filter edit, presses/calls cancel, and verifies the mark survives.
