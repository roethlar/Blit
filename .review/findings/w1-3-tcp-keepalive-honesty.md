# w1-3-tcp-keepalive-honesty — real TcpKeepalive timing in the shared socket policy

**Branch**: `master`
**Commit**: `865fc1e`
**Source**: `docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` §W1.3 (ratified
D-2026-06-11-2), finding
`drift-set-keepalive-comments-oversell-liveness`.

## What

The audit-era finding: both `set_keepalive(true)` sites ran OS-default
keepalive timing (~2 h idle before the first probe on every supported
platform) while their comments claimed the option prevented idle stream
timeouts during long transfers — an oversell by two orders of
magnitude on transfer timescales. The ratified row offered two exits:
configure real `TcpKeepalive` timing, or rewrite both comments to admit
OS-default behavior; plus "make the daemon copy log its failure."

w1-2 collapsed the surface first: the daemon's silently-swallowing copy
was deleted and both sites became one — the `set_keepalive(true)` call
inside `configure_data_socket`. That already satisfied the
logs-failure clause structurally. This slice takes the first exit
(real timing) at that single site, which makes the surviving comments
*true* instead of softened.

## Approach

- `TcpKeepalive::new().with_time(60 s).with_interval(10 s)
  .with_retries(5)` via `socket2::Socket::set_tcp_keepalive`, which
  also flips `SO_KEEPALIVE` on — the bare `set_keepalive(true)` call
  is replaced, not supplemented. Best-effort-logged posture unchanged.
- Constants are module-`pub` (`TCP_KEEPALIVE_IDLE` / `_INTERVAL` /
  `_RETRIES`) with the rationale on the first: a vanished peer on an
  **idle** data socket (armed resize slot, a stream waiting for work
  while siblings transfer) is detected in ~2 minutes (60 s + 5×10 s)
  instead of ~2 h. The complementary failure — a stalled peer with
  data in flight — belongs to StallGuard's 30 s, not keepalive.
- Value choice: detection well inside a long transfer's lifetime,
  probes far apart enough to be noise-free, and strictly
  more-conservative than the gRPC control plane's HTTP/2 keepalive
  (audit-1b) so the data plane never declares death first on a merely
  slow network. All three knobs are supported on Linux/macOS/Windows
  (socket2 exposes `TCP_KEEPCNT` on Windows 10+; failure logs).
- blit-core's socket2 gains `features = ["all"]` — required for
  `with_retries` and for the getter side (`tcp_keepalive_time` etc.)
  the test reads back. No new dependency, no lockfile change.

## Deliberately out of scope

- w1-4 (accept/token constant consolidation) — untouched.
- design-3 (connect timeouts) — untouched.
- The gRPC control plane's HTTP/2 keepalive (audit-1b) is a different
  layer and stays as is.

## Tests added (blit-core 417 → 418)

- `socket::tests::keepalive_timing_is_explicit` (`#[cfg(unix)]` — the
  socket2 getters don't exist on Windows; Windows exercises the set
  path through the module's other tests): configures a loopback socket
  and reads idle/interval/retries back **through the kernel**,
  asserting each equals the policy constant — pinning what a peer
  experiences, not what was requested.

**Mutation verification**: reverting `set_tcp_keepalive(&keepalive)`
to the old bare `set_keepalive(true)` fails the test on the idle-time
assertion (kernel default ≠ 60 s); implementation restored, gate
re-run green.

Full suite: fmt clean, clippy clean (workspace, all targets,
`-D warnings`), `cargo test --workspace` 1446 passed / 0 failed /
2 ignored across 37 suites (was 1445).

## Known gaps

- The read-back test is unix-only (getter availability); Windows
  behavior of `set_tcp_keepalive` (WSAIoctl `SIO_KEEPALIVE_VALS` +
  `TCP_KEEPCNT`) is exercised but not value-asserted — windows-latest
  CI compiles and runs the set path on the next push.
- No test drives an actual dead-peer detection end-to-end (would need
  a ≥70 s wall-clock wait or kernel fault injection); the kernel
  read-back is the deterministic proxy, same evidence class the
  audit-1b HTTP/2 keepalive landed on.
