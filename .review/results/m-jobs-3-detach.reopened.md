# Reopened: m-jobs-3-detach

Reviewed sha: `d3e3a4d75c3f6230bc6b181691b7a50feaed6f1c`
Reviewed at: `2026-05-16T22:09:11Z`
Reviewer: `codex-reviewer`

Validation:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

## Findings

1. Medium - `--detach` does not actually detach the CLI.

   The new flag is documented as "Detach the transfer from the CLI process" ([crates/blit-cli/src/cli.rs](/Users/michael/Dev/Blit/crates/blit-cli/src/cli.rs:273)), and the wire comment says the CLI is free to exit after the daemon's `Started` event ([proto/blit.proto](/Users/michael/Dev/Blit/proto/blit.proto:597)). But the valid remote-to-remote path still awaits `run_delegated_pull(...).await` ([crates/blit-cli/src/transfers/remote_remote_direct.rs](/Users/michael/Dev/Blit/crates/blit-cli/src/transfers/remote_remote_direct.rs:140)), and the library loop continues reading the stream until `Summary` or `Error` ([crates/blit-app/src/transfers/remote.rs](/Users/michael/Dev/Blit/crates/blit-app/src/transfers/remote.rs:734)).

   The daemon-side `tx.closed()` disarm is useful, but from the CLI contract this is currently "survive manual client disconnect", not "detach". A normal `blit copy --detach ...` invocation still blocks until the transfer completes, and the default non-verbose path does not give the operator a clear "daemon owns it now" signal before they terminate the client themselves.

   Please either implement the promised exit-after-Started behavior for `--detach` (including enough output to let the operator find/cancel the job, such as a transfer id or explicit `jobs list` guidance), or narrow the user-facing flag/proto/help text so it does not claim detach/fire-and-forget semantics. Add coverage for the chosen contract.
