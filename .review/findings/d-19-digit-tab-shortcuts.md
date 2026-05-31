# d-19-digit-tab-shortcuts: 1-4 as aliases for F1-F4

**Severity**: Feature (polish — accessibility / robustness)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Bare `1`/`2`/`3`/`4` keys now navigate to F1/F2/F3/F4,
matching the existing F-key bindings.

```
F1 / 1   Daemons pane
F2 / 2   Transfers pane
F3 / 3   Browse pane
F4 / 4   Profile / Verify / Diagnostics / Transfer
```

The reason isn't just convenience — some terminal
environments don't carry F-keys cleanly:

- **mosh** maps some function keys to escape sequences
  that don't always round-trip.
- **SSH proxies / bastion hosts** can intercept and
  drop F-key escape sequences.
- **CI / multiplexer setups** (screen, tmux with custom
  bindings) sometimes claim F-keys before the TUI sees
  them.

Bare digits always pass through. They're a reliable
fallback when F-keys aren't getting through.

## Approach

### Key dispatch

`key_action` gets a second nav block after the F-key
block:

```rust
if let KeyCode::Char(c) = key.code {
    if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
        match c {
            '1' => return Some(UserAction::Navigate(Screen::F1)),
            '2' => return Some(UserAction::Navigate(Screen::F2)),
            '3' => return Some(UserAction::Navigate(Screen::F3)),
            '4' => return Some(UserAction::Navigate(Screen::F4)),
            _ => {}
        }
    }
}
```

Bare digits only — Ctrl-1 / Alt-1 fall through so the
operator's terminal can keep claiming those for window
management.

### Verify form interplay

When the F4 Verify form has edit focus,
`handle_verify_keystroke` captures the digit as text
input before `key_action` runs. So typing
"config/2/data" into the Source field still works —
the dispatcher never sees the `2` while the operator
is editing.

### Help overlay

`F1 / 1`, `F2 / 2`, etc. — the help modal now shows
both bindings for each pane.

## Files changed

- `crates/blit-tui/src/main.rs`:
  - `key_action` gains the digit-alias block.
  - `key_action_maps_function_keys` test extended to
    cover the digit aliases + the
    out-of-range-digit / Ctrl-1 / Alt-1 negative cases.
- `crates/blit-tui/src/help.rs`:
  - Help overlay tab rows now show "F1 / 1" etc.

## Tests

+0 new test functions (the existing key-mapping test
extends to cover the new bindings + negative cases),
but +9 new assertions inside it. The
`help_modal_documents_all_public_keys` regression test
remains green because the modal's "F1" string still
appears in each row (now followed by " / 1").

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green
(228 blit-tui tests).

## Known gaps

1. **No digit-to-pane indicator on the tab strip.**
   The tab labels still show "F1 Daemons" etc., not
   "F1/1 Daemons". Adding it would crowd the strip on
   narrow terminals; the help modal carries the dual
   binding instead.

2. **Single-digit only.** No `5`–`9` (no panes exist
   for them). A future polish slice that adds a 5th
   pane (e.g. F5 Logs) would need to extend this block
   in lock-step.

## Out of scope (next slices)

- **F2 transfer cancel** (needs F2 cursor first).
- **More themable colors** (warn / error / ok).
- **Hot-reload of tui.toml.**

## Reviewer comments

(empty — pending grade)
