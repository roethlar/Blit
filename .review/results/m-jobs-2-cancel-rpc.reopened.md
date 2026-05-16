# Reopened: m-jobs-2-cancel-rpc

Reviewed sha: `a96ca93e80e2d0346348ddd07dbb956f89a7fda4`
Reviewed at: `2026-05-16T21:50:29Z`
Reviewer: `codex-reviewer`

Validation:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

## Findings

1. Medium - `blit jobs cancel` does not implement the documented exit-code contract.

   The finding doc and the inline comment promise distinct script-visible outcomes: cancelled -> 0, not found -> 1, unsupported -> 2. But `run_jobs_cancel` returns `Err` for both non-success outcomes ([crates/blit-cli/src/jobs.rs](/Users/michael/Dev/Blit/crates/blit-cli/src/jobs.rs:42)), while `main` only propagates semantic `ExitCode`s for `check`; all other command errors flow through the normal `Result` error path ([crates/blit-cli/src/main.rs](/Users/michael/Dev/Blit/crates/blit-cli/src/main.rs:56)). In practice `NotFound` and `Unsupported` both exit as the generic error code, so scripts cannot distinguish them as advertised.

   Please make the jobs command path return or otherwise propagate `ExitCode` for `CancelJobOutcome`, similar to `check`, and add coverage that pins `Unsupported` as exit code 2 and `NotFound` as exit code 1. Keeping JSON/stdout formatting is fine; the issue is the process status.
