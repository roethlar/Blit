# a0-pull-execution reopened

Reviewer: `codex-reviewer`
Reviewed sha: `7f755394d0d52987505a064b7d46367991acc4c5`
Timestamp: `2026-05-16T16:50:00-04:00`

Validation was green on the submitted Rust tree:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Later commits on the branch touched only `.review/` workflow files and `REVIEW.md`; the reviewed Rust diff remains the one in `7f75539`.

## Findings

1. Medium / behavior regression: mirror-pull progress stays open through local mirror purge. Before this slice, `run_remote_pull_transfer_inner` dropped `progress_handle` and awaited the progress task immediately after `pull_sync`, then ran `delete_listed_paths`; that ended the transfer progress stream before local delete cleanup. After the move, `run_remote_pull` performs `delete_listed_paths` before returning (`crates/blit-app/src/transfers/remote.rs:304`), while the CLI cannot drop the progress handle until the library call returns (`crates/blit-cli/src/transfers/remote.rs:433`). For a mirror pull with progress enabled and a large delete list, the monitor can keep emitting stale transfer progress ticks during purge and only prints the final line after purge. That violates the no-behavior-change refactor contract and contradicts the new doc comment that says `progress` is borrowed for the duration of the PullSync RPC.

   Fix direction: keep mirror purge before final success output, but restore the progress lifecycle boundary around `pull_sync`. One reasonable shape is to have the app layer return the pull report plus delete list/state after `pull_sync`, let the CLI/TUI close or transition the progress channel, then invoke an app-layer mirror-purge helper before printing the final summary.

2. Medium / workflow contract: the review handoff artifacts for this slice were not committed with the sentinel. At review time, `.review/findings/a0-pull-execution.md` and `.review/ready/a0-pull-execution.json` were untracked. `REVIEW.md` got a pending row later as part of the unrelated `a0-remote-helpers` round-4 sentinel commit, but the ready file for this slice still was not in git. That means another reviewer or a fresh worktree would not reliably see this pending request, defeating the file-based workflow's goal of removing the user from the loop.

   Fix direction: for round 2, commit the finding doc, the ready sentinel, and the `REVIEW.md` row/status update in the sentinel commit, then let the reviewer delete the tracked ready file as part of the verdict.
