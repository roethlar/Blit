# Unified Transfer Engine REV2 - review candidate

**Status**: Draft
**Created**: 2026-06-20
**Review state**: ongoing plan review; coding is frozen pending the
owner's final plan decision (D-2026-06-20-4).
**Based on**: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (Active, parked)
plus the 2026-06-20 plan review findings.
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

REV2 keeps convergence, not rebuild. It tightens the plan where review
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
- [ ] The three static stream/dial sources
      (`remote/tuning.rs::determine_remote_tuning`,
      `push/control.rs::desired_streams`, and the pull single-stream
      behavior) are replaced by one dial source.
- [ ] The engine starts transfer work within about 1 second without a
      probe-then-go phase. If a mode cannot safely move bytes before full
      knowledge (for example a mirror/delete or resume edge case), that
      exception is explicit, tested, and reported to the owner instead of
      silently weakening RELIABLE.
- [ ] The planner is workload-shape-aware and can emit an initial safe
      work batch from partial enumeration, then refine as more headers
      arrive.
- [ ] The sender owns the dial within the receiver's advertised rich
      capacity profile. The weak end protects itself in both directions.
- [ ] The wire contract names the capacity-profile and stream-resize
      fields/messages, their field numbers, and the mixed-version
      behavior before code lands.
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
  protections, and a static client-side tuning table.
- Push daemon negotiation has its own `desired_streams` ladder.
- PullSync receives the full client manifest, fully enumerates the server,
  compares, then transfers; its data-plane path is single-stream today.
- The proto has `DataTransferNegotiation.stream_count`, but no capacity
  profile, no resize messages, and fields 5-10 are reserved for RDMA.

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
would accidentally delete useful FAST behavior. REV2 changes the rule:
small/huge/no-op local paths may remain, but only as engine-owned
strategies with common accounting and tests.

This preserves SIMPLE for the human operator: there is one command model
and one transfer behavior owner. It also preserves FAST by keeping the
low-overhead tiny and single-file paths where they are faster than the
full pipeline.

### 3. Streaming plan foundation

The 1s start requirement cannot be hidden inside the sequencer-converge
slice. Today the local path collects all headers before planning, and
PullSync waits for complete client and server manifests before transfer.
REV2 makes streaming planning a separate foundation:

- define an `InitialPlan` / `PlanUpdate` shape that can be produced from
  a partial header stream;
- feed the shared streaming pipeline as safe payload batches become
  available;
- refine tar-shard and bundle targets as enumeration reveals shape;
- preserve mirror/delete and scan-completeness reliability;
- measure first byte and first useful progress per transfer mode.

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
arrive in the later resize slice.

### 5. Wire contract before dependent code

REV2 makes wire shape an early deliverable. Proposed proto direction:

- append `CapacityProfile receiver_capacity = 11` to
  `DataTransferNegotiation` rather than using reserved RDMA fields 5-10;
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
single-stream today, so REV2 moves the parity-band gate after pull
multistream lands through the engine.

## Revised Slices

These are review-loop-sized docs/code slices. They replace the v1 slice
shape only if the owner accepts REV2 as the final plan.

1. **`ue-r2-1a-salvage-substrate`** - Cherry-pick adaptive PR1+PR2 up to
   `eafb187`, excluding `d9d4ec7`. Resolve `data_plane.rs`
   StallGuard-vs-`Probe`. Treat work-stealing as behavior, not inert
   substrate: add/keep slow-sink, failing-sink, cancellation,
   byte-accounting, StallGuard, and byte-identical tests.
2. **`ue-r2-1b-wire-dial-contract`** - Define capacity profile, peer
   capability, and resize proto shape. Add compatibility tests for old
   client/new daemon and new client/old daemon. No behavior depends on
   these fields until this slice is green.
3. **`ue-r2-1c-engine-shell-local-adapter`** - Add `TransferEngine` and
   convert `TransferOrchestrator` into a local adapter. Move local fast
   paths under engine-owned strategies, preserving behavior and
   accounting.
4. **`ue-r2-1d-streaming-plan-foundation`** - Introduce partial-scan
   initial plans and plan updates. Prove first-byte / first-useful-work
   timing for local and push shapes, and document any RELIABLE exception
   that cannot safely move bytes before complete knowledge.
5. **`ue-r2-1e-live-cheap-dials`** - Replace static chunk/prefetch/TCP
   buffer ladders with the single mutable dial. Start conservative within
   receiver profile, then adjust cheap dials from PR1 telemetry.
6. **`ue-r2-1f-push-converge`** - Route push through the engine while
   preserving manifest streaming, need-list batching, fallback timing,
   scan-completeness purge safety, and old/new compatibility.
7. **`ue-r2-1g-pull-multistream-converge`** - Route PullSync through the
   engine and make pull multistream there. Preserve resume, checksum
   refusal, delete-list authority, cancellation, per-stream failure, and
   gRPC fallback behavior.
8. **`ue-r2-1h-delete-deprecated-pull-rpc`** - Delete deprecated `Pull`
   after PullSync has harvested the needed multistream/fallback pattern
   and tests cover the replacement.
9. **`ue-r2-2-stream-resize`** - Finish negotiated
   `DataPlaneResize`/`DataPlaneResizeAck` and add/drop streams mid
   transfer from live telemetry, using the elastic work queue from
   `ue-r2-1a`.

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
- `DECISIONS.md` D-2026-06-20-1 still carries superseded warmup/size-gate
  language, but D-2026-06-20-2 and this Draft carry the live-from-first
  byte correction. If REV2 is accepted, either edit D-2026-06-20-1 with a
  note or add a superseding cleanup decision.

## Open Questions for Final Owner Decision

- Should REV2 replace `UNIFIED_TRANSFER_ENGINE.md`, or remain a review
  branch while the original Active plan is amended?
- Should first-byte timing be a hard invariant for every mode, or should
  RELIABLE exceptions be allowed for specific mirror/resume/checksum
  cases if they are explicit and tested?
- Should deprecated `Pull` deletion remain part of this plan, or move to
  a follow-up cleanup after unified PullSync is proven?
- Should D-2026-06-20-1 be edited now to remove the superseded
  warmup/size-gated wording, or should the later decisions remain the
  correction?
