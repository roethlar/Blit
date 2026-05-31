# d-21-f2-active-cursor: row cursor on F2 active table

**Severity**: Feature (prerequisite — enables a future
"cancel selected transfer" slice)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

F2's active table gains a row cursor. `j` / `k` (or
Up / Down) walk through the active transfers; the
selected row renders with a black-on-cyan highlight
(matching the existing tab-strip active-pane visual).

```
transfer_id  kind   peer    module/path
─────────────────────────────────────────────
abc-1        pull   peer-A  mod/file
def-2        push   peer-B  mod/big.tar  ← cursor
ghi-3        pull   peer-C  mod/photos
```

The cursor is the foundation for a follow-up slice
that adds a `K` (kill) hotkey to cancel the selected
transfer via `CancelJob` RPC. d-21 itself doesn't add
the cancel action — it's a clean state-machine slice
so the cancel slice can land as a pure dispatch +
RPC-plumbing change on a solid base.

## Approach

### State

`TransfersState` gains:

```rust
selected_active: Option<usize>,
```

Methods:

- `selected_active_index() -> Option<usize>` — clamped
  read: returns `None` if the cursor is off-list
  (idx >= active_count).
- `select_next_active()` — first call from `None`
  lands on 0 (newest, since `active_rows()` sorts
  newest-first); subsequent calls advance and clamp.
- `select_prev_active()` — saturates at 0; first call
  from `None` also lands on 0 so j/k work
  symmetrically.

The cursor falls off naturally when the underlying
transfer terminates (Subscribe `Complete` / `Error`
event drops the row). `selected_active_index()`
returns `None` in that case; operator presses j/k to
re-anchor.

### Dispatch

F2's `match` block in `handle_pane_action`:

```rust
Screen::F2 => match action {
    UserAction::Refresh => ...,
    UserAction::SelectNext => app.transfers.select_next_active(),
    UserAction::SelectPrev => app.transfers.select_prev_active(),
    _ => {}
},
```

The global `j`/`k`/Up/Down → `SelectNext`/`SelectPrev`
keymap from F1/F3 is reused — no new keys.

### Render

`render_active_table` switches from `render_widget` to
`render_stateful_widget` with a `TableState`. The
table gains `row_highlight_style` (black-on-cyan,
matching the d-15 tab-strip accent default). When
`selected_active_index()` returns `None`, no row
highlights (TableState's selected is None).

## Files changed

- `crates/blit-tui/src/state.rs`:
  - `selected_active: Option<usize>` field.
  - `selected_active_index`, `select_next_active`,
    `select_prev_active` methods.
- `crates/blit-tui/src/main.rs`:
  - F2 dispatch arm gains `SelectNext`/`SelectPrev`
    branches.
- `crates/blit-tui/src/screens/f2.rs`:
  - `render_active_table` uses `render_stateful_widget`
    + `TableState` + `row_highlight_style`.

## Tests

+6 unit tests (234 → 240):

In `state::tests`:
- `selected_active_index_is_none_until_first_navigation`
- `select_next_active_lands_on_index_zero_first_time`
- `select_next_active_walks_through_rows` — covers
  walk + clamp-at-end.
- `select_prev_active_saturates_at_zero`
- `select_next_active_no_op_on_empty_list`
- `selected_active_index_falls_off_when_row_terminates`
  — Complete event drops the row, cursor goes None,
  operator can re-anchor.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No cursor action yet.** d-21 only adds the cursor;
   a follow-up slice will wire `K` to cancel the
   selected transfer via
   `blit_app::admin::jobs::cancel`. That slice will
   also add a `selected_active_id() -> Option<String>`
   accessor (intentionally deferred so this slice
   doesn't carry dead methods).

2. **No cursor on the recent table.** d-21 only adds
   selection on Active. Recent is read-only — no
   actions to dispatch from a selection — so a cursor
   there would be visual cruft.

3. **Cursor doesn't persist across pane visits.** The
   cursor lives on TransfersState which survives, so
   actually it DOES persist. The "fall off" behavior
   above handles row turnover. No gap here; mentioned
   to head off the question.

## Out of scope (next slices)

- **F2 cancel-selected-transfer** (`K` hotkey + CancelJob
  RPC).
- **Recent-table cursor** (no actions to dispatch).
- **Selection across both tables** (Tab between active
  and recent).

## Reviewer comments

### Round 1 verdict — reopened (`.review/results/d-21-f2-active-cursor.reopened.md`)

One Medium-severity finding, addressed in round 2:

- **Index-anchored cursor silently retargets after row
  removal.** The round-1 cursor was `Option<usize>` —
  an index into `active_rows()`. When the selected row
  Completed / Errored:
  - **Middle-row case**: the index stayed the same, so
    the cursor moved to whatever transfer occupied that
    slot after the removal — a different transfer with
    no operator consent. Particularly dangerous for the
    planned `K` cancel slice, which would have killed
    the wrong transfer.
  - **Solo-row case**: index stayed `Some(0)` while the
    list was empty. A later new transfer popped into
    index 0 — and the cursor "came back" pointing at
    it, again without consent.

  Round 2 fix: cursor is now `Option<String>` (the
  `transfer_id`). `selected_active_index()` derives the
  display index by `position()` over the sorted view.
  When the id is no longer present, `position()`
  returns `None` and the cursor naturally falls off.
  An unrelated new transfer with a different id doesn't
  accidentally re-anchor.

  Bonus side-effect: `select_next_active` /
  `select_prev_active` now treat a stale-id cursor as
  "no cursor" — pressing j after a removal re-anchors
  at index 0, instead of walking forward from a stale
  index.

### Round 2 file changes

- `crates/blit-tui/src/state.rs`:
  - Field rename: `selected_active: Option<usize>` →
    `selected_active_id: Option<String>`.
  - `selected_active_index()` derives via `position()`.
  - `select_next_active` / `select_prev_active` resolve
    via id and snapshot the new row's id.

### Round 2 tests

+2 regression tests (240 → 242):

In `state::tests`:
- `middle_row_complete_does_not_retarget_cursor` — 3
  rows, cursor on the middle one, complete the middle.
  Cursor falls off (returns None) instead of jumping
  to the next-index transfer.
- `solo_row_complete_then_new_start_keeps_cursor_off`
  — solo row selected, completes (list empty), then a
  new unrelated transfer starts. Cursor stays off-list;
  operator must press j/k to anchor on the new row.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

The id-anchored model is also what the future `K`
cancel slice needs — it can read `selected_active_id`
directly without re-resolving an index against a list
that might have shifted between selection and action.
