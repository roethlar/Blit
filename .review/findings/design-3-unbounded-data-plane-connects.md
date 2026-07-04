# design-3 — TCP data-plane connects are unbounded (the audit-2 fix never reached the data plane)

**Source**: Design-coherence review Phase A (`docs/audit/DESIGN_MAP_2026-06-11.md` §1.1/§1.2),
mechanism re-verified by hand 2026-06-11 before filing.
**Severity**: Medium (RELIABLE — hangs for the OS SYN timeout, 60–127 s,
instead of failing plainly; exactly the failure mode audit-2 fixed for gRPC).

## What

Both TCP data-plane client connects call `TcpStream::connect` with **no
timeout**:

- `crates/blit-core/src/remote/transfer/data_plane.rs:92`
  (`DataPlaneSession::connect`, push direction) — also the subsequent
  handshake-token write at `:128`.
- `crates/blit-core/src/remote/pull.rs:1710`
  (`receive_data_plane_stream_inner`, pull/pull-sync direction) — also the
  token write at `:1714`.

Every **control-plane** connect in the workspace is bounded at 30 s (twice or
three times over), because the audit-2 wave fixed gRPC connects — but the fix
was never propagated to the TCP data plane that carries the actual bytes. A
firewalled or black-holed data port (the daemon advertises a fresh ephemeral
port per transfer; asymmetric firewalls that pass 9031 but block ephemerals
are common) hangs the transfer for the kernel SYN timeout with no message.

## Proposed fix (slice-sized)

Bound both connects (and the token writes) with the same 30 s policy the
control plane uses — ideally importing one shared constant rather than adding
a fifth literal (see map §1.2: the 30 s bound is already declared three times,
the accept/token pair four times). On timeout, fail with explanatory text
naming the host:port and the likely firewall cause, and preserve an
`io::ErrorKind::TimedOut` in the chain so `is_retryable` can classify it.
Regression test: connect to a non-routable address (e.g. RFC 5737 TEST-NET) and
assert a plain error within the bound instead of an OS-timeout hang.

## Cross-references

- Map §1.1 (channel construction) risk list; §1.2 (timeouts & liveness).
- Queued slice-2 transport work (STATE.md Queue item 2) — same policy family;
  this finding is the TCP-connect corner of it and can land independently.

---

## Implementation record (2026-07-04, the slice that closed this)

**Commit**: see REVIEW.md row. Landed as the coder's-pick smaller
alternative after w4-4, per the long-standing queue note.

### What / Approach

`remote::transfer::socket::dial_data_plane(addr, handshake,
tcp_buffer_size)` — the client-side mirror of the daemon's bounded
accept, owned by the same w1-family socket-policy module:

- connect bounded by the shared `DATA_PLANE_ACCEPT_TIMEOUT` (30 s, the
  row's sanctioned constant — no fifth literal);
- `configure_data_socket` (w1-2/w1-3 policy) applied;
- handshake-token write bounded by `DATA_PLANE_TOKEN_TIMEOUT` (15 s),
  mirroring the acceptor's bounded token read (the finding's "also the
  token write" clause);
- on either timeout the chain carries an `io::ErrorKind::TimedOut`
  source with explanatory text naming addr + the likely-firewall
  cause, so `remote::retry::is_retryable` classifies it transient
  (`--retry` re-dials).

Both connect sites collapsed onto it: `pull.rs connect_pull_stream`
(pull/pull-sync + resize-ADD dials) and
`data_plane.rs DataPlaneSession::connect_with_probe` (push TCP —
elastic/resize dials included, since ADD streams route through the
same constructor). A timeout-parameterized private core
(`dial_data_plane_with_timeouts`) exists solely so tests pin the
bounded shape without waiting the production 30 s. The socket.rs
module doc's "connect timeouts live at the call sites" paragraph
(written when w1-2 anticipated this slice) rewritten to match.

### Tests (blit-core lib 389 → 392)

- happy path: dial + policy landed + handshake bytes received;
- deterministic timeout SHAPE: a peer that accepts but never reads,
  against a 64 MiB handshake — write_all stalls, the token bound
  fires, the chain carries TimedOut and classifies retryable
  (mutation-verified: replacing the timeout error with a plain eyre
  message fails this pin);
- black-holed connect (RFC 5737 TEST-NET-1) fails within the bound —
  the timeout-shape assertions apply on the (common) black-hole arm;
  a network that fast-rejects TEST-NET still proves the bound.

### Known gaps

- The connect-bound pin is environment-tolerant (fast-reject networks
  skip the shape assertions); the shape is deterministically pinned
  via the token-write arm instead.
- No e2e drives a real firewalled daemon; the failure text is
  reviewed, not user-tested.
