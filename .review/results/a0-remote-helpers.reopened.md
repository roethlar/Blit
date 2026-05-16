# a0-remote-helpers reopened

Reviewer: `claude-reviewer`
Reviewed sha: `086fa497ffd0a8ce3fa372bb8ff8786ba757b921`
Timestamp: `2026-05-16T16:06:19Z`

Validation was green:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

The original round-1 code findings are addressed: the `delete_listed_paths`
safety tests now live with `blit_app::transfers::remote`, and the stale
`delegated_pull.rs` references now point at the shared library helpers.

## Findings

1. Medium / workflow correctness: `.review/coder-wait.sh` returns stale verdicts for re-review rounds. In the current state, `.review/ready/a0-remote-helpers.json` points at `086fa49`, but `.review/results/a0-remote-helpers.reopened.md` from round 1 still exists for `de78151`. Running `REVIEW_WAIT_TIMEOUT_SECONDS=1 .review/coder-wait.sh a0-remote-helpers` immediately prints the old round-1 reopened verdict instead of waiting for a verdict on the current ready sha. That defeats the process goal of removing the human from the loop: the coder can wake on stale state and either repeat already-fixed work or stop waiting before this review completes.

   Fix direction: make `coder-wait.sh` verdict matching round-aware. The simplest contract is `coder-wait.sh <id> <expected-sha>` and only return a verdict whose embedded sha matches `<expected-sha>`. For `verified.json`, compare the JSON `sha`. For `reopened.md`, compare the `Reviewed sha: \`...\`` line. If the only result file is for an older sha while `.review/ready/<id>.json` points at a newer sha, keep waiting.
