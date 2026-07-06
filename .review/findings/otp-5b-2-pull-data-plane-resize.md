# otp-5b-2 — pull data-plane resize (accept-based epoch-N socket)

**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-5.
**Contract**: `docs/TRANSFER_SESSION.md`.
**Builds on**: otp-4b-2 (the push data-plane resize: SOURCE initiator dials the
epoch-N socket, DESTINATION responder arms+accepts it) and otp-5b-1 (the
single-stream SOURCE-responder / DESTINATION-initiator pull data plane).

## Staging

otp-5 is staged like otp-4b. otp-5b-1 landed the single-stream pull data plane;
**this slice is otp-5b-2**: the mid-transfer sf-2 shape-correction resize for the
pull direction. otp-5b-3, if warranted, mirrors otp-4b-3's mid-transfer cancel
over the pull data plane (the control-lane CANCELLED framing is already
role-agnostic, so a distinct guard may not be needed).

## What

otp-5b-1 capped the pull data plane at the epoch-0 grant (1 stream): the SOURCE
responder's send plane was `resizable: false`, so `propose_resize` always
returned `None`, and the DESTINATION initiator treated any `DataPlaneResize`
frame as a protocol violation. A large pull (10k tiny files) therefore rode a
single stream where the equivalent push corrects upward — a converge-up
violation (plan §Constraints). This slice lifts the cap: the pull data plane
grows mid-transfer exactly as push does, via the same control-lane
`DataPlaneResize{ADD}` / `DataPlaneResizeAck` frames.

## Predicted observable failure (closed by this slice)

A 10k-tiny-file pull over the TCP data plane settles at `data_plane_streams == 1`
(no shape correction), whereas the identical push settles > 1. Pinned by the new
`pull_data_plane_shape_corrects_to_more_than_one_stream` role-suite test
(asserts `streams > 1`).

## Approach

The plan's transport rule is two orthogonal axes — **connection role** (the
RESPONDER binds+accepts, the INITIATOR dials; NAT reality) and **byte role** (the
SOURCE sends, the DESTINATION receives). The resize control-lane choreography is
identical in both directions (SOURCE proposes ADD, DESTINATION acks); only the
transport action flips. So the resize machinery is fully reused and the change is
localized to socket **acquisition** per connection role:

- **SOURCE responder grows by ACCEPT** (`data_plane.rs`). `SourceDataPlane` gained
  a `SourceSockets` enum (`Dial { host, tcp_port }` for the initiator / push;
  `Accept { listener }` for the responder / pull), replacing the `host`/`tcp_port`/
  `resizable` fields. `accept_source_data_plane` now retains its bound listener
  instead of dropping it. `propose_resize` lost the `!resizable` early-return, so
  both directions propose. `add_stream` branches on `SourceSockets`: the initiator
  dials the epoch-N socket (unchanged); the responder accepts the socket the
  DESTINATION dials after its ack (`accept_authenticated` with
  `session_token ‖ sub_token`), then hands it to the running elastic pipeline via
  `SinkControl::Add` — the same downstream path. The accept is bounded and
  unambiguous: at most one resize is in flight (`pending_resize`) and epoch-0 is
  already accepted, so the next connection off the listener is exactly this
  resize's socket (no arm set needed on this end).
- **DESTINATION initiator grows by DIAL** (`data_plane.rs`, `mod.rs`).
  `InitiatorReceivePlaneRun` retained the responder host/port + session token +
  the shared receive sink, and gained `add_dialed_stream(sub_token)`: dial one
  epoch-N socket (`dial_data_plane`, credential `session_token ‖ sub_token`), spawn
  a receive worker into its `JoinSet`, bump the settled stream count. The
  `destination_session` initiator branch now seeds `resize_live` from the epoch-0
  streams and sets `resize_ceiling` to its OWN advertised `max_streams` (it is the
  byte receiver — the same profile the SOURCE responder's dial already clamps to),
  instead of the otp-5b-1 `(0, 0)` that made every resize a violation. The
  `Frame::Resize` handler grew a role branch: the responder path arms
  (`run.arm`), the initiator path dials (`run.add_dialed_stream`). The initiator
  dials BEFORE it acks, so the SOURCE responder — which accepts on the ack — never
  commits to an accept the DESTINATION did not dial.

A dial failure on the DESTINATION initiator is FATAL, matching the SOURCE
initiator's `add_stream` (a same-build peer that established epoch-0 failing an
epoch-N socket is a transport fault worth surfacing). Because the DESTINATION
dials before acking, the fault fires before the SOURCE responder commits to
accepting — no orphaned accept.

## Files changed

- `crates/blit-core/src/transfer_session/data_plane.rs`
  - `SourceSockets` enum; `SourceDataPlane` uses it in place of
    `host`/`tcp_port`/`resizable`.
  - `dial_source_data_plane` / `accept_source_data_plane` construct the
    Dial / Accept variant; the accept path retains the listener.
  - `propose_resize` no longer gates on `resizable`; `add_stream` branches
    dial-vs-accept.
  - `InitiatorReceivePlaneRun` gained `host`/`tcp_port`/`session_token`/`sink`
    and `add_dialed_stream`; `dial_destination_data_plane` populates them.
  - `DestRecvPlane::Initiator` doc updated (resize-capable).
- `crates/blit-core/src/transfer_session/mod.rs`
  - `destination_session`: initiator branch seeds `resize_live` = epoch-0 streams
    and `resize_ceiling` = local `max_streams`.
  - `Frame::Resize` handler: role branch (responder arms, initiator dials).
- `crates/blit-core/tests/transfer_session_roles.rs`
  - new `pull_data_plane_shape_corrects_to_more_than_one_stream`; the single-stream
    test's rationale comment corrected (small tree, not a disabled resize).

## Tests / Guard proof

- `pull_data_plane_shape_corrects_to_more_than_one_stream`: a 10k-tiny-file PULL
  (DESTINATION initiator / SOURCE responder) over a real loopback TCP data plane
  lands byte-identically, rides TCP (`!in_stream_carrier_used`), and settles
  `data_plane_streams > 1`.
- **Guard proof (run)**: forcing the DESTINATION initiator's `resize_ceiling` to 0
  makes the new test fail at `settled at 1` while
  `pull_data_plane_single_stream_lands_bytes` still passes (byte correctness holds
  independent of stream count); restoring passes both.
- `pull_data_plane_single_stream_lands_bytes` still asserts `Some(1)` — a 4-file
  need list stays below the shape threshold, so no resize is proposed.

Suite: 1521 → **1522/0** (2 ignored).

## Known gaps (carried into otp-5b-3 / later)

- **Graceful DESTINATION dial-failure ack**: this slice makes an epoch-N dial
  failure fatal for symmetry with push. Because the DESTINATION dials before
  acking, it *could* instead ack `accepted: false` and let the SOURCE responder
  skip the accept (the SOURCE only accepts on an accepted ack) — a non-fatal
  recovery push cannot offer (its SOURCE dials before any ack). Deferred: keeping
  the two directions' resize semantics identical is simpler; revisit if pull
  transport flakiness warrants it.
- **Mid-transfer cancel over the pull data plane**: otp-5b-3 analog of otp-4b-3
  (the CANCELLED framing is already role-agnostic; a distinct guard may be
  unnecessary).
- Mirror/filters otp-6; resume otp-7; fallback-carrier otp-8; delegated otp-9;
  cutover/deletion otp-10.
