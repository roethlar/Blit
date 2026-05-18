# e-1-help-overlay reopened

Reviewed sha: `1de78c641214829415160cb298d8d9be9f8b2c78`

## Verdict

Reopened.

## Findings

1. **Medium — `?` is not actually a global help key while the F4 Verify form is being edited.**

   The finding contract says "Operator hits `?` to open" the global help overlay, and the key-dispatch section says `key_action` maps `Char('?')` to `ToggleHelp`. In the event loop, though, the overlay-visible branch runs first, then F4 Verify edit-mode handling runs before `key_action` (`crates/blit-tui/src/main.rs:452`). `handle_verify_keystroke` consumes every unmodified `KeyCode::Char(c)` as text input (`crates/blit-tui/src/main.rs:1390`), so pressing `?` while the Source/Destination field is focused inserts `?` into the path and never opens help.

   That leaves the help overlay unavailable from the one screen state where an operator is most likely to need keybinding help, and it contradicts the advertised "global" binding. It also makes the finding doc's known-gap statement about not being able to type a literal `?` inaccurate: the implementation currently does type a literal `?`.

   Fix direction: make `?` an explicit global intercept before `handle_verify_keystroke`, or teach the verify handler to return false for `Char('?')` so the dispatcher can toggle help. Add a regression test that starts in F4 edit mode and proves `?` toggles help instead of mutating the focused field.

## Gates

- `cargo fmt --all -- --check` passed at `1de78c6`.
- `cargo clippy --workspace --all-targets -- -D warnings` passed at `1de78c6`.
- `cargo test -p blit-tui` passed at `1de78c6`: 121 tests.
- `cargo test --workspace` passed at `1de78c6`.
