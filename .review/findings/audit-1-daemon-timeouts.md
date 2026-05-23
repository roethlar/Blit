# audit-1-daemon-timeouts: Network operation timeout gaps in blit-daemon

**Severity**: Robustness
**Status**: Open
**Branch**: (none yet)

## What

Ground-up codebase audit (2026-05-23) identified 6 sites in the daemon where
network operations lack explicit `tokio::time::timeout` guards. These gaps are
a direct violation of the project's documented principle
(`feedback_server_await_timeouts.md` memory) that every `.await` on a socket
read/RPC in a long-running handler needs a timeout.

## Approach

Add `tokio::time::timeout()` wrappers at each site:

1. **`crates/blit-daemon/src/delegation_gate.rs:257-258`** — `lookup_host((host, port)).await` for DNS resolution in the SSRF gate. No timeout means a slow/blackhole DNS server stalls the `DelegatedPull` handler for OS-resolver timeout (5-30s).

2. **`crates/blit-daemon/src/service/delegated_pull.rs:270`** — `RemotePullClient::connect(endpoint).await`. No timeout means a firewalled source daemon blocks the handler for OS TCP SYN timeout (60-180s Linux).

3. **`crates/blit-daemon/src/service/delegated_pull.rs:329-338`** — `pull_client.pull_sync_with_spec(...).await`. No deadline means a stalled source daemon ties up ActiveJobGuard row + resources until manual cancel.

4. **`crates/blit-daemon/src/service/core.rs:353-454`** — Subscribe forwarder has no idle/time-since-last-event timeout. Quiet-period subscribers hold a gRPC stream + broadcast Receiver + mpsc slot + spawned task indefinitely.

5. **`crates/blit-daemon/src/service/delegated_pull.rs:176-179`** — DelegatedPull accepts TCP port 0 (IANA reserved). Should reject at the boundary with clear DelegationRejected error before proceeding to DNS + connect.

6. **`crates/blit-daemon/src/active_jobs.rs:759`** — `arm_persistence_at` uses `mpsc::unbounded_channel()` for persistence signals. Under disk-full + sustained completion rate, signals accumulate without backpressure. (Low severity — writer coalesces via `try_recv()` loop.)

For sites 1-3: a 15-30s timeout matching the data-plane accept timeout (`PUSH_HANDSHAKE_TIMEOUT` at 15s) provides consistent failure latency across all network operations in the delegation path.

## Files changed

TBD by coder.

## Tests

- Unit tests for timeout behavior (timeout fires → appropriate error)
- Existing delegation gate + delegated_pull tests must still pass

## Known gaps

- Subscribe idle timeout needs a design decision: what is "idle" for the stream? Configurable? Hardcoded? The finding doc captures this for owner decision.
