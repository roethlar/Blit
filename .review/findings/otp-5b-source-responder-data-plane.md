# otp-5b — SOURCE-responder data plane (transport/role decoupling)

**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-5.
**Contract**: `docs/TRANSFER_SESSION.md`.
**Builds on**: otp-4b (the TCP data plane: DESTINATION responder binds+grants+
accepts+receives; SOURCE initiator dials+sends) and otp-5a (the daemon serves
BOTH roles by declared initiator role, pull-equivalent in-stream only).

## Staging (mirrors otp-4b)

otp-5 is staged like otp-4b. **This slice is otp-5b-1**: the single-stream
SOURCE-responder data plane, no resize. otp-5b-2 adds the mid-transfer
shape-correction resize for the pull direction (the accept-based epoch-N
socket); otp-5b-3, if needed, mirrors otp-4b-3's mid-transfer cancel over the
pull data plane.

## The decoupling this slice makes

Today (otp-4b) the data plane is keyed to **role**:

- the **DESTINATION** binds a listener, grants it in `SessionAccept`, accepts
  sockets, and **receives** bytes (`ResponderDataPlane`);
- the **SOURCE** dials the granted sockets and **sends** bytes
  (`SourceDataPlane`).

That coincidence holds only for push, where the DESTINATION *is* the responder
and the SOURCE *is* the initiator. The plan's transport rule (§Design,
§Transport facts) is two independent axes:

- **connection role** — the RESPONDER binds+accepts, the INITIATOR dials (NAT
  reality: the connection-initiating end always dials);
- **byte role** — the SOURCE sends, the DESTINATION receives.

For pull the client is the DESTINATION *initiator* (must dial+receive) and the
daemon is the SOURCE *responder* (must bind+accept+send). This slice adds those
two new combinations without disturbing the push pair:

| byte role \ conn role | initiator (dial)                    | responder (bind+accept)          |
|-----------------------|-------------------------------------|----------------------------------|
| SOURCE (send)         | push: `dial_source_data_plane` ✓    | **pull (new): accept + send**    |
| DESTINATION (receive) | **pull (new): dial + receive**      | push: `ResponderDataPlane` ✓     |

The byte machinery is fully reused — send is `DataPlaneSession` +
`DataPlaneSink` + `execute_sink_pipeline_elastic`; receive is `StallGuard` +
`execute_receive_pipeline`. Only socket **acquisition** (dial vs accept) is new
per byte role, and `DataPlaneSession::from_stream` already builds a send session
from an accepted socket (the old `pull_sync` path uses it).

## Scope of otp-5b-1: single stream, no resize

The pull data plane runs at **exactly the epoch-0 grant (1 stream)**. No
`DataPlaneResize` is proposed by the SOURCE responder and none is handled by the
DESTINATION initiator. Mechanically this is enforced by capping the SOURCE
responder's send dial to `max_streams = 1`, so `propose_resize` returns `None`
and no resize frame is ever emitted — the same suppression otp-4b-1 relied on
before otp-4b-2 lifted it. The DESTINATION initiator treats a `DataPlaneResize`
frame as a protocol violation (there is none in this slice). otp-5b-2 lifts the
cap and adds the accept-based epoch-N socket + the ack→dial choreography.

Resize choreography note (for otp-5b-2, not implemented here): the control-lane
frames are identical in both directions — the SOURCE proposes `Resize{ADD}`, the
DESTINATION acks. Only the transport action flips: in push the SOURCE=initiator
dials the epoch-N socket and the DESTINATION=responder arms+accepts; in pull the
SOURCE=responder arms+accepts and the DESTINATION=initiator dials.

## Approach (as implemented)

- **`responder_finish` binds for either role** (`transfer_session/mod.rs`): the
  `local_role == Destination` gate on the data-plane bind is removed; a
  responder binds a data plane whenever `!open.in_stream_bytes`, regardless of
  role. The bound listener + grant travel in `Negotiated.responder_data_plane`;
  the grant goes out in `SessionAccept`. `receiver_capacity` in the accept stays
  DESTINATION-only (the byte RECEIVER advertises capacity; a DESTINATION
  initiator advertises it in its own `SessionOpen.receiver_capacity`, already so
  since otp-4a).
- **SOURCE responder accept+send** (`transfer_session/data_plane.rs`):
  `accept_source_data_plane(bound, receiver_capacity, source)` accepts the
  epoch-0 socket(s) off the bound listener, wraps each in
  `DataPlaneSession::from_stream` → `DataPlaneSink`, and drives the SAME elastic
  send pipeline `dial_source_data_plane` builds — returning the same
  `SourceDataPlane` handle. Its dial is capped to a single stream (no resize).
  `source_send_half` picks accept-vs-dial by whether it holds a bound responder
  listener (`responder_data_plane`) or a received grant (`accept.data_plane`);
  everything after socket acquisition (`queue`/`finish`) is unchanged.
- **DESTINATION initiator dial+receive** (`transfer_session/data_plane.rs`):
  `dial_destination_data_plane(host, grant)` dials the epoch-0 socket(s), spawns
  one `execute_receive_pipeline` worker per socket into a `JoinSet`, and
  `finish()` joins them for the `ReceiveTotals` (settled stream count + write
  outcome) the sf pin reads. `destination_session` selects it when it holds no
  bound listener but a received grant + a `data_plane_host`.
- **Config threading**: `DestinationSessionConfig` gains `data_plane_host:
  Option<String>` (the initiator dials the responder's host, same host it
  reached the control plane on — symmetric with `SourceSessionConfig`).
  `drive_destination`/`destination_session` take it.
- **Client** (`session_client.rs`): `run_pull_session` sets
  `in_stream_bytes = options.in_stream_bytes` (default `false` = TCP data plane)
  and passes `data_plane_host: Some(endpoint.host)`. A `PullSessionOptions
  { in_stream_bytes }` knob keeps the in-stream fallback reachable (diagnostics),
  matching `PushSessionOptions`.
- **Daemon** (`service/transfer.rs`): unchanged — `run_responder` already routes
  the SOURCE-responder path; the bound listener now flows through it.

## Compare semantics

Unchanged from otp-5a: the DESTINATION is the one diff owner; same-size +
dest-NEWER resolves to the data-safe SKIP (the still-open owner-ack question, not
reopened here). A/B vs old `pull_sync` stays byte-identical with no caveat.

## Files

- `crates/blit-core/src/transfer_session/mod.rs` — `responder_finish` bind gate;
  `drive_source`/`source_send_half` accept-vs-dial selection; `drive_destination`/
  `destination_session` dial-vs-accept receive + `data_plane_host`.
- `crates/blit-core/src/transfer_session/data_plane.rs` —
  `accept_source_data_plane`, `dial_destination_data_plane` (+ its run handle).
- `crates/blit-core/src/remote/transfer/session_client.rs` — `run_pull_session`
  default carrier + `data_plane_host`; `PullSessionOptions.in_stream_bytes`.
- `crates/blit-core/tests/transfer_session_roles.rs` — pull data-plane
  single-stream invariance test.
- `crates/blit-daemon/src/service/transfer_session_e2e.rs` — pull over the data
  plane (byte-identical, `!in_stream_carrier_used`, `data_plane_streams == 1`).

## Tests

- Roles suite: a DESTINATION-initiator / SOURCE-responder session over a real
  loopback TCP data plane (control frames on the in-process pair, as the otp-4b
  push data-plane test does) lands byte-identically and reports
  `data_plane_streams == Some(1)`, `!in_stream_carrier_used`.
- e2e (real daemon as SOURCE responder):
  - `pull_session_lands_bytes_over_the_data_plane` — default carrier is TCP;
    byte-identical dest; `!in_stream_carrier_used`.
  - `pull_session_lands_bytes_over_in_stream_carrier` — the `in_stream_bytes`
    fallback still lands byte-identically (otp-5a path stays live).
  - the existing `old_pull_and_session_produce_identical_trees_and_counts` A/B
    now runs the NEW arm over the data plane (converge-up bar).

Guard proof: forcing the SOURCE responder to grant no data plane (or forcing the
DESTINATION initiator onto the in-stream branch) makes
`pull_session_lands_bytes_over_the_data_plane` fail its `!in_stream_carrier_used`
assertion; restoring passes. The A/B byte-identity guards correctness.

## Known gaps (carried into otp-5b-2 / later)

- **Multi-stream / resize on the pull data plane**: otp-5b-2 (accept-based
  epoch-N socket; SOURCE responder proposes, DESTINATION initiator dials+acks).
  This slice is single-stream by the dial cap.
- **Mid-transfer cancel over the pull data plane**: the otp-4b-3 analog for pull
  (otp-5b-3 if a distinct guard is warranted; the control-lane CANCELLED framing
  is already role-agnostic).
- Mirror/filters otp-6; resume otp-7; fallback-carrier otp-8; delegated otp-9;
  cutover/deletion otp-10.
</content>
</invoke>
