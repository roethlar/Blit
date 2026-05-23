# audit-2b-remote-connect-timeout: bound the remaining gRPC connects (DNS-aware)

**Severity**: Robustness
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `40ed2d6`
**Parent finding**: `audit-2-cli-timeouts` (part 2 of 2; audit-2a covered
the blit-app admin `BlitClient::connect` sites).

## What

The connect sites audit-2a left: the data-path `RemotePullClient` /
`RemotePushClient` connects, the two `transfers/remote.rs`
`BlitClient::connect` (delegated-pull) sites, and the `blit-cli`
`completions.rs` site. All connected with no deadline.

## Approach

- **blit-core `RemotePullClient::connect` + `RemotePushClient::connect`**:
  bound the connect at the **source**, so all three data-path call
  sites in `transfers/remote.rs` (remote pull, remote push, delegated
  pull) are fixed at once. Uses the **DNS-aware outer-timeout pattern**:
  an outer `tokio::time::timeout(30s)` around `Endpoint::connect()`
  (bounds DNS + TCP) plus `connect_timeout` as the inner TCP-phase
  bound. This deliberately avoids the `connect_timeout`-only flaw the
  reviewer caught in audit-2a round 1 (hyper-util resolves DNS before
  applying `connect_timeout`).
- **`transfers/remote.rs`** (2 `BlitClient::connect`, delegated-pull) →
  `crate::client::connect_with_timeout`, keeping the existing
  `dst_label` error context layered on top.
- **`blit-cli/src/completions.rs`** `BlitClient::connect` →
  `blit_app::client::connect_with_timeout`.
- Removed the now-unused `BlitClient` / `Context` imports.

## Files changed

- `crates/blit-core/src/remote/pull.rs`,
  `crates/blit-core/src/remote/push/client/mod.rs`: connect bodies →
  DNS-aware outer-timeout.
- `crates/blit-app/src/transfers/remote.rs`: 2 sites → helper; import
  cleanup.
- `crates/blit-cli/src/completions.rs`: 1 site → helper; import cleanup.

## Tests

No new unit tests. The connect-timeout is tonic's own mechanism, and the
DNS-aware outer-timeout pattern is unit-tested in audit-2a's
`client::connect_with_timeout` (and `net_timeout::within` from audit-1b).
The connect paths are behaviour-compatible on the happy path — existing
`blit-core` pull/push tests pass against the rewritten connects.

## Scope

This completes audit-2 (all CLI/app/data-path gRPC connects now bounded,
DNS included). Remaining audit backlog: audit-4 (Windows handle leak),
audit-5b (bridge server hardening), audit-6 (test gaps), audit-7b/c/d/e
(code health); plus audit-1c (transfer stall-timeout — design filed,
awaiting owner scope) and the `--retry`/`--wait` follow-up feature.

## Reviewer comments

(empty — pending review)
