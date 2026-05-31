# e-10-theme-f3f4-highlight: F3/F4 selection highlights honor theme accent

**Severity**: Feature (Milestone E polish — theme)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `895fe06`

## What

Completes the e-9 follow-up. e-9 themed the F2 active-row highlight; the
F3 browse-tree selected row (`f3.rs` `render_table`) and the F4 focused
Verify field (`f4.rs` `field_line`) still hardcoded
`fg(Color::Black).bg(Color::Cyan)`. e-10 makes both honor `[theme]
accent_color`, with the foreground from `super::contrasting_fg(accent)`
(white on dark accents) — so a dark accent stays readable, consistent
with the tab strip (e-7) and F2 (e-9).

## Approach

- `f3::render_into` → `render_table` take `accent: Color`; the browse
  table's `row_highlight_style` uses `bg(accent)` +
  `fg(super::contrasting_fg(accent))`.
- `f4::render_into` → `render_verify` → `field_line` take `accent:
  Color`; the focused-field `value_style` uses the same pairing.
- The router passes the `accent_color` it already computes each frame
  (hot-reloadable via `Ctrl+R`) to both `render_into` calls.

`contrasting_fg` is the private helper in `screens/mod.rs` (e-7); child
modules reach it via `super::` — no visibility change.

## Files changed

- `crates/blit-tui/src/screens/f3.rs`: `accent` threaded
  `render_into → render_table`; highlight uses it; render test.
- `crates/blit-tui/src/screens/f4.rs`: `accent` threaded
  `render_into → render_verify → field_line`; focused-field style uses
  it; `field_line` unit test.
- `crates/blit-tui/src/main.rs`: F3 + F4 render calls pass `accent_color`.

## Tests

602 total (+2):
- `f3::browse_row_highlight_uses_accent_with_contrast` — renders a
  selected browse row with a dark accent (red) on a `TestBackend`; every
  accent-bg cell carries the contrasting white fg.
- `f4::verify_focused_field_uses_accent_with_contrast` — `field_line`
  with a dark accent → bg=red, fg=white; light accent (cyan) → fg=black;
  unfocused field carries no accent bg.

## Scope

Selection / focus highlights only. The remaining F3 fg-only
`Color::Cyan` sites (`f3.rs:378/401/519`) are semantic foreground colors
(directory names, headers, filter fragment), not selection markers, so
they're intentionally left as-is.

## Reviewer comments

(empty — pending grade)
