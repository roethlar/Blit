# audit-1a-delegation-port-zero: reject IANA-reserved port 0 at the delegation gate

**Severity**: Robustness
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `a3147b6`
**Parent finding**: `audit-1-daemon-timeouts` (item 5 — port-0 rejection).
The audit's network-timeout items are split out (see Deferred below).

## What

The delegated-pull SSRF gate (`validate_source`) accepted a source
locator with **port 0** (IANA-reserved, not connectable). It fell
through to DNS resolution + an outbound `RemotePullClient::connect` that
would fail opaquely (or, on some stacks, bind an ephemeral port). audit-1
item 5: reject it at the boundary.

## Approach

- `GateDenial::InvalidPort(u16)` variant + `reason()` arm
  ("source port N is reserved and not connectable").
- `validate_source` rejects `locator.port == 0` immediately after the
  master-switch check — before host normalization, DNS, or any connect.
  The gate is the right home (it's the SSRF/validation boundary and is
  unit-tested), rather than deeper in the `delegated_pull` handler.

## Files changed

- `crates/blit-daemon/src/delegation_gate.rs`: `InvalidPort` variant,
  `reason()` arm, gate check, test.

## Tests

`blit-daemon` 143 (+1):

- `port_zero_rejected_before_dns` — delegation enabled, port 0, a
  resolver with **no** scripted responses (so reaching DNS would surface
  `UnresolvableHost`). Asserting `InvalidPort` proves the port-0 check
  fires before resolution / connect.

## Deferred (NOT in this slice)

audit-1's network-timeout items, split by risk:

- **audit-1b (clearly safe, planned next)**: wrap the delegation-path
  **DNS resolution** (`StdResolver::resolve` → `lookup_host`) and **TCP
  connect** (`RemotePullClient::connect`) in `tokio::time::timeout`.
  These are handshake-class operations that should complete in
  seconds — a 10-30s bound is unambiguously correct.
- **Needs an owner/design decision (flagged, not implemented)**: the
  audit also suggests a timeout on `pull_sync_with_spec` and on the
  Subscribe forwarder. Those bound **long-lived data operations** — a
  large transfer or a legitimately-idle subscriber can run far longer
  than any fixed deadline, so a total-duration timeout would kill
  legitimate work. The correct mechanism is an **idle/stall timeout**
  (no progress for N seconds), which is a design decision: what counts
  as "idle," and is it configurable? Captured here for owner input
  rather than guessed.

## Reviewer comments

(empty — pending review)
