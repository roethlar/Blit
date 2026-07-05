# otp-4 — daemon serves `Transfer`, client initiates as SOURCE

**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-4.
**Status**: scoped — approach recorded 2026-07-05, implementation next.
**Contract**: `docs/TRANSFER_SESSION.md` (`f861579` + otp-3 §Inv 2 annotation).
**Builds on**: otp-3 (`ef9ffa1`+`d5796a1`) — drivers exist, in-process
transport only; no gRPC `FrameTransport` yet.

## Scope (what otp-4 proves)

The remote push-equivalent rides the unified session end-to-end: a
real daemon serves the `Transfer` RPC by running `run_destination`
(responder), a real client runs `run_source` (initiator, role
SOURCE) over a gRPC-backed `FrameTransport`, with the TCP data plane
as the default byte carrier and the sf-2 shape-corrected dial as the
stream policy. A/B parity pins vs old push: byte-identical trees,
summary parity, sf-2 pin ported. Old push stays fully live (cutover
is otp-10); CLI verbs stay on old push until otp-10.

## Staging (two commits, each through the codex loop)

- **otp-4a — serve + initiate over the in-stream carrier**:
  - `transport.rs` gains the gRPC pair: client side wraps
    `BlitClient::transfer` (`IntoStreamingRequest<Message=TransferFrame>`
    out / `Streaming<TransferFrame>` in); daemon side wraps
    `Streaming<TransferFrame>` in / `mpsc → ReceiverStream` out
    (`core.rs:351` `type TransferStream`).
  - Daemon handler (`core.rs` `transfer()`, replacing the otp-1
    UNIMPLEMENTED body): register `ActiveJobKind` row, race
    `job.cancellation_token()` per the `resolve_streaming_outcome`
    pattern → peer sees `SessionError{CANCELLED}`; responder-side
    open validation hook does module/path resolve
    (`MODULE_UNKNOWN`), read-only refusal (`READ_ONLY`) — refusals
    as SessionError at OPEN, never silent close;
    `set_endpoint(module, path)` on the jobs row;
    `ActiveJobGuard::bytes_counter()` wired into the session sink
    via `with_byte_progress` (old push never wired it — parity-plus,
    same boundary the contract names). otp-4a serves the
    SOURCE-initiator layout only: a DESTINATION-declaring initiator
    (legal wire-wise) is refused with a clear
    "pull-equivalent lands at otp-5" SessionError, not a
    PROTOCOL_VIOLATION.
  - Client entry `remote/transfer/session_client.rs` (blit-core):
    connect via the existing `RemoteEndpoint::control_plane_uri()`
    channel pattern, run `run_source` as initiator; returns the
    session summary + needed count. Not wired to CLI verbs.
  - **SizeMtime semantic decision (flagged by otp-3)**: settled here
    because the parity pins force it. Old push's destination diff is
    exact size+mtime equality (`file_requires_upload`); the session
    currently inherits `manifest::compare_file`'s Default arm
    (transfer only when src NEWER). For the push-equivalent parity
    bar the session must match old push; direction of resolution
    (exact-match arm for the session's SizeMtime vs changing the
    shared Default arm, which would alter live pull_sync behavior
    pre-cutover) is settled in-slice with codex review; the choice
    and its blast radius get recorded in this doc + DECISIONS if the
    shared arm changes.
  - Parity pins (in-stream): same fixture through old push and the
    session → byte-identical destination trees + equal
    files/bytes counts; read-only refusal parity; unknown-module
    parity; cancel-mid-transfer produces CANCELLED.
- **otp-4b — TCP data plane + resize + sf-2 pin ported**:
  - Responder binds via `bind_data_plane_listener`, mints
    `generate_token()` + epoch-0 sub-token, arms
    `accept_data_connection_stream_resizable` (AbortOnDrop), returns
    `DataPlaneGrant{tcp_port, session_token, initial_streams
    (ACCEPT CEILING = min(engine floor, local capacity)),
    epoch0_sub_token}` inside SessionAccept. In-stream carrier
    remains the fallback (`SessionOpen.in_stream_bytes` or
    grant-less accept) — otp-3's record loop unchanged.
  - Source-side byte path: `MultiStreamSender`-equivalent over
    `DataPlaneSession::connect` (socket auth: `session_token ‖
    sub_token`, one write), payloads through the existing pipeline;
    `maybe_shape_resize` logic ported: `initial_stream_proposal` +
    `TransferDial::propose_shape_resize` as the need list
    accumulates; resize rides frames 16/17 (`DataPlaneResize`/`Ack`)
    on the session stream — the otp-3 "resize = violation" arm
    becomes data-plane-session behavior (in-stream sessions keep
    refusing).
  - Destination loop gains the data-plane carrier seam: control
    frames stay on the RPC; payload bytes land via the accept task
    (`TransferStats`); outstanding-set reconciliation + summary
    composition from stats; `SourceDone` semantics per contract.
  - sf-2 pin ported: 10k one-byte files through the SESSION on
    loopback must open >1 data-plane stream
    (`shape_resize_e2e.rs` pattern, real daemon + real client).
  - Converge-guard micro-check (not the otp-12 matrix): loopback
    wall-time of session-push within noise of old push on the
    10k-tiny fixture, recorded in this doc (regression tripwire
    only).

## Key integration facts (surveyed 2026-07-05, agent map)

- Handler: `core.rs:356` stub; trait `blit_server::Blit`; stream
  type `ReceiverStream<Result<TransferFrame, Status>>`.
- Jobs: `active_jobs.rs` `register:415`, `set_endpoint:971`,
  `cancellation_token:984`, `bytes_counter:1000`.
- Read-only + module resolve precedent: `push/control.rs:118-126`.
- Data plane server: `push/data_plane.rs` (`bind:45`, `token:59`,
  `resizable accept:274`, `ResizeArm:235`, TTL `:251`); ADD arm
  before ack (`control.rs:554-603`), ceiling
  `local_receiver_capacity().max_streams`.
- Client dial: `socket.rs:123 dial_data_plane` (handshake = one
  `write_all` of token‖sub), `DataPlaneSession::connect:100`,
  resize controller `client/mod.rs:534-551` + ack consume `:1105`.
- sf-2 pin: `push/shape_resize_e2e.rs:30` (10k files,
  `data_plane_streams > 1`, real loopback daemon).
- Five Transfer test fakes return UNIMPLEMENTED and stay valid until
  cutover (they stub the trait, not behavior).

## Files (planned)

- `crates/blit-core/src/transfer_session/transport.rs` (gRPC pair)
- `crates/blit-core/src/transfer_session/mod.rs` (responder open
  validation hook w/ module resolve callback; 4b: carrier seam,
  resize arms)
- `crates/blit-core/src/remote/transfer/session_client.rs` (new)
- `crates/blit-daemon/src/service/core.rs` (+ a
  `service/transfer.rs` handler body next to the pin test)
- `crates/blit-core/src/manifest.rs` (SizeMtime resolution, per the
  in-slice decision)
- Tests: `crates/blit-daemon/src/service/transfer.rs` (pin flips
  from UNIMPLEMENTED to behavior), new parity e2e in blit-daemon
  tests (old push vs session), 4b: ported sf-2 pin.

## Known gaps (carried)

- Daemon-as-SOURCE (pull-equivalent) and the four-layout dispatch:
  otp-5. Mirror/filters: otp-6. Resume: otp-7. Fallback-carrier
  parity vs old gRPC fallback: otp-8. Delegated: otp-9.
- Progress events (w6-1 DaemonEvent stream) attach where old push
  emits them; exact emission points verified in-slice.
- otp-1's reachability pin (UNIMPLEMENTED) is replaced by behavior
  pins — test count must not drop.
