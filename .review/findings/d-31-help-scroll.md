# d-31-help-scroll: scrollable help overlay

**Severity**: Feature (polish — small-terminal usability)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

The `?` help overlay has grown to 34 keymap lines
across d-22..d-30. The modal is 36 rows tall (34 inner
+ 2 borders). On an 80×24 terminal — the conventional
minimum — the bottom third of the keymap was clipped
with no way to reach it.

d-31 makes the overlay scrollable: while it's open,
`j` / `↓` scroll down and `k` / `↑` scroll up. The
offset clamps so the operator can't scroll the content
entirely off-screen, and resets to the top whenever the
overlay closes.

## Approach

### State

`HelpOverlay` gains `scroll_offset: u16`:

```rust
pub struct HelpOverlay {
    visible: bool,
    scroll_offset: u16,
}
```

Methods:

- `scroll_offset() -> u16` — read for the renderer.
- `scroll_down()` — `offset += 1`, clamped to
  `help_line_count() - MIN_VISIBLE_LINES` (3) so the
  bottom stays reachable without scrolling into a blank
  modal.
- `scroll_up()` — saturating `offset -= 1`.
- `close()` and `toggle()`-to-closed reset
  `scroll_offset = 0` so the next open starts at the
  top.

### Renderer

`render_overlay` now takes the `HelpOverlay` by value
and applies the offset via ratatui's built-in
`Paragraph::scroll((offset, 0))`. Paragraph clips lines
above the offset and below the modal height — exactly
the page-through behavior.

The keymap lines moved into a `help_lines()` function
so the renderer and `help_line_count()` (the scroll
clamp) share one source of truth.

### Router

The help-visible branch in the event loop already
absorbed all keystrokes except `?` / Esc (close) and
Ctrl-c (emergency quit). d-31 adds a scroll match:

```rust
match key.code {
    KeyCode::Char('j') | KeyCode::Down => app.help.scroll_down(),
    KeyCode::Char('k') | KeyCode::Up => app.help.scroll_up(),
    _ => {}
}
```

Everything else stays absorbed — the operator still
can't pane-switch or trigger actions while reading.

### Self-documentation

A new keymap row documents the scroll:

```
j / k   scroll this help (when open)
```

(Net +1 line, bringing the keymap to 34 lines — exactly
filling the 34-row inner modal. The scroll handles
overflow on shorter terminals.)

## Files changed

- `crates/blit-tui/src/help.rs`:
  - `HelpOverlay` gains `scroll_offset` + scroll API.
  - `close()` / `toggle()` reset scroll.
  - `help_lines()` / `help_line_count()` extracted.
  - `render_overlay` takes the overlay, applies
    `Paragraph::scroll`.
  - New `j / k` self-doc row.
  - 7 new tests.
- `crates/blit-tui/src/main.rs`:
  - `render_overlay` call passes `app.help`.
  - Help-visible branch handles j/k/↑/↓ scroll.

## Tests

+7 tests (337 → 344):

- `new_overlay_starts_at_top`.
- `scroll_down_advances_offset`.
- `scroll_up_is_saturating_at_top`.
- `scroll_down_then_up_returns_to_top`.
- `scroll_down_clamps_so_content_stays_visible` — pins
  the `help_line_count() - MIN_VISIBLE_LINES` clamp.
- `close_resets_scroll`.
- `toggle_closed_resets_scroll`.

The existing `help_modal_documents_all_public_keys`
test renders with `HelpOverlay::default()` (offset 0)
so every section is visible for the grep.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No scrollbar indicator.** The modal doesn't show
   a "more below" / "more above" affordance. The
   operator has to try scrolling to discover there's
   more. A future polish could add a ▲/▼ glyph in the
   border or a ratatui `Scrollbar` widget.

2. **No PageUp / PageDown.** Scroll is one line per
   keypress. For a 34-line keymap that's fine, but a
   future polish could add page-jump keys.

3. **Scroll offset is line-granular, not
   section-granular.** Can't jump straight to the "F4
   Verify" section. Future polish could add section
   anchors.

## Out of scope (next slices)

- **Hot-reload tui.toml.**
- **F3 filter regex/glob mode.**
- **Help overlay scrollbar indicator.**

## Reviewer comments

(empty — pending grade)
