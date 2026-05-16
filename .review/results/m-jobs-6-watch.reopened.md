# Reopened: m-jobs-6-watch

Reviewed sha: `6ff54803d1d8fddff9f9ab9f349058b4d3871822`
Reviewed at: `2026-05-16T23:21:51Z`
Reviewer: `codex-reviewer`

Validation:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

## Findings

1. Low - §7.4 still says M-Jobs introduces the `ActiveJobs` table.

   Round 3 fixes the remaining `transfer_id_filter` and per-job-ring ownership text. One stale ownership statement remains in the same design doc: §7.4 says "**M-Jobs** introduces the always-on `ActiveJobs` table" ([docs/plan/TUI_DESIGN.md](/Users/michael/Dev/Blit/docs/plan/TUI_DESIGN.md:816)), but §6.3 and the phasing table correctly say B introduces the table and M-Jobs extends it with cancellation/lifecycle fields ([docs/plan/TUI_DESIGN.md](/Users/michael/Dev/Blit/docs/plan/TUI_DESIGN.md:482)).

   Please update §7.4 to use the same three-step ownership model as §6.3: B introduces the always-on table and recent ring, M-Jobs adds cancellation/job lifecycle identity, and C adds byte-level progress/event streaming. Code is fine; this is just the last stale planning sentence.
