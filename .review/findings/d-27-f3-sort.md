# d-27-f3-sort: F3 rows sorted (dirs first, alphabetical)

**Severity**: Feature (polish — UX consistency across
fetches)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

F3 rows now come out in a stable sorted order:
**directories before files, then alphabetical
(case-insensitive) within each group**. Pre-d-27,
modules came out in `HashMap` hash order (the daemon
stores them in a `HashMap<String, ModuleConfig>`), so
the F3 module list could shuffle across reconnects.
Directory listings came out in `PathBuf` order, which
mixes dirs + files arbitrarily.

```
Pre-d-27 (HashMap hash order):     Post-d-27 (sorted):
  photos                             backups
  backups                            home
  home                               photos
  ...different next reconnect       ...stable every time
```

This is the file-manager convention (Finder, Explorer,
`ls --group-directories-first`) and the convention
operators expect when they're scanning a folder.

## Approach

### Sort helper

Two new free functions at module level (just above the
test mod):

```rust
fn sort_rows(rows: &mut [BrowseRow]) {
    rows.sort_by_cached_key(|row| {
        (sort_priority(&row.kind), row.name.to_lowercase())
    });
}

fn sort_priority(kind: &BrowseRowKind) -> u8 {
    match kind {
        BrowseRowKind::Module { .. } => 0,
        BrowseRowKind::Directory => 1,
        BrowseRowKind::File => 2,
    }
}
```

`sort_by_cached_key` builds the composite key once per
row, avoiding the `to_lowercase()` allocation on every
comparator call (`sort_by_key`'s closure runs for every
comparison, not every row).

### Call sites

Both fetch handlers call `sort_rows` after building
`self.rows`:

```rust
pub fn apply_modules(&mut self, modules, fetched_at) {
    self.rows = modules.into_iter().map(...).collect();
    sort_rows(&mut self.rows);
    /* selected = 0; status = Loaded; reset_filter(); */
}

pub fn apply_listing(&mut self, ..., entries, ...) {
    self.rows = entries.into_iter().map(...).collect();
    sort_rows(&mut self.rows);
    /* ditto */
}
```

The cursor reset to 0 already happens in both — it now
lands on the first sorted row instead of the first
fetched row. The d-26 filter reset stays as-is (fresh
fetch = fresh view = no filter).

### Existing tests

3 existing tests assumed the raw fetch order and
needed updating:

- `apply_modules_populates_rows_and_resets_cursor`:
  input `[home, backups]` → post-sort `[backups, home]`.
  Flipped the read-only assertion at row 0/1.
- `visible_indices_filters_by_substring`: `populated_state()`
  input `[home, backups, photos, scratch]` → post-sort
  `[backups, home, photos, scratch]`. The 's' filter
  now matches indices `[0, 2, 3]` (was `[1, 2, 3]`).
- `ascend_clears_filter` / `apply_listing_clears_stale_filter`:
  both descended from `populated_state()` row 0 and
  applied a listing for "home". Post-d-27 row 0 is
  "backups", so the apply_listing arg changed to
  "backups" to match the view.

No production-code path needed updating — `selected_row()`,
`descend()`, `visible_selected_position()` all reference
indices, not specific names, so they "just work" on the
sorted layout.

## Files changed

- `crates/blit-tui/src/browse.rs`:
  - New `sort_rows` + `sort_priority` free functions.
  - `apply_modules` / `apply_listing` call `sort_rows`.
  - Module-doc paragraph on d-27.
  - 4 existing tests updated for the new layout.

## Tests

+5 tests (303 → 308):

- `apply_modules_sorts_alphabetically` — reverse-input
  → ascending output.
- `apply_modules_sort_is_case_insensitive` —
  "Backups"/"alpha"/"Cache" sort by lowercase, not by
  raw bytes (otherwise ASCII would put "Backups" first).
- `apply_modules_sort_is_deterministic_regardless_of_input_order`
  — two different input orders produce identical
  output. This is the headline regression test for the
  daemon's `HashMap` non-determinism.
- `apply_listing_sorts_dirs_before_files` —
  `[zfile.txt, photos, afile.txt, docs]` →
  `[docs, photos, afile.txt, zfile.txt]`.
- `sort_priority_matrix` — pins the 0/1/2 numeric ranks
  directly so a future tweak (e.g. inserting a new
  kind) has to update the test deliberately.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No operator-chosen sort modes.** Always
   dirs-then-files alphabetical. A future polish could
   add a key to toggle sort-by-mtime or sort-by-size.

2. **Locale-naive sort.** `to_lowercase()` is
   Unicode-aware but does case-fold, not collation —
   accented characters and non-Latin scripts may sort
   unintuitively. A pure-Rust collation library
   (e.g. `icu_collator`) would fix this but adds a
   dependency. Out of scope.

3. **No sort-stability test.** `sort_by_cached_key`
   uses a stable sort algorithm so equal-key rows
   retain insertion order, but there's no test pinning
   this behavior. Hard to write a meaningful test
   without two rows with truly equal keys (same kind,
   same lowercased name) — possible with `"Foo"` and
   `"foo"`, but in F3 those rarely coexist.

## Out of scope (next slices)

- **Cancel confirmation prompt** (d-22 known gap #1).
- **Batch cancel Shift-K** (d-22 known gap #2).
- **F3 no-matches message** (d-26 known gap #4).
- **Hot-reload tui.toml.**

## Reviewer comments

(empty — pending grade)
