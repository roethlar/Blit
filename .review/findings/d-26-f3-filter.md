# d-26-f3-filter: F3 substring filter via `/`

**Severity**: Feature (polish — closes a long-standing
F3 navigation gap)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Pressing `/` on F3 opens a substring filter for the
current view's row list. The operator types text;
non-matching rows hide; cursor stays on a matching
row; Enter commits (filter persists, normal nav resumes);
Esc cancels (filter cleared). Filter is
case-insensitive and matches anywhere in the row name.

```
loaded · 12s ago · filter: pho_    ← editing (cyan)
loaded · 12s ago · filter: pho     ← committed (green)
```

Pre-d-26, the only way to find a specific module or
directory on a daemon with 30+ exports was eye-scrolling
with j/k. d-26 makes typeahead the primary navigation
gesture for that case.

## Approach

### State

Two new fields on `BrowseState`:

```rust
filter: String,           // empty = match-everything
editing_filter: bool,     // true while operator is typing
```

Filter mutation API:

```rust
pub fn begin_edit_filter(&mut self);       // `/`
pub fn commit_filter(&mut self);           // Enter
pub fn cancel_filter(&mut self);           // Esc
pub fn push_filter_char(&mut self, c);     // any char
pub fn pop_filter_char(&mut self) -> bool; // Backspace
pub fn filter(&self) -> &str;
pub fn is_editing_filter(&self) -> bool;
pub fn visible_indices(&self) -> Vec<usize>;
pub fn visible_selected_position(&self) -> Option<usize>;
```

### Cursor invariant

`self.selected` indexes into the unfiltered `self.rows`,
but is always kept on a row that matches the current
filter (or row 0 if no row matches). Every filter
mutation (`push`/`pop`/`cancel`) snaps the cursor via
`first_matching_row()`. `select_next`/`select_prev`
skip non-matching rows so j/k traverses only what's
visible.

The renderer then uses `visible_selected_position()` to
map this raw-row cursor into the visible-row ordinal
that ratatui's `TableState::with_selected` expects.

### View-change reset

`apply_modules`, `apply_listing`, `descend`, `ascend`
all call `reset_filter()` — the row set is changing
underneath the operator, so the filter they typed
no longer reflects what they're looking at. The new
view starts with full visibility.

### Keystroke routing

Mirrors the d-2 / e-1 verify-edit pattern. New helper
`handle_f3_filter_keystroke` absorbs chars / Backspace
/ Enter / Esc while `is_editing_filter()` is true:

```rust
if app.current_screen == Screen::F3
    && app.browse.is_editing_filter()
    && handle_f3_filter_keystroke(&key, &mut app)
{
    continue;
}
```

Bubbles through (returns `false`) for: Ctrl-c
(emergency quit), F-keys (pane nav), `?` (global help),
Ctrl-modified chars (so terminal shortcuts don't append
garbled text).

`/` while NOT editing is mapped via the action
dispatcher as `UserAction::F3FilterBegin`. F1/F2/F4
dispatch arms ignore the variant.

### Rendering

Three changes in `screens/f3.rs`:

1. `render_table` iterates `state.visible_indices()`
   instead of `state.rows()`. With an empty filter
   this is `0..len()` so pre-d-26 panes are
   unchanged.
2. `render_stats` shows `<V>/<N> entries` while
   filtered (so the operator sees how many rows the
   filter hides).
3. `render_footer` adds a filter fragment between the
   status and the key hints:
   - Hidden when filter is empty AND not editing.
   - `filter: foo_` (cyan) while editing.
   - `filter: foo` (green) after commit.

The d-26 module-doc layout sketch lists all three
states.

### Help overlay

New row under "F1 · F2 · F3 navigation":

```
/   filter rows (F3) — Esc clears, Enter commits
```

Modal height 34 → 35 to fit. The d-16 regression test
already pins keys to their sections; `/` is added to
the global key-presence check.

## Files changed

- `crates/blit-tui/src/browse.rs`:
  - `BrowseState`: `filter`, `editing_filter` fields.
  - Filter API methods + `row_matches` / `first_matching_row`
    / `reset_filter` helpers.
  - `select_next` / `select_prev` filter-aware.
  - `apply_modules` / `apply_listing` / `descend` /
    `ascend` reset filter.
  - `selected_index` gated to `#[cfg(test)]` (renderer
    moved to `visible_selected_position`).
  - Module-doc paragraph on d-26.
- `crates/blit-tui/src/screens/f3.rs`:
  - `render_table` uses `visible_indices`.
  - `render_stats` shows V/N count when filtered.
  - `render_footer` accepts `&BrowseState`, renders
    filter fragment.
  - Module-doc layout sketch + filter-fragment variant
    block.
- `crates/blit-tui/src/main.rs`:
  - `UserAction::F3FilterBegin` variant.
  - `key_action` maps `Char('/')`.
  - F3 dispatch arm handles `F3FilterBegin`.
  - `handle_f3_filter_keystroke` helper.
  - Router interception before action dispatch.
  - New tests + a `make_test_app_state` helper.
- `crates/blit-tui/src/help.rs`:
  - `/` row in F1·F2·F3 nav section.
  - Modal height 34 → 35.
  - `centered_clamps` test updated.
  - `help_modal_documents_all_public_keys` adds `/`.

## Tests

+28 tests (271 → 299):

**`browse::tests` — 18 new:**
- `new_state_has_empty_filter_and_not_editing`
- `begin_edit_filter_enters_edit_mode`
- `push_filter_char_appends_and_snaps_cursor`
- `push_filter_char_is_case_insensitive`
- `pop_filter_char_widens_match_set`
- `pop_filter_char_returns_false_on_empty_filter`
- `cancel_filter_clears_text_and_exits_mode`
- `commit_filter_keeps_text_and_exits_mode`
- `visible_indices_returns_all_rows_with_empty_filter`
- `visible_indices_filters_by_substring`
- `visible_indices_empty_when_no_match`
- `select_next_skips_non_matching_rows_when_filter_active`
- `select_prev_skips_non_matching_rows_when_filter_active`
- `visible_selected_position_maps_into_filtered_ordinal`
- `descend_clears_filter`
- `ascend_clears_filter`
- `apply_modules_clears_stale_filter`
- `apply_listing_clears_stale_filter`
- `select_next_with_no_match_keeps_cursor_at_zero`

**`main::tests` — 9 new:**
- `key_action_maps_slash_to_f3_filter_begin`
- `handle_f3_filter_keystroke_routes_chars_to_filter`
- `handle_f3_filter_keystroke_routes_backspace_to_pop`
- `handle_f3_filter_keystroke_routes_enter_to_commit`
- `handle_f3_filter_keystroke_routes_esc_to_cancel`
- `handle_f3_filter_keystroke_returns_false_for_question_mark`
- `handle_f3_filter_keystroke_returns_false_for_f_keys`
- `handle_f3_filter_keystroke_returns_false_for_ctrl_c`
- `handle_f3_filter_keystroke_returns_false_for_ctrl_chars`

**`help::tests` — 1 modification:** the
`help_modal_documents_all_public_keys` test now asserts
`/` is present.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No regex / glob.** Plain case-insensitive substring.
   An operator searching for "log" matches "catalog"
   too. A future polish could add `/regex/` syntax.

2. **No "match by extension" or "match by kind" shortcuts.**
   E.g. `:dir` to show only dirs or `*.log` to show
   only files matching a glob. d-26's scope is just
   the substring case.

3. **No persistence across pane switches.** Switching
   away from F3 and back keeps the filter (state isn't
   touched), but switching across a `descend` /
   `ascend` clears it. The latter is intentional (the
   new view's rows are different), but a future polish
   could offer a sticky "global filter" mode.

4. **No filter-aware "no rows match" message.** When
   the filter excludes every row, the table renders
   empty. The Stats block shows `0/N entries` so the
   operator can see what happened, but a dedicated
   "no matches" overlay row might be friendlier.

## Out of scope (next slices)

- **Cancel confirmation prompt** (d-22 known gap #1).
- **Batch cancel Shift-K** (d-22 known gap #2).
- **Hot-reload tui.toml.**
- **F3 filter regex/glob mode.**

## Reviewer comments

(empty — pending grade)
