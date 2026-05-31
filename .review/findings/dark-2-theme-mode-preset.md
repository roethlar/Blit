# dark-2-theme-mode-preset: `[theme] mode = dark|light` presets

**Severity**: Feature (Milestone E — dark/light, step 2)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `e0b631b`

## What

Second dark/light slice, built on the dark-1 base layer. Adds `[theme]
mode` — `"dark"` (black bg / white fg) or `"light"` (white bg / black
fg) — so an operator gets a full scheme from one setting instead of two
explicit colors. Empty (default) = no preset (terminal default unless
`background`/`foreground` are set).

## Approach

- **config**: `ThemeDefaults.mode` (default `""`). `mode_preset()`
  returns the `(bg, fg)` pair for `dark`/`light` (case-insensitive,
  trimmed), `None` otherwise. `mode_is_invalid()` (non-empty + not a
  known preset) drives the warning.
- **resolution**: `resolved_base_colors() -> (Option<RawColor>,
  Option<RawColor>)` combines preset + explicit **per-field**, with an
  explicit `background`/`foreground` **overriding** the preset. So
  `mode = "dark"` + `foreground = "green"` → black bg + green fg.
- **main**: the base layer now feeds from `resolved_base_colors()`
  instead of the raw `parse_background`/`parse_foreground`. Mode-invalid
  startup warning added.

## Files changed

- `crates/blit-tui/src/config.rs`: `mode` field; `mode_preset`,
  `mode_is_invalid`, `resolved_base_colors`; schema doc; Default; test.
- `crates/blit-tui/src/main.rs`: base layer uses `resolved_base_colors`;
  mode-invalid warning.

## Tests

615 blit-tui (workspace 28 binaries):
- `config::theme_mode_presets_and_override` — dark/light presets,
  case-insensitive; per-field explicit override; unknown mode →
  ignored + flagged; no mode → `(None, None)` (terminal default).
- dark-1's `base_layer_bg_shows_through_fg_only_widget` (the render
  mechanism) is unchanged — dark-2 only changes which colors feed it.

## Scope

`mode` presets resolving to the dark-1 base bg/fg. With dark-1 (explicit
base colors) + dark-2 (mode presets), the dark/light Milestone-E item is
functionally complete. Per-widget contrast follow-ups (any widget that
hardcodes a bg/fg fighting the base layer) remain possible if found, but
the fg-only widgets — the vast majority — inherit correctly.

## Round 2 (commit `ce4c50f`)

**Reopen finding:** `resolved_base_colors` used `parse_*().or(preset)`,
but `parse_background`/`parse_foreground` return `None` for **both** an
unset (empty) value **and** an invalid (non-empty unparseable) one. So
`mode = "dark"` + `background = "blurple"` (a typo) fell through to the
preset's black — contradicting the dark-1 startup warning ("using the
terminal default") and the dark-2 per-field-override contract.

**Fix (reviewer policy 1):** resolve each field on whether it is
**empty**, not on parse success. A non-empty (explicitly set) field
always overrides the preset — recognized → that color, unrecognized →
`None` (terminal default, matching the warning). Only an empty field
inherits the preset. This keeps an operator typo from silently becoming
a preset color after they explicitly set that field.

**Test:** `theme_mode_presets_and_override` extended — `mode = "dark"` +
`background = "blurple"` → `(None, Some(White))`: the invalid bg is
terminal-default (not preset black), while the unset fg still inherits
the preset white.

## Reviewer comments

(empty — pending round-2 grade)
