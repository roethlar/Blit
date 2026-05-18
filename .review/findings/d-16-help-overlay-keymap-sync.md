# d-16-help-overlay-keymap-sync: sync help with all F4 keys

**Severity**: Feature (polish — documentation hygiene)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

The `?` help overlay's keymap reference was last
updated in e-1 round 2. Since then d-4 through d-12
added 7 new keys to F4 that weren't documented:

- `C` copy
- `M` mirror
- `V` move
- `H` hash-mode toggle (Verify)
- `O` one-way toggle (Verify)
- `y` / `N` confirm-prompt keys
- `Esc` cancel confirm prompt

Operators pressing `?` would see only the original
nav / lifecycle / diagnostics keys, no hint that c/m/v
existed at all. This slice updates the modal so it
matches the actual keymap, and adds a regression test
that renders the modal into a `TestBackend` and asserts
each documented key appears.

## Approach

### Content reorganization

The overlay's flat "Global" + "Per-pane" structure
didn't scale to F4's now-substantial keymap. Restructured
into four sections:

```
Navigation (global)
F1 · F3 navigation
F4 · Profile lifecycle
F4 · Verify form
F4 · Local transfer
```

Each F4 sub-block matches the operator's mental model:
"I'm in Verify form, what can I press?" The `H`/`O`
toggles land under Verify; `C`/`M`/`V`/`y`/`N`/Esc land
under Local transfer.

### Modal size

70×30 → 70×32 to fit the additional content. The
`centered` helper already clamps to area for narrow
terminals.

### Regression test

`help_modal_documents_all_public_keys` renders the
overlay into an 80×40 `TestBackend`, flattens the cell
buffer into a single string, and `assert!(contains)`s
each tracked key. If a future slice adds a binding
without updating help, this catches it at test time.

## Files changed

- `crates/blit-tui/src/help.rs`:
  - Modal grows 30 → 32 lines.
  - Content reorganized into 5 sections; all new F4
    keys documented.
  - New `help_modal_documents_all_public_keys`
    regression test (uses `ratatui::backend::TestBackend`).
  - Existing `centered_clamps_to_area_when_smaller`
    updated to the new 70×32 dims.

## Tests

+1 unit test (190 → 191):

In `help::tests`:
- `help_modal_documents_all_public_keys` — renders the
  modal and asserts every public keystroke appears in
  the rendered buffer.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No pane-contextual help.** The modal lists every
   keystroke regardless of which pane is active. A
   future polish could surface only the keys that work
   from the operator's current pane, but the full
   keymap is also a useful reference document.

2. **Static text — no programmatic key derivation.**
   The help modal's content is hand-written and could
   theoretically fall out of sync with `key_action`
   again. The new regression test catches additions to
   the keymap that aren't reflected in the modal, but
   it can't catch the inverse (modal documenting a key
   that no longer exists). A future slice could parse
   the `key_action` source to derive the help content
   programmatically.

## Out of scope (next slices)

- **e-3 themes / config** — `~/.config/blit/tui.toml`.
- **Per-pane contextual help.**
- **Programmatic keymap derivation.**

## Reviewer comments

(empty — pending grade)
