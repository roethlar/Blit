# e-11-theme-f1-highlight: F1 daemon-list highlight honors theme accent

**Severity**: Feature (Milestone E polish — theme)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `ab85658`

## What

The last screen in the theme-accent rollout. The F1 daemon-list
selected row hardcoded `fg(Color::Black).bg(Color::Cyan)`, just as
F2/F3/F4 did before e-9/e-10. e-11 makes it honor `[theme] accent_color`
with `super::contrasting_fg(accent)` for the foreground (white on dark
accents). The selection highlight is now consistent and readable across
all four screens **and** the tab strip (e-7).

## Approach

- `f1::render_into` → `render_table` take `accent: Color`; the
  daemon-list `row_highlight_style` uses `bg(accent)` +
  `fg(super::contrasting_fg(accent))`.
- The router passes the `accent_color` it already computes each frame
  (hot-reloadable via `Ctrl+R`).

## Files changed

- `crates/blit-tui/src/screens/f1.rs`: `accent` threaded
  `render_into → render_table`; highlight uses it; render test; existing
  viewport test updated for the new arg.
- `crates/blit-tui/src/main.rs`: F1 render call passes `accent_color`.

## Tests

603 total (+1): `daemon_row_highlight_uses_accent_with_contrast` —
renders the default-selected Local row with a dark accent (red) on a
`TestBackend`; every accent-bg cell carries the contrasting white fg.

## Scope

The daemon-list row highlight. The F1 trigger-modal focused-field cue
(`f1.rs:191`, fg-only cyan — a modal field indicator, not a
row-selection background) is a separate surface, left as-is consistent
with e-10's treatment of F3's fg-only semantic colors.

With e-7 (tab strip) + e-9 (F2) + e-10 (F3/F4) + e-11 (F1), the theme
accent now covers every selection/active-highlight surface in the TUI.

## Reviewer comments

(empty — pending grade)
