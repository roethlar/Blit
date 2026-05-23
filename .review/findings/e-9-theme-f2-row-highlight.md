# e-9-theme-f2-row-highlight: F2 active-row highlight honors theme accent

**Severity**: Feature (Milestone E polish — theme)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `d1fa091`

## What

e-7 made the tab-strip active-tab background read from `[theme]
accent_color`, but the F2 **active-table** selected-row highlight stayed
a hardcoded `Color::Cyan`. The e-7 finding deferred it with a comment
("the row highlight is an internal selection marker"). That deferral is
worth revisiting now: with the multi-daemon F2 work (m2f-1..10) the
operator navigates the active-row selection to choose *which daemon's*
transfer to cancel (`K`/`X`), so the selection highlight is a primary,
frequently-used affordance — and the same colorblind / custom-palette
accessibility rationale that motivated e-7 applies to it. e-9 makes that
highlight honor the configured accent.

## Approach

- `f2::render_into` and `render_active_table` take an `accent: Color`;
  the active table's `row_highlight_style` uses it instead of the
  hardcoded cyan.
- The router passes the `accent_color` it already computes each frame
  (so a `Ctrl+R` theme reload re-colors the highlight live, same as the
  tab strip). Default remains cyan, so the visual is unchanged unless
  the operator sets `accent_color`.

## Files changed

- `crates/blit-tui/src/screens/f2.rs`: `accent` param threaded into
  `render_into` → `render_active_table`; highlight uses it; comment
  updated; render test.
- `crates/blit-tui/src/main.rs`: F2 render call passes `accent_color`.

## Tests

600 total (+1): `active_row_highlight_uses_accent_color` — renders a
selected row with an off-default accent (red) on a `TestBackend` and
asserts (a) some cell carries the red highlight background and (b) no
cell carries cyan (guards against a stray hardcoded cyan remaining).

## Scope

F2 active table only — the multi-daemon selection target. The F3 tree
and F4 panes also have hardcoded `Color::Cyan` selection/section colors;
threading the accent through those is a possible follow-up but out of
scope here to keep the slice atomic.

## Reviewer comments

(empty — pending grade)
