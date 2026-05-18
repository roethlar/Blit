# d-12-esc-cancels-confirm: Esc cancels mirror/move confirm

**Severity**: Feature (UX polish — closes a small
discoverability hole on the destructive-op prompt)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

While a mirror or move confirmation prompt is open, the
operator can now press **Esc** to dismiss it back to
Idle. Previously only `N` or `n` cancelled; bare Esc
would quit the TUI entirely (because `should_quit` maps
`KeyCode::Esc` to Quit). That's the wrong behavior on a
"do you want to delete files?" prompt — the universal
"get me out" gesture should cancel the operation, not
exit the whole app.

The confirm banner now spells out the new keymap:

```
mirror will DELETE extraneous files at destination · [y / N or Esc]
move will DELETE the SOURCE after copy · [y / N or Esc]
```

## Approach

A small intercept in the router's keystroke arm, placed
after the existing verify-keystroke and help-overlay
intercepts so the priority order is:

1. Help overlay visible → Esc closes help.
2. Verify form editing → Esc clears focus.
3. Confirm pending → Esc cancels confirm.
4. Otherwise → `key_action` runs (Esc would map to
   `should_quit` here, but only when none of the above
   absorbed it).

```rust
if key.code == KeyCode::Esc
    && !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    && app.transfer.is_confirming()
{
    app.transfer.cancel_confirm();
    continue;
}
```

Bare Esc only — Ctrl-Esc / Alt-Esc still fall through to
the regular dispatcher. The intercept calls
`cancel_confirm()` which handles BOTH `ConfirmingMirror`
and `ConfirmingMove` (introduced in d-5).

## Files changed

- `crates/blit-tui/src/main.rs`:
  - New Esc intercept arm in `run_router` (the
    keystroke select arm).
- `crates/blit-tui/src/screens/f4.rs`:
  - Confirm banner text updated to surface `Esc` as an
    accepted cancel key alongside `N`.

## Tests

+1 unit test (177 → 178):

In `main::tests`:
- `cancel_confirm_dismisses_either_confirm_kind` — pins
  the state-transition contract the Esc intercept relies
  on (cancel_confirm dismisses both ConfirmingMirror and
  ConfirmingMove back to Idle).

The full integration path (router event loop + Esc
keystroke) is not directly unit-tested — driving the
async router from a unit test would require a fake
crossterm event source. The intercept itself is small
and inline; correctness is pinned by
`cancel_confirm`'s contract being well-covered.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No Esc-cancel for the help overlay AND a confirm
   simultaneously.** That's not a real combined state —
   the keystroke arm absorbs help-overlay keys before
   reaching the confirm intercept, so Esc inside an
   open help overlay still just closes the help.

## Out of scope (next slices)

- **e-3 themes / config** — `~/.config/blit/tui.toml`.
- **Per-file progress** events during local transfers.
- **F3 multi-select** + transfer trigger from the
  browse-tree cursor.

## Reviewer comments

(empty — pending grade)
