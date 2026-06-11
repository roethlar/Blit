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
