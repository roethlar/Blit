# d-28-f3-no-matches-msg: differentiated empty-state on F3

**Severity**: Feature (polish — closes d-26 known gap #4)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

The F3 Stats block previously rendered `(no entries)`
for every empty state: fresh pre-fetch, empty module
list, AND when the d-26 filter excluded every row.
That last case is confusing: the operator typed a
filter, sees an empty table, and the Stats line says
"no entries" — but there ARE entries, just none
matching their filter.

d-28 distinguishes the two:

```
state           filter       message
─────────────── ──────────── ─────────────────────────────
empty rows      empty        (no entries)
empty rows      "foo"        (no entries)
populated rows  matches all  Selected: <row> ...
populated rows  "foo" hides  (no rows match filter)
                everything
```

The new "(no rows match filter)" message hints to the
operator that they should relax the filter rather than
wait for data that's already there.

## Approach

### Helper on `BrowseState`

```rust
pub fn empty_state_message(&self) -> &'static str {
    if !self.rows.is_empty()
        && !self.filter.is_empty()
        && self.visible_indices().is_empty()
    {
        "(no rows match filter)"
    } else {
        "(no entries)"
    }
}
```

The three-way conjunction is deliberate:

- `rows.is_empty()` → no data at all; "no entries" is
  the right phrasing (the operator should wait for the
  fetch).
- `filter.is_empty()` → no filter active; if the table
  is empty it's an honest empty rowset.
- Both rows AND filter non-empty AND visible empty →
  the filter is what's hiding everything; route the
  operator to relax it.

### Renderer

`screens/f3.rs::render_stats` swaps its hardcoded
`"(no entries)"` literal for `state.empty_state_message()`.
Everything else in the None-arm stays — same dark-gray
styling, same paragraph.

### Module-doc

f3.rs layout sketch updated to show the new variants
in the Stats-block region.

## Files changed

- `crates/blit-tui/src/browse.rs`:
  - `empty_state_message` accessor on `BrowseState`.
  - 4 new unit tests.
- `crates/blit-tui/src/screens/f3.rs`:
  - `render_stats` calls `state.empty_state_message()`.
  - Module-doc layout sketch lists the d-28 variants.

## Tests

+4 tests (310 → 314):

- `empty_state_message_when_no_rows_returns_no_entries`
  — fresh state baseline.
- `empty_state_message_when_filter_has_matches_returns_no_entries`
  — populated rows + filter that matches → "no entries"
  (the helper's domain extends to the
  non-None-cursor case too, defensively).
- `empty_state_message_when_filter_matches_nothing_returns_filter_hint`
  — the d-28 headline regression: populated rows + a
  filter that excludes everything.
- `empty_state_message_when_no_rows_with_filter_returns_no_entries`
  — edge case: filter typed before any fetch landed.
  The filter isn't "the reason" the view is empty, so
  the message stays generic.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No locale-aware phrasing.** Hardcoded English
   strings. The TUI doesn't have an i18n story today;
   this slice doesn't change that.

2. **No counter in the no-match case.** The Stats block
   doesn't say "filtered out 12 of 12 rows" — just the
   reason. The d-26 `<V>/<N> entries` line lives on
   the View row, but the None-arm replaces both Stats
   lines. Future polish could surface the total count
   alongside the message.

## Out of scope (next slices)

- **Cancel confirmation prompt** (d-22 known gap #1).
- **Batch cancel Shift-K** (d-22 known gap #2).
- **Hot-reload tui.toml.**

## Reviewer comments

(empty — pending grade)
