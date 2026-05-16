# Reopened: m-jobs-3-detach

Reviewed sha: `18f1cb28901ce986c3613c3da34f448663e926ca`
Reviewed at: `2026-05-16T22:22:42Z`
Reviewer: `codex-reviewer`

Validation:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

## Findings

1. Medium - detach accepts an empty `transfer_id` as success.

   Round 2 correctly adds `DelegatedPullStarted.transfer_id`, but the proto explicitly says it is empty when the daemon is older than m-jobs-3 ([proto/blit.proto](/Users/michael/Dev/Blit/proto/blit.proto:647)). `run_delegated_pull_until_started` returns any `Started` payload without validating that field ([crates/blit-app/src/transfers/remote.rs](/Users/michael/Dev/Blit/crates/blit-app/src/transfers/remote.rs:870)), and the CLI then prints/serializes the empty id as a detached success ([crates/blit-cli/src/transfers/remote_remote_direct.rs](/Users/michael/Dev/Blit/crates/blit-cli/src/transfers/remote_remote_direct.rs:157)).

   Against an older daemon that ignores the new `detach` field, this is especially bad: dropping the stream after `Started` will let the old daemon's `tx.closed()` path cancel the transfer, while the CLI reports a detached job with no usable id. Please treat an empty Started transfer id as an incompatibility/error instead of success, and add coverage for that branch.

2. Medium - the human cancel/status hints drop ports and break IPv6 destinations.

   `destination_host_hint` uses `split_once(':')` ([crates/blit-cli/src/transfers/remote_remote_direct.rs](/Users/michael/Dev/Blit/crates/blit-cli/src/transfers/remote_remote_direct.rs:233)). For `host:9444:/module/path`, the printed follow-up command becomes `blit jobs cancel host <id>`, which targets the default port instead of `9444`. For bracketed IPv6 such as `[::1]:9444:/module/path`, the hint becomes just `[`, which is unusable.

   Please derive the hint from the parsed destination endpoint rather than string-splitting the raw argument. Preserve non-default ports and bracketed IPv6, and add unit coverage for at least `host:port:/module/path` and bracketed IPv6.
