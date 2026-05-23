# keys-4-config-movement: configurable list-cursor movement aliases

**Severity**: Feature (Milestone E — key remapping, step 4)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `f9e3378`

## What

Fourth key-remapping slice. The list-cursor aliases (`j`/`k`/`g`/`G` →
down / up / top / bottom on the F1 and F3 list panes) are now
configurable via `[keys] move_down`, `move_up`, `move_top`,
`move_bottom`. The arrow keys plus `Home`/`End` always move the cursor
regardless (conventional, not remappable) — they are the failsafe motion
so a bad `[keys]` value can never strand the operator without a way to
move.

This extends the keys-3 collision policy uniformly to a fourth,
lowest-precedence binding rather than inventing a new mechanism.

## Approach

- **config**: `move_down`/`move_up`/`move_top`/`move_bottom: String`
  (defaults `"j"`/`"k"`/`"g"`/`"G"`) + `movement_chars()` and
  `DEFAULT_MOVE`. `KeysDefaults::resolved()` now extends the greedy
  precedence chain to **quit > nav (F1..F4) > refresh > movement
  (down/up/top/bottom)**: the first claim on a character wins; a later
  binding colliding with a higher-precedence one is **disabled**
  (`None`). `ResolvedKeys` gains `movement: [Option<char>; 4]`.
- **main**: `KeyMap` gains `movement: [Option<KeyCode>; 4]`.
  `key_action` looks the plain press (no Ctrl/Alt) up against `movement`
  — placed after the refresh check, before the static action match — and
  the old hardcoded `j`/`k`/`g`/`G` arms are removed (the arrow / Home /
  End arms stay). A `None` slot is skipped (its arrow/Home/End still
  moves).
- **shadowing note**: like refresh, movement is checked before the
  static action map, so remapping a movement key onto a static action
  char (e.g. `move_down = "t"`) shadows that action. This matches the
  verified keys-2/keys-3 behaviour for refresh/nav and is documented on
  the field + dispatch site. The arrow/Home/End failsafe always covers
  the motion itself.
- **warnings**: each non-single-char `move_*` (→ default char) and each
  disabled (collided) movement binding is named at startup.

## Files changed

- `crates/blit-tui/src/config.rs`: `move_*` fields; `ResolvedKeys.movement`;
  `movement_chars()`; `DEFAULT_MOVE`; extended `resolved()`; Default;
  schema doc.
- `crates/blit-tui/src/main.rs`: `KeyMap.movement`; `from_config` via
  `ResolvedKeys.movement`; `key_action` movement lookup + arrows-only
  static arms; movement single-char + collision warnings; tests.

## Tests

619 blit-tui (workspace 28 binaries), +4 over keys-3's 615:

- `key_action_default_movement_and_arrows` — default keymap: j/k/g/G
  **and** Down/Up/Home/End all move the cursor (behaviour-preservation
  of extracting j/k/g/G into the configurable block; g/G had no prior
  dedicated test).
- `key_action_honours_remapped_movement` — remapped `n`/`p` move; old
  default `j` no longer maps; arrows still move; unremapped `g` keeps its
  default.
- `movement_collision_disables_alias` — `move_down="q"` (quit),
  `move_up="r"` (refresh) → both disabled; the chars resolve to their
  higher-precedence action; arrows still move.
- `resolved_movement_invalid_and_internal_collision` — non-single-char
  `move_top="gg"` → default `g`; `move_bottom="j"` collides with
  `move_down` → disabled.

## Scope

The four list-cursor keys are a bounded, coherent group — the
highest-value remap (vim vs. non-vim operators differ most on cursor
keys), failsafe-backed by the always-on arrows. Per-screen *command*
keys (F2 `K`/`X`, F3 `p`/`m`/`v`/`D`/`u`/space/`a`, F4 profile keys)
remain a larger, niche surface with a growing collision matrix against
the static action map and are intentionally **not** taken on here.

## Reviewer comments

(empty — pending review)
