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

### Round 1 verdict — reopened (`.review/results/d-12-esc-cancels-confirm.reopened.md`)

One Low-severity finding, addressed in round 2:

- **Esc still fails to cancel confirm after Verify focus
  is re-entered.** The round-1 intercept ran AFTER
  `handle_verify_keystroke`. If the operator hit `M`
  (confirm opens), then `Tab` (Verify form gains edit
  focus, the confirm prompt stays visible), then `Esc`
  — the Verify keystroke handler consumed the Esc to
  clear focus and the confirm-cancel branch never ran.
  Operator was stuck with a destructive prompt and no
  Esc escape.

  Round 2 reorders the dispatch: the Esc-cancels-confirm
  intercept now runs BEFORE `handle_verify_keystroke`,
  so confirm-pending always wins over Verify edit-mode
  Esc handling. Comment block at the call site spells
  out the priority explicitly.

  Also factored the gate into a testable helper
  `esc_cancels_confirm(&KeyEvent, &AppState) -> bool` so
  the priority matrix can be regression-tested directly
  (the previous round just had a state-machine test for
  cancel_confirm that wouldn't have caught this).

### Round 2 file changes

- `crates/blit-tui/src/main.rs`:
  - New `esc_cancels_confirm(&KeyEvent, &AppState) -> bool`
    helper.
  - Router's Esc intercept now uses the helper AND
    runs before `handle_verify_keystroke`.

### Round 2 tests

+1 unit test (178 → 179):

In `main::tests`:
- `esc_cancels_confirm_priority_matrix`: pins the gate
  matrix — confirm pending alone, confirm + Verify
  edit focus (the regression case), Ctrl-Esc / Alt-Esc
  (no-trigger), Move confirm (also handled), non-Esc
  keys (no-trigger).

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test -p blit-tui` all green.
