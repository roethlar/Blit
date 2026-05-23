# keys-3-config-pane-switch: configurable pane-switch digit aliases

**Severity**: Feature (Milestone E — key remapping, step 3)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `43d5842`

## What

Third key-remapping slice. The digit aliases (`1`-`4` → navigate to
F1-F4 — a fallback for terminals that drop F-keys) are now configurable
via `[keys] pane_f1..pane_f4`. The function keys F1-F4 still navigate
regardless (conventional, not remappable).

This is the first multi-binding slice, so it **generalizes the collision
policy** keys-2 introduced for the growing keymap.

## Approach

- **config**: `pane_f1..pane_f4: String` (defaults `"1".."4"`) +
  `pane_chars()`. `KeysDefaults::resolved()` now returns a
  `ResolvedKeys { quit: char, nav: [Option<char>; 4], refresh:
  Option<char> }`. It applies a **greedy precedence** matching
  `key_action`'s dispatch order — quit > pane aliases (F1..F4) >
  refresh: the first claim on a character wins; any later binding that
  collides with a higher-precedence one is **disabled** (`None`).
- **main**: `KeyMap` gains `nav: [Option<KeyCode>; 4]`. `key_action`
  looks the plain press (no Ctrl/Alt) up against `nav` instead of the
  hardcoded `'1'-'4'` match; a `None` slot is skipped (its F-key still
  navigates).
- **warnings**: each non-single-char `pane_fN` (→ default digit), and
  each disabled (collided) nav/refresh binding, is named at startup.

## Files changed

- `crates/blit-tui/src/config.rs`: `pane_f*` fields; `ResolvedKeys`;
  `pane_chars()`; generalized `resolved()`; `DEFAULT_PANE`; Default;
  schema doc; collision-policy test.
- `crates/blit-tui/src/main.rs`: `KeyMap.nav`; `from_config` via
  `ResolvedKeys`; `key_action` nav lookup; pane single-char + collision
  warnings; remapped-pane test.

## Tests

611 blit-tui (workspace 28 binaries):
- `config::keys_resolved_collision_policy` extended — nav-vs-quit
  collision disables the nav alias; nav-vs-nav disables the later one;
  refresh loses to a colliding nav alias (lowest precedence).
- `key_action_honours_remapped_pane` — `pane_f2 = "t"` → `t` navigates
  F2; old `2` is inert; `1` still → F1; `F2` key always navigates.
- The default-keymap F-key / digit nav tests are unchanged (defaults
  `1`-`4`).

## Scope

The global pane-switch aliases. Per-screen action keys (j/k/g/G, the
F2 cancel `K`/`X`, F3 `p`/`m`/`v`/`D`, etc.) remain hardcoded; routing
those through the keymap is later `keys-N` work, each reusing the
`resolved()` collision framework this slice generalized.

## Reviewer comments

(empty — pending grade)
