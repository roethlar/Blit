# audit-1b-net-timeouts-keepalive: delegation DNS/connect timeouts + HTTP/2 keepalive

**Severity**: Robustness
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `1d88fea`
**Parent finding**: `audit-1-daemon-timeouts` (items 1, 2, 4). Item 5
(port 0) shipped as audit-1a; item 3 (transfer stall-timeout) is
audit-1c. Item 6 (unbounded persistence channel) is low-severity and
not pursued.

## What

The clearly-safe, handshake-class network deadlines in the daemon's
delegation path, plus the owner-decided fix for the Subscribe
subscriber-leak (audit-1 item 4). Per the `feedback-server-await-timeouts`
memory, an unbounded `.await` on DNS / connect / a long-lived stream can
pin a handler (and its `ActiveJobs` row + resources) indefinitely.

## Approach

- **`net_timeout::within(deadline, fut) -> Option<F::Output>`** (new
  module): a generic, error-type-free deadline helper — a thin
  `tokio::time::timeout` wrapper returning `None` on elapse so each call
  site maps the timeout to its own domain error. Unit-testable with
  `std::future::pending`.
- **DNS** (`delegation_gate.rs` `StdResolver::resolve`): `lookup_host`
  bounded by `DNS_RESOLVE_TIMEOUT` (10s) → `io::ErrorKind::TimedOut` on
  elapse. Was: the OS resolver's own timeout (5-30s+) against a
  slow/black-holed server, stalling `DelegatedPull`.
- **Connect** (`delegated_pull.rs`): the dst→src
  `RemotePullClient::connect` bounded by `SOURCE_CONNECT_TIMEOUT` (30s,
  matching the data-plane accept timeout) → `Phase::ConnectSource` error
  frame on elapse. Was: the OS TCP SYN timeout (60-180s Linux) against a
  firewalled source.
- **HTTP/2 keepalive** (`main.rs` daemon `Server::builder`):
  `http2_keepalive_interval(30s)` + `http2_keepalive_timeout(20s)`. The
  audit's item 4 (Subscribe forwarder holding resources for a vanished
  client) is fixed at the transport layer: keepalive PINGs idle
  connections and reaps any that don't answer, reclaiming the gRPC
  stream + broadcast Receiver + forwarder task. **Owner decision
  (2026-05-23):** keepalive rather than an app-level "no events for N
  seconds → close", because F2's Subscribe stream is legitimately silent
  during quiet periods — an idle-event close would churn healthy
  subscribers (constant reconnect + GetState refetch), whereas keepalive
  leaves healthy idle streams untouched and only reaps genuinely-dead
  peers.

## Files changed

- `crates/blit-daemon/src/net_timeout.rs` (new): `within` + tests.
- `crates/blit-daemon/src/main.rs`: `mod net_timeout`; HTTP/2 keepalive
  on the Server builder.
- `crates/blit-daemon/src/delegation_gate.rs`: `DNS_RESOLVE_TIMEOUT`;
  `StdResolver` wraps `lookup_host` in `within`.
- `crates/blit-daemon/src/service/delegated_pull.rs`:
  `SOURCE_CONNECT_TIMEOUT`; connect wrapped in `within`.

## Tests

`blit-daemon` 145 (+2):

- `net_timeout::within_returns_none_when_the_deadline_elapses` — a
  `pending()` future under a short deadline yields `None` (deterministic).
- `net_timeout::within_passes_through_a_prompt_value` — a prompt future's
  value passes through.

The DNS/connect wraps reuse this tested helper; their elapse→error
mapping is straightforward. The keepalive is `Server`-builder
configuration (tonic behaviour) with no meaningful unit test — verified
against the tonic 0.14 API.

## Scope / next

- **audit-1c** (owner-approved): transfer **stall-timeout** — no bytes
  for 30s on the delegated `pull_sync_with_spec` byte path (an
  idle/stall detector, NOT a total-duration deadline, which would kill
  legitimate large transfers). Prerequisite for the owner-approved
  `--retry`/`--wait` follow-up feature.

## Reviewer comments

(empty — pending review)
