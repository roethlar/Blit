# w1-2-data-socket-policy-helper — one shared configure_data_socket for every data-plane socket

**Branch**: `master`
**Commit**: `16237e2`
**Source**: `docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` §W1.2 (ratified
D-2026-06-11-2), finding
`boundaries-pull-direction-bypasses-socket-policy` in
`docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`. Coordinates with
design-3 (missing connect timeouts at the same call sites — **not**
implemented here; the helper configures an already-established stream,
so design-3 stays a clean follow-up at the call sites).

## What

The NODELAY/keepalive/tuned-buffer socket policy existed in two
divergent copies, both push-only, and the entire pull direction ran
bare:

- `DataPlaneSession::connect_with_probe` (blit-core) — nodelay hard
  error, keepalive logged, buffers logged, via an `into_std`/`from_std`
  round trip with two extra failure paths.
- A private daemon twin of the same name (`push/data_plane.rs`) —
  nodelay + keepalive both **silently swallowed** (`let _ =`), never
  buffers.
- The pull client connect (`connect_pull_stream`) and all three daemon
  pull_sync accept paths (epoch-0, resize, resume): **no options at
  all** — Nagle stayed on for the path that carries pull bytes, and the
  pull_sync `TransferDial`'s `tcp_buffer_bytes` ramped to 8 MiB and was
  never read (computed-and-discarded, exactly as the audit filed).

The audit's site list predates REV4; the live sites were re-mapped this
session (the Pull RPC died at ue-r2-1h; ue-r2-2 added the resize accept
paths on both directions).

## Approach

- New `blit_core::remote::transfer::socket::configure_data_socket(
  &TcpStream, Option<usize>) -> io::Result<()>` — the single policy
  owner. `SockRef`-based, in place: the `into_std`/`from_std` round
  trip and its two error paths are gone. Posture (the core side's,
  which was the deliberate one): nodelay hard error; keepalive
  best-effort logged (POST_REVIEW_FIXES §1.1); buffers best-effort
  logged, applied iff `Some`. Re-exported from `remote::transfer`.
- Call sites:
  - `connect_with_probe`: inline block replaced by the helper
    (identical semantics, minus the round trip).
  - `connect_pull_stream` (pull client): helper with `None` — the pull
    dial lives on the daemon; the client has no value to apply. This is
    the Nagle fix for the pull direction's client end.
  - Daemon push accepts (3 sites): private twin **deleted**; shared
    helper with `None` (receiver side holds no dial). Posture change:
    option failures now surface — `Status::internal` on the two fatal
    accept paths, logged-lapse on the non-fatal resize arm — instead of
    being silently swallowed.
  - Daemon pull_sync accepts: `accept_and_wrap_sinks` takes a new
    `tcp_buffer_size` param (caller passes `dial.tcp_buffer_bytes()`,
    a connect-time snapshot — `None` at epoch-0, matching push
    semantics); `accept_one_resize_socket` reads the dial **live** at
    accept (the one pull socket that actually gets the ramped 8 MiB —
    mirror of push `add_stream`); the resume accept passes the dial
    snapshot (always `None` in practice — resume runs no tuner).
- blit-daemon's `socket2` dependency dropped (its only use was the
  deleted twin).

## Deliberately out of scope

- **Connect timeouts** (design-3's slice; same call sites, disjoint
  lines — this change makes that slice smaller, not larger).
- **Keepalive timing honesty** (w1-3). The helper inherits the existing
  `set_keepalive(true)`-with-OS-default-timing behavior and comment
  posture; note the "daemon copy logs failure" half of w1-3 is now
  satisfied structurally (there is no daemon copy).
- **Accept/token constant consolidation** (w1-4) — untouched.

## Tests added (blit-core 414 → 417)

- `socket::tests::applies_nodelay_keepalive_and_buffers` — loopback
  pair; `Some(256 KiB)`: nodelay + keepalive read back true, both
  buffer directions ≥ requested (kernels round up, never down at this
  size).
- `socket::tests::none_leaves_kernel_default_buffers` — `None`:
  nodelay/keepalive land, buffer sizes bit-identical to the kernel
  defaults captured before the call.
- `pull::data_plane_receive_tests::connect_pull_stream_applies_socket_policy`
  — asserts nodelay + keepalive **on the stream returned by
  `connect_pull_stream`** and that the handshake still arrives; guards
  the call-site wiring, not just the helper (reverting the pull.rs call
  fails this test while the helper's own tests stay green).

**Mutation verification** (run pre-commit, implementation restored and
gate re-run green after each):
M1 revert the `connect_pull_stream` call-site wiring → wiring test
fails on the nodelay assertion while the helper's own tests stay green;
M2 drop `set_tcp_nodelay` from the helper → all three fail;
M3 drop the buffer-sizing block → `applies_nodelay_keepalive_and_buffers`
fails, the `None` test (correctly) still passes.

Full suite: fmt clean, clippy clean (workspace, all targets,
`-D warnings`), `cargo test --workspace` 1445 passed / 0 failed /
2 ignored across 37 suites. blit-core 414 → 417; blit-daemon 168
unchanged (HEAD-measured baseline — the w4-3 record's "167" was the
Windows-host count).

## Known gaps

- No daemon-side test asserts options on an accepted socket: the accept
  helpers hand the socket straight into session/sink internals, so the
  option state is unobservable without restructuring those paths. The
  daemon wiring is exercised by every existing accept-path test
  (compile + runtime), and the policy itself is pinned at the helper
  and pull-client levels.
- The pull_sync epoch-0/resume sockets still get `None` in practice
  (dial not yet ramped / no tuner) — that is REV4's documented
  connect-time-snapshot semantics, identical to push epoch-0, not a gap
  this slice may unilaterally change.
- Windows run not executed from this macOS session; the code is
  platform-neutral (`SockRef` needs tokio's `AsFd`/`AsSocket`, present
  on both) and windows-latest CI covers it on the next push.
