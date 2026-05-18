# e-7-config-theme: themable accent color

**Severity**: Feature (polish — fifth slice on the e-3
config scaffold)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

`[theme] accent_color` lets the operator pick the
active-tab background color in the tab strip. Default
`"cyan"` matches the d-15 baseline. Useful for
operators with red-green colorblindness or a custom
terminal palette where the default cyan reads as muddy.

```toml
[theme]
accent_color = "magenta"
```

Recognized values (case-insensitive): `black`, `red`,
`green`, `yellow`, `blue`, `magenta`, `cyan`,
`gray`/`grey`, `darkgray`/`dark_gray`/`grey`-variants,
`lightred`...`lightcyan` (with underscores accepted),
and `white`. Unknown values warn at startup and fall
back to cyan — same buffer-then-flush contract as
parse errors, so the warning is visible after the TUI
exits.

## Approach

### Config

`TuiConfig` gains a `theme: ThemeDefaults` section:

```rust
pub struct ThemeDefaults {
    pub accent_color: String,  // default "cyan"
}

impl ThemeDefaults {
    pub const DEFAULT_ACCENT: &'static str = "cyan";
    pub fn parse_accent(&self) -> Option<RawColor>;
}
```

`RawColor` is a renderer-agnostic enum (16 ANSI named
colors) — `config` doesn't depend on ratatui types,
preserving the clean schema layer. `main.rs` has a
`raw_color_to_ratatui` helper that bridges.

### Render path

`screens::render_tab_strip` and `build_tab_spans` gain
an `accent: Color` parameter. The active-tab style
swaps `Color::Cyan` (previously hardcoded) for the
operator's choice.

### Startup validation

`main` calls `tui_config.theme.parse_accent()` once
after load. `None` pushes a warning into the same
buffer as parse errors, so an unknown color surfaces
exactly like an unknown field: visible after exit, no
TUI corruption. The renderer falls back to
`Color::Cyan` silently if `parse_accent` returns
`None` so it never panics on a bad config.

## Files changed

- `crates/blit-tui/src/config.rs`:
  - `ThemeDefaults` + `RawColor` enum + parser.
  - `TuiConfig` gains `theme` section.
- `crates/blit-tui/src/main.rs`:
  - `raw_color_to_ratatui` bridge function.
  - `main` validates accent + buffers warning if unknown.
  - `run_router` computes `accent_color` once + passes
    to draw call.
- `crates/blit-tui/src/screens/mod.rs`:
  - `render_tab_strip` + `build_tab_spans` gain
    `accent: Color` parameter.
  - Test callers updated to pass `Color::Cyan`.

## Tests

+5 unit tests (220 → 225):

In `config::tests`:
- `theme_default_is_cyan` — default value + parses
  cleanly.
- `theme_parses_each_supported_color` — every name in
  the recognized palette (including `grey` /
  `dark_grey` / underscore variants).
- `theme_parse_is_case_insensitive` — `"CyAn"` works.
- `theme_parse_unknown_color_returns_none` — `fuchsia`
  doesn't match, returns `None` so the caller can warn.
- `theme_round_trips_through_toml` — `[theme]
  accent_color = "magenta"` parses through serde + the
  bridge.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **Only one themable surface.** The active-tab
   background is the only color this slice exposes.
   F4 verify mode-hint magenta, transfer error red,
   verify done green, etc. all stay hardcoded. A future
   polish could expose `warn_color` / `error_color` /
   `ok_color` separately.

2. **No hex / RGB support.** Only the 16 named ANSI
   colors. Operator who wants a custom RGB shade can't
   express it. Ratatui supports `Color::Rgb(r, g, b)`
   but adding it to the config schema requires either
   a tagged-union or a string parser ("#ff8800"). Out
   of scope.

3. **No live preview.** Operator changes the color in
   `tui.toml`, must relaunch the TUI to see the
   effect. Hot-reload of `tui.toml` would be nice but
   needs file-watcher infrastructure.

## Out of scope (next slices)

- **More themable surfaces** (warn / error / ok).
- **RGB / hex color support.**
- **Hot-reload of tui.toml.**
- **Per-pane refresh intervals.**

## Reviewer comments

(empty — pending grade)
