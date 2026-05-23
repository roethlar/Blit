# keys-2-config-refresh: configurable refresh key via `[keys]` config

**Severity**: Feature (Milestone E — key remapping, step 2)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `a5ecfcd`

## What

Second key-remapping slice. keys-1 established the `[keys]` config +
`KeyMap` indirection and made the quit key configurable; keys-2 extends
the same machinery to the global **refresh** key (default `r`).

## Approach

- **config**: `KeysDefaults.refresh: String` (default `"r"`) +
  `refresh_char()`. A shared `single_char()` helper now backs both
  `quit_char` and `refresh_char` (DRY — keys-1's inline parse moved
  into it).
- **main**: `KeyMap` gains `refresh: KeyCode`. `key_action` returns
  `Refresh` when a **plain** press (no Ctrl/Alt) matches
  `keymap.refresh` — the same plain-press-vs-chord care as keys-1
  (round 2), so a remap can't shadow `Ctrl+R` config reload (which is
  also handled earlier in `key_action`). The hardcoded `Char('r') =>
  Refresh` match arm is removed; the default `r` now flows through the
  configurable check.
- Startup warning (buffer-then-flush) for a non-single-char `[keys]
  refresh`.

## Files changed

- `crates/blit-tui/src/config.rs`: `refresh` field + `refresh_char()` +
  `single_char()` helper + schema doc + parse test.
- `crates/blit-tui/src/main.rs`: `KeyMap.refresh`; configurable refresh
  check in `key_action` (replaces the hardcoded arm); startup warning;
  test.

## Tests

608 blit-tui (workspace 28 binaries):
- `config::keys_refresh_parses_and_defaults` — `[keys] refresh` parses;
  defaults to `"r"`; rejects multi-char; quit keeps its default when
  only refresh is set.
- `key_action_honours_remapped_refresh` — remapped `R` → `Refresh`; old
  default `r` no longer refreshes; `Ctrl+R` still `ReloadConfig`.
- The default-keymap `key_action_maps_quit_and_refresh` test is
  unchanged (default refresh is still `r`).

## Scope

Refresh key only. The remaining global keys `key_action` classifies
(pane-switch F1-F4 / digit aliases) and the per-screen action keys come
in later `keys-N` slices, each applying the same plain-press-vs-chord
discipline.

## Reviewer comments

(empty — pending grade)
