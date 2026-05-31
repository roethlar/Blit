# d-18-verify-form-clear: Ctrl-U clears focused Verify field

**Severity**: Feature (polish — small UX win)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

While the F4 Verify form has edit focus on Source or
Destination, `Ctrl-U` wipes the focused field's text in
one keystroke. Matches the readline-style "kill-line"
convention every terminal user already knows.

Pre-d-18 the operator had to hold Backspace (or Tab
through both fields) to retype after a typo or context
switch. With paths often 50-100+ characters that's
annoying. `Ctrl-U` is one tap.

The clear is per-field — Source-focused Ctrl-U clears
Source only; Destination preserved. Operator tabs to
the other field if they need both wiped.

## Approach

### `VerifyState::clear_focused_field`

New method:

```rust
pub fn clear_focused_field(&mut self) -> bool
```

- Source-focused → clears `self.source`.
- Destination-focused → clears `self.destination`.
- No focus → no-op, returns `false`.
- Already empty field → no-op (don't bump `request_id`
  needlessly).
- Otherwise calls the existing `invalidate_run` hook —
  same contract as `insert_char`/`backspace`: any
  pending or completed run for the prior text gets
  dropped on arrival.

### Dispatch

In `handle_verify_keystroke`, add an early-return arm
for `Ctrl+'u'`:

```rust
if key.code == KeyCode::Char('u') && key.modifiers.contains(KeyModifiers::CONTROL) {
    app.verify.clear_focused_field();
    app.transfer.cancel_confirm();
    return true;
}
```

The `cancel_confirm` call mirrors the existing edit
arms (insert_char / backspace) — a clear under a
pending mirror confirm should drop the prompt for the
same reason a typed char would: the paths underneath
the prompt are no longer what the operator confirmed.

### Help overlay

`Ctrl-U` added to the F4 Verify form section of the
help modal.

## Files changed

- `crates/blit-tui/src/verify.rs`:
  - `clear_focused_field(&mut self) -> bool` method.
- `crates/blit-tui/src/main.rs`:
  - `handle_verify_keystroke` Ctrl-U arm.
- `crates/blit-tui/src/help.rs`:
  - Help overlay's F4 Verify section gains a Ctrl-U row.

## Tests

+5 unit tests (195 → 200):

In `verify::tests`:
- `clear_focused_field_clears_source_when_source_focused`
- `clear_focused_field_clears_destination_when_destination_focused`
- `clear_focused_field_noop_when_no_focus`
- `clear_focused_field_returns_false_for_already_empty_field`
- `clear_focused_field_invalidates_pending_run` — stale
  reply drop contract via `apply_result` returning false.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No "clear both fields" shortcut.** The operator
   uses Ctrl-U twice (Tab to switch, Ctrl-U again).
   A future polish could add Alt-U or similar for
   the both-fields case, but Ctrl-U twice is fine.

2. **No way to undo a clear.** Hits Ctrl-U on a
   100-char path then realizes they wanted to edit
   it — they have to retype. Not a regression; the
   prior Backspace flow had the same loss. Real undo
   would need a small history stack on each field.

## Out of scope (next slices)

- **e-3 themes / config** — `~/.config/blit/tui.toml`.
- **Multi-keystroke undo stack** for Verify fields.

## Reviewer comments

(empty — pending grade)
