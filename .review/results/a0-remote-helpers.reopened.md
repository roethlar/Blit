# a0-remote-helpers reopened

Reviewer: `claude-reviewer`
Reviewed sha: `2c9029ef2b9c0351bb4cf6e556eda56e1c1720b2`
Timestamp: `2026-05-16T16:14:15Z`

Validation was green:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Round-3 `coder-wait.sh` behavior checked out:

- `REVIEW_POLL_INTERVAL_SECONDS=1 REVIEW_WAIT_TIMEOUT_SECONDS=1 .review/coder-wait.sh a0-remote-helpers` timed out with `NO_VERDICT` while only an older reopened verdict existed.
- `REVIEW_POLL_INTERVAL_SECONDS=1 REVIEW_WAIT_TIMEOUT_SECONDS=1 .review/coder-wait.sh a0-remote-helpers 086fa497ffd0a8ce3fa372bb8ff8786ba757b921` returned the prior reopened verdict.
- After a temporary verified verdict for `2c9029e` was written, `REVIEW_POLL_INTERVAL_SECONDS=1 REVIEW_WAIT_TIMEOUT_SECONDS=1 .review/coder-wait.sh a0-remote-helpers 2c9029ef2b9c0351bb4cf6e556eda56e1c1720b2` returned the verified verdict.

## Findings

1. Medium / workflow correctness: `.review/check-state.sh` fails in a verified-only result state. When the reviewer accepts this finding, the natural state is `.review/results/a0-remote-helpers.verified.json` plus no `.review/results/*.reopened.md`. Running `.review/check-state.sh` in that state printed the verified files, then failed with `.review/check-state.sh: line 118: reopened_files[@]: unbound variable`. That means the state-lint added to keep the file workflow healthy breaks immediately after a clean acceptance with no reopened verdict files present.

   Fix direction: make `.review/check-state.sh` safe under `set -u` when either `verified_files` or `reopened_files` is empty. For example, initialize both arrays before nullglob expansion or iterate using a nounset-safe expansion.
