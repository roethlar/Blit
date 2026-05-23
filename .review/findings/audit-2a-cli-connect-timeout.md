# audit-2a-cli-connect-timeout: bounded connect on blit-app admin gRPC connections

**Severity**: Robustness
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `5ded6c9`
**Parent finding**: `audit-2-cli-timeouts` (part 1 of 2). audit-2b will
cover the `transfers/remote.rs` BlitClient sites, the
`Remote{Pull,Push}Client::connect` data-path connects, and the
`blit-cli` `completions.rs` site.

## What

Every `BlitClient::connect(uri)` in `blit-app` connected with no
deadline, so an unreachable daemon (slow DNS, hung TCP handshake,
network partition) made admin verbs — and the Prometheus bridge, which
calls `jobs::query` — hang for the OS TCP timeout (60-127s). Per
`feedback-server-await-timeouts`, these need a bound.

## Approach

- **`blit_app::client::connect_with_timeout(uri) -> Result<BlitClient<Channel>>`**
  (new module): builds the client via
  `tonic::transport::Endpoint::from_shared(uri)?.connect_timeout(30s)
  .connect()`, then `BlitClient::new(channel)`. A drop-in for
  `BlitClient::connect(uri)` that bounds the TCP connect (and DNS, on
  tonic's connector). Errors carry the URI for context.
- Swapped all **10** `admin/` connect sites to the helper: the six
  verbs (`ls`, `du`, `find`, `df`, `rm`, `list_modules`) and the four
  `jobs.rs` sites (`query`, `cancel`, `clear_recent`, `subscribe`).
  `jobs::query` gaining the timeout means the bridge inherits it.
- Removed the now-unused `BlitClient` / `Context` imports (via
  `cargo fix`).

## Files changed

- `crates/blit-app/src/client.rs` (new): helper + test.
- `crates/blit-app/src/lib.rs`: `pub mod client`.
- `crates/blit-app/src/admin/{ls,du,find,df,rm,list_modules,jobs}.rs`:
  connect sites → `connect_with_timeout`; import cleanup.

## Tests

`blit-app` (+1):

- `client::connect_with_timeout_rejects_a_malformed_uri` — a non-URI
  surfaces a clear `invalid daemon endpoint` error (the `from_shared`
  parse path), exercising the helper's error wiring. The connect-timeout
  firing itself is tonic's `connect_timeout` mechanism (its own
  behaviour), not re-tested here.

## Scope / next

audit-2b: the remaining connect sites — `transfers/remote.rs` (2
`BlitClient::connect` + the `Remote{Pull,Push}Client::connect`
data-path connects) and `blit-cli/src/completions.rs`. The data-path
clients (`RemotePullClient`/`RemotePushClient`) have their own
`connect` constructors in `blit-core`, so audit-2b may add the timeout
there or wrap at the call sites.

## Reviewer comments

(empty — pending review)
