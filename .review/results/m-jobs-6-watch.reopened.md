# Reopened: m-jobs-6-watch

Reviewed sha: `5ab9eefae622ae02ff5554e669cfa6c5185c6719`
Reviewed at: `2026-05-16T23:16:58Z`
Reviewer: `codex-reviewer`

Validation:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

## Findings

1. Medium - `TUI_DESIGN.md` still has stale M-Jobs ownership text for Subscribe-scoped pieces.

   Round 2 fixed the CLI-surface paragraph and TODO rows, and the code fixes for timeout JSON plus `WatchSnapshot` rustdoc look correct. The design doc still contradicts the new scope in its later phasing section: Milestone M-Jobs still lists "Per-job event ring inside each `ActiveJob` row" and "`transfer_id_filter` field on `SubscribeRequest`" ([docs/plan/TUI_DESIGN.md](/Users/michael/Dev/Blit/docs/plan/TUI_DESIGN.md:882)), the phasing summary still says M-Jobs adds `transfer_id_filter` ([docs/plan/TUI_DESIGN.md](/Users/michael/Dev/Blit/docs/plan/TUI_DESIGN.md:1030)), and the structural commitments still call the `transfer_id_filter` field part of the fixed contract without reflecting that it lands in C ([docs/plan/TUI_DESIGN.md](/Users/michael/Dev/Blit/docs/plan/TUI_DESIGN.md:1065)).

   Please finish the doc sweep so every M-Jobs/C section agrees: M-Jobs ships detach, CancelJob, cancellation-token rows, and `jobs watch` polling; milestone C ships the per-job event ring, `Subscribe`, `transfer_id_filter`, and the streaming watch upgrade. The code does not need changes for this finding.
