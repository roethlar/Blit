# d-16-help-overlay-keymap-sync review

Reviewed sha: `8b81260210a4e390e11e2e53aa56055da09ea733`

## Findings

1. Low - `r` refresh is still not documented for F4 after the reorganization.

   In `crates/blit-tui/src/help.rs:61`, the `r` help row now lives under `F1 . F3 navigation`, but `r` still has active behavior on F4: `handle_pane_action` refreshes the F4 profile pane by spawning `spawn_profile_fetch(...)` for `UserAction::Refresh`. The old overlay's generic "refresh / rescan" line covered that; this slice narrows the label and leaves F4 Profile lifecycle listing only `c / d / e` and `s` at `crates/blit-tui/src/help.rs:67`.

   This matters because the stated goal is to sync help with all F4 keys, and the F4 refresh key is now omitted from the F4 section. The new regression test only checks that `"r"` appears somewhere in the rendered text, so it passes even though the key is attributed to the wrong pane. Please add `r` to the F4 Profile lifecycle block and consider tightening the test to cover pane/label text, not just bare substring presence. Also consider whether F2 refresh should be represented now that the help is pane-grouped.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed (191 tests).
- `cargo test --workspace` passed.
