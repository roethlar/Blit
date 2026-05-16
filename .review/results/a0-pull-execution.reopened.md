# a0-pull-execution reopened

Reviewer: `codex-reviewer`
Reviewed sha: `e9e168fb79ae6e9ff950d3dd1f486bc69f713bc7`
Timestamp: `2026-05-16T17:08:00-04:00`

Validation was green:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Round-1 behavior finding is closed: `run_pull_sync` now stops before purge, the CLI drops/drains the progress monitor at `crates/blit-cli/src/transfers/remote.rs:443`, and `apply_pull_mirror_purge` runs after that boundary.

## Findings

1. Medium / workflow correctness: `.review/reviewer-wait.sh` still wakes on any filesystem-visible `.review/ready/*.json`, including an untracked or not-yet-committed sentinel. That exact race happened during this round: the monitor printed `READY: a0-pull-execution.json` while `git status` still showed `.review/ready/a0-pull-execution.json`, `.review/findings/a0-pull-execution.md`, and `REVIEW.md` as untracked/modified; the sentinel commit landed shortly afterward. The human stayed out of the loop this time because the reviewer manually rechecked, but the monitor contract is still fragile. The wake script should not emit `READY` until the ready file is tracked and clean in HEAD, otherwise a reviewer can grade the wrong state or write a verdict before the sentinel commit exists.

   Fix direction: make `.review/reviewer-wait.sh` filter candidates before printing. For each `.review/ready/*.json`, require `git ls-files --error-unmatch "$file"` to succeed and both `git diff --quiet -- "$file"` and `git diff --cached --quiet -- "$file"` to be clean. Skip untracked/staged/unstaged ready files and keep polling. Optionally validate that the sentinel's `sha` resolves with `git cat-file -e "$sha^{commit}"`.

2. Low / stale code comment: `crates/blit-cli/src/transfers/remote.rs:469` still says the pull orchestration in `blit-app` is `PullExecution` / `run_remote_pull`. Round 2 intentionally replaced those names with `PullSyncExecution` / `run_pull_sync` plus `apply_pull_mirror_purge`, so the comment now points future readers at APIs that no longer exist.

   Fix direction: update the comment to name the round-2 split API.
