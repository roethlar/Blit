# Unified Transfer Engine REV4 — the Active convergence plan (code-reality corrected)

**Status**: Active
**Created**: 2026-06-20
**Activated**: 2026-06-20 (owner: "rev4 replaces v1" — D-2026-06-20-5).
Replaces `UNIFIED_TRANSFER_ENGINE.md` (v1, now Superseded) and the review
candidates REV2/REV3 (now Superseded-by-REV4). The plan-review freeze
(D-2026-06-20-4) is lifted as to the **plan decision**; per AGENTS.md §9
no code slice starts without a fresh per-slice owner authorization.
**Based on**: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md` plus a
2026-06-20 re-verification of every code claim against the tree at
`HEAD` (`09268eb`). REV4 = REV3 with its "Current Code Reality" and the
stream-ladder acceptance criterion **corrected against the actual code**
(REV3's "two static tables, not three" correction was itself wrong), the
pull single-stream claim re-scoped to PullSync, the local-fast-path
strategy names fixed to their real values, every named symbol grounded
with `file:line`, and v1's lineage/absorption header carried forward so
this doc can stand as plan-of-record if the owner flips it.
**Lineage** (carried forward from `UNIFIED_TRANSFER_ENGINE.md` so
supersession is not lost if REV4 replaces v1):
- Supersedes the "ground-up redesign" framing of the 2026-06-14 open
  question (resolved as *convergence*, not rebuild — D-2026-06-20-1).
- Continues the lineage of `PIPELINE_UNIFICATION.md` and
  `UNIFIED_RECEIVE_PIPELINE.md` (both **Historical**): they landed the
  shared byte-moving leaf but never converged the sequencer+dials layer.
- **Absorbs** `MULTISTREAM_PULL.md` (w2-3) as slice `ue-r2-1g` (goal
  survives; the path-specific premise does not).
- The engine's workload-shape-aware planner and 1s-start commitment are
  **not** the H10b streaming-planner concept and do **not** supersede
  D-2026-06-04-3 (owner vetoed that merger — D-2026-06-20-3).
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

REV4 keeps convergence, not rebuild. It tightens the plan where review
found that v1 compressed too much into one slice or left compatibility
implicit, and it corrects the code-reality errors that crept into REV2/
REV3:

- The first-byte-within-about-1s requirement is a real architecture
  change and gets its own streaming-plan slice.
- Existing local fast paths are preserved as engine-owned strategies
  unless the owner later decides to delete one; they must not remain
  side doors around the engine.
- Work-stealing is treated as a scheduling behavior change, not as
  "substrate only".
- Capacity profile and resize wire shape are designed before code that
  depends on them.
- Pull parity is measured only after PullSync is actually multistream.
- The stream-count ladders the engine must subsume are enumerated
  **accurately** (REV3 under-counted them — see Current Code Reality).

## Non-goals

- No ground-up transfer rewrite.
- No zero-copy receive revival (D-2026-06-12-1; revisit gated on the
  10 GbE benchmarks).
- No H10b merger. The engine's workload-shape-aware planner and 1s start
  requirement stand on their own; D-2026-06-04-3 remains queued after
  audit Round 1.
- The **gRPC fallback path stays single-logical-stream by design**
  (unchanged from w2-3's non-goal). "Pull is single-stream today" below
  is about PullSync's TCP data plane, not this fallback.
- No coding during this review.

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
- Wire changes are allowed (proto unfrozen, D-2026-06-11-1), but mixed
  old/new peers must negotiate down to today's behavior. New fields are
  advisory until both peers advertise support.
- The 1370-test baseline must not drop.
- Windows parity remains required unless a test is genuinely platform
  specific.
- **Every stage serves FAST, SIMPLE, or RELIABLE** — a purely structural
  change with no goal payoff is out.

## Acceptance Criteria

- [ ] A single `TransferEngine` (or owner-approved equivalent) is the
      public transfer sequencer for local↔local, push, pull, and
      delegated daemon↔daemon transfers.
- [ ] Existing local fast paths are either engine-owned strategies
      (their real names: `journal_no_work`, `no_work`, `tiny_manifest`,
      `single_huge_file`, and the single-file copy shortcut at
      `orchestrator.rs:178`) or explicitly deleted by owner decision. No
      local path bypasses the transfer behavior owner by accident.
- [ ] **The three static code-level stream/dial ladders plus the
      negotiated proto field are replaced by one dial source** (corrected
      against code — see Current Code Reality). Concretely, the dial
      subsumes:
      1. `remote/tuning.rs::determine_remote_tuning` (size-keyed
         `initial_streams`/`max_streams`/`chunk_bytes`/`tcp_buffer_size`/
         `prefetch_count`; the *client's* ladder, consumed by push and by
         the daemon pull paths);
      2. `blit-daemon .../push/control.rs::desired_streams` (the daemon
         push-negotiation ladder, already keyed on **file count** as well
         as bytes — the daemon's ladder "wins" per `tuning.rs`'s own doc
         comment);
      3. `blit-daemon .../pull.rs::pull_stream_count` (the deprecated
         Pull RPC ladder, byte-keyed, capped by `tuning.max_streams`);
      and the negotiated `DataTransferNegotiation.stream_count` (proto
      field 4) those ladders feed onto the wire. After convergence no
      static size→streams table remains in any path.
- [ ] The engine starts transfer work within about 1 second without a
      probe-then-go phase. This holds for **both** novel workloads (no
      telemetry extant — start copying immediately at conservative
      defaults and tune live from the first byte) **and** known workloads
      (telemetry extant — replay the last run if it was optimal, else
      recalculate onto the live-tune path). Novel vs known is a
      tuning-strategy choice, not an exception. The only exceptions are
      modes where moving any byte before full knowledge would itself be
      unsafe — mirror/delete (scan-completeness), resume, and
      checksum-refusal — and those are explicit, tested, and reported to
      the owner instead of silently weakening RELIABLE.
- [ ] The planner is workload-shape-aware (file count and per-file
      overhead, not bytes alone) and can emit an initial safe work batch
      from partial enumeration, then refine as more headers arrive. Note
      `desired_streams` already carries partial shape-awareness today
      (it branches on `file_count`); the engine generalizes this rather
      than inventing it from nothing.
- [ ] The sender owns the dial within the receiver's advertised rich
      capacity profile. The weak end protects itself in both directions
      (strong→weak and weak→strong).
- [ ] The wire contract names the capacity-profile and stream-resize
      fields/messages, their field numbers, and the mixed-version
      behavior before code lands. (Grounded: `DataTransferNegotiation`
      uses fields 1–4 today and reserves 5–10 for RDMA, so
      `CapacityProfile receiver_capacity = 11` is the first free number.)
- [ ] **C-ready by construction, not by retrofit** (restored from v1):
      the dial is a live mutable object read by both ends from the
      live-dials slice onward, and the stream-set is elastic
      (work-stealing, work not pinned to a stream) from the salvage slice
      onward. Continuous mid-transfer stream add/drop (`ue-r2-2`) wires
      the resize proto onto this; it does not restructure the dial or the
      stream-set.
- [ ] Work-stealing is validated as behavior: slow sink, failing sink,
      cancellation, byte accounting, and StallGuard tests stay green.
- [ ] **Pull is not counted in the loopback parity band until PullSync is
      actually multistream through the unified engine.** (PullSync is the
      single-stream path today; the deprecated `Pull` RPC is already
      multistream — see Current Code Reality.)
- [ ] Deprecated `Pull` RPC deletion waits until its multistream/fallback
      pattern has been harvested into PullSync and compatibility/fallback
      tests cover old/new peer pairs.
- [ ] **Loopback parity band:** once pull is multistream, local↔local,
      local→daemon, and daemon→local all measure within a tight band on
      the same hardware (the one-engine property, measured) — no 10×/2×
      gap.
- [ ] The 10 GbE benchmark (`BENCHMARK_10GBE_PLAN.md`) remains the
      sign-off measure for final parity and stream resize, not a
      prerequisite to start the owner-approved coding slices.

## Current Code Reality

*(All references re-verified against `HEAD` `09268eb` on 2026-06-20.)*

The existing code already has useful convergence substrate:

- `TransferSource` (`remote/transfer/source.rs:16`) and `TransferSink`
  (`remote/transfer/sink.rs:44`) define the source/sink seam.
- `execute_sink_pipeline_streaming` (`remote/transfer/pipeline.rs:70`)
  and `execute_receive_pipeline` (`pipeline.rs:200`) are the shared
  byte-moving leaves; `plan_transfer_payloads`
  (`remote/transfer/payload.rs:115`, aliased `plan_push_payloads` on the
  push side) is the payload planner.
- Push already streams manifest/need-list work and feeds the shared sink
  pipeline as work arrives.
- The planner already accounts for workload shape in part: file size
  classes, file count, tar shards, raw bundles, and large-file tasks.
- Cross-run history exists in-tree: `perf_history::PerformanceRecord`
  (`perf_history.rs:135`) is appended per transfer; `perf_predictor`
  (`perf_predictor.rs`) `load()`s it (:220) and maintains per-profile
  coefficients (`HashMap<ProfileKey, PredictorProfile>`, :201). This is
  the substrate for the known-workload replay path in Design §3.

The gaps are above that leaf:

- **Local copy** still owns a local-shaped `TransferOrchestrator`
  (`orchestrator/orchestrator.rs:116`), which builds its own
  multi-thread runtime (`Builder::new_multi_thread().build()`, :137) and
  takes `LocalMirrorOptions` (:134), runs local-only fast paths
  (`orchestrator/fast_path.rs` via `maybe_select_fast_path` →
  `FastPathDecision::{NoWork, Tiny, Huge, …}`), collects all headers, and
  only then plans. **Local does not consume `determine_remote_tuning`** —
  its parallelism comes from this runtime + worker model, so the dial
  must subsume the local parallelism source as well as the remote
  ladders.
- **Three static stream-count ladders exist** (REV3 claimed two and
  called the other two "stale" — that was wrong; all three are live and
  the `tuning.rs` doc comment itself flags the multi-ladder problem):
  1. `remote/tuning.rs::determine_remote_tuning(total_bytes)` (:11) —
     size-keyed `TuningParams`; the client's ladder. Callers:
     `remote/push/client/mod.rs:232`, daemon `pull_sync.rs:500/550/687`,
     daemon `pull.rs:141/261`. Its own doc comment (`tuning.rs:7-10`):
     *"the daemon's push negotiation currently runs its own ladder and
     wins (single-owner consolidation is w2-2)."*
  2. `blit-daemon .../push/control.rs::desired_streams(files)` (:476,
     called :198/:267) — daemon push negotiation, keyed on **both**
     `total_bytes` and `file_count` (e.g. `file_count >= 200_000 → 16`).
  3. `blit-daemon .../pull.rs::pull_stream_count(total_bytes,
     tuning_max)` (:904, called :167/:277) — deprecated Pull RPC,
     byte-keyed, clamped to `tuning.max_streams`.
  These feed the negotiated `DataTransferNegotiation.stream_count`
  (field 4) carried on the wire.
- **Pull is split**: the **deprecated `Pull` RPC** is already
  multistream (client reads `stream_count` and branches on
  `if stream_count <= 1`; daemon computes `pull_stream_count`).
  **PullSync** — the current path — is **single-stream today**:
  `blit-daemon .../pull_sync.rs:568` hardcodes `let stream_count = 1u32`.
  So multistream-pull work (`ue-r2-1g`) is about bringing the deprecated
  Pull's pattern into PullSync through the engine; it is **not** adding
  multistream to a path that never had it.
- The proto has `DataTransferNegotiation.stream_count` (field 4) and
  reserves 5–10 for RDMA (`proto/blit.proto:121-127`); there is no
  capacity profile and no resize messages. Field 11 is the first free
  number.
- `data_plane.rs` carries `StallGuard`/`StallGuardWriter`
  (`remote/transfer/stall_guard.rs:75/139`) today; the `Probe` generic
  (`DataPlaneSession<P: Probe = NoProbe>`) does **not** exist at HEAD —
  it arrives with the PR1 salvage commit (`e6ef095`). The hand-resolved
  conflict at salvage is `StallGuardWriter` vs the `Probe` generic.

## Design

### 1. Engine ownership

Introduce a new `TransferEngine` rather than renaming
`TransferOrchestrator` in place (it is local-shaped: own runtime,
`LocalMirrorOptions`). `TransferOrchestrator` becomes the local adapter
that constructs local `Source`, `Sink`, options, and local strategy
inputs, then calls the engine. Push, PullSync, and delegated transfers
call the same engine with different source/sink and negotiation
adapters.

The engine owns:

- strategy selection (`journal_no_work`, `no_work`, `tiny_manifest`,
  `single_huge_file`, the single-file shortcut, streaming pipeline);
- dial creation and updates (subsuming all three ladders + local
  parallelism);
- payload work queue;
- progress and telemetry wiring;
- invariants around first work, fallback, cancellation, and finish.

Path-specific code remains only at boundaries where the protocol
differs: path resolution, module authorization, source/destination
manifest exchange, mirror/delete authority, and legacy compatibility.

**Engine type (the q3 open question, owner-deferred to the agent):** the
agent recommends the new `TransferEngine` + local adapter above, not an
in-place rename. Ratified at the `ue-r2-1c` slice; owner may override.

### 2. Local fast paths are strategies, not bypasses

V1 said "no separate small-transfer path"; review found that wording
would accidentally delete useful FAST behavior. REV4 keeps the REV2/REV3
rule: the small/huge/no-op local paths may remain, but only as
engine-owned strategies with common accounting and tests. Their real
names are `journal_no_work`, `no_work`, `tiny_manifest`,
`single_huge_file`, plus the single-file copy shortcut
(`orchestrator.rs:178`, which currently bypasses history recording —
folding it under the engine gives it accounting it lacks today).

This preserves SIMPLE for the operator (one command model, one behavior
owner) and FAST (the low-overhead tiny/single-file paths stay where they
beat the full pipeline).

### 3. Streaming plan foundation

The 1s start requirement cannot be hidden inside the sequencer-converge
slice. Today the local path collects all headers before planning, and
PullSync waits for complete client and server manifests before transfer.
REV4 makes streaming planning a separate foundation:

- define an `InitialPlan` / `PlanUpdate` shape produced from a partial
  header stream;
- feed the shared streaming pipeline as safe payload batches become
  available;
- refine tar-shard and bundle targets as enumeration reveals shape;
- preserve mirror/delete and scan-completeness reliability;
- measure first byte and first useful progress per transfer mode.

The initial-plan strategy splits on whether telemetry exists for this
workload shape, reusing the cross-run history already in-tree
(`perf_history` appends a `PerformanceRecord` per transfer;
`perf_predictor` loads it and trains per-profile coefficients):

- **Novel workload** (no extant record): emit a conservative initial
  plan from the partial scan, start copying immediately, and let the live
  tuner (PR1 telemetry, landed with `ue-r2-1a`) adjust from the first
  byte.
- **Known workload** (extant record): if the last run looks optimal,
  reproduce that plan immediately; otherwise recalculate onto the
  live-tune path.

Both branches meet the 1s start. Neither is an exception — the exception
class is only the unsafe-before-full-knowledge modes (mirror/delete,
resume, checksum-refusal) named in the Acceptance Criteria.

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

The rich capacity profile should include, at minimum: CPU cores
available to the transfer; disk class or drain class; current load
estimate; maximum accepted streams; drain-rate estimate; max safe chunk
bytes; max safe prefetch / in-flight bytes. (Owner: more data serves the
ubergoal; do not minimize the negotiation payload.)

The initial dial starts **below** the profile ceiling with margin.
Telemetry can increase or decrease the cheap dials (chunk, prefetch, TCP
buffers) immediately; stream-count changes arrive in the later resize
slice. The dial is a **mutable object read by both ends from the
live-dials slice onward** — this is what makes continuous (`ue-r2-2`) a
wire-up rather than a retrofit (see the C-ready acceptance criterion).

### 5. Wire contract before dependent code

REV4 makes wire shape an early deliverable. Proposed proto direction:

- append `CapacityProfile receiver_capacity = 11` to
  `DataTransferNegotiation` (field 11 is the first free number after the
  5–10 RDMA reservation);
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

Loopback parity is only meaningful after local↔local, local→daemon, and
daemon→local all use comparable engine paths. PullSync is single-stream
today (`pull_sync.rs:568`), so REV4 moves the parity-band gate after pull
multistream lands through the engine.

## Risks

- **Cherry-pick StallGuard regression.** Hand-resolving the
  `data_plane.rs` conflict (`StallGuardWriter` vs the `Probe` generic)
  during salvage could regress the byte-identical property PR2 pins.
  Mitigation: the byte-identical regression tests, the 1370 baseline, and
  the new work-stealing behavior tests in `ue-r2-1a`.
- **Receiver over-advertises its capacity profile.** A receiver claiming
  more drain capacity than it has could overwhelm itself on the first
  byte, because there is no probe phase to catch it. Mitigation: the
  *initial* conservative setting starts below the advertised ceiling with
  margin (fewer streams than allowed, ramping up as telemetry proves the
  link); the live tuner then backs off via `write_blocked` / retransmit
  telemetry. The engine is never exposed at the full advertised ceiling
  on the first byte.
- **1s-start pathological source.** A slow first enumeration over a huge
  directory could blow the 1s budget. Mitigation: the planner yields an
  initial plan from a partial scan and refines; it does not wait for full
  enumeration. Any mode that genuinely cannot meet the budget must be an
  explicit, tested, reported RELIABLE exception — not a silent miss.
- **Wire-compat break with mixed peers.** Adding field 11 and resize
  messages could mispair old/new peers. Mitigation: the wire slice lands
  with explicit old-client/new-daemon and new-client/old-daemon
  compatibility tests *before* any behavior depends on the new fields,
  and resize messages are gated on advertised peer capability bits.
- **RELIABLE-exception loophole.** Allowing tested first-byte exceptions
  risks exceptions proliferating until RELIABLE is silently eroded.
  Mitigation: every exception is explicit, tested, individually reported
  to the owner, and revisited at the `ue-r2-1d` sign-off — none is added
  by coder fiat.
- **Under-converged ladders (new in REV4).** Because REV3 mis-counted the
  ladders, a coder following it would have converged only
  `determine_remote_tuning` and left `desired_streams` and
  `pull_stream_count` as live side doors — re-creating the exact
  "daemon runs its own ladder and wins" split this plan exists to kill.
  Mitigation: the acceptance criterion now enumerates all three plus the
  proto field by `file:line`; `ue-r2-1e`/`ue-r2-1f`/`ue-r2-1g` each name
  the ladder they retire.

## Revised Slices

Review-loop-sized docs/code slices; one coherent, testable change each.
They replace the v1 slice shape only if the owner accepts REV4 as the
final plan. Slice IDs are unchanged from REV3 (the slice *shape* did not
change; only the code-reality grounding did).

1. **`ue-r2-1a-salvage-substrate`** — Cherry-pick adaptive PR1+PR2 up to
   `eafb187` (`e6ef095` → `af66ff5` → `b797b73` → `eafb187`), excluding
   `d9d4ec7` (PR3 WIP, does not build). Resolve `data_plane.rs`
   `StallGuardWriter`-vs-`Probe`. Treat work-stealing as behavior, not
   inert substrate: add/keep slow-sink, failing-sink, cancellation,
   byte-accounting, StallGuard, and byte-identical tests. The elastic
   work-stealing stream-set exists from this slice onward (C-ready seam).
2. **`ue-r2-1b-wire-dial-contract`** — Define capacity profile, peer
   capability, and resize proto shape (`receiver_capacity = 11`,
   `DataPlaneResize`/`Ack`). Add compatibility tests for old client/new
   daemon and new client/old daemon. No behavior depends on these fields
   until this slice is green.
3. **`ue-r2-1c-engine-shell-local-adapter`** — Add `TransferEngine` and
   convert `TransferOrchestrator` into a local adapter. Move the local
   fast paths (`journal_no_work`, `no_work`, `tiny_manifest`,
   `single_huge_file`, single-file shortcut) under engine-owned
   strategies, preserving behavior and adding accounting where the
   single-file shortcut lacked it.
4. **`ue-r2-1d-streaming-plan-foundation`** — Introduce partial-scan
   initial plans and plan updates (novel vs known per Design §3). Prove
   first-byte / first-useful-work timing for local and push shapes, and
   document any RELIABLE exception that cannot safely move bytes before
   complete knowledge.
5. **`ue-r2-1e-live-cheap-dials`** — Replace the static
   `determine_remote_tuning` chunk/prefetch/TCP-buffer ladder with the
   single mutable dial. Start conservative within the receiver profile,
   then adjust cheap dials from PR1 telemetry. The dial is a mutable
   object read by both ends from this slice onward (C-ready seam).
6. **`ue-r2-1f-push-converge`** — Route push through the engine while
   preserving manifest streaming, need-list batching, fallback timing,
   scan-completeness purge safety, and old/new compatibility. **Retire
   the daemon `desired_streams` ladder** into the dial (this is the
   ladder `tuning.rs` says currently "wins").
7. **`ue-r2-1g-pull-multistream-converge`** — Route PullSync through the
   engine and make it multistream there (it is single-stream today),
   harvesting the deprecated Pull RPC's multistream pattern. Preserve
   resume, checksum refusal, delete-list authority, cancellation,
   per-stream failure, and gRPC fallback behavior. Absorbs
   `MULTISTREAM_PULL.md` acceptance criteria.
8. **`ue-r2-1h-delete-deprecated-pull-rpc`** — Delete the deprecated
   `Pull` RPC (and its `pull_stream_count` ladder) after PullSync has
   harvested the needed multistream/fallback pattern and tests cover the
   replacement, including old/new peer pairs.
9. **`ue-r2-2-stream-resize`** — Finish negotiated
   `DataPlaneResize`/`DataPlaneResizeAck` and add/drop streams mid
   transfer from live telemetry, using the elastic work queue from
   `ue-r2-1a`. Wires onto the already-mutable dial and elastic
   stream-set — a wire-up, not a restructuring.

### Slice dependencies

Explicit blocking, since REV4 has nine slices and ordering matters:

- `1a` (substrate) blocks everything — all later slices build on the
  elastic work-stealing stream-set.
- `1b` (wire contract) blocks `1e` (dial uses the capacity profile), `2`
  (resize proto), and contributes the compat tests `1h` gates on.
- `1c` (engine shell) blocks `1d`, `1e`, `1f`, `1g` — the streaming plan,
  dial, push, and pull all run inside the engine.
- `1d` (streaming plan) blocks the 1s-start acceptance and feeds `1g`
  (pull 1s-start).
- `1f` (push) and `1g` (pull) both depend on `1c`; `1g` also depends on
  `1d`. `1f` retires `desired_streams`; `1g`/`1h` retire
  `pull_stream_count`; `1e` retires the `determine_remote_tuning` ladder.
  All three ladders are gone only once `1e`+`1f`+`1g`+`1h` land.
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
  redefined them as engine strategies, **with their real names**
  (`journal_no_work`, not `journal_skip`; no distinct `single_file`
  strategy — it is a copy shortcut at `orchestrator.rs:178`).
- "No behavior change" on salvage was inaccurate for work-stealing:
  tests now treat scheduling as observable.
- Proto compatibility was too implicit: wire shape is now a first-class
  early slice (`receiver_capacity = 11` grounded against the proto).
- Pull parity gate was too early: moved until after PullSync multistream.
- **REV3's "two static tables, not three" correction was itself wrong
  (corrected in REV4).** All three ladders are live —
  `determine_remote_tuning`, `desired_streams` (control.rs:476),
  `pull_stream_count` (pull.rs:904) — and `tuning.rs`'s own doc comment
  confirms the daemon "runs its own ladder and wins". REV3 also said
  `determine_remote_tuning` drives "local and push"; it drives push +
  daemon pull, **not** local (local has no caller). And REV3's
  "pull is single-stream via force_grpc, not a third ladder" conflated
  the deprecated multistream `Pull` RPC with single-stream PullSync.
- REV2 dropped the Risks section and the "C-ready by construction"
  acceptance criterion; REV3 restored both; REV4 keeps them and adds the
  "under-converged ladders" risk.
- `DECISIONS.md` D-2026-06-20-1 still carries superseded warmup/size-gate
  language; D-2026-06-20-2 and this Draft carry the live-from-first-byte
  correction. If REV4 is accepted, either edit D-2026-06-20-1 with a note
  or add a superseding cleanup decision.

## Open Questions for Final Owner Decision

- **(RESOLVED 2026-06-20, owner)** First-byte timing: hard invariant for
  every mode *except* the unsafe-before-full-knowledge modes
  (mirror/delete, resume, checksum-refusal). Novel vs known workload is a
  tuning-strategy choice, not an exception. Recorded in Acceptance
  Criteria and Design §3.
- **(RESOLVED 2026-06-20, owner)** Deprecated `Pull` deletion stays
  in-plan as `ue-r2-1h`, sequenced last, gated on `ue-r2-1g` + the
  `ue-r2-1b` compat tests.
- **(RESOLVED 2026-06-20, owner — D-2026-06-20-5)** REV4 **replaces**
  `UNIFIED_TRANSFER_ENGINE.md`. v1 is Superseded; REV2/REV3 are
  Superseded-by-REV4. v1's lineage/absorption is carried into REV4's
  header, so nothing is lost. One Active plan; no candidates left live.
- Should D-2026-06-20-1 be edited now to remove the superseded
  warmup/size-gated wording, or should the later decisions remain the
  correction?
  *Agent recommendation: edit now with a one-line superseded note
  pointing at -2/-4.* Per AGENTS.md §1, fix the lower-precedence doc when
  it disagrees, rather than leaving stale wording a future reader might
  implement.
