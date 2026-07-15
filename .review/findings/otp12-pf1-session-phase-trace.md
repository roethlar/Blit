# otp12-pf1-session-phase-trace — role-complete TCP P1 timing probe

**Slice**: OTP12 performance-finding pf-1 instrumentation, P1 only.
The active plan requires phase evidence before any topology-changing
counterfactual or performance fix.

## What

The existing hidden `--trace-data-plane` flag is not a usable timing probe.
It reaches the initiating client only and can print per-file output, so a
10,000-file trace can perturb the very wall time being measured. The daemon
responder has no corresponding phase timeline. There was therefore no shared,
low-frequency evidence for whether the P1 gap sits in need emission, planning,
resize arm/dial/accept/ACK choreography, or first-payload start.

## Approach

- Add a separate wire-neutral session phase trace. It is disabled by default
  and enabled on each process with `BLIT_TRACE_SESSION_PHASES=1`; the harness
  supplies a common `BLIT_TRACE_RUN_ID`.
- Correlate both endpoints with the first 16 hexadecimal characters of a
  BLAKE3 digest of the existing random session token. Neither the token nor a
  resize credential is logged.
- Emit compact JSONL records with Unix and local monotonic timestamps,
  per-producer sequence, SOURCE/DESTINATION endpoint role, initiator role,
  resize epoch/socket, batch/count, and accepted/live/target fields where
  relevant. A dedicated writer thread serializes the low-frequency records so
  async transfer workers never block on stderr; the two terminal session paths
  flush the writer before returning. Event vocabulary contains no push/pull
  names.
- Cover epoch-zero and resize socket dial/accept, resize proposal/send/receive,
  destination preparation, ACK send/receive, source settlement, manifest and
  need-batch boundaries, planner entry/exit, first payload queued, first write
  and first payload received on each socket that carries payload, data-plane
  completion, and summary exchange. Analysis uses the earliest per-socket
  payload markers rather than assuming one globally winning socket.
- Keep trace-off behaviorally inert and observer-minimal: it performs no token
  hash, timeline allocation, clock read, writer-thread spawn, or formatting.
  Trace-on first-event probes use locally disarmed booleans; subsequent
  payloads do not take clocks or perform a contended read-modify-write. The
  first-queue stamp is captured only when a trace is bound and before the
  payload is released to a worker.
- Preserve the public receive-pipeline entry point and add a session-only
  traced wrapper so unrelated data-plane callers remain unchanged.
- Carry the resize epoch through the destination responder's existing internal
  arm channel. `resize_arm_queue_begin` is the causal pre-handoff marker,
  `arm_queued` records a successful control-loop handoff, and
  `resize_arm_ready` records the accept loop actually dequeuing the arm before
  accept begins. This changes no frame, credential, connection topology,
  worker policy, or transfer ordering.

## Files

- `crates/blit-core/src/remote/transfer/session_phase.rs`
- `crates/blit-core/src/remote/transfer/data_plane.rs`
- `crates/blit-core/src/remote/transfer/pipeline.rs`
- `crates/blit-core/src/remote/transfer/mod.rs`
- `crates/blit-core/src/transfer_session/data_plane.rs`
- `crates/blit-core/src/transfer_session/mod.rs`
- instrument-construction call sites in `session_client.rs` and `local.rs`
- `crates/blit-core/tests/transfer_session_roles.rs`
- `docs/plan/OTP12_PERF_FINDINGS.md` (name the canonical probe and repair stale
  symbol/line guidance)
- `docs/STATE.md` (record the owner's pf-1 rig-W choice)

## Guard

`session_phase_trace_is_complete_and_inert_under_both_initiators` runs a real
loopback TCP transfer with 256 tiny files, which deterministically reaches two
streams, under all four combinations of initiator role and trace off/on. It
asserts:

- trace off emits no records;
- on/off summaries, need paths, final trees, carrier, and stream count are
  identical;
- each traced transfer has exactly one 16-character lowercase-hex fingerprint,
  the two independent transfers have different fingerprints, and every record
  carries the expected schema/run identifiers;
- endpoint and initiator roles remain separate attributes and each producer's
  sequence is contiguous;
- a fixed inventory covers both roles' epoch-zero acquisition, both roles'
  epoch-one acquisition and trace attachment, all resize choreography, each
  payload-carrying socket's write/receive pair, need/planner batch zero, and
  summary exchange;
- manifest/queue/write/receive, need/planner, epoch-one resize, and summary
  events obey causal partial orders using pre-send markers where the receiving
  endpoint is a different producer; and
- both endpoint data planes complete.

Mutation proofs: removing only the trace attachment from the SOURCE responder's
accepted epoch-zero and resize sockets left all four transfers successful but
made the guard fail because the destination-initiated layout lost
`first_socket_write`. After deterministic attachment markers were added,
separately removing the common SOURCE-sender and DESTINATION-receiver attachment
events again left the transfers successful but failed on the missing role's
epoch-zero/epoch-one attachment inventory. Removing the threaded writer's flush
action made its focused unit guard fail (`0` flushes versus `1`). Restoring each
mutation returned its guard green.

The restored role guard passed 100 consecutive runs after the deterministic
attachment inventory replaced the scheduler-dependent “both sockets carried
payload” assertion.

Full workspace gate: fmt check, strict clippy, and 1,493 passed / 2 ignored
(three new guards over the reviewed 1,490-pass baseline). The complete
`transfer_session_roles` target passed 41/41, and `check-docs` passed.

Production-path smoke: running the existing real-TCP resize test with
`BLIT_TRACE_SESSION_PHASES=1 BLIT_TRACE_RUN_ID=production-env-smoke` and
otherwise-default source/destination instruments passed and emitted prefixed,
parseable schema-1 JSON records for both endpoint roles with the same safe
session fingerprint. The smoke exercises the same default-instrument paths the
client and daemon responder use without adding process-global environment
mutation to the parallel Rust test suite.

The unit guard also drives the factored production environment-to-writer path,
waits on its real threaded flush barrier, and parses the resulting prefixed JSON.

## Known gaps

- This is the P1 probe surface. The active plan's P2 investigation still needs
  the separately reviewable high-volume per-member claim/sink/shard timings;
  adding those here would make the focused P1 observer materially noisier.
- The trace is intentionally TCP-session-only. A forced-gRPC run remains an
  external timing control, not a phase-trace input.
- Environment activation is process-wide. A rig pair is trace-off only after
  both exact daemons have been restarted without the flag; the harness must
  restart both endpoints whenever it switches trace state.
- The trace-off data path retains one predictable optional-hook branch per
  record/payload. It performs none of the probe work above and is identical in
  both role layouts, but is not literally zero instructions; final performance
  claims come from the full measured build, not an assertion of zero observer
  footprint.
- No rig result is claimed by this code slice. The q ↔ netwatch-01 run must use
  instrumentation-on/off pairs and register its paired within-session floor
  before interpreting a recovery. This instrumentation slice alone does not
  satisfy pf-1's HARD GATE, which also requires the reviewed harness, phase
  report, and the plan's `0f922de` historical control.
- Cross-host phase subtraction requires the harness's clock-offset samples.
  Per-endpoint monotonic durations and causal wire ordering remain valid
  without synchronized clocks.

## Reviewer comments

(pending Codex review and requested Grok second eye)
