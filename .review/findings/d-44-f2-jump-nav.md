# d-44-f2-jump-nav: g/G on the F2 active-transfer cursor

**Severity**: Feature (navigation polish — closes d-42 gap #1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `8a41657`

## What

d-42 added `g`/`G` jump-to-first/last to the F1 and F3 list
cursors but deliberately skipped F2 — its active-row cursor is
anchored by `transfer_id` (d-21), not an index, so it needed its
own methods. d-44 closes that gap: `g`/`G` now jump the F2
active-transfer cursor too.

## Approach

`TransfersState::select_first_active` / `select_last_active`
re-anchor `selected_active_id` on the first / last row of
`active_rows()` (the same start-time-sorted ordering `j`/`k`
walk). No-op when there are no active transfers — they clear the
anchor to `None` rather than inventing a selection, matching the
d-21 R2 "never lie about which transfer is selected" contract.

The `UserAction::SelectFirst` / `SelectLast` variants and the
`g`/`G`/`Home`/`End` key mappings already exist from d-42; d-44
just adds the F2 dispatch arm:

```rust
UserAction::SelectFirst => app.transfers.select_first_active(),
UserAction::SelectLast  => app.transfers.select_last_active(),
```

## Files changed

- `crates/blit-tui/src/state.rs`: `select_first_active` /
  `select_last_active` + 2 tests.
- `crates/blit-tui/src/main.rs`: F2 dispatch arm wires the two
  existing actions.
- `crates/blit-tui/src/help.rs`: `g / G` row description
  `(F1, F3)` → `(F1, F2, F3)`.

## Tests

+2 tests (436 → 438):

- `select_first_and_last_active_anchor_the_cursor` — 3 active
  rows; `G` anchors index 2, `g` anchors index 0.
- `select_first_last_active_noop_on_empty_list` — no active
  transfers → cursor stays unanchored.

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

## Known gaps

1. **Stale-anchor fall-off is unchanged.** If the first/last
   transfer terminates after a jump, the cursor falls off (the
   d-21 R2 contract: `selected_active_id` resolves to `None`
   until the next nav). `g`/`G` re-anchor cleanly on the next
   press. No new behavior here — same as `j`/`k`.

## Out of scope

- Jumping within the F2 recent/history list (no cursor there).

## Reviewer comments

(empty — pending grade)
