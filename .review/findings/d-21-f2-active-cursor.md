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

(empty — pending grade)
