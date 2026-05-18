# d-12-esc-cancels-confirm reopened

Reviewed sha: `fe25cb194ac2861fdfed91d588fe4b139f7ab147`

Verdict: reopened

## Findings

### Low — Esc still fails to cancel confirm after Verify focus is re-entered

`crates/blit-tui/src/main.rs:499` runs `handle_verify_keystroke` before the new confirm-cancel intercept at `crates/blit-tui/src/main.rs:511`. That leaves a reachable state where the destructive prompt is visible but Esc does not dismiss it:

1. On F4, press `M` or `V` to enter `ConfirmingMirror` / `ConfirmingMove`.
2. Press `Tab`, which still cycles the Verify form focus while the confirm prompt remains visible.
3. Press `Esc`.

Because Verify is now editing, `handle_verify_keystroke` consumes Esc to clear field focus and the d-12 confirm-cancel branch is skipped. The transfer stays in the confirm state. The banner says `[y / N or Esc]`, and the feature contract says Esc cancels while the prompt is open, so the confirm intercept needs to take priority over Verify edit-mode Esc when `app.transfer.is_confirming()` is true. Add a regression test for confirm + Verify focus.

## Gates

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed.
- `cargo test --workspace` passed.
