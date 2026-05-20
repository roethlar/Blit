# d-51-f3-select-all: `a` selects/clears all visible F3 rows

**Severity**: Feature (polish — closes d-49 known gap #2)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `e914084`

## What

d-49 added per-row `space` multi-select; its known gap #2 was "no
select-all / clear-all". d-51 adds `a`: one keystroke marks every
visible F3 row, or clears them all if they're already all marked.

## Approach

`BrowseState::toggle_mark_all_visible` operates on
`visible_indices()` (the filter-aware visible set):

- If every visible row is already marked → clear them.
- Otherwise → mark them all.

Because it works on the *visible* set, a select-all under an
active `/` filter marks only the rows the operator can see — not
the hidden ones. (Marks remain view-scoped: any view change /
re-fetch clears them, per d-49.)

`a` → `UserAction::F3ToggleMarkAll` → F3 dispatch. Other panes
ignore the variant. While the F3 filter / pull-dest prompt is
open the text handlers absorb `a` as a character; in the F4
Verify form, edit mode absorbs it as text — so `a` only
select-alls in F3 nav mode.

## Files changed

- `crates/blit-tui/src/browse.rs`: `toggle_mark_all_visible` +
  2 tests.
- `crates/blit-tui/src/main.rs`: `F3ToggleMarkAll` action, `a`
  key mapping, F3 dispatch arm; 1 key test; the
  unmapped-keys test now uses `z` (since `a` is now mapped).
- `crates/blit-tui/src/help.rs`: `a` keymap row; modal height
  42→43; keymap test asserts the row.

## Tests

+3 tests (475 → 478):

- `toggle_mark_all_marks_then_clears_everything_visible`.
- `toggle_mark_all_is_filter_scoped` — under filter `s`, marks
  only the 3 matching rows, not `home`.
- `key_action_maps_a_to_f3_toggle_mark_all`.

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

## Known gaps

1. **No invert-selection.** `a` is mark-all / clear-all; there's
   no "invert" toggle. Rarely needed; out of scope.

## Out of scope

- Invert-selection.
- Batch transfer over the marked set (the larger remaining
  consumer of multi-select).

## Reviewer comments

(empty — pending grade)
