# keys-1-config-quit: configurable quit key via `[keys]` config

**Severity**: Feature (Milestone E — key remapping, step 1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `16e9466`

## What

First slice of Milestone E's key remapping. The input dispatch is
scattered (per-screen prefilters + inline `match key.code`), so rather
than refactor everything at once, this slice establishes the foundation
— a `[keys]` config section and a `KeyMap` indirection — and routes the
**global quit key** through it. Later slices extend the `KeyMap` to
refresh, pane-switch, and per-screen keys.

The quit key is the natural first binding: it's classified in one
central place (`key_action` → `should_quit`), so wiring it is contained.

## Approach

- **config**: `KeysDefaults { quit: String }` (default `"q"`) on
  `TuiConfig`. `quit_char()` returns the single `char`, or `None` for a
  multi-char/empty value (caller warns + falls back to `'q'`, exactly
  like `[theme] accent_color`).
- **main**: `KeyMap { quit: KeyCode }`, built **per-keystroke** from the
  (hot-reloadable) config so a `Ctrl+R` remap takes effect live.
  `is_quit(code, modifiers, quit)` replaces `should_quit`: matches the
  configured quit char **or** `Esc` **or** `Ctrl+C`. Esc and Ctrl+C
  always quit, so a bad `[keys] quit` value can never lock the operator
  in.
- `key_action` now takes `&KeyMap`; the event loop builds it from
  `tui_config` at the single call site.
- Startup warning (buffer-then-flush, like the theme warning) when
  `[keys] quit` isn't a single character.

## Files changed

- `crates/blit-tui/src/config.rs`: `KeysDefaults` + `keys` field +
  `quit_char()` + schema doc + parse test.
- `crates/blit-tui/src/main.rs`: `KeyMap`; `is_quit` (was `should_quit`);
  `key_action(&KeyMap)`; call-site keymap build; startup warning; tests.

## Tests

605 blit-tui (workspace 28 binaries):
- `config::keys_quit_parses_and_defaults` — `[keys] quit` parses;
  defaults to `"q"`; `quit_char` accepts one char, rejects multi/empty.
- `is_quit_recognises_default_quit_esc_ctrl_c` and
  `is_quit_honours_remapped_key_and_failsafes` — the predicate honours
  the configured char and keeps Esc/Ctrl+C as failsafes (old `q` no
  longer quits once remapped).
- `key_action_honours_remapped_quit` — a remapped key flows config →
  `KeyMap` → `key_action`.
- The ~40 existing `key_action` tests route through a new default-keymap
  `ka` test wrapper (no behavioral change).

## Scope

Quit key only. This is the keymap-infrastructure slice; subsequent
`keys-N` slices extend the `KeyMap` to other global keys (refresh,
pane-switch) and eventually per-screen actions. Per-screen keys still
dispatch through their existing handlers — those move under the keymap
in later slices.

## Round 2 (commit `19c6b7f`)

**Reopen finding (Medium):** `is_quit` matched `code == quit`
regardless of modifiers, and `key_action` checks `is_quit` before the
modifier-aware branches. So `[keys] quit = "r"` made `Ctrl+R` return
`Quit` before the `ReloadConfig` branch could run — hijacking the
documented `Ctrl+R` config reload (and, generally, any Ctrl/Alt chord
for whichever character the operator picked).

**Fix:** the configured quit char now claims only a **plain** press —
`is_quit` requires `!modifiers.intersects(CONTROL | ALT)` for the
`code == quit` arm. Shift is still allowed (capitals are distinct
`KeyCode`s anyway). `Esc` and `Ctrl+C` remain the modifier-aware
failsafes regardless of the configured char.

**Test (+1, 606 blit-tui):**
`remapped_quit_does_not_steal_ctrl_chord` — with `quit = "r"`: plain
`r` → `Quit`, `Ctrl+R` → `ReloadConfig` (not quit); `is_quit` rejects
`Ctrl/Alt + char`; `Esc` / `Ctrl+C` still quit under the remap.

## Reviewer comments

(empty — pending round-2 grade)
