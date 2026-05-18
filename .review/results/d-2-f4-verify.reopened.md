# d-2-f4-verify reopened

Reviewed sha: `ab105fb356aa4e8cd2c0ab11450b128dc0a0cbca`

## Verdict

Reopened.

## Findings

1. **Medium — In-flight Verify results can be applied to edited paths.**

   `handle_verify_keystroke` allows `Char` and `Backspace` edits while `VerifyStatus::Running` is active (`crates/blit-tui/src/main.rs:1200`, `crates/blit-tui/src/main.rs:1213`). Those edits mutate `VerifyState::source` / `destination`, but `VerifyState::insert_char` and `backspace` only invalidate `Done` / `Error`, not `Running` (`crates/blit-tui/src/verify.rs:106`, `crates/blit-tui/src/verify.rs:123`). Since `request_id` is only bumped by `begin_run` (`crates/blit-tui/src/verify.rs:144`), the original worker reply still matches and `apply_result` renders the old comparison under the newly edited paths (`crates/blit-tui/src/verify.rs:152`).

   Repro shape: enter `/tmp/a` and `/tmp/b`, press Enter, then immediately type more characters before the blocking compare returns. The visible fields now describe a different comparison, but the old run's result can still land as `Done`.

   Fix direction: either make fields read-only while `Running`, or make any edit during `Running` invalidate/bump the generation so the in-flight reply is dropped. Add a unit test that begins a run, edits one field before applying the old `request_id`, and verifies the old result is not accepted.

## Gates

Run at the sentinel SHA in detached worktree `/private/tmp/blit-review-d-2-f4-verify` because the main worktree already contained unrelated uncommitted coder WIP:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed: 107 tests.
- `cargo test --workspace` passed.
