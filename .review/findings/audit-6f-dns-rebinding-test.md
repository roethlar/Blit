# audit-6f-dns-rebinding-test: DNS-rebinding regression tests for the delegation gate

**Severity**: Test Gap
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `28e0b95`
**Parent finding**: `audit-6-test-gaps` (item 6).

## What

`delegation_gate.rs` already implements the DNS-rebinding mitigation
("resolve once, bind the validated IP" — `validate_source` returns the
`SocketAddr` the handler connects to, and the handler never re-resolves),
and a `ScriptedResolver` test double that can return different answers on
successive lookups existed — but no test actually exercised the
multi-answer path to lock the behavior in (finding item 6).

## Approach

Added two `#[tokio::test]`s driving a `ScriptedResolver` scripted with two
different responses:

- `validate_source_binds_first_resolution_against_rebind`: the first
  lookup returns an allowlisted public IP; the second returns
  `169.254.169.254` (the cloud metadata endpoint). The gate returns the
  first IP and leaves the second answer **unconsumed** (asserted by a
  follow-up `resolver.resolve()` that still yields the malicious IP) —
  proving exactly one resolution, so a rebind cannot redirect the
  connect.
- `validate_source_decides_on_first_resolution_only` (converse): the
  first lookup returns a special-range IP that a hostname allowlist entry
  cannot authorize, so the gate denies with `SpecialRangeNeedsIpAuth`
  even though a later resolution would have passed — locking in that only
  the first answer is consulted.

## Files changed

- `crates/blit-daemon/src/delegation_gate.rs`: 2 tests in the existing
  `tests` module. No production change — the mitigation already existed;
  this is regression coverage.

## Scope

One sub-item of audit-6. Remaining test gaps: 6a (blit-app inline tests),
6b (TUI render), 6c (bridge HTTP integration), 6e (pull-move/push-move),
6g (copy fast-path fallback). 6d (path_safety Unicode) verified.

## Reviewer comments

(empty — pending review)
