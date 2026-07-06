# otp-4b — TCP data plane onto the unified session

**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-4.
**Contract**: `docs/TRANSFER_SESSION.md` §Transport selection.
**Builds on**: otp-4a (`4b07bbb`+`25f538b`) — daemon serves `Transfer`,
client `run_source`s as SOURCE over the **in-stream** carrier.
**Status**: 4b-1 (single-stream data plane) **CLOSED** — codex loop, 3
passes (`881d412`; fix `e1aafcc` for 2 High; fix `777dfc5` for the race
that fix introduced; confirming re-review PASS). Suite 1509 → **1512/0**.
4b-2 (resize + multi-stream + sf-2 pin) **implemented, in review**; 4b-3
(cancel e2e) queued.

---

## otp-4b-2 (resize + multi-stream + sf-2 pin) — implemented

### What
Port mid-transfer stream growth onto the unified session so the
zero-knowledge single-stream grant shape-corrects upward as the need
list accumulates (the sf-2 mechanism), over real data-plane sockets.
No proto change — consumes the frames otp-1 froze (`DataPlaneResize`
16, `DataPlaneResizeAck` 17).

### Approach
- **SOURCE owns the live dial** (`TransferDial::conservative_within(
  receiver_capacity)`, seeded to the granted epoch-0 count). As needs
  accumulate it re-runs the shape table
  (`initial_stream_proposal(needed_bytes, needed_count, ceiling)`) and
  calls `propose_shape_resize` — one ADD per epoch, one in flight. The
  driver mints a 16-byte sub-token, sends `DataPlaneResize{ADD}` on the
  control lane, and on the `DataPlaneResizeAck` dials the epoch-N socket
  (`session_token ‖ sub_token`) and hands it to the running elastic
  pipeline via `SinkControl::Add`. `resize_settled` advances the live
  count. (`transfer_session/data_plane.rs`: `SourceDataPlane` +
  `dial_source_data_plane` now build the dial and an
  `execute_sink_pipeline_elastic` with a `SinkControl` channel;
  `mod.rs`: `source_send_half` accumulates `needed_bytes/count`,
  `maybe_propose_resize`, `process_source_event` handles `ResizeAck`,
  `resolve_in_flight_resize` drains the last proposal before finish.)
- **DESTINATION** runs a resize-aware accept loop
  (`ResponderDataPlane::spawn` → `accept_loop`): accepts epoch-0, then a
  `select!` that arms resize credentials (an `mpsc` fed by the control
  loop), accepts one socket per arm (authenticating `session_token ‖
  sub_token`), and joins receive workers. The control loop
  (`destination_session`) handles `Frame::Resize`: ceiling-checks, arms,
  bumps `resize_live`, and replies `DataPlaneResizeAck`. At `SourceDone`
  it `finish()`es the run (drops the arm sender = "no more"), joining the
  loop for the settled stream count, surfaced on
  `DestinationOutcome.data_plane_streams`.
- **Orphan-free termination**: a source resize-dial failure is FATAL
  (the session faults and AbortOnDrop kills the dest accept loop), and
  the source drains its one in-flight proposal before finishing, so a
  dest armed slot is always consumed — the accept loop never waits on a
  socket that will not arrive. (Trade vs old push's non-fatal arm-TTL
  recovery — see Known gaps.)

### Bug caught in self-test (pre-commit)
The dest accept loop busy-spun once `arm_tx` dropped: a closed `mpsc`
resolves `recv()` to `None` instantly every poll, and as the biased-first
select arm it starved `join_next`, so finished receive workers were never
collected and `finish()` hung (reproduced on the gRPC data-plane e2e).
Fixed by parking the arm branch on `pending()` once the channel closes
(the same guard `execute_sink_pipeline_elastic` uses for its control_rx).

### Files
- `crates/blit-core/src/transfer_session/data_plane.rs` — dial-owning
  `SourceDataPlane` (propose/add_stream/dial); `ResponderDataPlaneRun` +
  `accept_loop` (select-driven, arm channel); `ReceiveTotals`;
  `accept_raw`/`authenticate_resize`/`spawn_receive` helpers.
- `crates/blit-core/src/transfer_session/mod.rs` — `SourceEvent::ResizeAck`;
  `source_recv_half` forwards it; `source_send_half` shape-correction +
  in-flight drain; `destination_session` `Frame::Resize` arm +
  `resize_live`/ceiling + `finish()`; `DestinationOutcome.data_plane_streams`.
- `crates/blit-core/tests/transfer_session_roles.rs` — the sf-2 pin.

### Tests
- `many_tiny_files_shape_correct_to_more_than_one_stream` (role suite):
  10k tiny files over the TCP data plane settle `data_plane_streams > 1`.
  **Guard proof**: neutering `maybe_propose_resize` settles at 1 and the
  pin fails ("settled at 1"); restored → passes.

### Known gaps (carried / new)
- Mid-transfer cancel e2e → otp-4b-3.
- Cheap-dial live tuner (chunk/prefetch growth) still deferred; otp-4b-2
  moves only the stream count.
- Resize-dial failure is fatal (vs old push's arm-TTL non-fatal recovery)
  — deliberate simplification; a same-build LAN/loopback epoch-N dial to
  an already-accepting listener essentially never fails, and fatal
  fail-fast keeps the dest accept loop orphan-free with no TTL reaper.
- Progress-byte threading still deferred (session rows report
  `bytes_completed=0`, as today's push rows).

## Goal (this slice)

Port the TCP data plane onto the unified session so a client push rides
real data-plane sockets (not the in-stream gRPC carrier), byte-identical
to old push, with the sf-2 shape-correction resize as the one and only
stream-growth policy. The wire contract is already frozen at otp-1
(`DataPlaneGrant` in `SessionAccept`, frames 16/17); this slice only
*consumes* it — no proto change.

## Key architectural facts (established by tracing the old push path)

- The reusable **byte plumbing** all lives in `blit-core` and is the
  plan's "kept" engine: `DataPlaneSession` (record framing, double
  buffering, StallGuard — `remote/transfer/data_plane.rs`),
  `socket::dial_data_plane`, `execute_sink_pipeline_elastic` +
  `SinkControl::{Add,RetireOne}` and `execute_receive_pipeline`
  (`remote/transfer/pipeline.rs`), `DataPlaneSink` (`sink.rs`),
  `TransferDial::{conservative_within,propose_shape_resize,resize_settled,
  live_streams,ceiling_max_streams}`, `initial_stream_proposal`,
  `local_receiver_capacity`, `generate_sub_token` (16 bytes).
- The **orchestration** (daemon bind/arm/accept loop; client
  multi-stream send + resize driver) is push-specific code in
  `blit-daemon/src/service/push/` and `blit-core/src/remote/push/client/`
  — the per-direction drivers ONE_TRANSFER_PATH deletes at otp-10. The
  session therefore grows its **own** orchestration in `transfer_session/`,
  reusing the blit-core primitives above. Nothing here calls into
  `remote::push` or the daemon push service.
- **Streaming consequence**: the responder issues the grant inside
  `SessionAccept` — *before* it has seen a single manifest entry. So
  `initial_streams` is always the zero-knowledge floor
  (`initial_stream_proposal(0,0,ceiling) == 1`). The session data plane
  **always starts single-stream and grows only via SOURCE-driven resize**
  (sf-2). This is why multi-stream lives entirely in 4b-2, not 4b-1.
- **Token sizes (new contract, `docs/TRANSFER_SESSION.md` §Transport)**:
  `session_token` = 16 bytes, `epoch0_sub_token` = 16 bytes; an epoch-0
  socket opens with `session_token ‖ epoch0_sub_token` (32 bytes), a
  resize-ADD socket with `session_token ‖ resize.sub_token`. (Old push
  used a 32-byte session token; the session uses 16 per the otp-1
  contract. Both minted by `generate_sub_token`.)

## Staging (each sub-slice is one commit through the codex loop)

- **otp-4b-1 (single-stream data plane)** — *this commit*. Responder
  (DESTINATION) binds a listener, mints tokens, grants
  `initial_streams = 1` in `SessionAccept`; SOURCE reads the grant,
  dials one socket (`session_token ‖ epoch0_sub_token`), and sends every
  payload over it via a `DataPlaneSink`; DESTINATION accepts the socket
  and drains it with `execute_receive_pipeline` into the same
  `FsTransferSink` the control loop already builds. No resize. Fallback
  to the in-stream carrier when the responder cannot bind or the
  initiator set `in_stream_bytes`. A/B parity vs old push **over the
  data plane**.
- **otp-4b-2 (resize + multi-stream + sf-2 pin)** — SOURCE drives
  `TransferDial::propose_shape_resize` as the need list accumulates:
  emits `DataPlaneResize{ADD, epoch, target, sub_token}` (frame 16) on
  the control stream; DESTINATION arms a new accept slot and replies
  `DataPlaneResizeAck` (frame 17); SOURCE dials the epoch-N socket and
  hands its sink to the running elastic pipeline (`SinkControl::Add`).
  Port the sf-2 10k-file `>1-stream` pin onto the session (assert the
  session's settled `live_streams() > 1`).
- **otp-4b-3 (mid-transfer cancel e2e)** — deterministic test that fires
  `CancelJob` while bytes flow over the data plane and asserts the client
  surfaces `SessionFault{CANCELLED}` and the daemon tears down cleanly.

## otp-4b-1 design

**Responder (DESTINATION) side — `run_destination` / `establish`:**
- Before sending `SessionAccept`, if the initiator did not request
  `in_stream_bytes`, the responder prepares a data plane: bind
  `TcpListener` on `0.0.0.0:0`, mint `session_token` + `epoch0_sub_token`
  (16 bytes each), compute `initial_streams = 1`, and put the resulting
  `DataPlaneGrant{tcp_port, session_token, initial_streams,
  epoch0_sub_token}` in the accept. A bind failure logs and falls back to
  a grant-less accept (in-stream). `establish` returns the bound listener
  + tokens to `run_destination` via `Negotiated` so the accept loop can
  run after the handshake.
- After establish, `destination_session` runs the control loop
  (manifest→needs→SourceDone→summary) *concurrently* with a data-plane
  accept task: accept exactly `initial_streams` socket(s) under the
  shared bounded-accept timeout, verify `session_token ‖ epoch0_sub_token`,
  then `execute_receive_pipeline(&mut socket, sink.clone(), None)` per
  socket into the shared `FsTransferSink`. Payload records no longer
  arrive on the control stream in data-plane mode; a `file_begin`/
  `tar_shard_header` on the control lane there is a PROTOCOL_VIOLATION
  (the in-stream grammar is the fallback carrier only). The DESTINATION
  tallies files/bytes from the receive pipeline outcome(s), waits for
  `SourceDone` + all receive tasks, then sends `TransferSummary`
  (`in_stream_carrier_used = false`).

**Initiator (SOURCE) side — `run_source` / `source_send_half`:**
- After establish, inspect `negotiated.accept.data_plane`. If present,
  the payload phase dials one socket via `DataPlaneSession::connect`
  (handshake `session_token ‖ epoch0_sub_token`), wraps it in a
  `DataPlaneSink`, and feeds planned `TransferPayload`s (from
  `diff_planner::plan_push_payloads`) into `execute_sink_pipeline_streaming`
  (single sink) instead of `send_payload_records`. On NeedComplete +
  all needs flushed, `finish()` the sink (writes the END record) and send
  `SourceDone` on the control stream. The manifest/need/summary
  choreography on the control stream is unchanged from otp-4a.
- If `data_plane` is absent, the in-stream path from otp-4a runs verbatim
  (fallback carrier).

**Why this is byte-identical to old push**: the record framing, the
double-buffered send/receive, and the `FsTransferSink` write path are the
exact same blit-core code old push uses; only the choreography around
them is the unified session's. The A/B parity test proves it.

## Files (planned, 4b-1)
- `crates/blit-core/src/transfer_session/mod.rs` — grant prep on the
  Responder, data-plane accept loop on DESTINATION, data-plane send on
  SOURCE; `Negotiated` carries the responder listener/tokens.
- `crates/blit-core/src/transfer_session/data_plane.rs` (new) — the
  session-side data-plane orchestration helpers (accept+auth,
  socket→sink send), reusing the blit-core primitives.
- `crates/blit-daemon/src/service/transfer_session_e2e.rs` — data-plane
  parity + lands-bytes tests (drop `in_stream_bytes`).
- `crates/blit-core/src/remote/transfer/session_client.rs` — the client
  entry stops forcing `in_stream_bytes` (or gains an option).

## Files (4b-1, as implemented)
- `crates/blit-core/src/transfer_session/data_plane.rs` (new) — the
  session-side data-plane orchestration: `prepare_responder_data_plane`
  (bind + mint tokens + grant), `ResponderDataPlane::{grant,
  accept_and_receive}`, `accept_authenticated`, `dial_source_data_plane`,
  `SourceDataPlane::{queue, finish}`. Reuses the blit-core primitives;
  no call into `remote::push` or the daemon push service.
- `crates/blit-core/src/transfer_session/mod.rs` — `mod data_plane`;
  `Negotiated` carries the responder listener/tokens; `establish`
  Responder branch prepares + grants the data plane (DESTINATION, unless
  `in_stream_bytes` or bind fails); `source_send_half` dials up front and
  queues planned payloads to the data plane; `destination_session` (now
  by-value) arms the accept+receive task, treats control-lane payload
  frames as violations under a data plane, and joins the receive task at
  `SourceDone` for the authoritative counts (completeness = files
  received == need-list size).
- `crates/blit-core/src/remote/transfer/session_client.rs` —
  `PushSessionOptions.in_stream_bytes` (default `false` = data plane);
  threads `data_plane_host`.
- `crates/blit-daemon/src/service/transfer_session_e2e.rs` — data-plane
  parity + in-stream fallback tests.
- `crates/blit-core/tests/transfer_session_roles.rs` — `data_plane_host:
  None` on the in-process configs (they ride the in-stream carrier).

## Tests (4b-1)
Suite 1509 → **1511** (+2: `session_lands_bytes_over_in_stream_carrier`
e2e + `responder_grant_is_single_stream_with_16_byte_tokens` unit; the
old `session_lands_bytes_and_scores_them` became
`session_lands_bytes_over_the_data_plane`). New/changed:
- `session_lands_bytes_over_the_data_plane` — default rides the TCP data
  plane (`!in_stream_carrier_used`), byte-identical trees + counts.
- `old_push_and_session_produce_identical_trees_and_counts` — **A/B
  parity over the data plane**: old push and the session (both data
  plane) yield byte-identical trees + equal shared counters.
- `session_lands_bytes_over_in_stream_carrier` — the in-stream fallback
  still lands bytes and reports `in_stream_carrier_used`.
- `responder_grant_is_single_stream_with_16_byte_tokens` — grant shape.

Gate: `cargo fmt --check` ✓, `clippy --workspace --all-targets
-D warnings` ✓, `cargo test --workspace` **1511/0** ✓.

## Guard proof (4b-1)
`session_lands_bytes_over_the_data_plane` asserts
`summary.in_stream_carrier_used == false` + byte-identical trees.
**Proven**: forcing `prepare_responder_data_plane` to return `None`
(grant-less accept ⇒ in-stream fallback) flips the flag and fails the
assertion (`otp-4b default rides the TCP data plane, not the in-stream
carrier`); restored, the suite is green. A/B parity vs old push guards
the byte identity of the data-plane path.

## Known gaps (carried)
- Resize / multi-stream / sf-2 pin → otp-4b-2.
- Mid-transfer cancel e2e → otp-4b-3.
- Progress-byte threading (`with_byte_progress`) still deferred (session
  rows report `bytes_completed=0`, as today's push rows).

## Reviewer comments
codex (gpt-5.5) — 3 passes, all findings adjudicated in
`.review/results/otp-4b1-data-plane.gpt-verdict.md`:
- pass 1 (`881d412`): F1 weak count-proxy completion + F2 missing
  read-side StallGuard — both Accepted, fixed in `e1aafcc`.
- pass 2 (fix `e1aafcc`): a real dedup/claim race from conflating dedup
  and completion in one set — Accepted, fixed in `777dfc5` (two-set
  split: local monotonic `granted` + shared `outstanding`).
- pass 3 (`777dfc5`): PASS, no findings.
