# Restore live dial tuning to TransferSession

**Status**: Active
**Created**: 2026-07-16
**Supersedes**: The shape-only stream-policy wording in
`docs/plan/ONE_TRANSFER_PATH.md` and `docs/TRANSFER_SESSION.md`; restores the
still-binding D-2026-06-20-1/-2 design
**Decision ref**: D-2026-06-20-1, D-2026-06-20-2, D-2026-06-20-5;
D-2026-07-16-2 (owner activated after neutral Claude review)

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

The checked code criteria are backed by the accepted ldt-1, ldt-2, and ldt-3
records at `f8f3c51`, `65a0f9f`, and `406a7e5`. Windows CI and live rig-W
evidence remain open and are not implied by those local guards.

- [x] Production `TransferSession` has exactly one stream-count policy:
      SOURCE-owned live telemetry. `initial_stream_proposal`,
      `propose_shape_resize`, accumulated need bytes/counts, and the forced
      post-payload shape ramp have no production role and are deleted rather
      than left as a second authority.
- [x] Epoch-0 worker count is the conservative floor clamped to the same
      receiver ceiling at both ends. It is independent of manifest size,
      file count, initiator role, and CLI verb.
- [x] Every epoch-0 and ADDed SOURCE data socket uses `LiveProbe`; one shared
      registry represents exactly the workers eligible to take new payloads;
      `spawn_dial_tuner_with_resize` runs once per TCP SOURCE data plane and
      stops promptly on every completion, refusal, cancellation, and fault.
- [x] Under a deterministic clean/non-idle sample trace with an advertised
      receiver ceiling of at least 17, both initiator layouts emit the same
      epoch/op/target sequence through target 17. The guard must turn red when
      production growth is clamped at either 8 or 16; reaching 9 alone is not
      proof that the retired table maximum is gone.
- [x] Under a deterministic sustained-blocking trace, both layouts emit the
      same REMOVE sequence and shrink below their startup count, down to one
      when the signal persists. An idle trace and a trace inside the
      hysteresis band hold steady.
- [x] A receiver limit below the proposed count clamps both layouts
      identically. Unknown/zero capacity resolves to the documented default
      safety limit in both layouts. No trace can cross the limit or floor.
- [x] ADD and REMOVE validate monotonic epoch, exact one-step target, token
      shape, floor/ceiling, and current settled count. Duplicate/stale frames
      cannot repeat a membership change; rejected or locally failed changes
      cannot be reported as settled.
- [x] Need completion during an in-flight resize has one result in both
      layouts. A proposal not accepted by the peer settles unchanged; an
      accepted ADD completes socket authentication and admits a member that
      immediately retires with `END` from the closed queue; an accepted REMOVE
      is satisfied by the named member's requested or already completed normal
      retirement. None of these healthy completion races faults or leaks a
      socket, member, probe, or receive task.
- [x] REMOVE retires the exact worker/probe membership chosen by the elastic
      pipeline, emits `END`, and lets the matching receiver worker finish in
      both dial/accept layouts. Final reported stream count is settled logical
      membership, not cumulative sockets ever opened.
- [x] Role-parameterized integration guards cover ADD while busy, REMOVE while
      busy, REMOVE while idle, need completion during a pending resize,
      terminal refusal, malformed/stale resize frames, cancellation, peer
      fault, source fault, and normal close. All successful cases produce
      identical requested-path inventories, byte counts, trees, timestamps,
      and permissions across initiator layouts.
- [x] Default-off dial observation records sample bytes, blocked ratio, cheap
      dial values, live count, receiver ceiling, epoch, decision/reason, and
      settlement. The schema uses semantic SOURCE/DESTINATION and
      initiator/responder fields only for attribution; policy never reads
      those role labels.
- [x] The exact-8 parity assertions are replaced by adaptive assertions. Repo
      status and contracts say what was actually proved: static-orientation
      parity was closed, while live up/down tuning is complete only after this
      plan's code and hardware slices pass.
- [ ] `cargo fmt --all -- --check`, strict workspace clippy, the full workspace
      suite, focused release-mode dial tests, `scripts/agent/check-docs.sh`,
      and the relevant Windows CI path are green without reducing the prior
      test baseline except for explicitly enumerated obsolete shape-table
      tests replaced by stronger adaptive guards.
- [ ] A reviewed rig-W run on quiet `q`↔`netwatch-01` endpoints records live
      decision traces for identical large, 10k-small, and mixed fixtures under
      both initiator layouts. Acceptance is adaptive behavior and role-invariant
      policy, not convergence on a preselected worker number. Any material
      performance or decision-trace asymmetry is a finding, not a threshold
      exception.

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
pipeline. Every SOURCE scan/count/filter/checksum helper, tuner, and pipeline
worker is owned and joined on normal, error, and cancellation exits. The
DESTINATION likewise owns and joins every receive worker, including a retiring
worker's final payload and `END`, before scoring the summary. Drop/cancellation
uses the existing fault contract rather than pretending an accepted operation
was refused. The exposed stream count is the final logical count; the observer
reports a distinct peak count without changing that contract.

Extend the existing session-phase instrumentation with one low-frequency dial
event schema. Every sample records the raw aggregate used by policy — bytes,
blocked nanoseconds, elapsed nanoseconds, stream count, validity, and computed
ratio — plus the exact sample reason: `idle`, `rebaseline`, `hysteresis`,
`cheap-up`, `cheap-down`, `sustain`, `cooldown`, `bound`, `add`, or `remove`.
Lifecycle events use their own closed taxonomy: `dial_pending` records
`pending`, while `dial_settlement` records `add`, `remove`, or `refused`, along
with the proposal and settlement fields. Construction-time chunk, prefetch,
and TCP choices are cheap snapshots, not worker authority. The observer is
disabled by default and must be wire-neutral and policy-inert under the
existing OFF/ON parity discipline. Hardware reviews consume these events;
stderr anecdotes or final worker count alone are not evidence that the tuner
ran correctly.

### 6. Durable contract correction

The ldt-2 cutover makes the active parent plan and session contract name live
telemetry as the worker authority, removes the pre-cutover drift warning, and
updates `docs/STATE.md`, `REVIEW.md`, and the obsolete sf-2/exact-8 records with
pointers to this plan and its candidate evidence. Historical findings remain
historical; they are annotated, not rewritten to pretend they proved adaptive
tuning. Acceptance and reviewed evidence are recorded only after the fixed-SHA
review passes.

## Affected files

- `crates/blit-core/src/dial.rs` — live policy, sampling seam, obsolete shape
  authority removal, receiver safety naming.
- `crates/blit-core/src/remote/transfer/progress.rs` and
  `data_plane.rs` — existing live probes and probe-aware sessions.
- `crates/blit-core/src/remote/transfer/pipeline.rs` — acknowledged elastic
  membership and exact retire identity.
- `crates/blit-core/src/remote/transfer/abort_on_drop.rs` and `source.rs` —
  cancellation-safe task ownership and ordered scan/helper teardown.
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
- `scripts/bench_ldt4_rigw.sh` — registered non-destructive 96-arm rig-W
  harness, exact-artifact gates, identity-scoped process ownership, additive
  evidence retention, and Windows runtime restoration.
- `scripts/ldt4_rigw_analyze.py` and `scripts/ldt4_rigw_analyze_test.py` —
  fail-closed evidence validation, role/path/trace/integrity grading, and
  mutation-sensitive synthetic coverage.
- `docs/plan/ONE_TRANSFER_PATH.md`, `docs/TRANSFER_SESSION.md`,
  `docs/STATE.md`, `REVIEW.md`, `.review/`, and `DEVLOG.md` — intent, status,
  review, and evidence.

## Slices

When formal review is selected, the slice is one committed whole-change Fable
openreview candidate before any one-finding-per-commit review fixes. Grok may
advise but cannot accept the slice (D-2026-07-16-4).

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
   convergence. Land deterministic identical-trace role guards that grow
   through target 17 and kill both 8- and 16-clamp mutations,
   below-floor-start shrink, capacity/refusal/stale-frame guards, and update the
   session contract in the same slice. Accepted at `65a0f9f` after neutral
   Claude Fable 5/max openreview; its sole Low suggestion predicted no runtime
   failure and was declined at intake as cleanup-only.
3. **ldt-3 — lifecycle and observer closure.** Prove tuner/pipeline teardown,
   accepted and unaccepted ADD/REMOVE at need-complete in both layouts (no
   healthy fault, false unchanged settlement, duplicate retirement, or leaked
   authenticated socket), cancellation and fault paths, final logical versus
   peak stream accounting, default-off observer inertness, and complete
   decision reasons. Run mutation proofs and the full debug/release/docs/CI
   gates; correct all live status text that still calls exact 8 adaptive parity.
   Candidate `436e1bb` passed exact local gates; neutral Claude Fable 5/max
   openreview admitted one Low observer-ordering defect. The isolated fix
   `406a7e5` is mutation-proved; neutral r2 returned clean with an independent
   guard. Accepted at `406a7e5`.
4. **ldt-4 — quiet rig-W `q`↔`netwatch-01` evidence.** After ldt-1..3 are
   independently accepted, build and stage exact clean artifacts, verify
   endpoint quietness, and run identical large/10k-small/mixed fixtures under
   both initiator layouts with dial observation. Record raw traces, adaptive
   decisions, floor/peak/final counts, throughput, integrity, and
   role-invariance verdicts.
   Do not tune constants from this one session inside the evidence slice; any
   policy change becomes a new reviewed finding with a repeatable guard.
   Fable round one reviewed exact harness/analyzer head `0e48721`; all seven
   admitted fixes landed one per commit, and a neutral re-review accepted exact
   head `4e0fdc3`. After exact artifacts were staged, its first launch staged
   the small fixture, then voided before any arm or daemon swap because the two
   large fixtures differed in content. Guarded candidate `b0c6ce3` makes
   Windows large/mixed fixtures canonical and promotes validated copies into
   stable q source paths. Canonical-fixture Fable round one reviewed exact
   `ef48920` and admitted two Low corrections: promote with the harness's
   existing exclusive atomic rename primitive, and validate the fetched
   canonical shape before the large copy/free-space gate. The rename correction
   is fixed and mutation-proved at `1302b90`; `f2` validates and sizes from the
   fetched manifest before copy at `fdf7b37`. Full local gates pass, and final
   Fable review accepts exact `5a2265e` with an independent red/green guard.
   Exact harness/analyzer `96a4e3b` then completed all 96 rig-W arms in
   session `ldt4-20260721T224319Z-96a4e3b03caf`; retained evidence and an
   independent byte-for-byte analysis recomputation are recorded at
   `docs/bench/ldt4-rigw-2026-07-21/`. The structurally valid result is
   `REVIEW_REQUIRED`: arm review 0, decision review 14, performance review 2.
   Every arm stayed at floor = peak = final = 4; 74 arms produced no tuner
   sample and 22 produced one, so this matrix did not exercise adaptive
   membership. `ldt-4-live-f12` owns a separate four-arm sustained supplement
   bound to the original evidence digest, preserving its fixed matrix and
   avoiding another additive 96-arm payload-retention cost. Exact `04e8008` is
   mutation-proved, full-gate green, tactically reviewed clean, and additively
   staged on q. Session `ldt4-20260722T001611Z-04e80082e12c` completed all
   four arms with normal restoration, but exact independently reproduced
   analysis is `REVIEW_REQUIRED`: arm review 4, decision/performance review 0.
   Each pair matched empty operations; all arms had zero samples because five
   files reached terminal SOURCE demand and queued before the first 500 ms
   tuner tick, even while bytes drained for 4.3–20.6 seconds. A new finding
   must keep admission backpressured across required busy ticks within q's
   additive retained-space floor.
   `ldt-4-live-f13` registers 40 separate 1 GiB payloads. The initial SOURCE
   mpsc/forwarder/shared-queue/workers can admit at most 25 whole-file payloads,
   so at least 15 GiB must finish before terminal admission; the 10Gbase-T line
   ceiling makes that longer than the earliest tick-7 ADD. Because q's internal
   volume cannot retain another large pair above its no-delete floor, f13 pins
   fresh roots to q's exact local Apps APFS SSD UUID while retaining the
   internal evidence and quietness gates. The candidate is Bash-3.2
   self-tested, mutation-proved at the exact 40-file horizon, covered by 86
   analyzer tests, backward-compatible byte-for-byte with retained f12
   analysis, and full-repository green. Opus 4.8/max found the candidate
   correct and fail-closed, admitting one Low guard gap because synthetic tests
   hid the analyzer's production horizon tuple. The literal tuple pin is
   mutation-proved with 87 tests and exact `a0c3e3f` re-reviewed clean by the
   same Opus session with an independent red/green guard. Exact `a0c3e3f` is
   additively staged on q with native Bash 3.2/87-test guards green. Its four
   live arms all completed and exercised resize, but final analysis voided on
   `ldt-4-live-f14`: production SOURCE control events spell action as the exact
   protobuf enum while the analyzer's synthetic contract used shorthand.
   The bounded f14 candidate now derives the exact expected SOURCE enum string
   from each validated dial operation while leaving dial and DESTINATION action
   contracts unchanged. Synthetic SOURCE traces match production, a focused
   guard pins both namespaces, and the full 88-test analyzer suite is
   mutation-proved red/green. Diagnostic reanalysis of an additive copy of the
   retained void session validates all four arms and reports the expected
   `REVIEW_REQUIRED` (arm 3, decision 1, performance 0), without promoting that
   void evidence to acceptance. Tactical Opus 4.8/max confirmed the exact
   production/dial contracts and 27-test shorthand mutation, then found one Low
   guard gap: deleting both SOURCE action comparisons still left all 88 tests
   green. `ldt-4-live-f14-r1-f1` adds independent negative guards for
   `resize_proposed` and `source_settled`; each fails alone when its comparison
   is disabled, and restoration returns all 90 tests green. The same Opus
   session re-reviewed exact `7050a29`, independently reproduced both isolated
   failures, restored reviewed hashes, and returned clean with no findings.
   Exact `7050a29` is staged through new complete-history bundle
   `/Users/michael/blit-ldt4-stage-7050a29.bundle` (SHA-256
   `f885f21dfc35cb7c47a2778c6cddae5552e51b46f8ee3b909b1f9670edc87e00`)
   and clean detached q checkout
   `/Users/michael/Dev/blit_v2_harness_7050a29`. q's native Bash 3.2
   syntax/self-test and all 90 analyzer tests pass. Fresh session
   `ldt4-20260722T022350Z-7050a2997ac5` completed all four arms with normal
   restoration and exact independently reproduced analysis at
   `docs/bench/ldt4-rigw-horizon-2026-07-22/`: arm review 3, decision review 1,
   performance review 0. q→Windows matched REMOVE 4→1 at 47.7/47.7 seconds.
   Windows→q split ADD 4→10 versus REMOVE 4→1 at 45.3/34.7 seconds; the same
   split repeated within 7/10 ms of the earlier void run. Both sessions used
   the same fixed order, however: the first Windows-source arm follows 80 GiB
   of destination writes and the second immediately rereads the same 40 GiB
   source. A new reviewed finding must reverse pair order before attributing
   the split to socket layout or changing controller policy.
   `ldt-4-live-f15` registers that causal check as a distinct
   `horizon_order` matrix bound to the valid horizon inventory. It reverses
   both role pairs under the unchanged artifact, fixture, volumes, and policy,
   then classifies the Windows→q result as order tracking, role tracking, or
   inconclusive only if the reversed q→Windows control remains stable. It must
   pass mutation-sensitive analyzer/harness guards and tactical Opus 4.8/max
   review before additive staging; no outcome directly authorizes a controller
   change or regrades the fixed-cell findings.
   The implementation now binds the exact prior inventory and source-manifest
   bytes, validates the reversed schedule and all four causal outcomes with 98
   analyzer tests, passes native Bash 3.2 self-test, and re-renders every fixed,
   sustained, and horizon report byte-for-byte. Three isolated schedule,
   digest, classifier, and exact-source mutations turn the intended guards
   red. Full repository gates pass; tactical Opus review remains before
   staging.
   Keep the `q_to_windows_large` 1.197 and
   `q_to_windows_mixed` 1.131 performance asymmetries separate so longer
   payloads cannot hide their fixed overhead.

## Open questions

- None. Controller ownership, live-from-first-byte tuning, bidirectional
  resize, and receiver-bounded authority are already settled by
  D-2026-06-20-1/-2. The owner activated this plan in D-2026-07-16-2.
