# Reopened: m-jobs-6-watch

Reviewed sha: `16c52012b6cf1306d768a4d3f70b8de5bb8cd50e`
Reviewed at: `2026-05-16T23:07:27Z`
Reviewer: `codex-reviewer`

Validation:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

## Findings

1. Medium - repo planning docs still describe the old Subscribe-based M-Jobs scope.

   The slice intentionally ships `blit jobs watch` as a `GetState` polling stopgap, and the finding doc explicitly says the per-job event ring plus `SubscribeRequest.transfer_id_filter` are deferred to milestone C. The tracked planning docs were not brought along. `TUI_DESIGN.md` still says `blit jobs watch` opens `Subscribe` with `transfer_id_filter` ([docs/plan/TUI_DESIGN.md](/Users/michael/Dev/Blit/docs/plan/TUI_DESIGN.md:649)), while `TODO.md` still says M-Jobs adds the per-job event ring and `SubscribeRequest.transfer_id_filter` ([TODO.md](/Users/michael/Dev/Blit/TODO.md:268)).

   That leaves future agents with two contradictory sources of truth: the implementation/finding says "polling now, Subscribe in C", but the roadmap says M-Jobs already includes Subscribe-shaped infrastructure. Please update the roadmap docs so M-Jobs owns detach/cancel/watch-as-polling, and C owns the event ring / `Subscribe` / transfer-id filter / streaming upgrade.

2. Medium - JSON watch timeout exits without the promised terminal outcome line.

   `JobsWatchArgs::json` documents JSON-Lines as "one object per poll, plus a final outcome line" ([crates/blit-cli/src/cli.rs](/Users/michael/Dev/Blit/crates/blit-cli/src/cli.rs:130)). Finished and NotFound satisfy that because their snapshot is printed before returning, but the timeout branch returns exit code 3 without printing any timeout JSON object ([crates/blit-cli/src/jobs.rs](/Users/michael/Dev/Blit/crates/blit-cli/src/jobs.rs:164)).

   A JSON consumer can see an active row and then EOF with status 3, but the stream itself has no terminal state. Please either emit a final JSON object such as `{"state":"timeout","transfer_id":"...","timeout_secs":...}` before returning 3, or narrow the CLI help/finding contract and add coverage for the intended JSON behavior.

3. Low - `WatchSnapshot` inherited the `kind_label` rustdoc.

   The `WatchSnapshot` enum was inserted immediately after the doc comment that used to describe `kind_label`, so the public enum's generated docs now start with "Human-readable label for a `TransferKind` proto enum value" ([crates/blit-app/src/admin/jobs.rs](/Users/michael/Dev/Blit/crates/blit-app/src/admin/jobs.rs:104)). Please split the comments so `WatchSnapshot` and `kind_label` each describe their own symbol.
