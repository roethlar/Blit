# ue-r2-2: stream resize — mid-transfer add/drop from live telemetry

**Slice**: ue-r2-2 — ninth and FINAL slice of
`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`
**Status**: Coded, pending GPT review
**Branch**: master (no agent branches — AGENTS.md §8)
**Commits**: `042ca4b` (engine policy + growable tuner), `ce0e396`
(elastic pipeline), `04c9c6d` (push path), `b08d80e` (pull path +
delegated), plus a fmt-only sweep and this doc's records commit.

## What

Finish the negotiated `DataPlaneResize`/`DataPlaneResizeAck` contract
(wire landed dormant at 1b) and add/drop streams mid-transfer from
live telemetry, on the elastic work-stealing queue from 1a and the
mutable dial from 1e — REV4's "a wire-up, not a restructuring"
criterion. After this slice every piece of the 1b contract is live:
capability bits advertised, `resize_enabled` folded
daemon-authoritatively, `epoch0_sub_token` echoed on epoch-0 sockets,
per-epoch `sub_token`s registered-before-dial, acks settled with what
actually happened.

## Interpretation of scope (stated for review)

- **In**: mid-transfer stream ADD/REMOVE on the two live-TCP full-file
  paths — push (client = sender/controller/dialer, daemon = armed
  acceptor) and pull_sync (daemon = sender/controller, client =
  dialer/acker). Delegated pull inherits the client side whole through
  `pull_sync_with_spec` (dst_capabilities flips the bit).
- **Out, per existing contracts**: resume stays single-stream (the 1g
  RELIABLE exception — its negotiation never sets `resize_enabled`);
  gRPC fallback is unresizable (the proto's own gate: `tcp_fallback`
  → false); the relay's metadata/single-file sessions are force_grpc
  by construction.
- **Out, judged beyond "wire-up"**: pull's 1s-start. The 1g Known gap
  said pull 1s-start "rides on ue-r2-2" because starting early needs
  mid-transfer add — the MECHANISM now exists, but actually starting
  the pull data plane before enumeration completes means moving the
  negotiation ahead of the manifest compare, a control-flow
  restructuring of the PullSync protocol that REV4's slice text
  ("wires the resize proto onto this; it does not restructure")
  excludes. Recorded as residue for the post-REV4 queue, not silently
  skipped.
- **Perf validation**: REV4 names the 10 GbE benchmark as the
  sign-off measure for stream resize. This slice ships the mechanism
  plus correctness/behavior tests; whether the policy's thresholds
  earn their keep under real load is measured there, deliberately.

## Design

- **Engine-owned policy** (`engine/dial.rs`): streams are the
  EXPENSIVE dial. `resize_tick` proposes ±1 stream (one per epoch —
  the wire carries one `sub_token` per ADD) only when the cheap dials
  are already pinned at their ceiling (ADD) or floor (REMOVE), the
  signal held `RESIZE_SUSTAIN_TICKS`(2) consecutive busy ticks, and
  `RESIZE_COOLDOWN_TICKS`(4) passed since the last settle; never while
  an epoch is in flight; bounded `1..=ceiling_max_streams` (the
  receiver's `CapacityProfile.max_streams` folded at construction —
  the proto's authority). `resize_settled(epoch, effective, accepted)`
  records what ACTUALLY happened: refusals and failed dials keep the
  live count; stale epochs are ignored. The tuner samples a growable
  `SharedStreamProbes` registry (a shrinking sum re-baselines as
  no-signal) and forwards proposals to the adapter that owns the
  control stream — the engine never touches wire types.
- **Elastic pipeline** (`pipeline.rs`): `SinkControl::Add` spawns a
  worker on the shared flume queue at any time (post-EOS adds just
  emit their END); `RetireOne` signals exactly one worker via a
  per-worker watch raced (biased) against `recv_async` — it drains at
  the payload boundary even on an idle queue, `finish()` emits its
  per-stream END record, and survivors steal the queued payloads
  (dequeue = ownership ⇒ exactly-once). Floored at one live worker:
  zero workers would make the forwarder's send-failure path silently
  drop the payload stream's tail. The receiving end of a retired
  stream terminates through its normal END handling — REMOVE needs no
  receiver-side protocol at all.
- **Credential model**: every resize-enabled socket handshake is
  `one_time_token ‖ sub_token` (48 bytes). Epoch-0 sockets echo the
  negotiation's `epoch0_sub_token`; each ADD epoch mints a fresh
  16-byte `sub_token` (client-side on push, daemon-side on pull),
  registered with the accepting side BEFORE the dialer dials. Sockets
  are identified by WHICH credential they echo, not arrival order —
  this dissolves the epoch-0/epoch-N accept race outright.
- **Armed-only accepts (the W1 collision, resolved)**: the 1g
  sequential-accept deferral to W1 was premised on the accept phase
  being bounded and pre-transfer. Resize keeps the listener alive for
  the whole transfer, but the daemon only ever *accepts* while a live
  armed slot exists (epoch-0's N, plus one TTL-bounded slot per acked
  ADD) — unarmed stray dials rot in the OS backlog and are never
  serviced. Every accept remains credential-gated and 30s/15s
  timeout-bounded, so the W1 premise survives in spirit; the row still
  owns the constants/policy consolidation.
- **Failure posture** (the map critic's G3): epoch-0 accept/handshake
  failures stay transfer-fatal exactly as today. Everything about an
  OPTIONAL epoch-N ADD is non-fatal: a stray/hostile socket is dropped
  without consuming the armed slot (pull) or with the registry entry
  intact (push), an expired slot lapses with a warning, a failed
  client dial logs and settles the epoch at the current width. Once a
  socket is authorized and live, its errors are fatal on any epoch —
  a live stream dying mid-transfer is data loss.
- **Cancellation** (critic's G1): client-side, the receiver task's
  fixed AbortOnDrop worker set became a JoinSet — the same
  abort-on-drop cascade (R32-F2), now covering resize-added workers,
  so delegated pulls dropped by CancelJob still tear everything down.
  Push-daemon-side the resizable acceptor's JoinSet aborts sibling
  receive workers on first error — strictly better than the fixed
  path's detach (design-2 shape; w4-1 still owns the family).

## Files changed

- `crates/blit-core/src/engine/dial.rs` (+`engine/mod.rs` exports) —
  resize state/policy/settlement; `spawn_dial_tuner_with_resize`;
  `SharedStreamProbes`; `spawn_dial_tuner` is now a compat wrapper.
- `crates/blit-core/src/remote/transfer/pipeline.rs` (+`mod.rs`) —
  `execute_sink_pipeline_elastic` + `SinkControl`; streaming fn is a
  wrapper; JoinSet supervisor.
- `crates/blit-core/src/remote/transfer/data_plane.rs` —
  `SUB_TOKEN_LEN`, `generate_sub_token` (blit-core gains `rand 0.10`,
  already locked via blit-daemon).
- `crates/blit-core/src/remote/push/client/mod.rs` — header bit;
  negotiation consumption; elastic `MultiStreamSender`
  (`ResizeRuntime`, `add_stream`/`retire_stream`/`take_resize_rx`);
  proposal + ack arms in the select loop.
- `crates/blit-daemon/src/service/push/{control.rs,data_plane.rs}` —
  capability capture; fold + epoch-0 token at both TCP negotiation
  literals; `accept_data_connection_stream_resizable` (armed
  registry, 48-byte handshakes, non-fatal epoch-N failures);
  `handle_resize_request` acks after arming; the transfer phase now
  services the request stream concurrently (a select over the data
  plane and `stream.message()`).
- `crates/blit-daemon/src/service/pull_sync.rs` — dial Arc'd;
  capability fold; probes + 48-byte handshake in
  `accept_and_wrap_sinks` (pool hoisted to caller; timeouts hoisted to
  module scope); `stream_via_data_plane` grows the resize controller
  (proposals → command frames; acks read from the request stream —
  the sole mid-transfer reader on this path; single armed accept slot
  with TTL; `accept_one_resize_socket`).
- `crates/blit-core/src/remote/pull.rs` — caps bit; negotiation
  consumption + growth channel; growable JoinSet receiver
  (connect/receive split so an added stream's dial failure is
  non-fatal); command arm clamps via `bounded_stream_count` and acks;
  refusal posture preserved for non-negotiated sessions.
- `crates/blit-daemon/src/service/delegated_pull.rs` —
  `dst_capabilities` advertises resize; R25-F2 override test updated
  (dst now asserts its true bit in both directions).
- `crates/blit-core/tests/pull_sync_with_spec_wire.rs` — canned server
  captures resize acks; 2 new wire tests.

## Tests

Entering baseline (Windows host): 1393 / 0 / 3. +12 new tests:

- dial policy (6): escalation ladder gating (cheap dials first),
  cooldown + sustain + pending-blocks semantics with exact epochs,
  REMOVE floor at 1, sustain resets on idle/in-band ticks,
  refusal/stale-settle handling, ceiling clamp, and the tuner
  forwarding a real proposal over the shared registry under paused
  time.
- elastic pipeline (4): gated add-mid-run provably processes the
  queued payload; retire mid-stream keeps exactly-once with both END
  emissions; retire floor holds at one worker; add-after-drain
  finishes the late sink cleanly.
- pull resize wire (2): a command on a session that never negotiated
  resize is acked `accepted:false` and the transfer completes; a full
  ADD against a real listener verifies the epoch-0 AND epoch-1
  handshake bytes (`token ‖ sub`), the accepted ack, and clean
  completion.

Everything pre-existing stays green — notably every push/pull e2e now
runs with resize NEGOTIATED (new client + new daemon), so the 48-byte
handshake, the folds, and the concurrent control-stream servicing are
exercised by the whole existing suite; the loopback workloads never
saturate long enough to trigger a proposal (by design of the policy).

## Known gaps

- **No end-to-end telemetry-triggered resize test.** Triggering the
  policy for real needs sustained saturation with cheap dials pinned —
  loopback e2e can't do it deterministically, and the repo's
  owner-directed env-var purge rules out test-only env switches. The
  policy is unit-tested at tick granularity, the mechanism is
  wire/behavior-tested at both ends; the composed behavior under real
  load is exactly what REV4 defers to the 10 GbE sign-off.
- **Pull 1s-start residue** (see Interpretation): the mechanism
  exists; moving pull's negotiation ahead of enumeration is a
  restructuring for the post-REV4 queue.
- **Daemon-side controller paths partially exercised**: the pull
  controller's ADD accept and the push acceptor's armed path are
  driven by the wire tests from the CLIENT side and by unit-level
  construction, not by a daemon-initiated e2e (same saturation
  problem). The refusal/ack plumbing is total by construction
  (`handle_resize_request`, command arm) and covered for the client.
- **BufferPool stays epoch-0-sized** on both senders; ADDed streams
  share it through a FIFO-fair semaphore (bounded throughput, not
  starvation). Growing the pool live belongs to W3.1
  (memory-aware pool), noted there.
- **REMOVE on push is accounting-only trust**: the daemon acks and
  waits for the retired stream's END; a client that lies about
  retiring changes nothing (its own worker keeps running — no daemon
  resource depends on the count).
- **Narrow teardown race**: if a transfer completes while an ADD is
  armed-but-undialed, the acceptor returns, the listener drops, and
  the late dial fails — non-fatal on both ends (client logs + settles;
  daemon slot dies with the task). Documented rather than locked out.
- `StreamState`/`generation` telemetry fields remain unused (the
  registry add/remove model made the draining-state filter
  unnecessary); they stay for a future controller that samples
  per-stream utilization.
