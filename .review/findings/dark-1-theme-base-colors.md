# dark-1-theme-base-colors: configurable base background/foreground

**Severity**: Feature (Milestone E — dark/light, step 1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `775bbe7`

## What

First slice of Milestone E's dark/light support. The TUI historically
sets explicit *foreground* colors but leaves the *background* as the
terminal default. dark-1 adds `[theme] background` + `[theme]
foreground` — a base color layer painted under the whole TUI. Empty
(default) preserves the historical behavior (terminal's own colors); an
operator can set `background="black" foreground="white"` to force a dark
scheme, or `background="white" foreground="black"` for light.

## Approach / mechanism

A base `Block` styled with the configured colors is rendered over the
full frame **first**, before the tab strip and panes. Every fg-only
widget drawn on top inherits the base background, because ratatui leaves
a cell's `bg` unchanged when the widget's `Style` has `bg: None` (and
most of the TUI's widgets style fg only). This is the standard ratatui
base-layer theming approach, and the slice includes a `TestBackend`
test (`base_layer_bg_shows_through_fg_only_widget`) that **proves the
mechanism** rather than assuming it.

`None`/`None` (both unset) → no base layer is drawn at all, so the
terminal default genuinely shows through (not an explicit Reset).

## Approach details

- **config**: `ThemeDefaults.background` / `.foreground` (default `""`).
  `parse_background()` / `parse_foreground()` → `Option<RawColor>`
  (empty → `None` = unset; non-empty unknown name → `None`, reusing the
  e-7 `accent_color_from_str` palette). `background_is_invalid()` /
  `foreground_is_invalid()` (non-empty + unparseable) drive the startup
  warning, since empty is the valid "unset" case.
- **main**: `base_theme_style(bg, fg) -> Option<Style>` (pure; `None`
  only when both colors are unset). Computed per frame (so a `Ctrl+R`
  reload re-colors live, like the accent) and applied at the top of the
  `terminal.draw` closure. Buffer-then-flush warnings for invalid
  base-color names.

## Files changed

- `crates/blit-tui/src/config.rs`: `background`/`foreground` fields;
  `parse_*` + `*_is_invalid`; `base_color()` helper; schema doc;
  Default; theme test initializers updated; parse test.
- `crates/blit-tui/src/main.rs`: `base_theme_style`; per-frame
  computation + base-layer render; invalid-color warnings; two tests.

## Tests

614 blit-tui (workspace 28 binaries):
- `config::theme_base_colors_parse_and_default_to_unset` — empty →
  unset; recognized name parses; unknown → `None` + invalid flag.
- `base_theme_style_built_from_set_colors` — `None` only when both
  unset; carries whichever colors are set.
- `base_layer_bg_shows_through_fg_only_widget` — the core mechanism: a
  fg-only widget over a bg base layer keeps the base bg.

## Scope

Explicit base bg/fg colors only. A later slice can add `[theme] mode =
"dark"|"light"` presets that expand to these. Per-widget contrast
refinements (any widget that explicitly sets a conflicting bg/fg) are
follow-ups if found; the fg-only widgets — the vast majority — inherit
correctly. The help overlay uses `Clear` (its own bg), unaffected.

## Reviewer comments

(empty — pending grade)
