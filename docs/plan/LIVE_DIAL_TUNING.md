# Restore live dial tuning to TransferSession

**Status**: Draft
**Created**: 2026-07-16
**Supersedes**: Upon activation, the shape-only stream-policy wording in
`docs/plan/ONE_TRANSFER_PATH.md` and `docs/TRANSFER_SESSION.md`; restores the
still-binding D-2026-06-20-1/-2 design
**Decision ref**: D-2026-06-20-1, D-2026-06-20-2, D-2026-06-20-5;
Draft→Active approval pending

## Goal

Every TCP data-plane transfer is controlled by one live `TransferDial` owned
by the semantic SOURCE, regardless of which endpoint initiated the session.
The active stream count rises and falls from measured per-stream telemetry
during the transfer, within the DESTINATION's advertised receiver limit. No
file-count or byte-count table selects a terminal worker count, and `8` has no
special runtime meaning. The same controller, resize protocol, elastic work
queue, and tests cover SOURCE-initiator and SOURCE-responder layouts.

## Non-goals

- Reintroducing push, pull, or delegated-transfer drivers, RPCs, tuning
  tables, or controller branches. Connection topology may select dial versus
  accept; it may not select tuning policy.
- Treating the default receiver stream limit (currently 32) as an optimal
  operating point. It is a defensive resource ceiling. The live count below
  it is the tuner's decision.
- Inventing a second adaptive policy for the in-stream carrier. It has one
  HTTP/2 byte lane and no elastic TCP stream set.
- Folding local-apply concurrency into socket telemetry. Local transfers keep
  the same `TransferSession` choreography, but their filesystem-worker policy
  remains owned by `docs/plan/LOCAL_SMALL_FILE_PATH.md`, which has different
  signals and costs.
- Changing the same-build-only compatibility rule or adding a new wire
  message. `DataPlaneResizeOp::{Add,Remove}` already exists.
- Declaring a performance win from unit tests. Hardware evidence is a final
  slice after the behavior and its observer have passed review.

## Constraints

- SOURCE is the only resize controller because SOURCE owns the socket-write
  telemetry. DESTINATION advertises capacity and admits or refuses proposals;
  it never runs a competing controller.
- Initiator, CLI verb, endpoint name, and source/destination path must not
  enter stream-policy decisions. Both connection layouts meet in
  `SourceDataPlane`; only `SourceSockets::{Dial,Accept}` may differ.
- Epoch 0 starts immediately at a conservative, receiver-bounded floor. The
  floor is `min(DIAL_FLOOR_INITIAL_STREAMS, receiver_stream_ceiling)`; it is a
  startup cost/risk choice, not a workload prediction or terminal target.
- The settled live range is `1..=receiver_stream_ceiling`. A profile value of
  zero remains unknown/default, not a one-stream cap. The repository-wide
  absolute default limit remains a safety guard and must be named/documented
  as such.
- Tuning begins with the first non-idle sample. A tick with no transferred
  bytes is no signal. The existing blocked-ratio hysteresis, sustained-signal
  requirement, cooldown, one-stream step, and cheap-dials-before-streams
  ordering remain the initial policy unless measurement supports a separately
  reviewed change.
- Exactly one resize epoch may be in flight. Every proposal is settled with
  the membership that actually took effect. A refusal consumes the epoch and
  is terminal for further resize in that session, matching the current
  same-build contract.
- ADD carries one fresh sub-token and adds exactly one authenticated socket.
  REMOVE carries no sub-token and retires exactly one sender worker. Targets
  are absolute and must equal the current logical count plus or minus one.
- Retirement happens at a payload boundary. Work already claimed completes;
  queued work stays on the shared work-stealing queue; the retiring worker
  emits its normal per-stream `END`; no payload is lost, duplicated, or pinned
  to the retiring stream.
- Pipeline membership, probe-registry membership, dial settlement, and the
  peer's effective count must not drift. A one-way control-channel send is not
  sufficient proof that a worker joined or retired.
- Need completion is a terminal membership transition, not a local resize
  failure. Once the peer has accepted an operation, a live pipeline must apply
  it or fault; a pipeline already closing must satisfy it through normal member
  admission/retirement and `END`. An accepted operation may never be rewritten
  as a refused proposal at the unchanged count.
- `SourceDone`, cancellation, first-error-wins, `StallGuard`, progress, byte
  accounting, resume ordering, and clean socket teardown retain their current
  semantics. Tuner and validation tasks may not outlive the session.
- Production observation is aggregate, low-frequency, path-free, and disabled
  by default. It must expose enough information to explain every hold, ADD,
  REMOVE, refusal, and bound without changing the decision.
- New tests use deterministic samples or paused time; wall-clock sleeps and
  hardware timing are not correctness gates. Every behavior guard receives a
  red/restored-green proof before its slice is accepted.

## Acceptance criteria

- [ ] Production `TransferSession` has exactly one stream-count policy:
      SOURCE-owned live telemetry. `initial_stream_proposal`,
      `propose_shape_resize`, accumulated need bytes/counts, and the forced
      post-payload shape ramp have no production role and are deleted rather
      than left as a second authority.
- [ ] Epoch-0 worker count is the conservative floor clamped to the same
      receiver ceiling at both ends. It is independent of manifest size,
      file count, initiator role, and CLI verb.
- [ ] Every epoch-0 and ADDed SOURCE data socket uses `LiveProbe`; one shared
      registry represents exactly the workers eligible to take new payloads;
      `spawn_dial_tuner_with_resize` runs once per TCP SOURCE data plane and
      stops promptly on every completion, refusal, cancellation, and fault.
- [ ] Under a deterministic clean/non-idle sample trace, both initiator
      layouts emit the same epoch/op/target sequence and grow beyond eight
      workers when the receiver ceiling permits it. This proves that neither
      the retired target of 8 nor the old table maximum of 16 is a cap.
- [ ] Under a deterministic sustained-blocking trace, both layouts emit the
      same REMOVE sequence and shrink below their startup count, down to one
      when the signal persists. An idle trace and a trace inside the
      hysteresis band hold steady.
- [ ] A receiver limit below the proposed count clamps both layouts
      identically. Unknown/zero capacity resolves to the documented default
      safety limit in both layouts. No trace can cross the limit or floor.
- [ ] ADD and REMOVE validate monotonic epoch, exact one-step target, token
      shape, floor/ceiling, and current settled count. Duplicate/stale frames
      cannot repeat a membership change; rejected or locally failed changes
      cannot be reported as settled.
- [ ] Need completion during an in-flight resize has one result in both
      layouts. A proposal not accepted by the peer settles unchanged; an
      accepted ADD completes socket authentication and admits a member that
      immediately retires with `END` from the closed queue; an accepted REMOVE
      is satisfied by the named member's requested or already completed normal
      retirement. None of these healthy completion races faults or leaks a
      socket, member, probe, or receive task.
- [ ] REMOVE retires the exact worker/probe membership chosen by the elastic
      pipeline, emits `END`, and lets the matching receiver worker finish in
      both dial/accept layouts. Final reported stream count is settled logical
      membership, not cumulative sockets ever opened.
- [ ] Role-parameterized integration guards cover ADD while busy, REMOVE while
      busy, REMOVE while idle, need completion during a pending resize,
      terminal refusal, malformed/stale resize frames, cancellation, peer
      fault, source fault, and normal close. All successful cases produce
      identical requested-path inventories, byte counts, trees, timestamps,
      and permissions across initiator layouts.
- [ ] Default-off dial observation records sample bytes, blocked ratio, cheap
      dial values, live count, receiver ceiling, epoch, decision/reason, and
      settlement. The schema uses semantic SOURCE/DESTINATION and
      initiator/responder fields only for attribution; policy never reads
      those role labels.
- [ ] The exact-8 parity assertions are replaced by adaptive assertions. Repo
      status and contracts say what was actually proved: static-orientation
      parity was closed, while live up/down tuning is complete only after this
      plan's code and hardware slices pass.
- [ ] `cargo fmt --all -- --check`, strict workspace clippy, the full workspace
      suite, focused release-mode dial tests, `scripts/agent/check-docs.sh`,
      and the relevant Windows CI path are green without reducing the prior
      test baseline except for explicitly enumerated obsolete shape-table
      tests replaced by stronger adaptive guards.
- [ ] A reviewed Mac↔Mac run on quiet endpoints records live decision traces
      for identical large, 10k-small, and mixed fixtures under both initiator
      layouts. Acceptance is adaptive behavior and role-invariant policy, not
      convergence on a preselected worker number. Any material performance or
      decision-trace asymmetry is a finding, not a threshold exception.

## Design

### 1. One authority and one observation path

`SourceDataPlane` is the sole production ownership boundary. Both
`dial_source_data_plane` (SOURCE initiator) and `accept_source_data_plane`
(SOURCE responder) construct the same state:

- one receiver-bounded `Arc<TransferDial>`;
- one `SharedStreamProbes` registry;
- one tuner task from `spawn_dial_tuner_with_resize`;
- one proposal receiver consumed by the SOURCE control loop;
- one elastic pipeline control handle;
- one monotonic stream/member ID allocator; and
- one optional aggregate dial observer.

The socket constructor is the only role-specific operation. Epoch-0 and ADD
sockets use `DataPlaneSession::{connect,from_stream}_with_probe` and
`LiveProbe`. The ordinary `NoProbe` constructors remain for paths with no live
TCP dial, not for `TransferSession` TCP SOURCE workers.

Split sampling from policy so correctness tests do not depend on scheduler or
kernel timing. A production sampler folds live probes into a tick containing
elapsed time, delta bytes, and delta blocked-write nanoseconds; a pure policy
step converts that tick into cheap-dial changes and at most one
`ResizeProposal`. Tests feed the same tick sequence to both real session
layouts through a test-only source seam and compare the emitted control trace.

Workload shape remains useful to enumeration, payload grouping, tar-shard
selection, and progress. It does not map files or bytes to a stream count.
Delete `initial_stream_proposal`, `TransferDial::propose_shape_resize`, the
need-list resize accumulators, `settle_shape_resizes`, and their exact-target
tests after the live controller replaces them. Delete or rename legacy
`max_streams`/floor constants and accessors that no longer express a live
contract; do not leave an unused value named as though eight were a maximum.

### 2. Receiver-bounded conservative start

The current responder binds before it sees a manifest, which is correct for
fast start but not a reason to invent a workload target of one. Listener/token
preparation stays early; the epoch-0 count is finalized after semantic roles
and the DESTINATION capacity profile are known:

- DESTINATION responder: clamp the floor to its local advertised profile;
- DESTINATION initiator: clamp the same floor to the profile it placed in
  `SessionOpen`;
- unknown/zero `max_streams`: use `receiver_stream_ceiling`'s documented
  default; and
- an explicit lower receiver limit always wins.

`DataPlaneGrant.initial_streams`, the SOURCE dial's epoch-0 settlement, the
number of initiating sockets, and the DESTINATION's logical receive count must
all use that one resolved value. No manifest prefix or need-list total is
consulted.

### 3. Membership acknowledgement before settlement

The elastic pipeline must answer membership commands. Replace fire-and-forget
`SinkControl::Add`/`RetireOne` use with an acknowledgement that names the
logical member ID and says whether it joined or was marked to retire. The
pipeline remains the authority on which LIFO member is retired because it owns
the worker flags. `SourceDataPlane` uses the returned member ID to update the
matching probe registry entry; it does not blindly pop a separate vector.

While payload admission is live, an accepted ADD is settled only after the
authenticated socket is wrapped, the pipeline accepts its member, and its
probe is registered. An accepted REMOVE is settled only after the pipeline
marks one member ineligible for new work and the matching probe is removed.
The worker may finish its already claimed payload afterward, then calls
`finish()` and emits `END`. If a live pipeline refuses or errors on membership
after peer acceptance, the same-build session faults; it must not publish a
false effective count.

Payload closing is different from live-pipeline refusal. Keep a terminal
member ledger until every in-flight epoch settles, and make the membership
acknowledgement distinguish ordinary join/retire from terminally satisfied
join/retire. For an accepted ADD, complete the role-specific authenticated
socket acquisition even if need completion has closed the work queue, admit
the new member and probe, and let that member immediately execute the normal
no-payload `finish()`/`END` path. For an accepted REMOVE, a named member that
has already executed normal END retirement satisfies the decrement; do not
try to retire a second member. Settlement records the accepted target and the
terminal retirement outcome so both peers observe the same transition without
stranding the destination-initiated ADD socket. Need completion by itself is
never a `SessionFault`.

### 4. One op-aware resize loop

Replace shape-specific `PendingResize` with one record containing epoch,
absolute target, operation, and an ADD-only sub-token. The SOURCE send loop
selects tuner proposals beside payload queueing, peer events, and faults:

1. For ADD, mint a sub-token and send `DataPlaneResize{ADD}`. DESTINATION
   validates the next epoch/target/ceiling and prepares transport: responder
   arms an authenticated accept; initiator dials the authenticated socket.
   It then ACKs. SOURCE completes its dial/accept side, admits the new member,
   registers its probe, and settles the epoch.
2. For REMOVE, send `DataPlaneResize{REMOVE}` with an empty token.
   DESTINATION validates the next epoch/target/floor and ACKs the logical
   decrement; it does not close a socket. SOURCE retires the acknowledged
   member, updates probes, and settles. The normal `END` closes the matching
   destination receive worker.
3. Peer refusal, or a local proposal that fails before peer acceptance, settles
   at the unchanged count and permanently stops resize for the session. After
   peer acceptance, an operation is no longer optional: a live pipeline applies
   it or faults, while a closing pipeline uses the terminal membership outcomes
   in section 3. A transport/authentication error after an accepted ADD still
   faults; need completion alone does not.

The destination keeps a small resize state: settled epoch, logical live count,
and the last ACK. It rejects out-of-order or inconsistent frames and replays a
duplicate epoch's ACK without performing the operation twice. ADD's socket
preparation remains role-specific; all validation, accounting, REMOVE, and ACK
construction are shared.

Closing the payload input ends demand. There is no tail loop that opens idle
sockets to reach a predetermined number. A proposal already in flight is
classified by whether the peer accepted it: an unaccepted proposal settles
unchanged, while an accepted proposal completes through live membership or the
terminal drain rule above. No new proposal is accepted after controller
shutdown begins.

### 5. Lifecycle and observability

`SourceDataPlane::finish` stops proposal intake and the tuner, closes payload
input, but keeps the membership command endpoint and terminal member ledger
alive until the one in-flight epoch is classified and settled. A never-accepted
local proposal may settle unchanged; an accepted proposal must take the live or
terminal membership path above. It then joins the tuner and drains the elastic
pipeline. Drop/cancellation aborts both tuner and pipeline under the existing
fault contract rather than pretending an accepted operation was refused.
The destination drains every receive worker, including a retiring worker's
final payload and `END`, before scoring the summary. Its exposed stream count
is the final logical count; a separate observer field may report peak or total
opened sockets without changing that contract.

Extend the existing session-phase instrumentation with one low-frequency dial
event schema. Every sample records the raw aggregate used by policy and the
decision reason (`idle`, `hysteresis`, `cheap-up`, `cheap-down`, `cooldown`,
`bound`, `add`, `remove`, `pending`, `refused`) plus proposal/settlement fields.
It is disabled by default and must be inert under the existing observer OFF/ON
parity discipline. Hardware reviews consume these events; stderr anecdotes or
final worker count alone are not evidence that the tuner ran correctly.

### 6. Durable contract correction

The active parent plan and session contract must say, before implementation,
that live telemetry is the intended authority and that current HEAD is in
drift. When the cutover slice lands, remove the drift warning and update
`docs/STATE.md`, `REVIEW.md`, and the obsolete sf-2/exact-8 records with
pointers to this plan and its reviewed evidence. Historical findings remain
historical; they are annotated, not rewritten to pretend they proved adaptive
tuning.

## Affected files

- `crates/blit-core/src/dial.rs` — live policy, sampling seam, obsolete shape
  authority removal, receiver safety naming.
- `crates/blit-core/src/remote/transfer/progress.rs` and
  `data_plane.rs` — existing live probes and probe-aware sessions.
- `crates/blit-core/src/remote/transfer/pipeline.rs` — acknowledged elastic
  membership and exact retire identity.
- `crates/blit-core/src/transfer_session/data_plane.rs` — common SOURCE data
  plane ownership, probe registry, tuner lifecycle, ADD/REMOVE membership.
- `crates/blit-core/src/transfer_session/mod.rs` — op-aware SOURCE proposal
  loop, shared DESTINATION validation/accounting, removal of shape ramp.
- `crates/blit-core/src/remote/transfer/session_phase.rs` — default-off dial
  sample/decision observer.
- `crates/blit-core/tests/transfer_session_roles.rs` and focused module tests
  — deterministic role, protocol, membership, lifecycle, and inertness guards.
- `proto/blit.proto` — comments/contract precision only unless review proves
  the existing ADD/REMOVE shape insufficient; any shape change requires a
  separate reviewed plan amendment before code.
- `docs/plan/ONE_TRANSFER_PATH.md`, `docs/TRANSFER_SESSION.md`,
  `docs/STATE.md`, `REVIEW.md`, `.review/`, and `DEVLOG.md` — intent, status,
  review, and evidence.

## Slices

Each slice is one reviewloop finding and one commit before review fixes.

1. **ldt-1 — acknowledged elastic membership.** Make pipeline ADD/REMOVE
   return exact membership outcomes; bind probes to member IDs; preserve the
   existing ADD behavior while making false settlement impossible. Retain a
   terminal member ledger and return explicit joined/retire-marked versus
   joined-then-ended/already-ended outcomes until the pending epoch settles.
   Guard busy and idle retirement, failed control delivery, LIFO identity,
   normal `END`, first-error-wins, and exactly-once payload outcomes.
2. **ldt-2 — live controller cutover.** Finalize the receiver-bounded epoch-0
   floor; attach probes and tuner in both `SourceDataPlane` constructors;
   consume one op-aware proposal stream; implement shared ADD/REMOVE
   validation and settlement; delete shape-table production policy and forced
   convergence. Land deterministic identical-trace role guards, >8 growth,
   below-floor-start shrink, capacity/refusal/stale-frame guards, and update
   the session contract in the same slice.
3. **ldt-3 — lifecycle and observer closure.** Prove tuner/pipeline teardown,
   accepted and unaccepted ADD/REMOVE at need-complete in both layouts (no
   healthy fault, false unchanged settlement, duplicate retirement, or leaked
   authenticated socket), cancellation and fault paths, final logical versus
   peak stream accounting, default-off observer inertness, and complete
   decision reasons. Run mutation proofs and the full debug/release/docs/CI
   gates; correct all live status text that still calls exact 8 adaptive parity.
4. **ldt-4 — quiet Mac↔Mac evidence.** After ldt-1..3 are independently
   accepted, build and stage exact clean artifacts, verify endpoint quietness,
   and run identical large/10k-small/mixed fixtures under both initiator
   layouts with dial observation. Record raw traces, adaptive decisions,
   floor/peak/final counts, throughput, integrity, and role-invariance verdicts.
   Do not tune constants from this one session inside the evidence slice; any
   policy change becomes a new reviewed finding with a repeatable guard.

## Open questions

- None. Controller ownership, live-from-first-byte tuning, bidirectional
  resize, and receiver-bounded authority are already settled by
  D-2026-06-20-1/-2. Draft→Active remains an owner checkpoint after review.
