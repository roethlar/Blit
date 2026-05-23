# audit-9-cancel-auth: CancelJob lacks peer authorization

**Severity**: Bug
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `3c5a398`

## What

Ground-up audit found that the `CancelJob` RPC handler does not verify that the
requesting peer is the same peer that started the transfer.

**`crates/blit-daemon/src/service/core.rs:1102-1130`** — `cancel_job` rejects
empty `transfer_id` but never calls `peer_addr_string(&request)` or compares
the caller's address to the `peer` field stored in the active job row. Any gRPC
client that can reach the daemon can cancel any active transfer, including
`DelegatedPull` transfers initiated by another operator.

**`crates/blit-daemon/src/active_jobs.rs:456-466`** — `ActiveJobs::cancel`
looks up the row and fires the `CancellationToken`, but the table entry carries
a `peer` string that is never inspected for authorization.

This is a cross-tenant/cross-user security gap in a multi-operator deployment.

## Approach

In `cancel_job`, extract the peer address from the gRPC request metadata and
compare it to `entry.job.peer` (or whatever field stores the initiator). Return
`PermissionDenied` (or `Unauthenticated`) when they don't match.

For local-Unix-socket deployments where the peer address is always the same,
a `localhost` / Unix-socket bypass is acceptable, but the check should still
exist and be explicit.

## Files changed

TBD by coder. Primarily `crates/blit-daemon/src/service/core.rs` and
`crates/blit-daemon/src/active_jobs.rs`.

## Tests

- Unit test: peer A starts a transfer, peer B tries to cancel → `PermissionDenied`.
- Unit test: peer A starts a transfer, peer A cancels → `Cancelled`.
- Existing cancel tests must still pass.

## Resolution (commit `3c5a398`)

Added `ActiveJobs::cancel_authorized(transfer_id, caller: Option<SocketAddr>)`
and a free helper `cancel_peer_authorized(caller, owner)`; `cancel_job`
now captures `request.remote_addr()` *before* `into_inner()` and routes
through it. New `CancelOutcome::Unauthorized` → `Status::permission_denied`.

**Key design point — compare host/IP, not `IP:port`.** The cancel RPC and
the transfer arrive over different connections, so their ephemeral source
ports differ; only the host is stable. The naive "compare the stored
`peer` string" would have denied every legitimate same-operator cancel.

Authorization rules (in `cancel_peer_authorized`):
- caller `None` (tonic over a Unix socket → `remote_addr()` is `None`) →
  allow (UDS callers are local/trusted; matches the finding's accepted
  UDS bypass).
- loopback caller → allow (single-host deployments).
- otherwise caller IP must equal the owner's parsed IP.
- owner string that doesn't parse as `SocketAddr` (e.g. `"unknown"`,
  from a UDS-initiated transfer) → deny any non-loopback caller.

`NotFound` and `Unsupported` are still decided before the authz check.
The pre-existing no-auth `cancel` is kept for the in-crate cancel tests.

**Tests (blit-daemon, +3):**
`cancel_authorized_denies_a_different_host` (B→A denied, token NOT fired),
`cancel_authorized_allows_same_host_different_port` (A's new connection
authorized, token fired — proves port-insensitivity),
`cancel_authorized_bypass_and_precedence` (loopback v4+v6 and `None`
allowed; `"unknown"` owner denies remote; matching host allowed;
`NotFound`/`Unsupported` precede authz). Full workspace gate green.

## Reviewer comments

(empty — pending review)
