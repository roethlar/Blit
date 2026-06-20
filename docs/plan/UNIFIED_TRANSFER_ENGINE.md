# Unified Transfer Engine — one src/dst-agnostic sequencer, live dials

**Status**: Active
**Created**: 2026-06-20
**Activated**: 2026-06-20 (owner approved the four bound parameters;
D-2026-06-20-2)
**Supersedes**: the "ground-up redesign" framing of the 2026-06-14 open
question (resolved as *convergence*, not rebuild — D-2026-06-20-1).
Continues the lineage of `PIPELINE_UNIFICATION.md` and
`UNIFIED_RECEIVE_PIPELINE.md` (both **Historical**): those landed the
shared byte-moving leaf but never converged the sequencer+dials layer
above it, which is why the fragmentation this plan fixes still exists.
**Absorbs** `MULTISTREAM_PULL.md` (w2-3) as slice `ue-1d` (goal survives,
path-specific premise does not). The engine's planner is workload-shape
-aware and meets a first-byte-within-~1s commitment (Design §2) on its own
merits — it is **not** the separate H10b streaming-planner concept and does
not supersede D-2026-06-04-3 (owner vetoed that merger — D-2026-06-20-3).
**Decision ref**: D-2026-06-20-1 (direction), D-2026-06-20-2 (approval +
bound parameters), D-2026-06-20-3 (H10b veto).

## Goal

One transfer engine owns all four src/dst combinations — local↔local,
local→remote (push), remote→local (pull), daemon↔daemon (delegated).
One sequencer decides stream count and every transfer dial, adjusts them
**live from measured telemetry**, and drives the already-shared
byte-moving pipeline underneath. Where the human issues the command is
irrelevant to how bits move.

When this is done there is **one place to change transfer behavior**. No
path-specific orchestration loops, no three competing static stream-count
tables. The class of bug where local↔local was 10× slower than
local→daemon (which was 2× faster than daemon→local) becomes impossible
by construction, not by vigilance.

Governed by the three design goals: **FAST** (measured dials + work-stealing
+ no overhead on small transfers), **SIMPLE** (one engine, one dial, one
place to change), **RELIABLE** (the weak end self-protects via the
ceiling; green salvaged code; byte-identical transfers preserved).

## Non-goals

- **Not a ground-up rebuild.** The shared leaf pipeline
  (`execute_sink_pipeline_streaming` / `execute_receive_pipeline`) and the
  payload planner (`plan_transfer_payloads`) stay. This converges the layer
  *above* them (sequencer + dials), where the fragmentation lives.
- **Zero-copy receive** stays out (D-2026-06-12-1 deleted it; revisit gated
  on the 10 GbE benchmarks).
- **gRPC fallback path** stays single-logical-stream by design (unchanged
  from w2-3's non-goal).
- **No probe-then-go phase.** There is no "measure first, then move"
  warmup stage that blocks the start. Measurement happens *during* the
  transfer and feeds the dial live from the first byte. (The earlier
  "A-first warmup probe" framing is superseded by this — see Design §2.)

## Constraints

- Wire changes allowed (proto unfrozen, D-2026-06-11-1), but an old client
  ↔ new daemon pair (either direction) must degrade gracefully to today's
  behavior — negotiation, not assumption.
- **First byte within ~1 second.** The engine must begin moving data
  within roughly one second of invocation, regardless of transfer shape —
  it must not block on full enumeration or on a tuning probe before
  moving. This is the replacement for the size-gated fast-path (see Design
  §2) and is the planner's streaming commitment.
- **The planner is workload-shape-aware.** Transfer cost is not a function
  of total bytes alone: 100,000 × 10-byte files are a different (harder)
  problem than one 20 MB file. The planner must account for file count and
  per-file overhead when shaping payloads and parallelism, not just byte
  totals.
- No regression of: StallGuard coverage, byte-progress accuracy (design-1
  class), resume semantics, or the **byte-identical transfer** property the
  adaptive PR2 verification pinned.
- Windows parity; new tests ungated unless genuinely platform-specific.
- The 1370-test workspace baseline must not drop.
- **Every stage serves FAST, SIMPLE, or RELIABLE** — no stage serves only
  one. A change that is purely structural with no goal payoff is out.

## Acceptance criteria

- [ ] All four src/dst paths route through **one sequencer**; `push` and
      `pull` no longer contain path-specific orchestration loops (verified:
      the sequencer — `TransferOrchestrator` renamed or a successor — is the
      sole entry; push/pull clients hand it a Source and a Sink and say go).
- [ ] The three static stream-count ladders (`remote/tuning.rs`,
      `push/control.rs::desired_streams`, `pull.rs::pull_stream_count`) are
      replaced by **one dial source** owned by the engine.
- [ ] **First byte within ~1 second**, measured from invocation, across
      transfer shapes (single large file; many tiny files; mixed) — the
      engine does not block on full enumeration or a tuning probe. No
      separate "small-transfer" path; the same engine handles all sizes.
- [ ] **Dials adjust live from the first byte**, not probe-then-set: PR1
      telemetry (bytes, write-blocked time, stream state, Linux TCP_INFO)
      feeds the dial during the transfer and the dial moves in response —
      no static size→streams table remains in the path.
- [ ] **Bounded-unilateral:** the receiver advertises a **rich capacity
      profile** (CPU cores, disk class, current load, max streams, drain
      estimate — more data serves the tool's overkill/ubergoal, per owner);
      the sender owns the dial and adjusts within it. An asymmetric-hardware
      test (strong initiator ↔ weak receiver, and the reverse) shows the
      weak end is not overwhelmed and the transfer completes at the weak
      end's real capacity.
- [ ] **Planner is workload-shape-aware:** file count and per-file overhead
      shape payload sizing and parallelism, not just total bytes — a
      100k×10-byte transfer is planned differently than one 20 MB file,
      and both start within the 1s commitment.
- [ ] **C-ready by construction, not by retrofit:** the dial is a live
      mutable object read by both ends from day one, and the stream-set is
      elastic (work-stealing, work not pinned to a stream). Continuous
      mid-transfer stream add/drop (`ue-2`, in scope) wires the resize proto
      onto this; it does not restructure the dial or the stream-set.
- [ ] Existing remote / parity / resume suites stay green; byte-identical
      transfers preserved; 1370-test baseline does not drop.
- [ ] **Loopback parity band:** local↔local, local→daemon, daemon→local all
      measure within a tight band on the same hardware (the one-engine
      property, measured) — no 10×/2× gap.
- [ ] Owner sign-off waits for the 10 GbE rig (`BENCHMARK_10GBE_PLAN.md`):
      all three directions at parity on the matrix, or the gap explained.
      This is also the gate for `ue-2` (continuous).

## Design

Two seams + one convergence. The byte-mover at the bottom is already shared
(map confirmed 2026-06-20); we converge the layer above it.

### 1. The engine — sequencer convergence

Today `TransferOrchestrator` owns only local copy; push and pull hand-wire
their own control loops in `RemotePushClient::push`
(`remote/push/client/mod.rs`) and `RemotePullClient::pull`
(`remote/pull.rs`), bypassing the orchestrator. Lift a **src/dst-agnostic
sequencer** that takes a `Source` and a `Sink` (trait impls: local fs,
remote socket, daemon) plus a `Plan` and a `Dial`, and runs. Path
differences become *inputs* to the sequencer, not separate code. The
shared leaf (`execute_sink_pipeline_streaming` / `execute_receive_pipeline`)
and `plan_transfer_payloads` stay as the engine's bottom.

Affected: `crates/blit-core/src/orchestrator/`, `remote/push/client/`,
`remote/pull.rs`, `blit-daemon/src/service/{push,pull}_sync.rs`.

**Engine type (the q3 open question, owner-deferred to the agent):** the
agent recommends introducing a new src/dst-agnostic `TransferEngine`
(takes `Source` + `Sink` + `Plan` + `Dial`) as the one sequencer, and
reducing `TransferOrchestrator` to the local entry point that constructs
local `Source`/`Sink` and calls the engine — *not* an in-place rename,
because the current `TransferOrchestrator` is local-shaped (builds its own
runtime, takes `LocalMirrorOptions`). Ratified at the `ue-1c` slice; owner
may override.

### 2. The dials + the planner — start-within-1s, bounded-unilateral, live

Collapse the three static ladders into **one dial object** owned by the
engine, and split the "how should this transfer behave" question into a
**planner** half (workload shape) and a **tuner** half (live dials).

- **Start within 1s, conservative, then adapt — no probe phase.** The
  engine does not measure-then-move. It begins moving bytes within ~1s at
  conservative defaults **bounded by the receiver's advertised ceiling**,
  and the tuner adjusts the dial live from the first byte as PR1 telemetry
  arrives. This **obviates the small-transfer threshold entirely**: there
  is no probe to skip and no size gate — every transfer starts fast and
  adapts. (Supersedes the earlier "A-first warmup probe" framing and the
  "size-gated skip-probe" mechanism.)
- **Bounded-unilateral, with a rich capacity profile.** The **receiver
  advertises a capacity profile** — CPU cores, disk class, current load,
  max streams, drain-rate estimate. The owner's principle: the app is
  deliberately overkill (a model-capability test as much as a tool), so
  *more data serves the ubergoal* — advertise a real profile, not a
  single number. The **sender owns the dial and adjusts it live, but cannot
  exceed the receiver's ceiling.** Handles asymmetric hardware both
  directions: a 32-core Threadripper pulling from an underpowered UNAS-8
  Pro respects the UNAS's advertised ceiling; a UNAS pulling from a
  Threadripper sets its own ceiling and drives within it. Negotiation-lite:
  one profile exchanged at setup, then one decider. SIMPLE (no haggling
  loop), handles asymmetry, FAST (one round-trip), RELIABLE (the weak end
  protects itself).
- **The planner is workload-shape-aware.** Transfer cost is not total
  bytes alone: 100,000 × 10-byte files differ from one 20 MB file. The
  planner accounts for file count and per-file overhead when shaping
  payloads and parallelism, and it yields an initial plan from a partial
  scan fast enough to meet the 1s start commitment, refining as
  enumeration completes — it does not block on full enumeration before
  moving. This is an engine-internal requirement stated on its own merits;
  it is **not** the separate H10b streaming-planner concept and does not
  supersede D-2026-06-04-3 (owner vetoed that merger — D-2026-06-20-3).
- **Live object from day one.** The dial is a mutable object read by both
  ends from `ue-1b` onward. `ue-1b` starts conservative within the ceiling
  and adjusts the cheap dials (chunk, prefetch, TCP buffers) live from
  telemetry; `ue-2` adds mid-transfer stream add/drop via the resize
  proto. No retrofit — the mutable dial and the elastic (work-stealing)
  stream-set exist from `ue-1b`.
- The capacity-profile + resize fields reuse the capability/negotiation
  proto scaffolding from adaptive PR3 (`d9d4ec7`, referenced only — not
  cherry-picked); finished cleanly here.

### 3. The substrate — salvage (D-2026-06-07-2)

Cherry-pick the adaptive-streams stack **up to `eafb187`** onto current
master: PR1 per-stream telemetry with zero-cost `Probe` (`e6ef095`) → PR2
work-stealing pipeline queue (`af66ff5`) → PR2 review fix (`b797b73`) →
`eafb187`. **Exclude `d9d4ec7`** (PR3 WIP, does not build). Hand-resolve
the known `data_plane.rs` conflict (`StallGuardWriter` vs the `Probe`
generic, flagged in D-2026-06-07-2).

- **PR1** is the measurement source that feeds the live dial (bytes sent,
  write-blocked time, stream state; Linux `TCP_INFO`).
- **PR2** is both a perf win (a slow sink can no longer head-of-line-block
  the others) **and the C-enabling seam**: because work is not pinned to
  a stream, mid-transfer add/drop is natural, not a rewrite.

### Live from the first byte — staged by what adjusts, not by a probe phase

The owner's call (q1/q4): there is no warmup-then-go phase. The engine
starts moving within 1s at conservative defaults bounded by the receiver
ceiling, and the tuner adjusts **live from the first byte** as PR1
telemetry streams in. The staging is by *what gets adjusted*, not by a
probe-to-continuous progression:

- **`ue-1b` — cheap dials live:** chunk size, prefetch, TCP buffers move
  in response to in-flight telemetry, within the receiver's ceiling. This
  already delivers "tuned live" — no static table remains.
- **`ue-2` — stream count live (in scope at Active):** mid-transfer
  add/drop of streams via PR3's `DataPlaneResize`/`Ack`, riding the
  elastic work-stealing stream-set from PR2. This is the genuinely hard
  piece; it is sequenced after the foundation slices because it needs the
  converged engine + the finished resize protocol, not because it is
  optional.

The two expensive-to-retrofit pieces — a mutable dial read by both ends,
and an elastic stream-set — exist from `ue-1b`. That is the answer to
"does starting simple paint us into a corner for continuous?": **no**,
because we build on the adaptive substrate that was itself designed as
the continuous controller's foundation. The 10 GbE rig is the **sign-off
measure** for both, not a gate that blocks starting `ue-2` (owner: 11
months of benchmarking is the justification).

### Risks

- The hand-resolved cherry-pick conflict could regress StallGuard.
  Mitigation: the byte-identical regression tests PR2 already pins, plus
  the 1370 baseline.
- A receiver that over-advertises its capacity profile (claims more than
  it can drain). The live tuner catches this via `write_blocked` / retransmit
  telemetry and backs off. Because there is no probe phase, the *initial*
  conservative setting must already respect the ceiling with margin
  (start fewer streams than the profile allows, ramp up as telemetry
  proves the link) — the engine is never exposed at the full advertised
  ceiling on the first byte.
- The 1s start commitment is a hard target on the planner's streaming
  path; a pathological source (slow first enumeration, huge directory)
  must still produce an initial plan within budget. Mitigation: the
  planner yields an initial plan from a partial scan and refines — it
  does not wait for full enumeration.
- *(Removed: the flagged inference to fold in the H10b streaming planner
  was vetoed by the owner — D-2026-06-20-3. Workload-shape-awareness stands
  alone.)*

## Slices

Review-loop-sized; one coherent, testable change each. All slices
(`ue-1a`–`ue-1e`, `ue-2`) are in the Active scope; `ue-2` is sequenced
last because it needs the converged engine + finished resize protocol,
not because it is optional.

1. **`ue-1a-salvage`** — cherry-pick adaptive PR1+PR2 (up to `eafb187`)
   onto current master per D-2026-06-07-2; hand-resolve `data_plane.rs`
   StallGuard-vs-`Probe`; verify byte-identical + 1370 baseline + the new
   work-stealing test. **No behavior change** — substrate in tree only.
2. **`ue-1b-live-dial`** — introduce the single mutable dial object
   replacing the three static ladders; engine starts within 1s at
   conservative defaults bounded by the receiver's capacity profile and
   adjusts the **cheap dials** (chunk, prefetch, TCP buffers) live from
   first-byte PR1 telemetry. Rich capacity-profile field in proto +
   receiver-side computation; sender bounds to it with margin. Compat:
   absent/zero profile → today's behavior.
3. **`ue-1c-sequencer-converge`** — introduce the src/dst-agnostic
   `TransferEngine` (per the engine-type recommendation above); `Source`/
   `Sink` traits; route push through it (replace the push client's
   hand-wired loop); pull follows; `TransferOrchestrator` becomes the local
   adapter. Make the planner **workload-shape-aware** (file count vs bytes)
   and **first-byte-within-~1s** (partial-scan initial plan, refine) — an
   engine-internal behavior, not the H10b concept (D-2026-06-20-3). Verify
   the one-entry property + loopback parity band.
4. **`ue-1d-pull-multistream`** — pull gains multi-stream through the
   unified sequencer (the w2-3 goal, now via the engine not a path-specific
   hack). Absorbs `MULTISTREAM_PULL.md` acceptance criteria: negotiation,
   per-stream failure, cancellation mid-transfer, old↔new compat.
5. **`ue-1e-delete-pull-rpc`** — w2-4: delete the deprecated Pull RPC now
   that its multi-stream pattern is harvested into the engine.
6. **`ue-2-stream-resize`** — wire PR3's `DataPlaneResize`/`Ack`; add/drop
   streams mid-transfer from live telemetry, riding the elastic
   work-stealing stream-set. **In scope at Active** (owner: 11 months of
   benchmarking is the justification); 10 GbE is the sign-off measure, not
   a gate. Sequenced after `ue-1c` because it needs the one engine.

## Open questions

- **(RESOLVED — q1)** Small-transfer threshold — **obviated.** No probe
  phase and no size gate; the 1s-start + live-adjust model handles all
  sizes through one engine. The planner carries the workload-shape
  judgment (file count vs bytes) that the old size threshold was a proxy
  for.
- **(RESOLVED — q2)** Capacity-profile shape — **rich profile**
  (CPU cores, disk class, load, max streams, drain estimate). Owner: more
  data serves the ubergoal; do not minimize the negotiation payload.
- **(RESOLVED — q4)** `ue-2` (stream resize) — **in scope at Active**,
  sequenced last. 10 GbE is sign-off, not a gate.
- **(RESOLVED — veto, D-2026-06-20-3)** The agent's flagged inference to
  fold the H10b streaming planner (D-2026-06-04-3) into the engine was
  **vetoed by the owner.** D-2026-06-04-3 stands unchanged; the engine's
  workload-shape-awareness + 1s-start requirements stand alone, not as the
  H10b concept.
- **Engine type (q3, owner-deferred to agent)** — agent recommends a new
  `TransferEngine` + local adapter over renaming `TransferOrchestrator`
  in place (see Design §1). Ratified at the `ue-1c` slice; owner may
  override.