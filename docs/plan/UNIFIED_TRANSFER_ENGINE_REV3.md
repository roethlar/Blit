# Unified Transfer Engine REV3 - review candidate

**Status**: Draft
**Created**: 2026-06-20
**Review state**: ongoing plan review; coding is frozen pending the
owner's final plan decision (D-2026-06-20-4).
**Based on**: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (Active, parked)
plus the 2026-06-20 plan review findings. REV3 = REV2 with the Risks
section and the "C-ready by construction" acceptance criterion restored
(both dropped in REV2), the static-ladder references corrected against the
code, explicit slice dependencies added, and a labeled agent
recommendation under each open question.
**Decision refs**: D-2026-06-20-1 (convergence direction),
D-2026-06-20-2 (bound parameters), D-2026-06-20-3 (H10b veto),
D-2026-06-20-4 (plan-review freeze).

## Goal

Keep the v1 direction: converge the transfer subsystem around one
src/dst-agnostic engine, one live dial, and the already-shared
byte-moving leaf. The operator should experience one simple transfer
model regardless of where the command is issued. The implementation may
be complex underneath, but the behavioral contract must be FAST, SIMPLE,
and RELIABLE.

REV3 keeps convergence, not rebuild. It tightens the plan where review
found that v1 compressed too much into one slice or left compatibility
implicit:

- The first-byte-within-about-1s requirement is a real architecture
  change and gets its own streaming-plan slice.
- Existing local fast paths are preserved as engine-owned strategies
  unless the owner later decides to delete one; they must not remain
  side doors around the engine.
- Work-stealing is treated as a scheduling behavior change, not as
  "substrate only".
- Capacity profile and resize wire shape are designed before code that
  depends on them.
- Pull parity is measured only after pull is actually multistream.

## Non-goals

- No ground-up transfer rewrite.
- No zero-copy receive revival.
- No H10b merger. The engine's workload-shape-aware planner and 1s start
  requirement stand on their own; D-2026-06-04-3 remains queued after
  audit Round 1.
- No coding during this review. This Draft is not an implementation
  authorization.

## Constraints

- **FAST**: bytes begin quickly, stream scheduling avoids slow-sink
  head-of-line blocking, tuning comes from measured telemetry, and small
  local transfers keep their low-overhead path.
- **SIMPLE**: there is one transfer behavior owner. Local fast paths,
  push negotiation, pull sync, and delegated transfers become strategies
  or inputs under the engine, not separate operator-visible models.
- **RELIABLE**: mirror/delete safety, scan-completeness checks, resume,
  StallGuard behavior, cancellation, byte-progress accounting, and
  byte-identical transfer tests cannot regress.
- Wire changes are allowed, but mixed old/new peers must negotiate down
  to today's behavior. New fields are advisory until both peers advertise
  support.
- The 1370-test baseline must not drop.
- Windows parity remains required unless a test is genuinely platform
  specific.

## Acceptance Criteria

- [ ] A single `TransferEngine` (or owner-approved equivalent) is the
      public transfer sequencer for local-to-local, push, pull, and
      delegated daemon-to-daemon transfers.
- [ ] Existing local fast paths are either engine-owned strategies
      (`journal_skip`, `single_file`, `tiny_manifest`, `single_huge_file`)
      or explicitly deleted by owner decision. No local path bypasses the
      transfer behavior owner by accident.
- [ ] The static stream/dial sources are replaced by one dial source.
      (Corrected against code: today there are **two static tables, not
      three** — the client-side `remote/tuning.rs::determine_remote_tuning`
      ladder, whose `initial_streams`/`max_streams`/`prefetch_count`
      drive both local and push; and the daemon-side
      `DataTransferNegotiation.stream_count` (proto field 4). Pull is
      single-stream today via the `force_grpc` single-file path, not a
      third ladder. The earlier "push/control.rs::desired_streams" and
      "pull.rs::pull_stream_count" references were stale; both paths
      consume `determine_remote_tuning`.)
- [ ] The engine starts transfer work within about 1 second without a
      probe-then-go phase. This holds for **both** novel workloads (no
      telemetry extant — start copying something immediately at
      conservative defaults and tune live from the first byte) **and**
      known workloads (telemetry extant — replay the last run if it was
      optimal, else recalculate onto the live-tune path). Novel vs known
      is a tuning-strategy choice, not an exception. The only exceptions
      are modes where moving any byte before full knowledge would itself
      be unsafe — mirror/delete (scan-completeness), resume, and
      checksum-refusal — and those are explicit, tested, and reported to
      the owner instead of silently weakening RELIABLE.
- [ ] The planner is workload-shape-aware and can emit an initial safe
      work batch from partial enumeration, then refine as more headers
      arrive.
- [ ] The sender owns the dial within the receiver's advertised rich
      capacity profile. The weak end protects itself in both directions.
- [ ] The wire contract names the capacity-profile and stream-resize
      fields/messages, their field numbers, and the mixed-version
      behavior before code lands. (Grounded: `DataTransferNegotiation`
      uses fields 1–4 today and reserves 5–10 for RDMA, so
      `CapacityProfile receiver_capacity = 11` is the first free number.)
- [ ] **C-ready by construction, not by retrofit** (restored from v1):
      the dial is a live mutable object read by both ends from the
      live-dials slice onward, and the stream-set is elastic (work-stealing,
      work not pinned to a stream) from the salvage slice onward.
      Continuous mid-transfer stream add/drop (`ue-r2-2`) wires the resize
      proto onto this; it does not restructure the dial or the stream-set.
- [ ] Work-stealing is validated as behavior: slow sink, failing sink,
      cancellation, byte accounting, and StallGuard tests stay green.
- [ ] Pull is not counted in the loopback parity band until PullSync is
      actually multistream through the unified engine.
- [ ] Deprecated `Pull` RPC deletion waits until the multi-stream pattern
      has been harvested and compatibility/fallback tests cover old/new
      peer pairs.
- [ ] The 10 GbE benchmark remains the sign-off measure for final parity
      and stream resize, not a prerequisite to start the owner-approved
      coding slices.

## Current Code Reality

The existing code already has useful convergence substrate:

- `TransferSource` and `TransferSink` define the source/sink seam.
- `execute_sink_pipeline_streaming` and `execute_receive_pipeline` are
  shared byte-moving leaves.
- Push already streams manifest/need-list work and feeds the shared sink
  pipeline as work arrives.
- The planner already accounts for workload shape in part: file size
  classes, file count, tar shards, raw bundles, and large-file tasks.

The gaps are above that leaf:

- Local copy still owns a local-shaped `TransferOrchestrator`, builds a
  local runtime, runs local-only fast paths, collects all headers, and
  only then plans.
- Push has its own control loop, negotiation timing, fallback wedge
  protections, and consumes the static client-side tuning table
  (`determine_remote_tuning`).
- The daemon advertises a static `stream_count` (field 4) at negotiation;
  there is no capacity profile and no resize messages.
- PullSync receives the full client manifest, fully enumerates the server,
  compares, then transfers; its data-plane path is single-stream today
  (`force_grpc` single-file path).
- The proto has `DataTransferNegotiation.stream_count` (field 4), but no
  capacity profile, no resize messages, and fields 5–10 are reserved for
  RDMA.

## Design

### 1. Engine ownership

Introduce a new `TransferEngine` rather than renaming
`TransferOrchestrator` in place. `TransferOrchestrator` becomes the
local adapter that constructs local `Source`, `Sink`, options, and local
strategy inputs, then calls the engine. Push, PullSync, and delegated
transfers call the same engine with different source/sink and
negotiation adapters.

The engine owns:

- strategy selection (`journal_skip`, `single_file`, `tiny_manifest`,
  `single_huge_file`, streaming pipeline);
- dial creation and updates;
- payload work queue;
- progress and telemetry wiring;
- invariants around first work, fallback, cancellation, and finish.

Path-specific code remains only at boundaries where the protocol differs:
path resolution, module authorization, source/destination manifest
exchange, mirror/delete authority, and legacy compatibility.

### 2. Local fast paths are strategies, not bypasses

V1 said "no separate small-transfer path"; review found that wording
would accidentally delete useful FAST behavior. REV3 keeps the rule from
REV2: small/huge/no-op local paths may remain, but only as engine-owned
strategies with common accounting and tests.

This preserves SIMPLE for the human operator: there is one command model
and one transfer behavior owner. It also preserves FAST by keeping the
low-overhead tiny and single-file paths where they are faster than the
full pipeline.

### 3. Streaming plan foundation

The 1s start requirement cannot be hidden inside the sequencer-converge
slice. Today the local path collects all headers before planning, and
PullSync waits for complete client and server manifests before transfer.
REV3 makes streaming planning a separate foundation:

- define an `InitialPlan` / `PlanUpdate` shape that can be produced from
  a partial header stream;
- feed the shared streaming pipeline as safe payload batches become
  available;
- refine tar-shard and bundle targets as enumeration reveals shape;
- preserve mirror/delete and scan-completeness reliability;
- measure first byte and first useful progress per transfer mode.

The initial-plan strategy splits on whether telemetry exists for this
workload shape, reusing the cross-run history already in-tree
(`perf_history` appends a `PerformanceRecord` per transfer;
`perf_predictor` loads it and trains per-profile coefficients):

- **Novel workload** (no extant record): emit a conservative initial plan
  from the partial scan, start copying something immediately, and let
  the live tuner (PR1 telemetry, landed with `ue-r2-1a`) adjust from the
  first byte.
- **Known workload** (extant record): if the last run looks optimal,
  reproduce that plan immediately; otherwise recalculate onto the
  live-tune path.

Both branches meet the 1s start. Neither is an exception — the exception
class is only the unsafe-before-full-knowledge modes (mirror/delete,
resume, checksum-refusal) named in the Acceptance Criteria.

The requirement remains ambitious, but it is now explicit work rather
than a hidden subtask of "introduce the engine".

### 4. Dial and bounded-unilateral negotiation

The byte sender owns the live dial, bounded by the byte receiver's
capacity profile. The profile travels from receiver to sender during
setup:

- push: destination daemon advertises receiver capacity to the push
  client;
- pull: pull client advertises receiver capacity in the pull spec or
  setup message so the source daemon can send within it;
- delegated: destination daemon advertises its receiver capacity when it
  asks the source daemon to send.

The rich capacity profile should include, at minimum:

- CPU cores available to the transfer;
- disk class or drain class;
- current load estimate;
- maximum accepted streams;
- drain-rate estimate;
- max safe chunk bytes;
- max safe prefetch / in-flight bytes.

The initial dial starts below the profile ceiling with margin. Telemetry
can increase or decrease cheap dials immediately; stream count changes
arrive in the later resize slice. The dial is a **mutable object read by
both ends from the live-dials slice onward** — this is what makes
continuous (`ue-r2-2`) a wire-up rather than a retrofit (see the C-ready
acceptance criterion).

### 5. Wire contract before dependent code

REV3 makes wire shape an early deliverable. Proposed proto direction:

- append `CapacityProfile receiver_capacity = 11` to
  `DataTransferNegotiation` rather than using reserved RDMA fields 5–10
  (field 11 is the first free number after the reservation);
- add a capacity profile to the request/setup side where the receiver is
  the client, especially PullSync and delegated pull;
- add explicit peer capability bits/fields so resize messages are never
  sent to an old peer;
- add `DataPlaneResize` and `DataPlaneResizeAck` as negotiated control
  messages in the relevant control streams, not as blind TCP data-plane
  records.

Exact field names and numbers are part of the wire slice acceptance
criteria. Old peers must see current behavior: no capacity profile means
use today's static/conservative behavior; no resize support means no
mid-transfer add/drop.

### 6. Work-stealing is behavior

The adaptive PR2 work queue is required for continuous stream add/drop,
but it also changes scheduling. It must land with behavior tests:

- slow sink does not block all other sinks;
- one sink failure propagates the real error and shuts down cleanly;
- cancellation still aborts spawned workers;
- byte totals and file totals remain correct;
- StallGuard coverage survives the `Probe` conflict resolution.

### 7. Pull parity after pull multistream

Loopback parity is only meaningful after local-to-local, local-to-daemon,
and daemon-to-local all use comparable engine paths. The PullSync path is
single-stream today, so REV3 moves the parity-band gate after pull
multistream lands through the engine.

## Risks

- **Cherry-pick StallGuard regression.** Hand-resolving the `data_plane.rs`
  conflict (`StallGuardWriter` vs the `Probe` generic) during salvage
  could regress the byte-identical property PR2 pins. Mitigation: the
  byte-identical regression tests, the 1370 baseline, and the new
  work-stealing behavior tests in `ue-r2-1a`.
- **Receiver over-advertises its capacity profile.** A receiver that
  claims more drain capacity than it has could overwhelm itself on the
  first byte, because there is no probe phase to catch it. Mitigation:
  the *initial* conservative setting starts below the advertised ceiling
  with margin (fewer streams than the profile allows, ramping up as
  telemetry proves the link); the live tuner then backs off via
  `write_blocked` / retransmit telemetry. The engine is never exposed at
  the full advertised ceiling on the first byte.
- **1s-start pathological source.** A slow first enumeration over a huge
  directory could blow the 1s budget. Mitigation: the planner yields an
  initial plan from a partial scan and refines; it does not wait for full
  enumeration. Any mode that genuinely cannot meet the budget must be an
  explicit, tested, reported RELIABLE exception — not a silent miss.
- **Wire-compat break with mixed peers** (new in REV3). Adding field 11
  and resize messages could mispair old/new peers. Mitigation: the wire
  slice lands with explicit old-client/new-daemon and new-client/old-daemon
  compatibility tests *before* any behavior depends on the new fields,
  and resize messages are gated on advertised peer capability bits.
- **RELIABLE-exception loophole** (new in REV3). Allowing tested
  first-byte exceptions risks exceptions proliferating until RELIABLE is
  silently eroded. Mitigation: every exception is explicit, tested,
  individually reported to the owner, and revisited at the `ue-r2-1d`
  sign-off — none is added by coder fiat.

## Revised Slices

These are review-loop-sized docs/code slices. They replace the v1 slice
shape only if the owner accepts REV3 as the final plan.

1. **`ue-r2-1a-salvage-substrate`** — Cherry-pick adaptive PR1+PR2 up to
   `eafb187`, excluding `d9d4ec7`. Resolve `data_plane.rs`
   StallGuard-vs-`Probe`. Treat work-stealing as behavior, not inert
   substrate: add/keep slow-sink, failing-sink, cancellation,
   byte-accounting, StallGuard, and byte-identical tests. The elastic
   work-stealing stream-set exists from this slice onward (C-ready seam).
2. **`ue-r2-1b-wire-dial-contract`** — Define capacity profile, peer
   capability, and resize proto shape (`receiver_capacity = 11`,
   `DataPlaneResize`/`Ack`). Add compatibility tests for old client/new
   daemon and new client/old daemon. No behavior depends on these fields
   until this slice is green.
3. **`ue-r2-1c-engine-shell-local-adapter`** — Add `TransferEngine` and
   convert `TransferOrchestrator` into a local adapter. Move local fast
   paths under engine-owned strategies, preserving behavior and accounting.
4. **`ue-r2-1d-streaming-plan-foundation`** — Introduce partial-scan
   initial plans and plan updates. Prove first-byte / first-useful-work
   timing for local and push shapes, and document any RELIABLE exception
   that cannot safely move bytes before complete knowledge.
5. **`ue-r2-1e-live-cheap-dials`** — Replace the static
   `determine_remote_tuning` chunk/prefetch/TCP-buffer ladders with the
   single mutable dial. Start conservative within the receiver profile,
   then adjust cheap dials from PR1 telemetry. The dial is a mutable
   object read by both ends from this slice onward (C-ready seam).
6. **`ue-r2-1f-push-converge`** — Route push through the engine while
   preserving manifest streaming, need-list batching, fallback timing,
   scan-completeness purge safety, and old/new compatibility.
7. **`ue-r2-1g-pull-multistream-converge`** — Route PullSync through the
   engine and make pull multistream there. Preserve resume, checksum
   refusal, delete-list authority, cancellation, per-stream failure, and
   gRPC fallback behavior.
8. **`ue-r2-1h-delete-deprecated-pull-rpc`** — Delete deprecated `Pull`
   after PullSync has harvested the needed multistream/fallback pattern
   and tests cover the replacement, including old/new peer pairs.
9. **`ue-r2-2-stream-resize`** — Finish negotiated
   `DataPlaneResize`/`DataPlaneResizeAck` and add/drop streams mid
   transfer from live telemetry, using the elastic work queue from
   `ue-r2-1a`. Wires onto the already-mutable dial and elastic
   stream-set — a wire-up, not a restructuring.

### Slice dependencies

Explicit blocking, since REV3 has nine slices and ordering matters:

- `1a` (substrate) blocks everything — all later slices build on the
  elastic work-stealing stream-set.
- `1b` (wire contract) blocks `1e` (dial uses the capacity profile), `2`
  (resize proto), and contributes the compat tests `1h` gates on.
- `1c` (engine shell) blocks `1d`, `1e`, `1f`, `1g` — the streaming plan,
  dial, push, and pull all run inside the engine.
- `1d` (streaming plan) blocks the 1s-start acceptance and feeds `1g`
  (pull 1s-start).
- `1f` (push) and `1g` (pull) both depend on `1c`; `1g` also depends on
  `1d`.
- `1h` (delete Pull RPC) blocks on `1g` (multistream harvested) and the
  `1b` compat tests.
- `2` (resize) blocks on `1a` (elastic queue), `1b` (resize proto), and
  `1c` (engine).
- `1e` and `1f` are independent of each other once `1c` lands and may
  proceed in parallel.

## Review Findings Rolled In

- `ue-1c` was too large: split streaming-plan foundation, local adapter,
  push convergence, and pull convergence into separate slices.
- Local fast paths conflicted with "no separate small-transfer path":
  redefined them as engine strategies.
- "No behavior change" on salvage was inaccurate for work-stealing:
  tests now treat scheduling as observable.
- Proto compatibility was too implicit: wire shape is now a first-class
  early slice.
- Pull parity gate was too early: moved until after PullSync multistream.
- The "three static ladders" enumeration was inaccurate against code
  (corrected in REV3): there are two static tables
  (`determine_remote_tuning` client-side, `stream_count` daemon-side) plus
  pull's single-stream path.
- REV2 dropped the Risks section and the "C-ready by construction"
  acceptance criterion; REV3 restores both and adds two REV2-specific
  risks (wire-compat break, RELIABLE-exception loophole).
- `DECISIONS.md` D-2026-06-20-1 still carries superseded warmup/size-gate
  language, but D-2026-06-20-2 and this Draft carry the live-from-first
  byte correction. If REV3 is accepted, either edit D-2026-06-20-1 with a
  note or add a superseding cleanup decision.

## Open Questions for Final Owner Decision

- **(RESOLVED 2026-06-20, owner)** First-byte timing: hard invariant for
  every mode *except* the modes where moving any byte before full
  knowledge would itself be unsafe (mirror/delete, resume,
  checksum-refusal). Novel vs known workload is a tuning-strategy
  choice, not an exception: novel workloads start copying something
  immediately and tune live; known workloads replay the last run if it
  was optimal, else recalculate. Both meet 1s. Recorded in Acceptance
  Criteria and Design §3.
- **(RESOLVED 2026-06-20, owner)** Deprecated `Pull` deletion stays
  in-plan as `ue-r2-1h`, sequenced last, gated on `ue-r2-1g` + the
  `ue-r2-1b` compat tests.
- Should REV3 replace `UNIFIED_TRANSFER_ENGINE.md`, or remain a review
  branch while the original Active plan is amended? Owner: no flip yet —
  planning review in progress; REV3 stays Draft, v1 stays
  Active-but-parked.
  *Agent recommendation: replace when the review closes.* REV3 supersedes
  v1 on every substantive point (slice shape, corrected ladder references,
  restored risks/C-ready criterion) and v1's lineage is preserved in its
  header. Keeping both Active invites drift between them. Mark v1
  Superseded with a DECISIONS.md entry when accepted.
- Should D-2026-06-20-1 be edited now to remove the superseded
  warmup/size-gated wording, or should the later decisions remain the
  correction? Owner: not sure.
  *Agent recommendation: edit now with a one-line superseded note
  pointing at -2/-4.* Leaving stale wording in a settled decision invites
  a future reader to implement the superseded probe/size-gate. Per
  AGENTS.md §1, fix the lower-precedence doc when it disagrees.