# d-32-help-scrollbar: scrollbar indicator on help overlay

**Severity**: Feature (polish — closes d-31 known gap #1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

d-31 made the `?` help overlay scrollable but left a
known gap: no visual affordance that there's more
content above/below. The only cue was the self-doc
`j / k` keymap row. d-32 adds a ratatui `Scrollbar` on
the modal's right edge, shown only when the keymap
overflows the modal's inner height.

```
┌ Help · press ? or Esc to close ──────────────▲
│ Navigation (global)                          █
│  F1 / 1   Daemons pane                        │
│  ...                                          │
│  ...                                          ▼
└───────────────────────────────────────────────
```

On a terminal tall enough to show the whole keymap
(≥36 rows), the scrollbar is absent — it only appears
when it's useful.

## Approach

`render_overlay` computes whether the content overflows
and conditionally renders the scrollbar:

```rust
let inner_height = modal.height.saturating_sub(2); // borders
let total = help_line_count();
if total > inner_height {
    let mut sb_state = ScrollbarState::new(total as usize)
        .position(overlay.scroll_offset() as usize);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("▲"))
        .end_symbol(Some("▼"));
    frame.render_stateful_widget(scrollbar, modal, &mut sb_state);
}
```

`ScrollbarState::new(total).position(offset)` — ratatui
computes the thumb size and position from the rendered
track height. `VerticalRight` draws over the modal's
right border column (the intended look). The `▲` / `▼`
begin/end markers cap the track.

The overflow guard (`total > inner_height`) keeps the
scrollbar out of the way when the whole keymap fits,
matching the d-31 scroll clamp's intent: scrolling
only matters when there's more than fits.

## Files changed

- `crates/blit-tui/src/help.rs`:
  - Import `Scrollbar`, `ScrollbarOrientation`,
    `ScrollbarState`.
  - `render_overlay` renders the conditional scrollbar
    after the Paragraph.
  - 2 new render tests + a `render_to_string` test
    helper.

## Tests

+2 tests (344 → 346):

- `scrollbar_renders_when_content_overflows` — renders
  into an 80×12 backend (modal clamps to 12 rows, inner
  10 < 34 keymap lines) and asserts the `▼` end marker
  appears.
- `scrollbar_absent_when_content_fits` — renders into
  80×40 (modal 36, inner 34 == keymap length) and
  asserts neither `▲` nor `▼` appears.

A shared `render_to_string` test helper flattens a
TestBackend buffer to a string for grep — the existing
`help_modal_documents_all_public_keys` test used the
same pattern inline; future help tests can reuse the
helper.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No PageUp / PageDown.** Inherited from d-31 —
   scroll is still one line per keypress. The scrollbar
   is display-only; it's not draggable (TUI scrollbars
   rarely are).

2. **Thumb size approximation.** ratatui's
   `ScrollbarState` derives the thumb from
   `content_length` vs. track height; with very short
   modals the thumb may round to the whole track. Not
   worth tuning — the markers (`▲`/`▼`) carry the
   "more in this direction" signal regardless.

## Out of scope (next slices)

- **Hot-reload tui.toml.**
- **F3 filter regex/glob mode.**
- **Help overlay PageUp/PageDown.**

## Reviewer comments

(empty — pending grade)
