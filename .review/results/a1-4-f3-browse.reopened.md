# a1-4-f3-browse reopened

Reviewed sha: `283a217e9a70335ce4bf6c92779b99c12c1f1f1d`
Reviewed at: 2026-05-18T05:23:23Z
Reviewer: reviewer
Verdict: reopened

## Findings

### 1. Refresh hides the only actionable F3 endpoint error when no endpoint exists

Severity: Medium

Location: `crates/blit-tui/src/main.rs:708`

F3 intentionally keeps the loop alive when `--remote` is missing or malformed so the operator can read the status message and quit:

- missing remote: `--remote <host> is required for F3 Browse`
- malformed remote: `parse '<raw>': ...`

The refresh arm is unconditional, though. Pressing `r` calls `state.begin_fetch()` and then `state.note_fetch_error("refreshing")`. When `endpoint` is `None`, the kick condition at the top of the loop can never start a fetch, so the UI is left permanently showing `error: refreshing` instead of the actionable missing/parse message.

That violates the explicit missing/malformed remote behavior in the finding doc and makes a common operator mistake harder to diagnose after a single keypress.

Please make refresh a no-op when there is no parsed endpoint, or preserve/recompute the existing endpoint error instead of replacing it with `refreshing`. Add a small test around the refresh reducer/helper path so `--screen f3` without a remote keeps the original error after `r`.

### 2. Browse module docs still say Esc ascends, but Esc quits

Severity: Low

Location: `crates/blit-tui/src/browse.rs:20`

The new browse module-level docs say `esc` pops the path, but `key_action` checks `should_quit` first and maps `Esc` to `UserAction::Quit`; the F3 footer also advertises `q/Esc quit` and `Left/h` for ascending. The behavior is fine, but the new module docs contradict the actual keymap and will mislead the next slice that builds on F3 navigation.

Please update the doc comment to match the implemented keymap.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed: 80 tests.
- `cargo test --workspace` passed.
