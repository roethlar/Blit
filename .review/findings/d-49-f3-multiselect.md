# d-49-f3-multiselect: space marks F3 rows

**Severity**: Feature (designed — TUI_DESIGN §5.3 `space`)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `d9fa2e0`

## What

The F3 hotkey bar (TUI_DESIGN §5.3) lists `space: multi-select`
as the precursor to batch `c`/`m`/`v` transfer and `D` delete.
d-49 builds that foundation: `space` toggles a multi-select mark
on the cursor row; marked rows are visually flagged and counted.

This is a complete, self-contained unit (selection management),
deliberately scoped *without* a batch action yet — exactly how
d-33 shipped the pull-source *preview* before d-35 wired the pull
itself. A later slice consumes the marked set for batch
transfer/delete.

## Approach

- `BrowseState.marked: HashSet<String>` — names of marked rows in
  the **current view**. `toggle_mark()` flips the cursor row's
  membership (no-op when the filter-aware `selected_row()` is
  `None`, so a hidden cursor can't mark). `is_marked(name)` /
  `marked_count()` expose it to the renderer.
- **Marks are view-scoped.** Names are only unique within a
  single listing, so the existing row-set-change chokepoint
  (renamed `reset_filter` → `reset_view_state`, called from
  `descend`/`ascend`/`apply_modules`/`apply_listing`) now clears
  marks alongside the filter. Switching directories or re-fetching
  starts with a clean selection.
- **Render**: marked rows get a leading `◉ ` marker + bold name
  (unmarked rows get a 2-space pad so columns align); the footer
  shows `N selected` in magenta when any are marked.
- **Key**: `space` → `UserAction::F3ToggleMark` → F3 dispatch
  `toggle_mark()`. Other panes ignore the variant. While the F3
  filter / pull-dest prompt is open the text handlers absorb
  space as a character first; in the F4 Verify form, edit mode
  absorbs it as text — so `space` only marks in F3 nav mode.

## Files changed

- `crates/blit-tui/src/browse.rs`: `marked` set + `toggle_mark` /
  `is_marked` / `marked_count`; `reset_filter` → `reset_view_state`
  (now clears marks); 4 tests.
- `crates/blit-tui/src/screens/f3.rs`: row marker + bold for
  marked rows; footer `N selected` fragment.
- `crates/blit-tui/src/main.rs`: `F3ToggleMark` action, `space`
  key mapping, F3 dispatch arm; 1 key test.
- `crates/blit-tui/src/help.rs`: `space` keymap row; modal height
  41→42; keymap test asserts the row.

## Tests

+6 tests (461 → 467):

- `browse`: toggle marks/unmarks the cursor row; marks accumulate
  across rows; marks clear on descend (view change); toggle is a
  no-op when the filter hides the cursor.
- `main`: `space` → `F3ToggleMark`.

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

## Known gaps

1. **No batch action yet.** Marking is the foundation; `c`/`m`/`v`
   batch transfer and batch `D` delete (Purge takes a `Vec` of
   paths already) are the next slice. Today the marks are visible
   and counted but no key consumes them — same staged shape as
   d-33 (preview) → d-35 (action).
2. **No "select all / clear all" key.** Only per-row `space`
   toggling; a view change clears everything. A `*`/`Esc`-style
   bulk toggle could come with the batch-action slice.

## Out of scope

- Batch transfer / delete over the marked set (next slice).
- Select-all / clear-all shortcuts.

## Reviewer comments

### Round 1 verdict — reopened (`.review/results/d-49-f3-multiselect.reopened.md`)

One finding:

- **`Esc` from filter edit unexpectedly cleared marks.** The
  rename `reset_filter` → `reset_view_state` (now clearing marks)
  was also wired into `cancel_filter`, but cancelling a filter
  doesn't change the row set — so `/` + type + `Esc` dropped
  every mark. Marks should only clear on an actual row-set
  replacement.

### Round 2 fix

- Split `clear_filter_only` (filter text + edit mode, marks
  preserved) out of `reset_view_state`. `cancel_filter` now calls
  `clear_filter_only`; `reset_view_state` (= `clear_filter_only`
  + `marked.clear()`) stays wired to the genuine row-set changes
  (`descend` / `ascend` / `apply_modules` / `apply_listing`).

### Round 2 test

+1 test (467 → 468):

- `marks_survive_filter_cancel` — mark a row, enter filter edit,
  `cancel_filter`, assert the mark survives and the filter text
  is cleared.

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

### Lesson restated

Renaming a helper to take on a broader responsibility
(`reset_filter` → `reset_view_state` + mark-clearing) silently
changed every caller's behavior — including one (`cancel_filter`)
where the new responsibility was wrong. When widening what a
shared helper does, audit each call site against the *new*
semantics, not just the rename.
