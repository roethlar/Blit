# Reopened: m-jobs-1-cancel-token

Reviewed sha: `4a2eb0a77214978cc22976cd9a5cb2b66d5dc2d4`
Reviewed at: `2026-05-16T21:32:04Z`
Reviewer: `codex-reviewer`

Validation:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

## Findings

1. Medium - `ActiveJobs::cancel` reports success for rows that cannot observe cancellation.

   `ActiveJobs::cancel` currently returns `true` for any id present in the `cancellations` map ([crates/blit-daemon/src/active_jobs.rs](/Users/michael/Dev/Blit/crates/blit-daemon/src/active_jobs.rs:310)), and `register` creates a token for every `ActiveJobKind` ([crates/blit-daemon/src/active_jobs.rs](/Users/michael/Dev/Blit/crates/blit-daemon/src/active_jobs.rs:279)). Only `delegated_pull` actually races the token in its handler select ([crates/blit-daemon/src/service/core.rs](/Users/michael/Dev/Blit/crates/blit-daemon/src/service/core.rs:363)).

   Once `CancelJob` lands, a cancel request for an active `push`, `pull`, or `pull_sync` row can return success while the transfer keeps running. The finding doc says those job kinds are intentionally not cancellable from another client, so the `ActiveJobs` API should encode that now instead of treating token presence as cancellation support. Suggested shape: make `cancel` consult the active row kind and return a structured result such as cancelled / unsupported / not found, or otherwise ensure callers cannot acknowledge unsupported rows as cancelled. Please add coverage for unsupported kinds.

2. Low - `register` exposes a visible active row before its cancellation token exists.

   `register` inserts into `table` first, then inserts into `cancellations` under a separate mutex ([crates/blit-daemon/src/active_jobs.rs](/Users/michael/Dev/Blit/crates/blit-daemon/src/active_jobs.rs:280)). `GetState` already reads `snapshot()` from `table`, so there is a scheduler window where a client can see a transfer id in `active[]`, then the upcoming `CancelJob` path can call `cancel(id)` and get `false` because the token insertion has not happened yet.

   Please make "visible in active table" imply "cancellation lookup is initialized" from an observer's perspective. Inserting the token before publishing the row may be sufficient for this specific direction; a single locked state object for table + tokens would be more explicit if the API also needs to check kind atomically.
