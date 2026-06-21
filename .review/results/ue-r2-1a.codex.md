OpenAI Codex v0.141.0
--------
workdir: /home/michael/dev/blit_v2
model: gpt-5.5
provider: headroom
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019ee812-0520-7882-9b31-fc6837b9a04d
--------
user
Focused code review of ONE slice in the Blit repo (Rust). The slice is commits 515fb76..HEAD (4 commits): adaptive PR1 per-stream telemetry "Probe", PR2 shared work-stealing pipeline queue, the PR2 forwarder-halt-on-error fix, and a tests commit. Plan: docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md (slice ue-r2-1a); finding doc: .review/findings/ue-r2-1a.md.

First read the diff and surrounding code:
  git diff 515fb76..HEAD
then read crates/blit-core/src/remote/transfer/{data_plane.rs,pipeline.rs,progress.rs,sink.rs,tcp_info.rs,mod.rs} as needed.

Review focus:
1. Conflict resolution in data_plane.rs: it composes master's StallGuardWriter<TcpStream> stream with PR1's generic DataPlaneSession<P: Probe = NoProbe>. Is the stall guard still in effect on EVERY write path? Did the merge silently drop any prior behavior?
2. Work-stealing queue (pipeline.rs execute_sink_pipeline_streaming): any payload loss, duplication, or byte/file accounting error? Is first-error-wins / prompt shutdown correct via the shared `cancelled` flag? Is back-pressure preserved (bounded queue = prefetch*sink_count)?
3. NoProbe "zero cost": does the `if P::ACTIVE` gating actually compile out for DataPlaneSession<NoProbe> (no clock reads, no-op probe calls)?
4. The two new tests in pipeline.rs workqueue_tests (byte_and_file_totals_correct_under_work_stealing, producer_cancel_winds_down_pipeline_promptly): valid, or false-positive / flaky?
5. Any regression vs pre-slice behavior.

Already known and intentionally deferred — do NOT report as blockers, but say so if you think the deferral is wrong: (a) hard-abort of in-flight workers on dropping the pipeline future (workers are bare tokio::spawn) → deferred to w4-1 AbortOnDrop; (b) PR1 write_blocked_nanos times the whole tokio::join!(write,read) step → deferred to ue-r2-1e where the telemetry is first consumed.

Output ONLY a concise markdown list of findings — each with file:line, severity (Critical/High/Medium/Low), one-line rationale + concrete fix — then a final line "VERDICT: ship | fix-then-ship | block". Be concise; do not invoke any skills.

codex
I’ll read the required repo state and review inputs, then keep the final response to findings only as requested.
exec
/usr/bin/bash -lc "sed -n '1,220p' /home/michael/dev/blit_v2/.agents/skills/catchup/SKILL.md" in /home/michael/dev/blit_v2
exec
/usr/bin/bash -lc "sed -n '1,260p' docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md" in /home/michael/dev/blit_v2
 succeeded in 0ms:
---
name: catchup
description: Re-ground in current Blit project state from docs/STATE.md before doing any work. Use at session start or whenever context seems stale or was compacted.
---
Read docs/agent/PROTOCOL.md and execute the **catchup** procedure exactly.

 succeeded in 0ms:
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

exec
/usr/bin/bash -lc "sed -n '1,220p' docs/STATE.md" in /home/michael/dev/blit_v2
 succeeded in 0ms:
# STATE — single entry point for "what is true right now"

Last updated: 2026-06-20 (code→review→fix loop established, D-2026-06-20-6;
REV4 Active, D-2026-06-20-5) at commit `b663091` (loop-setup docs
uncommitted in working tree)

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **Transfer-core architecture conflict RESOLVED** (D-2026-06-20-1):
  convergence, not ground-up redesign. One src/dst-agnostic sequencer owns
  all four paths (local↔local, push, pull, daemon↔daemon); one live dial
  (streams + knobs) replaces the three static ladders, **bounded-unilateral**
  (receiver capacity profile + sender controls within it). **No probe
  phase** — the engine starts within ~1s at conservative defaults and the
  tuner adjusts dials live from the first byte; the planner is
  workload-shape-aware (file count vs bytes). Adaptive-streams PR1+PR2 (up
  to `eafb187`) salvaged as the substrate per D-2026-06-07-2; PR3 WIP
  `d9d4ec7` excluded.
- **Plan decided — REV4 is the Active convergence plan** (owner: "rev4
  replaces v1"; D-2026-06-20-5). `UNIFIED_TRANSFER_ENGINE_REV4.md` is
  **Active**; v1, REV2, and REV3 are **Superseded**. The D-2026-06-20-4
  coding freeze is lifted **as to the plan decision** — but per
  AGENTS.md §9 no code slice starts without a fresh per-slice owner
  authorization (next: `ue-r2-1a`). REV4 is REV3 with its
  code-reality section corrected against the tree at `HEAD`. REV3's
  headline "two static tables, not three" correction was itself **wrong**
  — all three stream-count ladders are live (`determine_remote_tuning`,
  `desired_streams` `push/control.rs:476`, `pull_stream_count`
  `pull.rs:904`); v1's three-ladder count was right; `tuning.rs`'s own
  doc comment confirms "the daemon's push negotiation runs its own ladder
  and wins". REV4 also re-scopes the pull single-stream claim to PullSync
  (deprecated `Pull` is already multistream), fixes strategy names
  (`journal_no_work`, not `journal_skip`; no distinct `single_file`),
  grounds every symbol with `file:line`, and carries v1's
  lineage/absorption header forward so it can stand as plan-of-record.
  (The D-2026-06-20-4 freeze that gated this is now resolved by the
  owner's decision.) REV4 keeps convergence and the four bound
  parameters from D-2026-06-20-2, but tighten slice shape: streaming
  initial planning gets its own slice, local fast paths become
  engine-owned strategies, work-stealing is treated as observable
  behavior, proto capacity/resize compatibility is designed before
  dependent code, and pull parity waits for multistream PullSync.
- **Code→review→fix loop established** (D-2026-06-20-6;
  `docs/agent/GPT_REVIEW_LOOP.md` Active): for `ue-r2-*` slices Claude
  codes+commits each slice, GPT-5.5 (`codex`, confirmed headless here via
  the local `headroom` proxy) reviews the commit, Claude adjudicates
  findings against source/tests, fixes accepted ones, proceeds.
  Per-slice commits to `master` are **ungated** (no branches, never
  push); per-slice code acceptance is owner-delegated (not a gate — the
  owner is not a developer). Async `.review/` sentinels dropped for this
  loop; `findings/`+`results/` records reused. Owner gates remaining:
  push, 10 GbE sign-off. Baseline: `cargo test --workspace` 1370 passed.

## Queue (ordered)

1. **Begin `ue-r2-1a` (salvage substrate)** — the code→review→fix loop
   is established (D-2026-06-20-6); awaiting the owner's single "go" to
   start coding. Once given, the loop runs autonomously per
   `GPT_REVIEW_LOOP.md` (no further per-slice gates): `ue-r2-1a` =
   cherry-pick adaptive PR1+PR2 up to `eafb187`, resolve the
   `data_plane.rs` StallGuard-vs-`Probe` conflict, add work-stealing
   behavior tests. Also pending separately: push approval for the Windows
   test-tuning commit (`439a2a7`, local-only — Windows CI red until it
   lands).
2. **Then** execute the rest of the REV4 slice list in order —
   `ue-r2-1b` → `1c` → `1d`/`1e`/`1f` → `1g` → `1h` → `ue-r2-2`
   (deps in REV4 §"Slice dependencies"), each through the GPT review loop.
3. **Design-review queue (independent, survives the convergence)** —
   `REVIEW.md` order governs. Highest open ratified row is w4-1
   (AbortOnDrop family, High); next include w4-3 and W1 socket-policy /
   timeout constants. These are correctness findings, unaffected by the
   engine convergence — may be folded into `ue-1c` or fixed standalone,
   owner's call.
4. **10 GbE benchmark session — DEFERRED** (owner 2026-06-12: rig assembly
   is real work; benchmarking pre-convergence is churn). Now also the
   `ue-1` sign-off measure (loopback parity band: local↔local /
   local→daemon / daemon→local within a tight band) AND the gate for
   `ue-2` (continuous/C). Capture before/after baselines there, not earlier.
   Remains the zero-copy revisit gate (D-2026-06-12-1). After `ue-1`:
   audit Round 1, TUI rework (Round 2), H10b streaming planner.

## Authoritative docs right now

- **Active plan: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** —
  convergence engine; flipped Active by D-2026-06-20-5 ("rev4 replaces
  v1"). Carries forward v1's absorption of `MULTISTREAM_PULL.md` (the
  pull-multistream goal lands as slice `ue-r2-1g`).
- Superseded by REV4 (history only, do not implement):
  `UNIFIED_TRANSFER_ENGINE.md` (v1), `…_REV2.md`, `…_REV3.md`. REV4 = REV3
  with the code-reality section corrected against `HEAD` (REV3's "two
  tables, not three" ladder claim was wrong — all three ladders are
  live), pull single-stream re-scoped to PullSync, strategy names fixed,
  every symbol grounded with `file:line`.
- Code→review→fix loop: `docs/agent/GPT_REVIEW_LOOP.md` (Active,
  D-2026-06-20-6) — governs `ue-r2-*` slices (codex/GPT-5.5 reviews each
  commit); the `.review/README.md` async two-agent loop still governs all
  other work.
- Design queue: `REVIEW.md` (13 design-queue rows `[x]`, 0 rows `[~]`) + the three
  `docs/audit/` 2026-06-11 deliverables
- Review loop: `REVIEW.md` + `.review/README.md` + `.review/findings/` +
  `.review/results/` (ready queue empty)
- Other plans: `docs/plan/ZERO_COPY_RECEIVE_EVAL.md` (delete ratified
  D-2026-06-12-1, executes in w8-1), `docs/plan/TUI_REWORK.md` (gated on
  Round 1), `docs/plan/BENCHMARK_10GBE_PLAN.md` (`ue-1` sign-off + `ue-2`
  gate)
- Findings: `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md` (R3 governs)
- Decisions: D-2026-06-20-1 (convergence direction), D-2026-06-20-5
  (REV4 replaces v1 as Active; v1/REV2/REV3 Superseded; plan-decision
  freeze lifted), D-2026-06-20-6 (code→review→fix loop; ungated per-slice
  commits; per-slice code acceptance owner-delegated)

## Blocked / waiting

- **Owner**: (1) single "go" to start coding `ue-r2-1a` — process is
  established (D-2026-06-20-6); once given, the loop runs autonomously
  with no further per-slice gates; (2) push approval for the Windows
  test-tuning commit (`439a2a7`, local-only — Windows CI red until it
  lands).

## Open questions

- **(RESOLVED 2026-06-20, D-2026-06-20-1 / -2)** Transfer-core architecture
  — convergence per `UNIFIED_TRANSFER_ENGINE.md` (Active). Closed.
- **(RESOLVED — veto, D-2026-06-20-3)** Agent's flagged inference to fold
  the H10b streaming planner into the engine — **vetoed by owner.**
  D-2026-06-04-3 stands unchanged; engine's workload-shape-awareness +
  1s-start stand alone.
- **Engine type** — deferred to agent (recommends new `TransferEngine` +
  local adapter); ratified at `ue-r2-1c`, owner may override.
- `UNIFIED_TRANSFER_ENGINE` plan-review decisions (2026-06-20, owner;
  `REV4.md` is now the **Active** plan — D-2026-06-20-5; REV3's ladder
  "correction" was wrong, all three ladders are live):
  - **(RESOLVED)** First-byte-within-~1s is a hard invariant for every
    mode except the modes where moving any byte before full knowledge
    would be unsafe (mirror/delete, resume, checksum-refusal). Novel vs
    known workload is a tuning-strategy choice (start-something-and-tune
    vs replay-optimal-last-run via the in-tree `perf_history`/
    `perf_predictor`), not an exception. Both meet 1s.
  - **(RESOLVED)** Deprecated `Pull` deletion stays in-plan as
    `ue-r2-1h`, gated on `ue-r2-1g` + `ue-r2-1b` compat tests.
  - **(RESOLVED 2026-06-20, owner — D-2026-06-20-5)** REV4 **replaces**
    v1. `UNIFIED_TRANSFER_ENGINE.md` Superseded; REV2/REV3
    Superseded-by-REV4; REV4 is the one Active plan.
  - **(OPEN)** Edit D-2026-06-20-1 now to strip superseded
    warmup/size-gate wording, or let later decisions stand? Owner: not
    sure.
- **(RESOLVED 2026-06-20)** SETUP.md — the generic two-agent guide
  predates this governance; owner is folding it into the governance as a
  playbook (re-applied on governance setup), so it is **not** kept in-repo
  (removed). `.review/README.md` lines 8/101 repo-relativized and the
  dangling SETUP pointer removed. Remaining: historical audit/finding docs
  (`audit-13/14/15`, `drift-*`) still embed `/Users/...` in recorded
  evidence — left as-is pending an owner call on whether to scrub.
- Disposition of adaptive-streams branch refs after `ue-1a` lands
  (D-2026-06-07-2).
- Windows: w9-1 ungated 27 tests; w9-5/w9-4/w4-2 added ungated
  daemon-spawn tests — unverified on Windows; next windows-latest CI run or
  run-blit-tests.ps1 triages real failures into findings.

## Handoff log (newest first, keep ≤ 3)

- **2026-06-20** @ `09268eb` (doc edits uncommitted) — reviewed all three
  unified-transfer candidates against the code, produced REV4 (= REV3 +
  corrected code reality, every symbol grounded with `file:line`), and —
  on owner's "rev4 replaces v1" — recorded **D-2026-06-20-5** and
  propagated it: REV4 flipped to **Active**, v1/REV2/REV3 marked
  **Superseded**, STATE + DECISIONS updated. Decisive finding: REV3's
  "two static tables, not three" ladder correction was itself wrong —
  all three ladders (`determine_remote_tuning`, `desired_streams`,
  `pull_stream_count`) are live. No git commit (owner gate). In-flight:
  none; coding still gated by AGENTS.md §9 absent a per-slice go-ahead.
  **Exact first action next session**: owner authorizes `ue-r2-1a` (or
  edits D-2026-06-20-1 / approves the `439a2a7` push — both still open).
- **2026-06-20** @ `7ecc355` (doc edits uncommitted) — owner requested
  plan review findings be rolled into `docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md`
  and recorded a freeze (D-2026-06-20-4): unified-transfer-engine coding
  is frozen pending owner final plan decision. Rev2 keeps convergence and
  the D-2026-06-20-2 bound parameters but splits oversized slices and
  clarifies local fast paths, work-stealing tests, proto compatibility,
  and pull parity timing. In-flight: plan review only, no code. **Exact
  first action next session**: owner decides whether rev2 replaces/amends
  the original Active plan or requests another revision; push approval for
  `439a2a7` still pending separately.
- **2026-06-12** @ `b5cbb38` — gemini-reviewer session: graded and accepted both pending sentinels (design-4 and design-5); verdicts committed, `REVIEW.md` rows `[x]`, ready/ queue empty. In-flight: none. **Exact first action next session**: owner decides the remaining gates (w2-3 Active flip, push approval).

exec
/usr/bin/bash -lc "sed -n '1,220p' .review/findings/ue-r2-1a.md" in /home/michael/dev/blit_v2
 succeeded in 0ms:
# ue-r2-1a: Salvage adaptive PR1+PR2 substrate (telemetry Probe + work-stealing queue)

**Slice**: ue-r2-1a — first slice of `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`
**Status**: Coded; under GPT review (`docs/agent/GPT_REVIEW_LOOP.md`)
**Branch**: master (no agent branches — AGENTS.md §8)
**Commits**: `e569eea` (PR1), `3844a15` (PR2), `ec561f2` (PR2 fix), plus the
tests/finding commit.

## What

Land the adaptive-streams substrate that was trapped behind the `-s ours`
octopus (D-2026-06-07-2): per-stream telemetry with a zero-cost `Probe`
(PR1), the shared work-stealing pipeline queue (PR2), and the
forwarder-halt-on-error fix (PR2 review). This is the C-ready seam REV4
builds on — an elastic work-stealing stream-set, work not pinned to a
stream. No operator-visible behavior change: the default `NoProbe`
monomorphization compiles to today's hot path.

## Approach

Cherry-picked (`-x`) the three code commits onto master rather than merging
— the octopus made them ancestors, so a plain merge no-ops
(D-2026-06-07-2):

- `e6ef095` → `e569eea` (PR1): `DataPlaneSession<P: Probe = NoProbe>`,
  `StreamTelemetry`/`StreamProbe`, `tcp_info` module. Conflicts in
  `data_plane.rs` (master's audit-h3b `StallGuardWriter` stream vs PR1's
  generic struct) and `mod.rs` (re-exports) hand-resolved: the stream
  stays `StallGuardWriter<TcpStream>` and the struct gains `<P: Probe>` +
  a `probe` field; `from_stream_with_probe` wraps the stream in the stall
  guard. `mod.rs` re-exports drop `Phase`/`TransferProgress`/
  `TransferProgressSnapshot` (master had already removed those types) and
  add the telemetry types. Added the missing `AtomicU8` import in
  `progress.rs`.
- `af66ff5` → `3844a15` (PR2): shared `flume` work-stealing queue. Applied
  cleanly (master's `pipeline.rs` matched the cherry-pick base).
- `b797b73` → `ec561f2` (PR2 fix): forwarder halts promptly on sink error
  via a shared `cancelled` flag. Applied with `-n` to drop the bundled
  `reviews/PR2-workqueue.codex.md` artifact (absolute `C:/Users` paths;
  review provenance, not substrate).

`eafb187` ("backup": doc moves + stray-file delete) and `d9d4ec7` (PR3 WIP,
does not build) excluded per REV4.

## Files changed

- `data_plane.rs` — generic `DataPlaneSession<P: Probe>`; hot-loop
  telemetry gated on the compile-time `P::ACTIVE`; StallGuard composition
  preserved.
- `progress.rs` — `StreamId`, `StreamState`, `StreamTelemetry`,
  `StreamProbe`, `Probe`/`NoProbe`/`LiveProbe` (+ `AtomicU8` import).
- `tcp_info.rs` — new; best-effort `getsockopt(TCP_INFO)` on Linux, `None`
  stub elsewhere.
- `sink.rs` — `DataPlaneSink<P: Probe = NoProbe>` (default keeps call
  sites unchanged).
- `pipeline.rs` — shared work-stealing queue + forwarder-halt fix + two
  new behavior tests.
- `mod.rs` — re-exports.

## Tests added

PR1/PR2 brought their own; this slice adds two to `pipeline.rs`
`workqueue_tests` to complete REV4's "work-stealing as behavior" set:

- `byte_and_file_totals_correct_under_work_stealing` — two sinks pull the
  shared queue; distinct per-file sizes pin byte + file totals and that
  every byte lands on exactly one sink (no double-count / drop).
- `producer_cancel_winds_down_pipeline_promptly` — producer drops the
  channel mid-stream; the executor winds down without hanging (the 5s
  timeout is the no-hang assertion) and writes only the fed payloads.

Kept: `fast_sink_steals_work_from_slow_sink` (slow-sink),
`forwarder_stops_promptly_on_worker_error` +
`pipeline_streaming_surfaces_underlying_sink_error` (failing-sink),
`receive_pipeline_aborts_on_stall` + `stall_guard.rs` (StallGuard),
`pipeline_copies_files_end_to_end` (byte-identical).

Validation: `cargo fmt --check` clean; `clippy -D warnings` clean;
`cargo test --workspace` **1377 passed / 0 failed / 2 ignored** (baseline
1370; +7).

## Known gaps

1. **Hard-abort on cancel is w4-1, not here.** The work-stealing workers
   are bare `tokio::spawn`; dropping a `JoinHandle` does not abort the
   task, so dropping the *pipeline future itself* detaches in-flight
   workers rather than aborting them — the AbortOnDrop family (REV4 w4-1,
   ratified High). ue-r2-1a pins the graceful producer-close cancellation
   path the substrate provides;
   `producer_cancel_winds_down_pipeline_promptly` documents the boundary.
2. **PR1 telemetry-accuracy findings carried forward → ue-r2-1e.** The
   original PR1 codex review (verdict: fix-then-ship) left two unaddressed:
   (Medium) `write_blocked_nanos` times the whole `tokio::join!(write,
   read)` step, so a slow *source read* inflates the write-backpressure
   signal; (Low) tar-shard chunk writes record bytes but no write-block
   time. The telemetry has no live consumer until the dial in ue-r2-1e, so
   the inaccuracy is latent — land faithfully now, fix when ue-r2-1e wires
   the signal to a decision.
3. PR1 telemetry types (`StreamProbe`, `StreamTelemetry`, `tcp_info`,
   `LiveProbe`) are substrate with no live consumer yet (wired in
   ue-r2-1e / the controller). Re-exported for that future use.

exec
/usr/bin/bash -lc "sed -n '1,220p' /home/michael/.codex/plugins/cache/openai-curated/superpowers/202e9242/skills/using-superpowers/SKILL.md" in /home/michael/dev/blit_v2
 succeeded in 0ms:
---
name: using-superpowers
description: Use when starting any conversation - establishes how to find and use skills, requiring Skill tool invocation before ANY response including clarifying questions
---

<SUBAGENT-STOP>
If you were dispatched as a subagent to execute a specific task, skip this skill.
</SUBAGENT-STOP>

<EXTREMELY-IMPORTANT>
If you think there is even a 1% chance a skill might apply to what you are doing, you ABSOLUTELY MUST invoke the skill.

IF A SKILL APPLIES TO YOUR TASK, YOU DO NOT HAVE A CHOICE. YOU MUST USE IT.

This is not negotiable. This is not optional. You cannot rationalize your way out of this.
</EXTREMELY-IMPORTANT>

## Instruction Priority

Superpowers skills override default system prompt behavior, but **user instructions always take precedence**:

1. **User's explicit instructions** (CLAUDE.md, GEMINI.md, AGENTS.md, direct requests) — highest priority
2. **Superpowers skills** — override default system behavior where they conflict
3. **Default system prompt** — lowest priority

If CLAUDE.md, GEMINI.md, or AGENTS.md says "don't use TDD" and a skill says "always use TDD," follow the user's instructions. The user is in control.

## How to Access Skills

**In Claude Code:** Use the `Skill` tool. When you invoke a skill, its content is loaded and presented to you—follow it directly. Never use the Read tool on skill files.

**In Copilot CLI:** Use the `skill` tool. Skills are auto-discovered from installed plugins. The `skill` tool works the same as Claude Code's `Skill` tool.

**In Gemini CLI:** Skills activate via the `activate_skill` tool. Gemini loads skill metadata at session start and activates the full content on demand.

**In other environments:** Check your platform's documentation for how skills are loaded.

## Platform Adaptation

Skills use Claude Code tool names. Non-CC platforms: see `references/copilot-tools.md` (Copilot CLI), `references/codex-tools.md` (Codex) for tool equivalents. Gemini CLI users get the tool mapping loaded automatically via GEMINI.md.

# Using Skills

## The Rule

**Invoke relevant or requested skills BEFORE any response or action.** Even a 1% chance a skill might apply means that you should invoke the skill to check. If an invoked skill turns out to be wrong for the situation, you don't need to use it.

```dot
digraph skill_flow {
    "User message received" [shape=doublecircle];
    "About to EnterPlanMode?" [shape=doublecircle];
    "Already brainstormed?" [shape=diamond];
    "Invoke brainstorming skill" [shape=box];
    "Might any skill apply?" [shape=diamond];
    "Invoke Skill tool" [shape=box];
    "Announce: 'Using [skill] to [purpose]'" [shape=box];
    "Has checklist?" [shape=diamond];
    "Create TodoWrite todo per item" [shape=box];
    "Follow skill exactly" [shape=box];
    "Respond (including clarifications)" [shape=doublecircle];

    "About to EnterPlanMode?" -> "Already brainstormed?";
    "Already brainstormed?" -> "Invoke brainstorming skill" [label="no"];
    "Already brainstormed?" -> "Might any skill apply?" [label="yes"];
    "Invoke brainstorming skill" -> "Might any skill apply?";

    "User message received" -> "Might any skill apply?";
    "Might any skill apply?" -> "Invoke Skill tool" [label="yes, even 1%"];
    "Might any skill apply?" -> "Respond (including clarifications)" [label="definitely not"];
    "Invoke Skill tool" -> "Announce: 'Using [skill] to [purpose]'";
    "Announce: 'Using [skill] to [purpose]'" -> "Has checklist?";
    "Has checklist?" -> "Create TodoWrite todo per item" [label="yes"];
    "Has checklist?" -> "Follow skill exactly" [label="no"];
    "Create TodoWrite todo per item" -> "Follow skill exactly";
}
```

## Red Flags

These thoughts mean STOP—you're rationalizing:

| Thought | Reality |
|---------|---------|
| "This is just a simple question" | Questions are tasks. Check for skills. |
| "I need more context first" | Skill check comes BEFORE clarifying questions. |
| "Let me explore the codebase first" | Skills tell you HOW to explore. Check first. |
| "I can check git/files quickly" | Files lack conversation context. Check for skills. |
| "Let me gather information first" | Skills tell you HOW to gather information. |
| "This doesn't need a formal skill" | If a skill exists, use it. |
| "I remember this skill" | Skills evolve. Read current version. |
| "This doesn't count as a task" | Action = task. Check for skills. |
| "The skill is overkill" | Simple things become complex. Use it. |
| "I'll just do this one thing first" | Check BEFORE doing anything. |
| "This feels productive" | Undisciplined action wastes time. Skills prevent this. |
| "I know what that means" | Knowing the concept ≠ using the skill. Invoke it. |

## Skill Priority

When multiple skills could apply, use this order:

1. **Process skills first** (brainstorming, debugging) - these determine HOW to approach the task
2. **Implementation skills second** (frontend-design, mcp-builder) - these guide execution

"Let's build X" → brainstorming first, then implementation skills.
"Fix this bug" → debugging first, then domain-specific skills.

## Skill Types

**Rigid** (TDD, debugging): Follow exactly. Don't adapt away discipline.

**Flexible** (patterns): Adapt principles to context.

The skill itself tells you which.

## User Instructions

Instructions say WHAT, not HOW. "Add X" or "Fix Y" doesn't mean skip workflows.

codex
I have the active state, plan, and finding doc. I’m checking the repo protocol now, then I’ll read the target diff and code paths without making any edits.
exec
/usr/bin/bash -lc 'git status --short' in /home/michael/dev/blit_v2
 succeeded in 0ms:
 M Cargo.lock
?? .review/results/ue-r2-1a.codex.md

exec
/usr/bin/bash -lc "sed -n '1,260p' docs/agent/PROTOCOL.md" in /home/michael/dev/blit_v2
 succeeded in 0ms:
# Agent procedures (PROTOCOL.md)

**Status**: Active

Single source for the trigger vocabulary defined in `AGENTS.md` §3. Claude Code
slash commands, Antigravity workspace skills, and plain-text triggers in Codex all
resolve here, so behavior is identical across tools. Execute the matching section
exactly; do not improvise steps.

## Invocation rules

Run a procedure's steps in order and do not improvise additional ones. Every
procedure ends by emitting an **attestation line** in this exact format:

    protocol: <name> | steps run: <list> | caps: <met, or which violated>

`<name>` is the procedure (e.g. `catchup`); `steps run:` lists the step numbers
actually executed; `caps:` states whether the procedure's output caps were met,
or names any that were violated.

- This file's text must be in context in the same turn a procedure runs — via
  the command's embedded copy or a fresh read in that turn. Memory of an earlier
  read never counts.
- A procedure ends at its final step. Proposing or staging actions beyond it is
  a violation even if nothing was modified.

---

## catchup
Re-ground in current project state before doing any work.
1. Run `bash scripts/agent/catchup.sh`.
2. Show its output to the owner verbatim — never rephrase,
   reorder, expand, or summarize them.
3. Append exactly one line: `Proposed first action: <one sentence>`.
4. Append the attestation line and stop. No modifications, no further
   plans, until the owner responds.

---

## plan `<topic>`

Turn a talked-through idea into a durable plan before any implementation.

1. Interview the owner: goal, non-goals, constraints, acceptance criteria,
   affected crates/files, risks. Ask focused questions until every template field
   can be filled without guessing. **Each requirement stated in chat must land in
   the draft doc in the same turn it is stated** — write incrementally, not at the
   end.
2. Create `docs/plan/<NAME>.md` from `docs/plan/TEMPLATE.md` with
   `**Status**: Draft`.
3. Slice the work into review-loop-sized slices (one coherent, testable change
   each) in the doc's Slices section.
4. Add the doc to STATE.md's Queue (and to "Authoritative docs" if it will be the
   active plan).
5. **Stop.** No implementation until the owner approves; record approval by
   flipping `**Status**: Draft` → `Active` and adding a DECISIONS.md entry.

---

## decision `<topic>`

Record a settled choice so no future session relitigates or misses it.

1. Append to `docs/DECISIONS.md`:

   ```
   ## D-<YYYY-MM-DD>-<n> — <short title>
   - Decision: <one line>
   - Why: <one line>
   - Supersedes: <doc §/decision ID, or "nothing">
   ```

2. If it supersedes plan text: edit that plan text **now** (rewrite or strike,
   annotate "superseded by D-…"). Do not leave stale text standing.
3. If it changes Now or the Queue: update `docs/STATE.md`.
4. Confirm to the owner with the entry ID.

---

## handoff

Make the current session's state durable for the next one. Run before ending a
session, when context is filling up, or on request.

1. Update `docs/STATE.md`: rewrite **Now**; reorder **Queue**; refresh
   **Blocked** and **Open questions**; prepend a handoff entry with date, HEAD
   sha, and 1–3 lines covering *done / in-flight / exact first action for the
   next session*.
2. Enforce caps: ≤ 200 lines total, ≤ 3 handoff entries. Move pruned material
   into `DEVLOG.md`.
3. If meaningful work landed, append a `DEVLOG.md` entry (newest-first, ISO
   timestamp, same style as existing entries).
4. Run `bash scripts/agent/check-docs.sh`; fix any failures.
5. Commit the doc updates alongside the work they describe (or as
   `Handoff: <date>` if docs-only).

---

## drift `[scope]`

Audit one document against reality. Never run unscoped.

1. Scope = the argument, or ask the owner for one doc or one subsystem.
2. Extract every checkable claim from the doc: behavior, file paths, flags,
   module names, status checkboxes, "X is done/not built" statements.
3. Verify each claim against code and tests (read source; run targeted tests
   where cheap).
4. Triage into three lists and act:
   - **Doc wrong** → fix the doc in this session.
   - **Code wrong** → file a finding per `.review/README.md`, or add to the
     STATE.md Queue if it isn't review-loop material.
   - **Ambiguous / needs owner** → add to STATE.md Open questions.
5. Record any supersessions via the `decision` procedure.
6. Report the three lists to the owner.

---

## slice

Pick up the next unit of review-loop work.

1. Run `catchup` first if you haven't this session.
2. Follow the Coder loop in `.review/README.md` exactly: pick the
   highest-priority `[ ]` item, branch per the branch model, implement with
   tests, pass the validation suite, write the finding doc, update the
   `REVIEW.md` row, write the sentinel atomically.
3. Finish with the `handoff` procedure.

exec
/usr/bin/bash -lc 'git diff 515fb76..HEAD -- crates/blit-core/src/remote/transfer/data_plane.rs crates/blit-core/src/remote/transfer/pipeline.rs crates/blit-core/src/remote/transfer/progress.rs crates/blit-core/src/remote/transfer/sink.rs crates/blit-core/src/remote/transfer/tcp_info.rs crates/blit-core/src/remote/transfer/mod.rs' in /home/michael/dev/blit_v2
 succeeded in 0ms:
diff --git a/crates/blit-core/src/remote/transfer/data_plane.rs b/crates/blit-core/src/remote/transfer/data_plane.rs
index 1d380aa..a882a77 100644
--- a/crates/blit-core/src/remote/transfer/data_plane.rs
+++ b/crates/blit-core/src/remote/transfer/data_plane.rs
@@ -8,6 +8,7 @@ use crate::buffer::BufferPool;
 use crate::generated::FileHeader;
 
 use super::payload::{prepared_payload_stream, PreparedPayload, TransferPayload};
+use super::progress::{NoProbe, Probe};
 use super::stall_guard::{StallGuardWriter, TRANSFER_STALL_TIMEOUT};
 use crate::remote::transfer::source::TransferSource;
 use std::sync::Arc;
@@ -19,21 +20,32 @@ pub const DATA_PLANE_RECORD_BLOCK: u8 = 2;
 pub const DATA_PLANE_RECORD_BLOCK_COMPLETE: u8 = 3;
 pub const DATA_PLANE_RECORD_END: u8 = 0xFF;
 
-pub struct DataPlaneSession {
-    // audit-h3b: writes go through StallGuardWriter so a stalled
-    // reader (TCP backpressure from a slow / wedged peer) trips after
-    // TRANSFER_STALL_TIMEOUT of no observable write progress instead
-    // of pinning the worker for OS-level TCP retransmit exhaustion
-    // (15+ minutes). All existing `self.stream.write_all/.flush`
-    // call sites in this file (~30 sites) compose against the
-    // AsyncWrite impl of StallGuardWriter, so no per-site change
-    // was needed.
+/// A single data-plane TCP stream and its send loop.
+///
+/// Generic over a [`Probe`] so the byte-copy hot path can carry
+/// per-stream telemetry under adaptive mode at **zero cost** when the
+/// probe is [`NoProbe`] (the default): the instrumented branches are
+/// gated on `P::ACTIVE`, a compile-time constant, so they fold away
+/// entirely for `DataPlaneSession<NoProbe>`. Existing callers name the
+/// bare type and get the `NoProbe` default; the adaptive controller
+/// constructs `DataPlaneSession<LiveProbe>` via
+/// [`from_stream_with_probe`](DataPlaneSession::from_stream_with_probe).
+///
+/// audit-h3b: writes go through [`StallGuardWriter`] so a stalled
+/// reader (TCP backpressure from a slow / wedged peer) trips after
+/// [`TRANSFER_STALL_TIMEOUT`] of no observable write progress instead
+/// of pinning the worker for OS-level TCP retransmit exhaustion
+/// (15+ minutes). All existing `self.stream.write_all/.flush` call
+/// sites compose against the `AsyncWrite` impl of `StallGuardWriter`,
+/// so no per-site change was needed.
+pub struct DataPlaneSession<P: Probe = NoProbe> {
     stream: StallGuardWriter<TcpStream>,
     pool: Arc<BufferPool>,
     trace: bool,
     chunk_bytes: usize,
     payload_prefetch: usize,
     bytes_sent: u64,
+    probe: P,
 }
 
 macro_rules! trace_client {
@@ -44,17 +56,17 @@ macro_rules! trace_client {
     };
 }
 
-impl DataPlaneSession {
+impl DataPlaneSession<NoProbe> {
     /// Create a session from an existing stream with buffer pooling.
     ///
-    /// audit-h3b: the stream is wrapped in [`StallGuardWriter`] so a
-    /// stalled peer (slow / wedged reader causing TCP backpressure)
-    /// trips after [`TRANSFER_STALL_TIMEOUT`] of no observable write
-    /// progress instead of pinning the worker for OS-level TCP
-    /// retransmit exhaustion. All three production call sites
-    /// (`daemon/service/pull.rs` regular pull,
-    /// `daemon/service/pull_sync.rs` regular pull-sync, same file's
-    /// resume mode) inherit the guard without code changes.
+    /// Produces the un-instrumented `NoProbe` variant — the default for
+    /// every non-adaptive caller. audit-h3b: the stream is wrapped in
+    /// [`StallGuardWriter`] (inside `from_stream_with_probe`) so a
+    /// stalled peer trips after [`TRANSFER_STALL_TIMEOUT`] of no
+    /// observable write progress instead of pinning the worker for
+    /// OS-level TCP retransmit exhaustion. The production call sites
+    /// (`daemon/service/pull.rs`, `daemon/service/pull_sync.rs`, and the
+    /// resume path) inherit the guard without code changes.
     pub async fn from_stream(
         stream: TcpStream,
         trace: bool,
@@ -62,16 +74,8 @@ impl DataPlaneSession {
         payload_prefetch: usize,
         pool: Arc<BufferPool>,
     ) -> Self {
-        let payload_prefetch = payload_prefetch.max(1);
-        let chunk_bytes = chunk_bytes.max(64 * 1024);
-        Self {
-            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
-            pool,
-            trace,
-            chunk_bytes,
-            payload_prefetch,
-            bytes_sent: 0,
-        }
+        Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, NoProbe)
+            .await
     }
 
     /// Connect to a data plane endpoint with buffer pooling.
@@ -131,6 +135,33 @@ impl DataPlaneSession {
 
         Ok(Self::from_stream(stream, trace, chunk_bytes, payload_prefetch, pool).await)
     }
+}
+
+impl<P: Probe> DataPlaneSession<P> {
+    /// Create a session carrying an arbitrary [`Probe`]. The generic
+    /// primitive behind [`from_stream`](DataPlaneSession::from_stream);
+    /// the adaptive controller calls this with a `LiveProbe` to enable
+    /// per-stream telemetry.
+    pub async fn from_stream_with_probe(
+        stream: TcpStream,
+        trace: bool,
+        chunk_bytes: usize,
+        payload_prefetch: usize,
+        pool: Arc<BufferPool>,
+        probe: P,
+    ) -> Self {
+        let payload_prefetch = payload_prefetch.max(1);
+        let chunk_bytes = chunk_bytes.max(64 * 1024);
+        Self {
+            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
+            pool,
+            trace,
+            chunk_bytes,
+            payload_prefetch,
+            bytes_sent: 0,
+            probe,
+        }
+    }
 
     pub async fn send_payloads(
         &mut self,
@@ -310,6 +341,15 @@ impl DataPlaneSession {
 
         // Main loop: write buf_a while reading into buf_b
         while remaining > 0 {
+            // Per-stream telemetry: time the overlapped write+read step
+            // as a backpressure proxy. Gated on the compile-time
+            // `P::ACTIVE` constant so `DataPlaneSession<NoProbe>` reads
+            // no clock and folds this to nothing.
+            let step_start = if P::ACTIVE {
+                Some(std::time::Instant::now())
+            } else {
+                None
+            };
             // Overlap: write from buf_a, read into buf_b concurrently
             let (write_result, read_result) = tokio::join!(
                 self.stream.write_all(&buf_a.as_slice()[..bytes_a]),
@@ -317,6 +357,12 @@ impl DataPlaneSession {
             );
 
             write_result.with_context(|| format!("sending {}", rel))?;
+            if P::ACTIVE {
+                if let Some(t) = step_start {
+                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
+                }
+            }
+            self.probe.record_bytes(bytes_a as u64);
             crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
 
             let bytes_b = read_result.with_context(|| format!("reading {}", rel))?;
@@ -339,12 +385,25 @@ impl DataPlaneSession {
             bytes_a = bytes_b;
         }
 
-        // Final write: send the last chunk in buf_a
+        // Final write: send the last chunk in buf_a. This is a pure
+        // write (no overlapped read), so the timing is cleanly
+        // attributable to socket-write backpressure.
         if bytes_a > 0 {
+            let tail_start = if P::ACTIVE {
+                Some(std::time::Instant::now())
+            } else {
+                None
+            };
             self.stream
                 .write_all(&buf_a.as_slice()[..bytes_a])
                 .await
                 .with_context(|| format!("sending {}", rel))?;
+            if P::ACTIVE {
+                if let Some(t) = tail_start {
+                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
+                }
+            }
+            self.probe.record_bytes(bytes_a as u64);
             crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
         }
 
@@ -417,6 +476,7 @@ impl DataPlaneSession {
                 .write_all(chunk)
                 .await
                 .context("writing tar shard payload")?;
+            self.probe.record_bytes(chunk.len() as u64);
             crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(chunk.len() as u64);
         }
         trace_client!(
diff --git a/crates/blit-core/src/remote/transfer/mod.rs b/crates/blit-core/src/remote/transfer/mod.rs
index 59b9b9e..c604d46 100644
--- a/crates/blit-core/src/remote/transfer/mod.rs
+++ b/crates/blit-core/src/remote/transfer/mod.rs
@@ -9,6 +9,7 @@ pub mod sink;
 pub mod source;
 pub mod stall_guard;
 pub mod tar_safety;
+pub mod tcp_info;
 
 pub use data_plane::{
     receive_stream_double_buffered, DataPlaneSession, CONTROL_PLANE_CHUNK_SIZE,
@@ -21,8 +22,12 @@ pub use payload::{
     TransferPayload, DEFAULT_PAYLOAD_PREFETCH,
 };
 pub use pipeline::{execute_sink_pipeline, execute_sink_pipeline_streaming};
-pub use progress::{ByteProgressSink, ProgressEvent, RemoteTransferProgress};
+pub use progress::{
+    ByteProgressSink, LiveProbe, NoProbe, Probe, ProgressEvent, RemoteTransferProgress, StreamId,
+    StreamProbe, StreamState, StreamTelemetry, StreamTelemetrySnapshot,
+};
 pub use sink::{
     DataPlaneSink, FsSinkConfig, FsTransferSink, GrpcFallbackSink, GrpcServerStreamingSink,
     NullSink, SinkOutcome, TransferSink,
 };
+pub use tcp_info::{sample_stream as sample_tcp_info, TcpInfoSample};
diff --git a/crates/blit-core/src/remote/transfer/pipeline.rs b/crates/blit-core/src/remote/transfer/pipeline.rs
index 1d03c1b..1792bf4 100644
--- a/crates/blit-core/src/remote/transfer/pipeline.rs
+++ b/crates/blit-core/src/remote/transfer/pipeline.rs
@@ -59,14 +59,21 @@ pub async fn execute_sink_pipeline(
 
 /// Execute a transfer pipeline with payloads arriving on a channel.
 ///
-/// Distributes payloads round-robin across `sinks` as they arrive. Each sink
-/// runs as a separate tokio task: it reads payloads from its dedicated queue,
-/// prepares them via `source.prepare_payload()`, writes them via
-/// `sink.write_payload()`, and finally calls `sink.finish()`. Errors from any
-/// worker propagate up.
+/// Payloads are distributed across `sinks` through a single shared
+/// **work-stealing** queue (a bounded `flume` MPMC channel): each sink
+/// runs as a tokio task that pulls the next available payload via
+/// `recv_async().await`, so a slow sink can never head-of-line-block the
+/// others (the failure mode of the previous round-robin per-sink
+/// channels). A forwarder task moves payloads from the incoming
+/// `payload_rx` onto the shared queue; dropping its sender on
+/// end-of-stream lets every worker observe `Disconnected` once the queue
+/// drains, at which point it calls `sink.finish()`. Errors from any
+/// worker propagate up (first error wins).
 ///
-/// `prefetch` controls the per-sink channel capacity — effectively the
-/// preparation-in-flight limit per sink.
+/// `prefetch` controls the per-sink preparation-in-flight limit; the
+/// shared queue is bounded at `prefetch * sinks.len()` so total
+/// in-flight capacity matches the previous per-sink-channel design
+/// (back-pressure preserved).
 pub async fn execute_sink_pipeline_streaming(
     source: Arc<dyn TransferSource>,
     sinks: Vec<Arc<dyn TransferSink>>,
@@ -81,71 +88,109 @@ pub async fn execute_sink_pipeline_streaming(
     }
 
     let sink_count = sinks.len();
-    let per_sink_capacity = prefetch.max(1);
+    let capacity = prefetch.max(1) * sink_count;
     let total = Arc::new(std::sync::Mutex::new(SinkOutcome::default()));
 
-    // Per-sink payload channels; dispatcher forwards round-robin to these.
-    let mut sink_senders: Vec<mpsc::Sender<TransferPayload>> = Vec::with_capacity(sink_count);
+    // Single shared work queue. Each worker owns exactly one sink but
+    // pulls payloads from the common queue, so work is stolen by
+    // whichever sink is free rather than pre-assigned round-robin.
+    let (work_tx, work_rx) = flume::bounded::<TransferPayload>(capacity);
     let mut sink_handles: Vec<tokio::task::JoinHandle<Result<()>>> = Vec::with_capacity(sink_count);
 
+    // Cancellation flag set by the first worker that errors. Without it,
+    // one sink failing only drops that worker's `work_rx` clone; as long
+    // as any other worker is alive `send_async` keeps succeeding, so the
+    // forwarder would keep draining `payload_rx` and queueing payloads
+    // that can never complete — delaying first-error-wins propagation
+    // (Codex review, PR2). With it, the forwarder stops at the next
+    // payload boundary and closes the queue so the survivors drain and
+    // finish promptly.
+    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
+
     for sink in sinks {
-        let (tx, mut rx) = mpsc::channel::<TransferPayload>(per_sink_capacity);
-        sink_senders.push(tx);
+        let work_rx = work_rx.clone();
         let source_clone = source.clone();
         let progress_clone = progress.cloned();
         let total_clone = total.clone();
+        let cancelled_worker = cancelled.clone();
         sink_handles.push(tokio::spawn(async move {
-            while let Some(payload) = rx.recv().await {
-                let prepared = source_clone
-                    .prepare_payload(payload)
-                    .await
-                    .context("preparing payload")?;
-                let files: Vec<(String, u64)> = match &prepared {
-                    PreparedPayload::File(h) => vec![(h.relative_path.clone(), h.size)],
-                    PreparedPayload::TarShard { headers, .. } => headers
-                        .iter()
-                        .map(|h| (h.relative_path.clone(), h.size))
-                        .collect(),
-                    // Resume-block payloads patch existing files; no
-                    // file-completion event from one-block-at-a-time.
-                    PreparedPayload::FileBlock { .. }
-                    | PreparedPayload::FileBlockComplete { .. } => Vec::new(),
-                };
-                let outcome = sink
-                    .write_payload(prepared)
-                    .await
-                    .context("writing payload")?;
-                if let Some(p) = &progress_clone {
-                    for (name, size) in &files {
-                        p.report_file_complete(name.clone(), *size);
+            // Wrap the body so any early-return error trips the shared
+            // cancel flag before the `?` unwinds the task.
+            let run = async {
+                while let Ok(payload) = work_rx.recv_async().await {
+                    let prepared = source_clone
+                        .prepare_payload(payload)
+                        .await
+                        .context("preparing payload")?;
+                    let files: Vec<(String, u64)> = match &prepared {
+                        PreparedPayload::File(h) => vec![(h.relative_path.clone(), h.size)],
+                        PreparedPayload::TarShard { headers, .. } => headers
+                            .iter()
+                            .map(|h| (h.relative_path.clone(), h.size))
+                            .collect(),
+                        // Resume-block payloads patch existing files; no
+                        // file-completion event from one-block-at-a-time.
+                        PreparedPayload::FileBlock { .. }
+                        | PreparedPayload::FileBlockComplete { .. } => Vec::new(),
+                    };
+                    let outcome = sink
+                        .write_payload(prepared)
+                        .await
+                        .context("writing payload")?;
+                    if let Some(p) = &progress_clone {
+                        for (name, size) in &files {
+                            p.report_file_complete(name.clone(), *size);
+                        }
                     }
+                    let mut t = total_clone.lock().unwrap();
+                    t.merge(&outcome);
                 }
-                let mut t = total_clone.lock().unwrap();
-                t.merge(&outcome);
+                sink.finish().await?;
+                Ok::<(), eyre::Report>(())
             }
-            sink.finish().await?;
-            Ok::<(), eyre::Report>(())
+            .await;
+            if run.is_err() {
+                // Signal the forwarder (and implicitly the other workers,
+                // once the queue closes) to stop feeding new work.
+                cancelled_worker.store(true, std::sync::atomic::Ordering::Relaxed);
+            }
+            run
         }));
     }
 
-    // Dispatcher: pull from the incoming channel, round-robin to sinks.
-    // Uses async send (which applies backpressure) — if one sink is slower,
-    // the dispatcher naturally blocks on that sink until it drains.
-    let dispatcher = tokio::spawn(async move {
-        let mut next = 0usize;
+    // Drop our own receiver handle so the channel closes once the
+    // forwarder drops its sender and the workers' clones drain — without
+    // this, `recv_async` would never see `Disconnected`.
+    drop(work_rx);
+
+    // Forwarder: move payloads from the incoming channel onto the shared
+    // work queue. `send_async` applies back-pressure (bounded queue); if
+    // every worker has gone away (e.g. all sinks errored) the send fails
+    // and we stop. It also bails as soon as a worker sets `cancelled`, so
+    // a single sink error halts intake promptly instead of waiting for
+    // every worker to drop. Dropping `work_tx` on end-of-stream (or on
+    // cancel) signals the workers.
+    let cancelled_fwd = cancelled.clone();
+    let forwarder = tokio::spawn(async move {
         while let Some(payload) = payload_rx.recv().await {
-            let idx = next % sink_count;
-            next = next.wrapping_add(1);
-            if sink_senders[idx].send(payload).await.is_err() {
-                // Sink worker dropped its receiver — treat as shutdown.
+            if cancelled_fwd.load(std::sync::atomic::Ordering::Relaxed) {
+                // A worker errored — stop draining the producer and let
+                // the queue close so survivors finish and the error
+                // surfaces without delay.
+                return;
+            }
+            if work_tx.send_async(payload).await.is_err() {
+                // All workers dropped their receivers — nothing left to
+                // feed; treat as shutdown.
                 return;
             }
         }
-        // Drop senders so sink workers see end-of-stream and finish().
-        drop(sink_senders);
+        // Dropping work_tx closes the queue → workers see Disconnected
+        // after draining and run finish().
+        drop(work_tx);
     });
 
-    // Wait for all sinks to finish and aggregate errors.
+    // Wait for all sinks to finish and aggregate errors (first wins).
     let mut first_err: Option<eyre::Report> = None;
     for h in sink_handles {
         match h.await {
@@ -158,7 +203,7 @@ pub async fn execute_sink_pipeline_streaming(
             Err(_) => {}
         }
     }
-    let _ = dispatcher.await;
+    let _ = forwarder.await;
 
     if let Some(err) = first_err {
         return Err(err);
@@ -954,3 +999,338 @@ mod tests {
         );
     }
 }
+
+#[cfg(test)]
+mod workqueue_tests {
+    //! PR2: the shared work-queue must let a fast sink steal work a slow
+    //! sink would otherwise have been assigned under the old round-robin
+    //! dispatcher. Without work-stealing, N payloads split evenly across
+    //! sinks and one slow sink bottlenecks the whole transfer; with it,
+    //! the fast sink absorbs the bulk.
+    use super::*;
+    use crate::remote::transfer::sink::{SinkOutcome, TransferSink};
+    use crate::remote::transfer::source::FsTransferSource;
+    use std::path::{Path, PathBuf};
+    use std::sync::atomic::{AtomicU64, Ordering};
+    use std::sync::{Arc, Mutex};
+    use std::time::Duration;
+    use tempfile::tempdir;
+
+    /// Counts payloads it writes; optionally sleeps per payload to model
+    /// a slow stream. Ignores the payload bytes — timing is governed
+    /// purely by the configured delay, isolating the dispatch behaviour.
+    struct CountingSink {
+        delay: Duration,
+        count: Arc<AtomicU64>,
+        root: PathBuf,
+    }
+
+    #[async_trait::async_trait]
+    impl TransferSink for CountingSink {
+        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
+            if !self.delay.is_zero() {
+                tokio::time::sleep(self.delay).await;
+            }
+            self.count.fetch_add(1, Ordering::Relaxed);
+            Ok(SinkOutcome {
+                files_written: 1,
+                bytes_written: 0,
+            })
+        }
+        fn root(&self) -> &Path {
+            &self.root
+        }
+    }
+
+    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
+    async fn fast_sink_steals_work_from_slow_sink() {
+        let tmp = tempdir().unwrap();
+        let src = tmp.path().join("src");
+        std::fs::create_dir_all(&src).unwrap();
+        let n = 40usize;
+        for i in 0..n {
+            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
+        }
+
+        let source = Arc::new(FsTransferSource::new(src.clone()));
+        let unreadable = Arc::new(Mutex::new(Vec::new()));
+        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
+        let mut headers = Vec::new();
+        while let Some(h) = header_rx.recv().await {
+            headers.push(h);
+        }
+        let _ = scan_handle.await.unwrap().unwrap();
+        // Feed each file as its OWN payload (not via plan_transfer_payloads,
+        // which bundles tiny files into a single tar shard — that would
+        // leave only one payload and nothing to steal).
+        assert_eq!(headers.len(), n, "expected one header per file");
+
+        let fast_count = Arc::new(AtomicU64::new(0));
+        let slow_count = Arc::new(AtomicU64::new(0));
+        let fast: Arc<dyn TransferSink> = Arc::new(CountingSink {
+            delay: Duration::ZERO,
+            count: Arc::clone(&fast_count),
+            root: PathBuf::from("/fast"),
+        });
+        let slow: Arc<dyn TransferSink> = Arc::new(CountingSink {
+            delay: Duration::from_millis(20),
+            count: Arc::clone(&slow_count),
+            root: PathBuf::from("/slow"),
+        });
+
+        let (tx, rx) = mpsc::channel::<TransferPayload>(4);
+        let feeder = tokio::spawn(async move {
+            for h in headers {
+                if tx.send(TransferPayload::File(h)).await.is_err() {
+                    break;
+                }
+            }
+        });
+        let outcome = execute_sink_pipeline_streaming(source, vec![fast, slow], rx, 2, None)
+            .await
+            .expect("pipeline ok");
+        let _ = feeder.await;
+
+        let fast_n = fast_count.load(Ordering::Relaxed);
+        let slow_n = slow_count.load(Ordering::Relaxed);
+        assert_eq!(outcome.files_written, n, "every payload written once");
+        assert_eq!(
+            fast_n + slow_n,
+            n as u64,
+            "every payload accounted to exactly one sink"
+        );
+        // Round-robin would force ~20/20 and the slow sink would gate the
+        // whole transfer. Work-stealing lets the zero-delay sink take the
+        // overwhelming majority while the slow sink sits in its sleeps.
+        assert!(
+            fast_n > slow_n * 3,
+            "fast sink should steal the bulk of the work: fast={fast_n} slow={slow_n}"
+        );
+    }
+
+    /// Codex-review (PR2) regression: when the only sink errors, the
+    /// forwarder must stop draining the producer promptly rather than
+    /// continuing to pull every remaining payload. We feed a large
+    /// payload set through a single always-failing sink and assert that
+    /// (a) the pipeline surfaces the error, and (b) the forwarder
+    /// consumed far fewer than all payloads before halting — proving the
+    /// cancel flag short-circuits intake instead of draining to the end.
+    struct ErrSink {
+        root: PathBuf,
+    }
+
+    #[async_trait::async_trait]
+    impl TransferSink for ErrSink {
+        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
+            eyre::bail!("synthetic immediate failure")
+        }
+        fn root(&self) -> &Path {
+            &self.root
+        }
+    }
+
+    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
+    async fn forwarder_stops_promptly_on_worker_error() {
+        let tmp = tempdir().unwrap();
+        let src = tmp.path().join("src");
+        std::fs::create_dir_all(&src).unwrap();
+        let n = 200usize;
+        for i in 0..n {
+            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
+        }
+        let source = Arc::new(FsTransferSource::new(src.clone()));
+        let unreadable = Arc::new(Mutex::new(Vec::new()));
+        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
+        let mut headers = Vec::new();
+        while let Some(h) = header_rx.recv().await {
+            headers.push(h);
+        }
+        let _ = scan_handle.await.unwrap().unwrap();
+        assert_eq!(headers.len(), n);
+
+        let sink: Arc<dyn TransferSink> = Arc::new(ErrSink {
+            root: PathBuf::from("/err"),
+        });
+
+        // Count how many payloads the forwarder actually pulled from the
+        // producer. With prefetch=1 and a single sink, the bounded queue
+        // holds 1; once the sink errors and trips `cancelled`, the
+        // forwarder must stop, so `sent` stays a tiny constant rather
+        // than reaching n.
+        let sent = Arc::new(AtomicU64::new(0));
+        let (tx, rx) = mpsc::channel::<TransferPayload>(1);
+        let sent_feeder = sent.clone();
+        let feeder = tokio::spawn(async move {
+            for h in headers {
+                if tx.send(TransferPayload::File(h)).await.is_err() {
+                    break;
+                }
+                sent_feeder.fetch_add(1, Ordering::Relaxed);
+            }
+        });
+
+        let result = execute_sink_pipeline_streaming(source, vec![sink], rx, 1, None).await;
+        let _ = feeder.await;
+
+        assert!(result.is_err(), "pipeline must surface the sink error");
+        let pulled = sent.load(Ordering::Relaxed);
+        assert!(
+            pulled < (n as u64) / 2,
+            "forwarder should halt soon after the error, not drain all {n} payloads; pulled={pulled}"
+        );
+    }
+
+    /// Reports each payload's real byte size so the executor's byte and
+    /// file aggregation can be checked end to end without touching disk.
+    struct ByteSink {
+        bytes: Arc<AtomicU64>,
+        root: PathBuf,
+    }
+
+    #[async_trait::async_trait]
+    impl TransferSink for ByteSink {
+        async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
+            let (files, bytes): (usize, u64) = match &payload {
+                PreparedPayload::File(h) => (1, h.size),
+                PreparedPayload::TarShard { headers, .. } => {
+                    (headers.len(), headers.iter().map(|h| h.size).sum())
+                }
+                _ => (0, 0),
+            };
+            self.bytes.fetch_add(bytes, Ordering::Relaxed);
+            Ok(SinkOutcome {
+                files_written: files,
+                bytes_written: bytes,
+            })
+        }
+        fn root(&self) -> &Path {
+            &self.root
+        }
+    }
+
+    /// REV4 ue-r2-1a (work-stealing as behaviour): byte and file totals
+    /// stay correct when two sinks pull from the shared queue. Distinct
+    /// per-file sizes mean any double-count or dropped payload shifts the
+    /// totals, and the per-sink sum pins that every byte lands on exactly
+    /// one sink.
+    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
+    async fn byte_and_file_totals_correct_under_work_stealing() {
+        let tmp = tempdir().unwrap();
+        let src = tmp.path().join("src");
+        std::fs::create_dir_all(&src).unwrap();
+        let n = 30usize;
+        let mut expected_bytes = 0u64;
+        for i in 0..n {
+            // Distinct sizes so a miscount (double-add / drop) is visible.
+            let body = vec![b'x'; i + 1];
+            expected_bytes += body.len() as u64;
+            std::fs::write(src.join(format!("f{i}.dat")), &body).unwrap();
+        }
+        let source = Arc::new(FsTransferSource::new(src.clone()));
+        let unreadable = Arc::new(Mutex::new(Vec::new()));
+        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
+        let mut headers = Vec::new();
+        while let Some(h) = header_rx.recv().await {
+            headers.push(h);
+        }
+        let _ = scan_handle.await.unwrap().unwrap();
+        assert_eq!(headers.len(), n, "one header per file");
+
+        let bytes_a = Arc::new(AtomicU64::new(0));
+        let bytes_b = Arc::new(AtomicU64::new(0));
+        let a: Arc<dyn TransferSink> = Arc::new(ByteSink {
+            bytes: Arc::clone(&bytes_a),
+            root: PathBuf::from("/a"),
+        });
+        let b: Arc<dyn TransferSink> = Arc::new(ByteSink {
+            bytes: Arc::clone(&bytes_b),
+            root: PathBuf::from("/b"),
+        });
+
+        let (tx, rx) = mpsc::channel::<TransferPayload>(4);
+        let feeder = tokio::spawn(async move {
+            for h in headers {
+                if tx.send(TransferPayload::File(h)).await.is_err() {
+                    break;
+                }
+            }
+        });
+        let outcome = execute_sink_pipeline_streaming(source, vec![a, b], rx, 2, None)
+            .await
+            .expect("pipeline ok");
+        let _ = feeder.await;
+
+        assert_eq!(outcome.files_written, n, "file total");
+        assert_eq!(outcome.bytes_written, expected_bytes, "byte total");
+        assert_eq!(
+            bytes_a.load(Ordering::Relaxed) + bytes_b.load(Ordering::Relaxed),
+            expected_bytes,
+            "every byte accounted to exactly one sink, none double-counted"
+        );
+    }
+
+    /// REV4 ue-r2-1a (cancellation): when the producer stops feeding and
+    /// drops the channel mid-stream, the work-stealing executor winds
+    /// down promptly — workers drain what was queued, run `finish`, and
+    /// the call returns without hanging (the timeout is the no-hang
+    /// assertion). Only the fed payloads complete; nothing past the
+    /// cancellation point is invented.
+    ///
+    /// Hard-abort of in-flight workers on dropping the pipeline future
+    /// itself is out of scope here: the workers are bare `tokio::spawn`
+    /// (a `JoinHandle` drop does not abort the task), which is the
+    /// AbortOnDrop family tracked under w4-1.
+    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
+    async fn producer_cancel_winds_down_pipeline_promptly() {
+        let tmp = tempdir().unwrap();
+        let src = tmp.path().join("src");
+        std::fs::create_dir_all(&src).unwrap();
+        let n = 50usize;
+        for i in 0..n {
+            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
+        }
+        let source = Arc::new(FsTransferSource::new(src.clone()));
+        let unreadable = Arc::new(Mutex::new(Vec::new()));
+        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
+        let mut headers = Vec::new();
+        while let Some(h) = header_rx.recv().await {
+            headers.push(h);
+        }
+        let _ = scan_handle.await.unwrap().unwrap();
+        assert_eq!(headers.len(), n);
+
+        let count = Arc::new(AtomicU64::new(0));
+        let sink: Arc<dyn TransferSink> = Arc::new(CountingSink {
+            delay: Duration::ZERO,
+            count: Arc::clone(&count),
+            root: PathBuf::from("/c"),
+        });
+
+        // Feed only the first 5 headers, then drop the sender to model a
+        // cancelled / aborted producer.
+        let (tx, rx) = mpsc::channel::<TransferPayload>(2);
+        let feeder = tokio::spawn(async move {
+            for h in headers.into_iter().take(5) {
+                if tx.send(TransferPayload::File(h)).await.is_err() {
+                    break;
+                }
+            }
+            // `tx` dropped here → channel closes → pipeline must wind down.
+        });
+
+        let outcome = tokio::time::timeout(
+            Duration::from_secs(5),
+            execute_sink_pipeline_streaming(source, vec![sink], rx, 2, None),
+        )
+        .await
+        .expect("pipeline must wind down promptly after producer cancels, not hang")
+        .expect("graceful shutdown is not an error");
+        let _ = feeder.await;
+
+        assert_eq!(
+            outcome.files_written, 5,
+            "only the fed payloads are written"
+        );
+        assert_eq!(count.load(Ordering::Relaxed), 5);
+    }
+}
diff --git a/crates/blit-core/src/remote/transfer/progress.rs b/crates/blit-core/src/remote/transfer/progress.rs
index 3ff9a94..f5111a9 100644
--- a/crates/blit-core/src/remote/transfer/progress.rs
+++ b/crates/blit-core/src/remote/transfer/progress.rs
@@ -1,4 +1,4 @@
-use std::sync::atomic::{AtomicU64, Ordering};
+use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
 use std::sync::Arc;
 use tokio::sync::mpsc::UnboundedSender;
 
@@ -100,6 +100,234 @@ mod tests {
     }
 }
 
+// =====================================================================
+// Per-stream telemetry (PR1 of the adaptive-streams work).
+//
+// The adaptive stream controller (added in a later PR) needs a live,
+// per-stream view of throughput and write-backpressure to steer
+// AIMD decisions. This module provides the lock-free counters plus a
+// zero-cost `Probe` abstraction so the byte-copy hot path pays nothing
+// when telemetry is off.
+//
+// Hot-path discipline mirrors `ByteProgressSink`: writers only do
+// `Relaxed` atomic adds; a sampler task reads `snapshot()` on a timer.
+// =====================================================================
+
+/// Cache-line-aligned wrapper so independent per-stream counters never
+/// share a cache line (false sharing would tax the hot path under high
+/// stream counts). A local 8-line equivalent of
+/// `crossbeam_utils::CachePadded`, kept here to avoid adding a
+/// dependency for one type. 64 bytes covers x86-64 / aarch64 lines.
+#[repr(align(64))]
+#[derive(Debug, Default)]
+struct CachePadded<T>(T);
+
+impl<T> CachePadded<T> {
+    fn new(value: T) -> Self {
+        Self(value)
+    }
+}
+
+impl<T> std::ops::Deref for CachePadded<T> {
+    type Target = T;
+    #[inline]
+    fn deref(&self) -> &T {
+        &self.0
+    }
+}
+
+/// Identifies one data-plane stream within a transfer. Stable for the
+/// life of the stream; an `ADD`'d stream gets a fresh id.
+#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
+pub struct StreamId(pub u32);
+
+/// Coarse lifecycle state of a single stream, stored as a `u8` atomic so
+/// the sampler can read it lock-free. The controller uses it to exclude
+/// draining/closed streams from marginal-gain math.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+#[repr(u8)]
+pub enum StreamState {
+    /// Connected, handshake done, no payload written yet.
+    Starting = 0,
+    /// Actively transferring payloads.
+    Active = 1,
+    /// `stop` requested; finishing in-flight payload then closing.
+    Draining = 2,
+    /// `RECORD_END` emitted, socket closed.
+    Closed = 3,
+}
+
+impl StreamState {
+    fn from_u8(v: u8) -> Self {
+        match v {
+            1 => StreamState::Active,
+            2 => StreamState::Draining,
+            3 => StreamState::Closed,
+            _ => StreamState::Starting,
+        }
+    }
+}
+
+/// Lock-free per-stream counters. Cache-padded so independent streams
+/// never share a cache line (false sharing would otherwise tax the hot
+/// path under high stream counts). Only the owning send loop writes
+/// `bytes_sent` / `write_blocked_nanos`; the sampler reads a snapshot.
+#[derive(Debug)]
+pub struct StreamTelemetry {
+    bytes_sent: CachePadded<AtomicU64>,
+    write_blocked_nanos: CachePadded<AtomicU64>,
+    state: AtomicU8,
+    /// Bumped each time the controller resizes; lets a stale snapshot be
+    /// discarded across a resize boundary.
+    generation: AtomicU64,
+}
+
+impl StreamTelemetry {
+    pub fn new() -> Self {
+        Self {
+            bytes_sent: CachePadded::new(AtomicU64::new(0)),
+            write_blocked_nanos: CachePadded::new(AtomicU64::new(0)),
+            state: AtomicU8::new(StreamState::Starting as u8),
+            generation: AtomicU64::new(0),
+        }
+    }
+}
+
+impl Default for StreamTelemetry {
+    fn default() -> Self {
+        Self::new()
+    }
+}
+
+/// Plain `Copy` view of a [`StreamTelemetry`], taken by the sampler each
+/// tick. Decoupled from the atomics so the sampler never holds a
+/// reference into the shared handle.
+#[derive(Clone, Copy, Debug)]
+pub struct StreamTelemetrySnapshot {
+    pub id: StreamId,
+    pub bytes_sent: u64,
+    pub write_blocked_nanos: u64,
+    pub state: StreamState,
+    pub generation: u64,
+}
+
+/// Cloneable handle to one stream's telemetry, held by the send loop.
+/// Clone is a cheap `Arc` bump; the increments are `Relaxed`. Mirrors
+/// the `ByteProgressSink` pattern so the data plane can carry it the
+/// same way it carries byte progress.
+#[derive(Clone, Debug)]
+pub struct StreamProbe {
+    id: StreamId,
+    inner: Arc<StreamTelemetry>,
+}
+
+impl StreamProbe {
+    pub fn new(id: StreamId) -> Self {
+        Self {
+            id,
+            inner: Arc::new(StreamTelemetry::new()),
+        }
+    }
+
+    pub fn from_telemetry(id: StreamId, inner: Arc<StreamTelemetry>) -> Self {
+        Self { id, inner }
+    }
+
+    pub fn id(&self) -> StreamId {
+        self.id
+    }
+
+    /// Shared `Arc` so a sampler can hold the telemetry independently of
+    /// the send loop's probe.
+    pub fn telemetry(&self) -> Arc<StreamTelemetry> {
+        Arc::clone(&self.inner)
+    }
+
+    /// Add `delta` bytes that just landed on the wire. `Relaxed` is
+    /// sufficient: the sampler only needs eventual visibility.
+    #[inline]
+    pub fn record_bytes(&self, delta: u64) {
+        self.inner.bytes_sent.fetch_add(delta, Ordering::Relaxed);
+    }
+
+    /// Add nanoseconds spent blocked on a socket write — the signal the
+    /// controller uses to tell "link-bound" from "source-bound".
+    #[inline]
+    pub fn add_write_blocked(&self, nanos: u64) {
+        self.inner
+            .write_blocked_nanos
+            .fetch_add(nanos, Ordering::Relaxed);
+    }
+
+    pub fn set_state(&self, state: StreamState) {
+        self.inner.state.store(state as u8, Ordering::Relaxed);
+    }
+
+    pub fn set_generation(&self, generation: u64) {
+        self.inner.generation.store(generation, Ordering::Relaxed);
+    }
+
+    pub fn snapshot(&self) -> StreamTelemetrySnapshot {
+        StreamTelemetrySnapshot {
+            id: self.id,
+            bytes_sent: self.inner.bytes_sent.load(Ordering::Relaxed),
+            write_blocked_nanos: self.inner.write_blocked_nanos.load(Ordering::Relaxed),
+            state: StreamState::from_u8(self.inner.state.load(Ordering::Relaxed)),
+            generation: self.inner.generation.load(Ordering::Relaxed),
+        }
+    }
+}
+
+/// Zero-cost telemetry abstraction for the byte-copy hot path.
+///
+/// The send loop is generic over `P: Probe`. The associated
+/// `const ACTIVE` lets the timing instrumentation (`Instant::now()`)
+/// be compile-time elided for the [`NoProbe`] monomorphization: an
+/// `if P::ACTIVE { … }` guarding the clock reads folds to nothing when
+/// `ACTIVE == false`, and the empty `#[inline(always)]` methods emit no
+/// code. The result is byte-identical codegen to the pre-telemetry hot
+/// loop — the hard "zero added cost on the byte-copy hot path"
+/// constraint, satisfied at compile time rather than via a runtime
+/// branch.
+pub trait Probe: Send + Sync + 'static {
+    /// When `false`, callers must skip all instrumentation work
+    /// (including clock reads) so the optimizer drops it entirely.
+    const ACTIVE: bool;
+    fn record_bytes(&self, delta: u64);
+    fn note_write_blocked(&self, nanos: u64);
+}
+
+/// The default probe: every method is an inlined no-op and `ACTIVE`
+/// is `false`, so a `DataPlaneSession<NoProbe>` send loop compiles to
+/// exactly today's code.
+#[derive(Clone, Copy, Debug, Default)]
+pub struct NoProbe;
+
+impl Probe for NoProbe {
+    const ACTIVE: bool = false;
+    #[inline(always)]
+    fn record_bytes(&self, _delta: u64) {}
+    #[inline(always)]
+    fn note_write_blocked(&self, _nanos: u64) {}
+}
+
+/// The instrumented probe, constructed only under adaptive mode. Wraps
+/// a [`StreamProbe`] and forwards into its lock-free counters.
+#[derive(Clone, Debug)]
+pub struct LiveProbe(pub StreamProbe);
+
+impl Probe for LiveProbe {
+    const ACTIVE: bool = true;
+    #[inline(always)]
+    fn record_bytes(&self, delta: u64) {
+        self.0.record_bytes(delta);
+    }
+    #[inline(always)]
+    fn note_write_blocked(&self, nanos: u64) {
+        self.0.add_write_blocked(nanos);
+    }
+}
+
 #[derive(Clone)]
 pub struct RemoteTransferProgress {
     sender: UnboundedSender<ProgressEvent>,
@@ -124,3 +352,61 @@ impl RemoteTransferProgress {
             .send(ProgressEvent::FileComplete { path, bytes });
     }
 }
+
+#[cfg(test)]
+mod stream_telemetry_tests {
+    use super::*;
+
+    #[test]
+    fn live_probe_accumulates_bytes_and_block_time() {
+        let probe = StreamProbe::new(StreamId(7));
+        let live = LiveProbe(probe.clone());
+        // Drive through the Probe trait, exactly as the hot loop does.
+        Probe::record_bytes(&live, 1000);
+        Probe::record_bytes(&live, 500);
+        Probe::note_write_blocked(&live, 250_000);
+        let snap = probe.snapshot();
+        assert_eq!(snap.id, StreamId(7));
+        assert_eq!(snap.bytes_sent, 1500);
+        assert_eq!(snap.write_blocked_nanos, 250_000);
+        assert_eq!(snap.state, StreamState::Starting);
+    }
+
+    // The optimizer relies on these constants to elide the
+    // instrumentation branches; pin them at compile time (a runtime
+    // `assert!` on a const is a clippy `assertions_on_constants` lint).
+    const _: () = assert!(!<NoProbe as Probe>::ACTIVE);
+    const _: () = assert!(<LiveProbe as Probe>::ACTIVE);
+
+    #[test]
+    fn no_probe_is_inert() {
+        // NoProbe must compile to a no-op; there is no observable state,
+        // so the contract is simply that the trait calls type-check and
+        // run without effect.
+        let n = NoProbe;
+        Probe::record_bytes(&n, 123);
+        Probe::note_write_blocked(&n, 456);
+    }
+
+    #[test]
+    fn state_and_generation_round_trip() {
+        let probe = StreamProbe::new(StreamId(0));
+        probe.set_state(StreamState::Draining);
+        probe.set_generation(42);
+        let snap = probe.snapshot();
+        assert_eq!(snap.state, StreamState::Draining);
+        assert_eq!(snap.generation, 42);
+    }
+
+    #[test]
+    fn clones_share_counters() {
+        let probe = StreamProbe::new(StreamId(1));
+        let clone = probe.clone();
+        probe.record_bytes(10);
+        clone.record_bytes(20);
+        assert_eq!(probe.snapshot().bytes_sent, 30);
+        // The telemetry Arc is shared.
+        let tel = probe.telemetry();
+        assert!(Arc::strong_count(&tel) >= 2);
+    }
+}
diff --git a/crates/blit-core/src/remote/transfer/sink.rs b/crates/blit-core/src/remote/transfer/sink.rs
index b5bfac4..c62a84b 100644
--- a/crates/blit-core/src/remote/transfer/sink.rs
+++ b/crates/blit-core/src/remote/transfer/sink.rs
@@ -16,7 +16,7 @@ use crate::copy::{copy_file, resume_copy_file};
 use crate::generated::{ComparisonMode, FileHeader};
 use crate::logger::NoopLogger;
 use crate::remote::transfer::payload::PreparedPayload;
-use crate::remote::transfer::progress::ByteProgressSink;
+use crate::remote::transfer::progress::{ByteProgressSink, NoProbe, Probe};
 use crate::remote::transfer::source::TransferSource;
 
 // Re-export for consumers.
@@ -746,15 +746,15 @@ async fn write_file_block_complete(
 ///
 /// Each instance wraps a single TCP stream (DataPlaneSession). For multi-stream
 /// transfers, the pipeline executor creates multiple DataPlaneSink instances.
-pub struct DataPlaneSink {
-    session: tokio::sync::Mutex<DataPlaneSession>,
+pub struct DataPlaneSink<P: Probe = NoProbe> {
+    session: tokio::sync::Mutex<DataPlaneSession<P>>,
     source: Arc<dyn TransferSource>,
     dst_root: PathBuf,
 }
 
-impl DataPlaneSink {
+impl<P: Probe> DataPlaneSink<P> {
     pub fn new(
-        session: DataPlaneSession,
+        session: DataPlaneSession<P>,
         source: Arc<dyn TransferSource>,
         dst_root: PathBuf,
     ) -> Self {
@@ -767,7 +767,7 @@ impl DataPlaneSink {
 }
 
 #[async_trait]
-impl TransferSink for DataPlaneSink {
+impl<P: Probe> TransferSink for DataPlaneSink<P> {
     async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
         let mut session = self.session.lock().await;
         match payload {
diff --git a/crates/blit-core/src/remote/transfer/tcp_info.rs b/crates/blit-core/src/remote/transfer/tcp_info.rs
new file mode 100644
index 0000000..6fc805a
--- /dev/null
+++ b/crates/blit-core/src/remote/transfer/tcp_info.rs
@@ -0,0 +1,88 @@
+//! Best-effort per-socket TCP statistics for the adaptive stream
+//! controller.
+//!
+//! On Linux the controller reads `TCP_INFO` via `getsockopt(2)` to see
+//! retransmits and smoothed RTT — the cleanest "the link is congesting"
+//! signal available without a userspace congestion model. Everywhere
+//! else the syscall has no portable equivalent, so [`sample_stream`]
+//! returns `None` and the controller falls back to its
+//! throughput-slope + `write_blocked_nanos` signals (which are
+//! cross-platform). Keeping the platform split behind one function lets
+//! the controller stay platform-agnostic.
+
+/// A point-in-time read of kernel TCP state for one stream. Fields are
+/// cumulative counters / current estimates; the controller diffs
+/// successive samples to derive a per-interval retransmit rate.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
+pub struct TcpInfoSample {
+    /// Total retransmitted segments over the life of the connection
+    /// (`tcpi_total_retrans`). Monotonic; diff across samples.
+    pub total_retransmits: u64,
+    /// Smoothed round-trip time in microseconds (`tcpi_rtt`).
+    pub rtt_micros: u64,
+}
+
+/// Read `TCP_INFO` for `stream`. Returns `None` when the platform has no
+/// equivalent or the syscall fails (the controller then leans on its
+/// portable signals). Never panics.
+#[cfg(target_os = "linux")]
+pub fn sample_stream(stream: &tokio::net::TcpStream) -> Option<TcpInfoSample> {
+    use std::os::fd::AsRawFd;
+    let fd = stream.as_raw_fd();
+    // SAFETY: `getsockopt` writes at most `len` bytes into `info`, which
+    // is a fully-owned zeroed `tcp_info`; `len` is initialised to its
+    // size and updated by the kernel. We read only after a success
+    // return. `fd` is borrowed from a live `TcpStream` for the duration
+    // of the call.
+    let mut info: libc::tcp_info = unsafe { std::mem::zeroed() };
+    let mut len = std::mem::size_of::<libc::tcp_info>() as libc::socklen_t;
+    let ret = unsafe {
+        libc::getsockopt(
+            fd,
+            libc::IPPROTO_TCP,
+            libc::TCP_INFO,
+            &mut info as *mut libc::tcp_info as *mut libc::c_void,
+            &mut len,
+        )
+    };
+    if ret != 0 {
+        return None;
+    }
+    Some(TcpInfoSample {
+        total_retransmits: info.tcpi_total_retrans as u64,
+        rtt_micros: info.tcpi_rtt as u64,
+    })
+}
+
+/// Non-Linux stub: no portable `TCP_INFO`, so the controller uses
+/// throughput + `write_blocked_nanos` instead.
+#[cfg(not(target_os = "linux"))]
+pub fn sample_stream(_stream: &tokio::net::TcpStream) -> Option<TcpInfoSample> {
+    None
+}
+
+#[cfg(all(test, target_os = "linux"))]
+mod tests {
+    use super::*;
+
+    /// A live loopback connection should yield a `TCP_INFO` read with a
+    /// plausible (non-huge) RTT and zero-ish retransmits. This proves
+    /// the `getsockopt` wiring works end-to-end on Linux.
+    #[tokio::test]
+    async fn samples_live_loopback_socket() {
+        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
+        let addr = listener.local_addr().unwrap();
+        let (client, _server) = tokio::join!(
+            async { tokio::net::TcpStream::connect(addr).await.unwrap() },
+            async { listener.accept().await.unwrap() },
+        );
+        let sample = sample_stream(&client).expect("TCP_INFO available on loopback");
+        // Loopback RTT is microseconds-to-low-milliseconds; assert it's
+        // not absurd rather than pinning a value.
+        assert!(
+            sample.rtt_micros < 5_000_000,
+            "loopback rtt should be well under 5s, got {} us",
+            sample.rtt_micros
+        );
+    }
+}

exec
/usr/bin/bash -lc 'git diff 515fb76..HEAD --stat' in /home/michael/dev/blit_v2
 succeeded in 0ms:
 .review/findings/ue-r2-1a.md                       | 101 +++++
 REVIEW.md                                          |   2 +-
 crates/blit-core/src/remote/transfer/data_plane.rs | 118 +++--
 crates/blit-core/src/remote/transfer/mod.rs        |   7 +-
 crates/blit-core/src/remote/transfer/pipeline.rs   | 484 ++++++++++++++++++---
 crates/blit-core/src/remote/transfer/progress.rs   | 288 +++++++++++-
 crates/blit-core/src/remote/transfer/sink.rs       |  12 +-
 crates/blit-core/src/remote/transfer/tcp_info.rs   |  88 ++++
 8 files changed, 1010 insertions(+), 90 deletions(-)

exec
/usr/bin/bash -lc 'git log --oneline --decorate --max-count=8' in /home/michael/dev/blit_v2
 succeeded in 0ms:
771a632 (HEAD -> master) ue-r2-1a: work-stealing behaviour tests + finding doc
ec561f2 fix(transfer): halt forwarder promptly on sink error (PR2 review)
3844a15 feat(transfer): shared work-stealing pipeline queue (adaptive PR2)
e569eea feat(transfer): per-stream telemetry with zero-cost Probe (adaptive PR1)
515fb76 Remove docs/agent/SETUP.md (folding into governance); make GPT-loop precedence explicit
926732e Install docs/agent/SETUP.md; repo-relativize live review-doc paths
e2c9521 Establish code->GPT-review->fix loop for unified-engine slices (D-2026-06-20-6)
b663091 (origin/master, origin/HEAD, github/master, github/HEAD) Handoff 2026-06-20: REV4 replaces v1 as Active transfer-engine plan (D-2026-06-20-5)

codex
The diff is centered on the expected files, with a dirty `Cargo.lock` and an untracked review result already present; I’m ignoring those because they are outside this read-only review. Next I’m walking the exact write and queue paths with line numbers and comparing against `515fb76` where behavior matters.
exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '1,260p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
     1	//! Unified transfer pipeline: source → prepare → sink(s).
     2	//!
     3	//! All transfer paths (local→local, local→remote push, remote→local pull,
     4	//! remote→remote) route through the same executor. Payloads can be supplied
     5	//! either upfront ([`execute_sink_pipeline`]) or incrementally as they are
     6	//! produced ([`execute_sink_pipeline_streaming`]). The one-shot form is a
     7	//! thin wrapper that sends every payload on a channel and delegates.
     8	
     9	use std::sync::Arc;
    10	
    11	use eyre::{Context, Result};
    12	use tokio::sync::mpsc;
    13	
    14	use super::payload::{PreparedPayload, TransferPayload};
    15	use super::progress::RemoteTransferProgress;
    16	use super::sink::{SinkOutcome, TransferSink};
    17	use super::source::TransferSource;
    18	
    19	/// Execute a transfer pipeline with all payloads known upfront.
    20	///
    21	/// This is a convenience wrapper around [`execute_sink_pipeline_streaming`]
    22	/// that spawns a task to send every payload into the channel and then drops
    23	/// the sender, signalling end-of-stream.
    24	pub async fn execute_sink_pipeline(
    25	    source: Arc<dyn TransferSource>,
    26	    sinks: Vec<Arc<dyn TransferSink>>,
    27	    payloads: Vec<TransferPayload>,
    28	    prefetch: usize,
    29	    progress: Option<&RemoteTransferProgress>,
    30	) -> Result<SinkOutcome> {
    31	    if sinks.is_empty() {
    32	        return Ok(SinkOutcome::default());
    33	    }
    34	    if payloads.is_empty() {
    35	        for sink in &sinks {
    36	            sink.finish().await?;
    37	        }
    38	        return Ok(SinkOutcome::default());
    39	    }
    40	
    41	    let capacity = prefetch.max(1);
    42	    let (tx, rx) = mpsc::channel::<TransferPayload>(capacity);
    43	
    44	    // Feed payloads in a background task so the pipeline can start writing
    45	    // before the whole vec is queued (the channel provides back-pressure).
    46	    let feeder = tokio::spawn(async move {
    47	        for payload in payloads {
    48	            if tx.send(payload).await.is_err() {
    49	                break;
    50	            }
    51	        }
    52	        // Dropping tx closes the channel and signals end-of-stream.
    53	    });
    54	
    55	    let result = execute_sink_pipeline_streaming(source, sinks, rx, prefetch, progress).await;
    56	    let _ = feeder.await;
    57	    result
    58	}
    59	
    60	/// Execute a transfer pipeline with payloads arriving on a channel.
    61	///
    62	/// Payloads are distributed across `sinks` through a single shared
    63	/// **work-stealing** queue (a bounded `flume` MPMC channel): each sink
    64	/// runs as a tokio task that pulls the next available payload via
    65	/// `recv_async().await`, so a slow sink can never head-of-line-block the
    66	/// others (the failure mode of the previous round-robin per-sink
    67	/// channels). A forwarder task moves payloads from the incoming
    68	/// `payload_rx` onto the shared queue; dropping its sender on
    69	/// end-of-stream lets every worker observe `Disconnected` once the queue
    70	/// drains, at which point it calls `sink.finish()`. Errors from any
    71	/// worker propagate up (first error wins).
    72	///
    73	/// `prefetch` controls the per-sink preparation-in-flight limit; the
    74	/// shared queue is bounded at `prefetch * sinks.len()` so total
    75	/// in-flight capacity matches the previous per-sink-channel design
    76	/// (back-pressure preserved).
    77	pub async fn execute_sink_pipeline_streaming(
    78	    source: Arc<dyn TransferSource>,
    79	    sinks: Vec<Arc<dyn TransferSink>>,
    80	    mut payload_rx: mpsc::Receiver<TransferPayload>,
    81	    prefetch: usize,
    82	    progress: Option<&RemoteTransferProgress>,
    83	) -> Result<SinkOutcome> {
    84	    if sinks.is_empty() {
    85	        // Drain incoming channel so the producer isn't left dangling.
    86	        while payload_rx.recv().await.is_some() {}
    87	        return Ok(SinkOutcome::default());
    88	    }
    89	
    90	    let sink_count = sinks.len();
    91	    let capacity = prefetch.max(1) * sink_count;
    92	    let total = Arc::new(std::sync::Mutex::new(SinkOutcome::default()));
    93	
    94	    // Single shared work queue. Each worker owns exactly one sink but
    95	    // pulls payloads from the common queue, so work is stolen by
    96	    // whichever sink is free rather than pre-assigned round-robin.
    97	    let (work_tx, work_rx) = flume::bounded::<TransferPayload>(capacity);
    98	    let mut sink_handles: Vec<tokio::task::JoinHandle<Result<()>>> = Vec::with_capacity(sink_count);
    99	
   100	    // Cancellation flag set by the first worker that errors. Without it,
   101	    // one sink failing only drops that worker's `work_rx` clone; as long
   102	    // as any other worker is alive `send_async` keeps succeeding, so the
   103	    // forwarder would keep draining `payload_rx` and queueing payloads
   104	    // that can never complete — delaying first-error-wins propagation
   105	    // (Codex review, PR2). With it, the forwarder stops at the next
   106	    // payload boundary and closes the queue so the survivors drain and
   107	    // finish promptly.
   108	    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
   109	
   110	    for sink in sinks {
   111	        let work_rx = work_rx.clone();
   112	        let source_clone = source.clone();
   113	        let progress_clone = progress.cloned();
   114	        let total_clone = total.clone();
   115	        let cancelled_worker = cancelled.clone();
   116	        sink_handles.push(tokio::spawn(async move {
   117	            // Wrap the body so any early-return error trips the shared
   118	            // cancel flag before the `?` unwinds the task.
   119	            let run = async {
   120	                while let Ok(payload) = work_rx.recv_async().await {
   121	                    let prepared = source_clone
   122	                        .prepare_payload(payload)
   123	                        .await
   124	                        .context("preparing payload")?;
   125	                    let files: Vec<(String, u64)> = match &prepared {
   126	                        PreparedPayload::File(h) => vec![(h.relative_path.clone(), h.size)],
   127	                        PreparedPayload::TarShard { headers, .. } => headers
   128	                            .iter()
   129	                            .map(|h| (h.relative_path.clone(), h.size))
   130	                            .collect(),
   131	                        // Resume-block payloads patch existing files; no
   132	                        // file-completion event from one-block-at-a-time.
   133	                        PreparedPayload::FileBlock { .. }
   134	                        | PreparedPayload::FileBlockComplete { .. } => Vec::new(),
   135	                    };
   136	                    let outcome = sink
   137	                        .write_payload(prepared)
   138	                        .await
   139	                        .context("writing payload")?;
   140	                    if let Some(p) = &progress_clone {
   141	                        for (name, size) in &files {
   142	                            p.report_file_complete(name.clone(), *size);
   143	                        }
   144	                    }
   145	                    let mut t = total_clone.lock().unwrap();
   146	                    t.merge(&outcome);
   147	                }
   148	                sink.finish().await?;
   149	                Ok::<(), eyre::Report>(())
   150	            }
   151	            .await;
   152	            if run.is_err() {
   153	                // Signal the forwarder (and implicitly the other workers,
   154	                // once the queue closes) to stop feeding new work.
   155	                cancelled_worker.store(true, std::sync::atomic::Ordering::Relaxed);
   156	            }
   157	            run
   158	        }));
   159	    }
   160	
   161	    // Drop our own receiver handle so the channel closes once the
   162	    // forwarder drops its sender and the workers' clones drain — without
   163	    // this, `recv_async` would never see `Disconnected`.
   164	    drop(work_rx);
   165	
   166	    // Forwarder: move payloads from the incoming channel onto the shared
   167	    // work queue. `send_async` applies back-pressure (bounded queue); if
   168	    // every worker has gone away (e.g. all sinks errored) the send fails
   169	    // and we stop. It also bails as soon as a worker sets `cancelled`, so
   170	    // a single sink error halts intake promptly instead of waiting for
   171	    // every worker to drop. Dropping `work_tx` on end-of-stream (or on
   172	    // cancel) signals the workers.
   173	    let cancelled_fwd = cancelled.clone();
   174	    let forwarder = tokio::spawn(async move {
   175	        while let Some(payload) = payload_rx.recv().await {
   176	            if cancelled_fwd.load(std::sync::atomic::Ordering::Relaxed) {
   177	                // A worker errored — stop draining the producer and let
   178	                // the queue close so survivors finish and the error
   179	                // surfaces without delay.
   180	                return;
   181	            }
   182	            if work_tx.send_async(payload).await.is_err() {
   183	                // All workers dropped their receivers — nothing left to
   184	                // feed; treat as shutdown.
   185	                return;
   186	            }
   187	        }
   188	        // Dropping work_tx closes the queue → workers see Disconnected
   189	        // after draining and run finish().
   190	        drop(work_tx);
   191	    });
   192	
   193	    // Wait for all sinks to finish and aggregate errors (first wins).
   194	    let mut first_err: Option<eyre::Report> = None;
   195	    for h in sink_handles {
   196	        match h.await {
   197	            Ok(Ok(())) => {}
   198	            Ok(Err(e)) if first_err.is_none() => first_err = Some(e),
   199	            Ok(Err(_)) => {}
   200	            Err(join) if first_err.is_none() => {
   201	                first_err = Some(eyre::eyre!("sink worker panicked: {}", join));
   202	            }
   203	            Err(_) => {}
   204	        }
   205	    }
   206	    let _ = forwarder.await;
   207	
   208	    if let Some(err) = first_err {
   209	        return Err(err);
   210	    }
   211	
   212	    let result = total.lock().unwrap().clone();
   213	    Ok(result)
   214	}
   215	
   216	// =====================================================================
   217	// Receive pipeline — symmetric counterpart of execute_sink_pipeline.
   218	// =====================================================================
   219	
   220	use crate::generated::FileHeader;
   221	use eyre::bail;
   222	use tokio::io::{AsyncRead, AsyncReadExt};
   223	
   224	use super::data_plane::{
   225	    DATA_PLANE_RECORD_BLOCK, DATA_PLANE_RECORD_BLOCK_COMPLETE, DATA_PLANE_RECORD_END,
   226	    DATA_PLANE_RECORD_FILE, DATA_PLANE_RECORD_TAR_SHARD,
   227	};
   228	
   229	/// Drive a `TransferSink` from a TCP wire stream.
   230	///
   231	/// This is the symmetric counterpart to [`execute_sink_pipeline_streaming`]:
   232	/// where the outbound executor takes a [`TransferSource`] and dispatches
   233	/// payloads round-robin across N sinks, this one consumes a single
   234	/// inbound wire (parsing record headers and producing
   235	/// [`PreparedPayload::FileStream`] / [`PreparedPayload::TarShard`] /
   236	/// [`PreparedPayload::FileBlock`] events) and feeds them to a single sink
   237	/// sequentially. Multi-stream parallelism comes from spawning N invocations,
   238	/// one per inbound TCP connection.
   239	///
   240	/// Both directions converge on `TransferSink::write_payload`: file data
   241	/// hits disk through `FsTransferSink::write_payload(FileStream { … })`,
   242	/// which uses the same `receive_stream_double_buffered` helper as the
   243	/// daemon's push receiver and the client's pull receiver — one path,
   244	/// one optimization surface.
   245	pub async fn execute_receive_pipeline<R: AsyncRead + Unpin + Send>(
   246	    socket: &mut R,
   247	    sink: Arc<dyn TransferSink>,
   248	    progress: Option<&RemoteTransferProgress>,
   249	) -> Result<SinkOutcome> {
   250	    let mut total = SinkOutcome::default();
   251	
   252	    loop {
   253	        let mut tag = [0u8; 1];
   254	        socket
   255	            .read_exact(&mut tag)
   256	            .await
   257	            .context("reading data-plane record tag")?;
   258	
   259	        match tag[0] {
   260	            DATA_PLANE_RECORD_END => break,

exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '1,560p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
     1	use eyre::{bail, Context, Result};
     2	use futures::StreamExt;
     3	use socket2::Socket;
     4	use tokio::io::{AsyncReadExt, AsyncWriteExt};
     5	use tokio::net::TcpStream;
     6	
     7	use crate::buffer::BufferPool;
     8	use crate::generated::FileHeader;
     9	
    10	use super::payload::{prepared_payload_stream, PreparedPayload, TransferPayload};
    11	use super::progress::{NoProbe, Probe};
    12	use super::stall_guard::{StallGuardWriter, TRANSFER_STALL_TIMEOUT};
    13	use crate::remote::transfer::source::TransferSource;
    14	use std::sync::Arc;
    15	
    16	pub const CONTROL_PLANE_CHUNK_SIZE: usize = 1024 * 1024;
    17	pub const DATA_PLANE_RECORD_FILE: u8 = 0;
    18	pub const DATA_PLANE_RECORD_TAR_SHARD: u8 = 1;
    19	pub const DATA_PLANE_RECORD_BLOCK: u8 = 2;
    20	pub const DATA_PLANE_RECORD_BLOCK_COMPLETE: u8 = 3;
    21	pub const DATA_PLANE_RECORD_END: u8 = 0xFF;
    22	
    23	/// A single data-plane TCP stream and its send loop.
    24	///
    25	/// Generic over a [`Probe`] so the byte-copy hot path can carry
    26	/// per-stream telemetry under adaptive mode at **zero cost** when the
    27	/// probe is [`NoProbe`] (the default): the instrumented branches are
    28	/// gated on `P::ACTIVE`, a compile-time constant, so they fold away
    29	/// entirely for `DataPlaneSession<NoProbe>`. Existing callers name the
    30	/// bare type and get the `NoProbe` default; the adaptive controller
    31	/// constructs `DataPlaneSession<LiveProbe>` via
    32	/// [`from_stream_with_probe`](DataPlaneSession::from_stream_with_probe).
    33	///
    34	/// audit-h3b: writes go through [`StallGuardWriter`] so a stalled
    35	/// reader (TCP backpressure from a slow / wedged peer) trips after
    36	/// [`TRANSFER_STALL_TIMEOUT`] of no observable write progress instead
    37	/// of pinning the worker for OS-level TCP retransmit exhaustion
    38	/// (15+ minutes). All existing `self.stream.write_all/.flush` call
    39	/// sites compose against the `AsyncWrite` impl of `StallGuardWriter`,
    40	/// so no per-site change was needed.
    41	pub struct DataPlaneSession<P: Probe = NoProbe> {
    42	    stream: StallGuardWriter<TcpStream>,
    43	    pool: Arc<BufferPool>,
    44	    trace: bool,
    45	    chunk_bytes: usize,
    46	    payload_prefetch: usize,
    47	    bytes_sent: u64,
    48	    probe: P,
    49	}
    50	
    51	macro_rules! trace_client {
    52	    ($session:expr, $($arg:tt)*) => {
    53	        if $session.trace {
    54	            eprintln!("[data-plane-client] {}", format_args!($($arg)*));
    55	        }
    56	    };
    57	}
    58	
    59	impl DataPlaneSession<NoProbe> {
    60	    /// Create a session from an existing stream with buffer pooling.
    61	    ///
    62	    /// Produces the un-instrumented `NoProbe` variant — the default for
    63	    /// every non-adaptive caller. audit-h3b: the stream is wrapped in
    64	    /// [`StallGuardWriter`] (inside `from_stream_with_probe`) so a
    65	    /// stalled peer trips after [`TRANSFER_STALL_TIMEOUT`] of no
    66	    /// observable write progress instead of pinning the worker for
    67	    /// OS-level TCP retransmit exhaustion. The production call sites
    68	    /// (`daemon/service/pull.rs`, `daemon/service/pull_sync.rs`, and the
    69	    /// resume path) inherit the guard without code changes.
    70	    pub async fn from_stream(
    71	        stream: TcpStream,
    72	        trace: bool,
    73	        chunk_bytes: usize,
    74	        payload_prefetch: usize,
    75	        pool: Arc<BufferPool>,
    76	    ) -> Self {
    77	        Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, NoProbe)
    78	            .await
    79	    }
    80	
    81	    /// Connect to a data plane endpoint with buffer pooling.
    82	    pub async fn connect(
    83	        host: &str,
    84	        port: u32,
    85	        token: &[u8],
    86	        chunk_bytes: usize,
    87	        payload_prefetch: usize,
    88	        trace: bool,
    89	        tcp_buffer_size: Option<usize>,
    90	        pool: Arc<BufferPool>,
    91	    ) -> Result<Self> {
    92	        let addr = format!("{}:{}", host, port);
    93	        if trace {
    94	            eprintln!("[data-plane-client] connecting to {}", addr);
    95	        }
    96	        let stream = TcpStream::connect(addr.clone())
    97	            .await
    98	            .with_context(|| format!("connecting to data plane {}", addr))?;
    99	
   100	        let std_stream = stream.into_std().context("converting to std stream")?;
   101	        let socket = Socket::from(std_stream);
   102	        socket
   103	            .set_tcp_nodelay(true)
   104	            .context("setting TCP_NODELAY")?;
   105	        // Keep idle connections alive during long transfers on
   106	        // other streams. Best-effort — kernel can refuse if the
   107	        // platform doesn't support keepalive on this socket type
   108	        // (uncommon but documented). Surface failures via log so a
   109	        // misconfigured run isn't silent. POST_REVIEW_FIXES §1.1.
   110	        if let Err(e) = socket.set_keepalive(true) {
   111	            log::warn!("set TCP keepalive on data-plane socket: {}", e);
   112	        }
   113	
   114	        if let Some(size) = tcp_buffer_size {
   115	            // Buffer-size knobs are advisory; the kernel can clamp.
   116	            // Log failures so operators can spot a sysctl/rlimit
   117	            // mismatch instead of wondering why throughput sat
   118	            // below the configured target.
   119	            if let Err(e) = socket.set_send_buffer_size(size) {
   120	                log::warn!("set TCP send buffer to {} bytes: {}", size, e);
   121	            }
   122	            if let Err(e) = socket.set_recv_buffer_size(size) {
   123	                log::warn!("set TCP recv buffer to {} bytes: {}", size, e);
   124	            }
   125	        }
   126	
   127	        let std_stream: std::net::TcpStream = socket.into();
   128	        let mut stream =
   129	            TcpStream::from_std(std_stream).context("converting back to tokio stream")?;
   130	
   131	        stream
   132	            .write_all(token)
   133	            .await
   134	            .context("writing negotiation token")?;
   135	
   136	        Ok(Self::from_stream(stream, trace, chunk_bytes, payload_prefetch, pool).await)
   137	    }
   138	}
   139	
   140	impl<P: Probe> DataPlaneSession<P> {
   141	    /// Create a session carrying an arbitrary [`Probe`]. The generic
   142	    /// primitive behind [`from_stream`](DataPlaneSession::from_stream);
   143	    /// the adaptive controller calls this with a `LiveProbe` to enable
   144	    /// per-stream telemetry.
   145	    pub async fn from_stream_with_probe(
   146	        stream: TcpStream,
   147	        trace: bool,
   148	        chunk_bytes: usize,
   149	        payload_prefetch: usize,
   150	        pool: Arc<BufferPool>,
   151	        probe: P,
   152	    ) -> Self {
   153	        let payload_prefetch = payload_prefetch.max(1);
   154	        let chunk_bytes = chunk_bytes.max(64 * 1024);
   155	        Self {
   156	            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
   157	            pool,
   158	            trace,
   159	            chunk_bytes,
   160	            payload_prefetch,
   161	            bytes_sent: 0,
   162	            probe,
   163	        }
   164	    }
   165	
   166	    pub async fn send_payloads(
   167	        &mut self,
   168	        source: Arc<dyn TransferSource>,
   169	        payloads: Vec<TransferPayload>,
   170	    ) -> Result<()> {
   171	        self.send_payloads_with_progress(source, payloads, None)
   172	            .await
   173	    }
   174	
   175	    pub async fn send_payloads_with_progress(
   176	        &mut self,
   177	        source: Arc<dyn TransferSource>,
   178	        payloads: Vec<TransferPayload>,
   179	        progress: Option<&super::progress::RemoteTransferProgress>,
   180	    ) -> Result<()> {
   181	        let mut stream = prepared_payload_stream(payloads, source.clone(), self.payload_prefetch);
   182	        while let Some(prepared) = stream.next().await {
   183	            match prepared? {
   184	                PreparedPayload::File(header) => {
   185	                    if let Err(err) = self.send_file(source.clone(), &header).await {
   186	                        return Err(err.wrap_err(format!("sending {}", header.relative_path)));
   187	                    }
   188	                    self.bytes_sent = self.bytes_sent.saturating_add(header.size);
   189	                    if let Some(progress) = progress {
   190	                        progress.report_file_complete(header.relative_path.clone(), header.size);
   191	                    }
   192	                }
   193	                PreparedPayload::TarShard { headers, data } => {
   194	                    let shard_bytes: u64 = headers.iter().map(|h| h.size).sum();
   195	                    if let Err(err) = self.send_prepared_tar_shard(headers.clone(), &data).await {
   196	                        return Err(err.wrap_err("sending tar shard"));
   197	                    }
   198	                    self.bytes_sent = self.bytes_sent.saturating_add(shard_bytes);
   199	                    if let Some(progress) = progress {
   200	                        for header in &headers {
   201	                            progress
   202	                                .report_file_complete(header.relative_path.clone(), header.size);
   203	                        }
   204	                    }
   205	                }
   206	                PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   207	                    bail!("DataPlaneSession::send_payloads does not handle resume payloads");
   208	                }
   209	            }
   210	        }
   211	
   212	        Ok(())
   213	    }
   214	
   215	    pub async fn finish(&mut self) -> Result<()> {
   216	        self.stream
   217	            .write_all(&[DATA_PLANE_RECORD_END])
   218	            .await
   219	            .context("writing transfer terminator")?;
   220	        self.stream
   221	            .flush()
   222	            .await
   223	            .context("flushing data plane stream")
   224	    }
   225	
   226	    pub fn bytes_sent(&self) -> u64 {
   227	        self.bytes_sent
   228	    }
   229	
   230	    pub async fn send_file(
   231	        &mut self,
   232	        source: Arc<dyn TransferSource>,
   233	        header: &FileHeader,
   234	    ) -> Result<()> {
   235	        let rel = &header.relative_path;
   236	        let mut file = source
   237	            .open_file(header)
   238	            .await
   239	            .with_context(|| format!("opening {}", rel))?;
   240	        self.send_file_from_reader(header, &mut file).await
   241	    }
   242	
   243	    /// Send a file payload whose bytes come from an arbitrary async
   244	    /// reader (not a local file). Used by `DataPlaneSink` for the
   245	    /// remote→remote relay case, where bytes arrive from an inbound
   246	    /// `DataPlaneSource` and need to be forwarded to the next hop.
   247	    ///
   248	    /// Same wire format and double-buffered loop as `send_file`.
   249	    pub async fn send_file_from_reader(
   250	        &mut self,
   251	        header: &FileHeader,
   252	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   253	    ) -> Result<()> {
   254	        let rel = &header.relative_path;
   255	        trace_client!(self, "sending file '{}' ({} bytes)", rel, header.size);
   256	
   257	        let path_bytes = rel.as_bytes();
   258	        if path_bytes.len() > u32::MAX as usize {
   259	            bail!("relative path too long for transfer: {}", rel);
   260	        }
   261	
   262	        self.stream
   263	            .write_all(&[DATA_PLANE_RECORD_FILE])
   264	            .await
   265	            .context("writing data-plane record tag")?;
   266	        self.stream
   267	            .write_all(&(path_bytes.len() as u32).to_be_bytes())
   268	            .await
   269	            .context("writing path length")?;
   270	        self.stream
   271	            .write_all(path_bytes)
   272	            .await
   273	            .context("writing path bytes")?;
   274	
   275	        self.stream
   276	            .write_all(&header.size.to_be_bytes())
   277	            .await
   278	            .context("writing file size")?;
   279	        // Wire-format extension (2026-05-01): include mtime + permissions
   280	        // inline so push and pull data plane records carry the same
   281	        // information. Lets the receive pipeline apply metadata via
   282	        // FsTransferSink without consulting an out-of-band manifest cache.
   283	        self.stream
   284	            .write_all(&header.mtime_seconds.to_be_bytes())
   285	            .await
   286	            .context("writing mtime")?;
   287	        self.stream
   288	            .write_all(&header.permissions.to_be_bytes())
   289	            .await
   290	            .context("writing permissions")?;
   291	
   292	        // Double-buffered I/O: overlaps source reads with network writes
   293	        self.send_file_double_buffered(reader, header, rel).await?;
   294	
   295	        trace_client!(self, "file '{}' sent ({} bytes)", rel, header.size);
   296	
   297	        Ok(())
   298	    }
   299	
   300	    /// Double-buffered file sending: overlaps disk reads with network writes.
   301	    /// Uses two buffers from the pool to enable concurrent I/O operations.
   302	    ///
   303	    /// Pattern: While buffer A is being written to network, buffer B is filled from disk.
   304	    /// This hides disk latency behind network latency for improved throughput.
   305	    async fn send_file_double_buffered(
   306	        &mut self,
   307	        file: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   308	        header: &FileHeader,
   309	        rel: &str,
   310	    ) -> Result<()> {
   311	        let mut remaining = header.size;
   312	        if remaining == 0 {
   313	            return Ok(());
   314	        }
   315	
   316	        // Acquire two buffers for double-buffering
   317	        let mut buf_a = self.pool.acquire().await;
   318	        let mut buf_b = self.pool.acquire().await;
   319	
   320	        // Initial read into buf_a
   321	        let mut bytes_a = file
   322	            .read(buf_a.as_mut_slice())
   323	            .await
   324	            .with_context(|| format!("reading {}", rel))?;
   325	
   326	        if bytes_a == 0 {
   327	            bail!(
   328	                "unexpected EOF while reading {} ({} bytes remaining)",
   329	                rel,
   330	                remaining
   331	            );
   332	        }
   333	        // Clamp to the declared size before subtracting. A source that
   334	        // returns more bytes than `header.size` — a file that grew after
   335	        // the manifest was computed, or a lying `TransferSource` — would
   336	        // otherwise underflow `remaining` (debug: panic; release: wrap to
   337	        // u64::MAX → runaway loop) and push undeclared bytes onto the
   338	        // framed stream. We send exactly `header.size` and ignore excess.
   339	        bytes_a = (bytes_a as u64).min(remaining) as usize;
   340	        remaining -= bytes_a as u64;
   341	
   342	        // Main loop: write buf_a while reading into buf_b
   343	        while remaining > 0 {
   344	            // Per-stream telemetry: time the overlapped write+read step
   345	            // as a backpressure proxy. Gated on the compile-time
   346	            // `P::ACTIVE` constant so `DataPlaneSession<NoProbe>` reads
   347	            // no clock and folds this to nothing.
   348	            let step_start = if P::ACTIVE {
   349	                Some(std::time::Instant::now())
   350	            } else {
   351	                None
   352	            };
   353	            // Overlap: write from buf_a, read into buf_b concurrently
   354	            let (write_result, read_result) = tokio::join!(
   355	                self.stream.write_all(&buf_a.as_slice()[..bytes_a]),
   356	                file.read(buf_b.as_mut_slice())
   357	            );
   358	
   359	            write_result.with_context(|| format!("sending {}", rel))?;
   360	            if P::ACTIVE {
   361	                if let Some(t) = step_start {
   362	                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
   363	                }
   364	            }
   365	            self.probe.record_bytes(bytes_a as u64);
   366	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
   367	
   368	            let bytes_b = read_result.with_context(|| format!("reading {}", rel))?;
   369	
   370	            if bytes_b == 0 && remaining > 0 {
   371	                bail!(
   372	                    "unexpected EOF while reading {} ({} bytes remaining)",
   373	                    rel,
   374	                    remaining
   375	                );
   376	            }
   377	            // Same clamp as the initial read: never subtract more than
   378	            // `remaining`, so an over-returning reader can neither
   379	            // underflow the counter nor send undeclared bytes.
   380	            let bytes_b = (bytes_b as u64).min(remaining) as usize;
   381	            remaining -= bytes_b as u64;
   382	
   383	            // Swap roles: buf_b becomes the write buffer, buf_a becomes read buffer
   384	            std::mem::swap(&mut buf_a, &mut buf_b);
   385	            bytes_a = bytes_b;
   386	        }
   387	
   388	        // Final write: send the last chunk in buf_a. This is a pure
   389	        // write (no overlapped read), so the timing is cleanly
   390	        // attributable to socket-write backpressure.
   391	        if bytes_a > 0 {
   392	            let tail_start = if P::ACTIVE {
   393	                Some(std::time::Instant::now())
   394	            } else {
   395	                None
   396	            };
   397	            self.stream
   398	                .write_all(&buf_a.as_slice()[..bytes_a])
   399	                .await
   400	                .with_context(|| format!("sending {}", rel))?;
   401	            if P::ACTIVE {
   402	                if let Some(t) = tail_start {
   403	                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
   404	                }
   405	            }
   406	            self.probe.record_bytes(bytes_a as u64);
   407	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
   408	        }
   409	
   410	        // Buffers return to pool automatically on drop
   411	        Ok(())
   412	    }
   413	
   414	    pub async fn send_prepared_tar_shard(
   415	        &mut self,
   416	        headers: Vec<FileHeader>,
   417	        data: &[u8],
   418	    ) -> Result<()> {
   419	        let shard_len = headers.len();
   420	        let preview = headers
   421	            .first()
   422	            .map(|h| h.relative_path.as_str())
   423	            .unwrap_or("<empty>");
   424	        trace_client!(
   425	            self,
   426	            "sending tar shard with {} file(s), {} bytes (first='{}')",
   427	            shard_len,
   428	            data.len(),
   429	            preview
   430	        );
   431	        self.stream
   432	            .write_all(&[DATA_PLANE_RECORD_TAR_SHARD])
   433	            .await
   434	            .context("writing tar shard record tag")?;
   435	        self.stream
   436	            .write_all(&(headers.len() as u32).to_be_bytes())
   437	            .await
   438	            .context("writing tar shard count")?;
   439	
   440	        for header in headers {
   441	            let rel_bytes = header.relative_path.as_bytes();
   442	            if rel_bytes.len() > u32::MAX as usize {
   443	                bail!(
   444	                    "relative path too long for transfer: {}",
   445	                    header.relative_path
   446	                );
   447	            }
   448	            self.stream
   449	                .write_all(&(rel_bytes.len() as u32).to_be_bytes())
   450	                .await
   451	                .context("writing shard path length")?;
   452	            self.stream
   453	                .write_all(rel_bytes)
   454	                .await
   455	                .context("writing shard path bytes")?;
   456	            self.stream
   457	                .write_all(&header.size.to_be_bytes())
   458	                .await
   459	                .context("writing shard size")?;
   460	            self.stream
   461	                .write_all(&header.mtime_seconds.to_be_bytes())
   462	                .await
   463	                .context("writing shard mtime")?;
   464	            self.stream
   465	                .write_all(&header.permissions.to_be_bytes())
   466	                .await
   467	                .context("writing shard permissions")?;
   468	        }
   469	
   470	        self.stream
   471	            .write_all(&(data.len() as u64).to_be_bytes())
   472	            .await
   473	            .context("writing tar shard length")?;
   474	        for chunk in data.chunks(self.chunk_bytes.max(1)) {
   475	            self.stream
   476	                .write_all(chunk)
   477	                .await
   478	                .context("writing tar shard payload")?;
   479	            self.probe.record_bytes(chunk.len() as u64);
   480	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(chunk.len() as u64);
   481	        }
   482	        trace_client!(
   483	            self,
   484	            "tar shard payload sent ({} file(s), {} bytes)",
   485	            shard_len,
   486	            data.len()
   487	        );
   488	
   489	        Ok(())
   490	    }
   491	
   492	    /// Send a single block for block-level resume.
   493	    /// Format: [type:1][path_len:4][path][offset:8][block_len:4][content]
   494	    pub async fn send_block(
   495	        &mut self,
   496	        relative_path: &str,
   497	        offset: u64,
   498	        content: &[u8],
   499	    ) -> Result<()> {
   500	        let path_bytes = relative_path.as_bytes();
   501	        if path_bytes.len() > u32::MAX as usize {
   502	            bail!("relative path too long for transfer: {}", relative_path);
   503	        }
   504	
   505	        trace_client!(
   506	            self,
   507	            "sending block for '{}' at offset {} ({} bytes)",
   508	            relative_path,
   509	            offset,
   510	            content.len()
   511	        );
   512	
   513	        self.stream
   514	            .write_all(&[DATA_PLANE_RECORD_BLOCK])
   515	            .await
   516	            .context("writing block record tag")?;
   517	        self.stream
   518	            .write_all(&(path_bytes.len() as u32).to_be_bytes())
   519	            .await
   520	            .context("writing path length")?;
   521	        self.stream
   522	            .write_all(path_bytes)
   523	            .await
   524	            .context("writing path bytes")?;
   525	        self.stream
   526	            .write_all(&offset.to_be_bytes())
   527	            .await
   528	            .context("writing block offset")?;
   529	        self.stream
   530	            .write_all(&(content.len() as u32).to_be_bytes())
   531	            .await
   532	            .context("writing block length")?;
   533	        self.stream
   534	            .write_all(content)
   535	            .await
   536	            .context("writing block content")?;
   537	        crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(content.len() as u64);
   538	
   539	        self.bytes_sent += content.len() as u64;
   540	        Ok(())
   541	    }
   542	
   543	    /// Signal that block-level transfer for a file is complete.
   544	    /// Format: [type:1][path_len:4][path][total_size:8][mtime:8][perms:4]
   545	    ///
   546	    /// Carries mtime + perms inline so the receiver can stamp the
   547	    /// destination metadata even when zero blocks transferred (the
   548	    /// "mtime touched, content identical" case for mirror).
   549	    pub async fn send_block_complete(
   550	        &mut self,
   551	        relative_path: &str,
   552	        total_size: u64,
   553	        mtime_seconds: i64,
   554	        permissions: u32,
   555	    ) -> Result<()> {
   556	        let path_bytes = relative_path.as_bytes();
   557	        if path_bytes.len() > u32::MAX as usize {
   558	            bail!("relative path too long for transfer: {}", relative_path);
   559	        }
   560	

exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '900,1360p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   900	
   901	    /// POST_REVIEW_FIXES §1.1b regression. When a sink errors mid-
   902	    /// pipeline, `execute_sink_pipeline_streaming` must return the
   903	    /// underlying error message — not the previous generic "data
   904	    /// plane pipeline closed unexpectedly" produced by
   905	    /// `MultiStreamSender::queue` when its `tx.send` saw the receiver
   906	    /// drop. The fix in `MultiStreamSender::queue` only works if this
   907	    /// invariant holds at the pipeline layer.
   908	    #[tokio::test]
   909	    async fn pipeline_streaming_surfaces_underlying_sink_error() {
   910	        let tmp = tempdir().unwrap();
   911	        let src = tmp.path().join("src");
   912	        std::fs::create_dir_all(&src).unwrap();
   913	        std::fs::write(src.join("a.txt"), b"alpha").unwrap();
   914	
   915	        let source = Arc::new(FsTransferSource::new(src.clone()));
   916	        let dst = tmp.path().join("dst");
   917	        let failing: Arc<dyn TransferSink> = Arc::new(FailingSink {
   918	            marker: "synthetic sink failure: disk full",
   919	            dst_root: dst,
   920	        });
   921	
   922	        let unreadable = Arc::new(Mutex::new(Vec::new()));
   923	        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
   924	        let mut headers = Vec::new();
   925	        while let Some(h) = header_rx.recv().await {
   926	            headers.push(h);
   927	        }
   928	        let _scanned = scan_handle.await.unwrap().unwrap();
   929	
   930	        let planned = crate::remote::transfer::payload::plan_transfer_payloads(
   931	            headers,
   932	            source.root(),
   933	            Default::default(),
   934	        )
   935	        .unwrap();
   936	
   937	        // Feed the planned payloads through the streaming variant
   938	        // exactly as MultiStreamSender::connect does it: spawn the
   939	        // pipeline in a task, push payloads via mpsc, then drop the
   940	        // sender to signal end-of-stream.
   941	        let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(4);
   942	        let source_clone = Arc::clone(&source);
   943	        let pipeline = tokio::spawn(async move {
   944	            execute_sink_pipeline_streaming(source_clone, vec![failing], payload_rx, 4, None).await
   945	        });
   946	
   947	        for payload in planned.payloads {
   948	            // Sink errors after the first write; later sends may
   949	            // race the channel close. We only care that the
   950	            // pipeline future resolves with the real error.
   951	            let _ = payload_tx.send(payload).await;
   952	        }
   953	        drop(payload_tx);
   954	
   955	        let result = pipeline.await.expect("pipeline task did not panic");
   956	        let err = result.expect_err("pipeline must surface the sink error");
   957	        let msg = format!("{:#}", err);
   958	        assert!(
   959	            msg.contains("synthetic sink failure: disk full"),
   960	            "expected pipeline error to include underlying sink message; got:\n{}",
   961	            msg
   962	        );
   963	    }
   964	
   965	    /// audit-1c2: a receive that stalls (no bytes) must abort with the
   966	    /// StallGuard's TimedOut rather than blocking forever. A duplex whose
   967	    /// writer half is held open but never written keeps the first record-
   968	    /// tag read perpetually Pending; the StallGuard wrapping it trips
   969	    /// after the (short, test) idle window and the pipeline surfaces it.
   970	    #[tokio::test]
   971	    async fn receive_pipeline_aborts_on_stall() {
   972	        use crate::remote::transfer::stall_guard::StallGuard;
   973	        use std::path::PathBuf;
   974	
   975	        let tmp = tempdir().unwrap();
   976	        let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
   977	            PathBuf::from("/nonexistent-src"),
   978	            tmp.path().to_path_buf(),
   979	            FsSinkConfig {
   980	                preserve_times: false,
   981	                dry_run: false,
   982	                checksum: None,
   983	                resume: false,
   984	                compare_mode: ComparisonMode::SizeMtime,
   985	            },
   986	        ));
   987	
   988	        // Writer half held open (bound to a name) but never written → the
   989	        // read side is perpetually Pending.
   990	        let (rx, _tx) = tokio::io::duplex(64);
   991	        let mut guarded = StallGuard::new(rx, std::time::Duration::from_millis(20));
   992	
   993	        let err = execute_receive_pipeline(&mut guarded, sink, None)
   994	            .await
   995	            .expect_err("a stalled receive must abort, not hang");
   996	        assert!(
   997	            format!("{err:#}").contains("stalled"),
   998	            "expected a StallGuard timeout in the error chain; got: {err:#}"
   999	        );
  1000	    }
  1001	}
  1002	
  1003	#[cfg(test)]
  1004	mod workqueue_tests {
  1005	    //! PR2: the shared work-queue must let a fast sink steal work a slow
  1006	    //! sink would otherwise have been assigned under the old round-robin
  1007	    //! dispatcher. Without work-stealing, N payloads split evenly across
  1008	    //! sinks and one slow sink bottlenecks the whole transfer; with it,
  1009	    //! the fast sink absorbs the bulk.
  1010	    use super::*;
  1011	    use crate::remote::transfer::sink::{SinkOutcome, TransferSink};
  1012	    use crate::remote::transfer::source::FsTransferSource;
  1013	    use std::path::{Path, PathBuf};
  1014	    use std::sync::atomic::{AtomicU64, Ordering};
  1015	    use std::sync::{Arc, Mutex};
  1016	    use std::time::Duration;
  1017	    use tempfile::tempdir;
  1018	
  1019	    /// Counts payloads it writes; optionally sleeps per payload to model
  1020	    /// a slow stream. Ignores the payload bytes — timing is governed
  1021	    /// purely by the configured delay, isolating the dispatch behaviour.
  1022	    struct CountingSink {
  1023	        delay: Duration,
  1024	        count: Arc<AtomicU64>,
  1025	        root: PathBuf,
  1026	    }
  1027	
  1028	    #[async_trait::async_trait]
  1029	    impl TransferSink for CountingSink {
  1030	        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
  1031	            if !self.delay.is_zero() {
  1032	                tokio::time::sleep(self.delay).await;
  1033	            }
  1034	            self.count.fetch_add(1, Ordering::Relaxed);
  1035	            Ok(SinkOutcome {
  1036	                files_written: 1,
  1037	                bytes_written: 0,
  1038	            })
  1039	        }
  1040	        fn root(&self) -> &Path {
  1041	            &self.root
  1042	        }
  1043	    }
  1044	
  1045	    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  1046	    async fn fast_sink_steals_work_from_slow_sink() {
  1047	        let tmp = tempdir().unwrap();
  1048	        let src = tmp.path().join("src");
  1049	        std::fs::create_dir_all(&src).unwrap();
  1050	        let n = 40usize;
  1051	        for i in 0..n {
  1052	            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
  1053	        }
  1054	
  1055	        let source = Arc::new(FsTransferSource::new(src.clone()));
  1056	        let unreadable = Arc::new(Mutex::new(Vec::new()));
  1057	        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
  1058	        let mut headers = Vec::new();
  1059	        while let Some(h) = header_rx.recv().await {
  1060	            headers.push(h);
  1061	        }
  1062	        let _ = scan_handle.await.unwrap().unwrap();
  1063	        // Feed each file as its OWN payload (not via plan_transfer_payloads,
  1064	        // which bundles tiny files into a single tar shard — that would
  1065	        // leave only one payload and nothing to steal).
  1066	        assert_eq!(headers.len(), n, "expected one header per file");
  1067	
  1068	        let fast_count = Arc::new(AtomicU64::new(0));
  1069	        let slow_count = Arc::new(AtomicU64::new(0));
  1070	        let fast: Arc<dyn TransferSink> = Arc::new(CountingSink {
  1071	            delay: Duration::ZERO,
  1072	            count: Arc::clone(&fast_count),
  1073	            root: PathBuf::from("/fast"),
  1074	        });
  1075	        let slow: Arc<dyn TransferSink> = Arc::new(CountingSink {
  1076	            delay: Duration::from_millis(20),
  1077	            count: Arc::clone(&slow_count),
  1078	            root: PathBuf::from("/slow"),
  1079	        });
  1080	
  1081	        let (tx, rx) = mpsc::channel::<TransferPayload>(4);
  1082	        let feeder = tokio::spawn(async move {
  1083	            for h in headers {
  1084	                if tx.send(TransferPayload::File(h)).await.is_err() {
  1085	                    break;
  1086	                }
  1087	            }
  1088	        });
  1089	        let outcome = execute_sink_pipeline_streaming(source, vec![fast, slow], rx, 2, None)
  1090	            .await
  1091	            .expect("pipeline ok");
  1092	        let _ = feeder.await;
  1093	
  1094	        let fast_n = fast_count.load(Ordering::Relaxed);
  1095	        let slow_n = slow_count.load(Ordering::Relaxed);
  1096	        assert_eq!(outcome.files_written, n, "every payload written once");
  1097	        assert_eq!(
  1098	            fast_n + slow_n,
  1099	            n as u64,
  1100	            "every payload accounted to exactly one sink"
  1101	        );
  1102	        // Round-robin would force ~20/20 and the slow sink would gate the
  1103	        // whole transfer. Work-stealing lets the zero-delay sink take the
  1104	        // overwhelming majority while the slow sink sits in its sleeps.
  1105	        assert!(
  1106	            fast_n > slow_n * 3,
  1107	            "fast sink should steal the bulk of the work: fast={fast_n} slow={slow_n}"
  1108	        );
  1109	    }
  1110	
  1111	    /// Codex-review (PR2) regression: when the only sink errors, the
  1112	    /// forwarder must stop draining the producer promptly rather than
  1113	    /// continuing to pull every remaining payload. We feed a large
  1114	    /// payload set through a single always-failing sink and assert that
  1115	    /// (a) the pipeline surfaces the error, and (b) the forwarder
  1116	    /// consumed far fewer than all payloads before halting — proving the
  1117	    /// cancel flag short-circuits intake instead of draining to the end.
  1118	    struct ErrSink {
  1119	        root: PathBuf,
  1120	    }
  1121	
  1122	    #[async_trait::async_trait]
  1123	    impl TransferSink for ErrSink {
  1124	        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
  1125	            eyre::bail!("synthetic immediate failure")
  1126	        }
  1127	        fn root(&self) -> &Path {
  1128	            &self.root
  1129	        }
  1130	    }
  1131	
  1132	    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
  1133	    async fn forwarder_stops_promptly_on_worker_error() {
  1134	        let tmp = tempdir().unwrap();
  1135	        let src = tmp.path().join("src");
  1136	        std::fs::create_dir_all(&src).unwrap();
  1137	        let n = 200usize;
  1138	        for i in 0..n {
  1139	            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
  1140	        }
  1141	        let source = Arc::new(FsTransferSource::new(src.clone()));
  1142	        let unreadable = Arc::new(Mutex::new(Vec::new()));
  1143	        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
  1144	        let mut headers = Vec::new();
  1145	        while let Some(h) = header_rx.recv().await {
  1146	            headers.push(h);
  1147	        }
  1148	        let _ = scan_handle.await.unwrap().unwrap();
  1149	        assert_eq!(headers.len(), n);
  1150	
  1151	        let sink: Arc<dyn TransferSink> = Arc::new(ErrSink {
  1152	            root: PathBuf::from("/err"),
  1153	        });
  1154	
  1155	        // Count how many payloads the forwarder actually pulled from the
  1156	        // producer. With prefetch=1 and a single sink, the bounded queue
  1157	        // holds 1; once the sink errors and trips `cancelled`, the
  1158	        // forwarder must stop, so `sent` stays a tiny constant rather
  1159	        // than reaching n.
  1160	        let sent = Arc::new(AtomicU64::new(0));
  1161	        let (tx, rx) = mpsc::channel::<TransferPayload>(1);
  1162	        let sent_feeder = sent.clone();
  1163	        let feeder = tokio::spawn(async move {
  1164	            for h in headers {
  1165	                if tx.send(TransferPayload::File(h)).await.is_err() {
  1166	                    break;
  1167	                }
  1168	                sent_feeder.fetch_add(1, Ordering::Relaxed);
  1169	            }
  1170	        });
  1171	
  1172	        let result = execute_sink_pipeline_streaming(source, vec![sink], rx, 1, None).await;
  1173	        let _ = feeder.await;
  1174	
  1175	        assert!(result.is_err(), "pipeline must surface the sink error");
  1176	        let pulled = sent.load(Ordering::Relaxed);
  1177	        assert!(
  1178	            pulled < (n as u64) / 2,
  1179	            "forwarder should halt soon after the error, not drain all {n} payloads; pulled={pulled}"
  1180	        );
  1181	    }
  1182	
  1183	    /// Reports each payload's real byte size so the executor's byte and
  1184	    /// file aggregation can be checked end to end without touching disk.
  1185	    struct ByteSink {
  1186	        bytes: Arc<AtomicU64>,
  1187	        root: PathBuf,
  1188	    }
  1189	
  1190	    #[async_trait::async_trait]
  1191	    impl TransferSink for ByteSink {
  1192	        async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
  1193	            let (files, bytes): (usize, u64) = match &payload {
  1194	                PreparedPayload::File(h) => (1, h.size),
  1195	                PreparedPayload::TarShard { headers, .. } => {
  1196	                    (headers.len(), headers.iter().map(|h| h.size).sum())
  1197	                }
  1198	                _ => (0, 0),
  1199	            };
  1200	            self.bytes.fetch_add(bytes, Ordering::Relaxed);
  1201	            Ok(SinkOutcome {
  1202	                files_written: files,
  1203	                bytes_written: bytes,
  1204	            })
  1205	        }
  1206	        fn root(&self) -> &Path {
  1207	            &self.root
  1208	        }
  1209	    }
  1210	
  1211	    /// REV4 ue-r2-1a (work-stealing as behaviour): byte and file totals
  1212	    /// stay correct when two sinks pull from the shared queue. Distinct
  1213	    /// per-file sizes mean any double-count or dropped payload shifts the
  1214	    /// totals, and the per-sink sum pins that every byte lands on exactly
  1215	    /// one sink.
  1216	    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  1217	    async fn byte_and_file_totals_correct_under_work_stealing() {
  1218	        let tmp = tempdir().unwrap();
  1219	        let src = tmp.path().join("src");
  1220	        std::fs::create_dir_all(&src).unwrap();
  1221	        let n = 30usize;
  1222	        let mut expected_bytes = 0u64;
  1223	        for i in 0..n {
  1224	            // Distinct sizes so a miscount (double-add / drop) is visible.
  1225	            let body = vec![b'x'; i + 1];
  1226	            expected_bytes += body.len() as u64;
  1227	            std::fs::write(src.join(format!("f{i}.dat")), &body).unwrap();
  1228	        }
  1229	        let source = Arc::new(FsTransferSource::new(src.clone()));
  1230	        let unreadable = Arc::new(Mutex::new(Vec::new()));
  1231	        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
  1232	        let mut headers = Vec::new();
  1233	        while let Some(h) = header_rx.recv().await {
  1234	            headers.push(h);
  1235	        }
  1236	        let _ = scan_handle.await.unwrap().unwrap();
  1237	        assert_eq!(headers.len(), n, "one header per file");
  1238	
  1239	        let bytes_a = Arc::new(AtomicU64::new(0));
  1240	        let bytes_b = Arc::new(AtomicU64::new(0));
  1241	        let a: Arc<dyn TransferSink> = Arc::new(ByteSink {
  1242	            bytes: Arc::clone(&bytes_a),
  1243	            root: PathBuf::from("/a"),
  1244	        });
  1245	        let b: Arc<dyn TransferSink> = Arc::new(ByteSink {
  1246	            bytes: Arc::clone(&bytes_b),
  1247	            root: PathBuf::from("/b"),
  1248	        });
  1249	
  1250	        let (tx, rx) = mpsc::channel::<TransferPayload>(4);
  1251	        let feeder = tokio::spawn(async move {
  1252	            for h in headers {
  1253	                if tx.send(TransferPayload::File(h)).await.is_err() {
  1254	                    break;
  1255	                }
  1256	            }
  1257	        });
  1258	        let outcome = execute_sink_pipeline_streaming(source, vec![a, b], rx, 2, None)
  1259	            .await
  1260	            .expect("pipeline ok");
  1261	        let _ = feeder.await;
  1262	
  1263	        assert_eq!(outcome.files_written, n, "file total");
  1264	        assert_eq!(outcome.bytes_written, expected_bytes, "byte total");
  1265	        assert_eq!(
  1266	            bytes_a.load(Ordering::Relaxed) + bytes_b.load(Ordering::Relaxed),
  1267	            expected_bytes,
  1268	            "every byte accounted to exactly one sink, none double-counted"
  1269	        );
  1270	    }
  1271	
  1272	    /// REV4 ue-r2-1a (cancellation): when the producer stops feeding and
  1273	    /// drops the channel mid-stream, the work-stealing executor winds
  1274	    /// down promptly — workers drain what was queued, run `finish`, and
  1275	    /// the call returns without hanging (the timeout is the no-hang
  1276	    /// assertion). Only the fed payloads complete; nothing past the
  1277	    /// cancellation point is invented.
  1278	    ///
  1279	    /// Hard-abort of in-flight workers on dropping the pipeline future
  1280	    /// itself is out of scope here: the workers are bare `tokio::spawn`
  1281	    /// (a `JoinHandle` drop does not abort the task), which is the
  1282	    /// AbortOnDrop family tracked under w4-1.
  1283	    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  1284	    async fn producer_cancel_winds_down_pipeline_promptly() {
  1285	        let tmp = tempdir().unwrap();
  1286	        let src = tmp.path().join("src");
  1287	        std::fs::create_dir_all(&src).unwrap();
  1288	        let n = 50usize;
  1289	        for i in 0..n {
  1290	            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
  1291	        }
  1292	        let source = Arc::new(FsTransferSource::new(src.clone()));
  1293	        let unreadable = Arc::new(Mutex::new(Vec::new()));
  1294	        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
  1295	        let mut headers = Vec::new();
  1296	        while let Some(h) = header_rx.recv().await {
  1297	            headers.push(h);
  1298	        }
  1299	        let _ = scan_handle.await.unwrap().unwrap();
  1300	        assert_eq!(headers.len(), n);
  1301	
  1302	        let count = Arc::new(AtomicU64::new(0));
  1303	        let sink: Arc<dyn TransferSink> = Arc::new(CountingSink {
  1304	            delay: Duration::ZERO,
  1305	            count: Arc::clone(&count),
  1306	            root: PathBuf::from("/c"),
  1307	        });
  1308	
  1309	        // Feed only the first 5 headers, then drop the sender to model a
  1310	        // cancelled / aborted producer.
  1311	        let (tx, rx) = mpsc::channel::<TransferPayload>(2);
  1312	        let feeder = tokio::spawn(async move {
  1313	            for h in headers.into_iter().take(5) {
  1314	                if tx.send(TransferPayload::File(h)).await.is_err() {
  1315	                    break;
  1316	                }
  1317	            }
  1318	            // `tx` dropped here → channel closes → pipeline must wind down.
  1319	        });
  1320	
  1321	        let outcome = tokio::time::timeout(
  1322	            Duration::from_secs(5),
  1323	            execute_sink_pipeline_streaming(source, vec![sink], rx, 2, None),
  1324	        )
  1325	        .await
  1326	        .expect("pipeline must wind down promptly after producer cancels, not hang")
  1327	        .expect("graceful shutdown is not an error");
  1328	        let _ = feeder.await;
  1329	
  1330	        assert_eq!(
  1331	            outcome.files_written, 5,
  1332	            "only the fed payloads are written"
  1333	        );
  1334	        assert_eq!(count.load(Ordering::Relaxed), 5);
  1335	    }
  1336	}

exec
/usr/bin/bash -lc "git show 515fb76:crates/blit-core/src/remote/transfer/data_plane.rs | nl -ba | sed -n '1,520p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
     1	use eyre::{bail, Context, Result};
     2	use futures::StreamExt;
     3	use socket2::Socket;
     4	use tokio::io::{AsyncReadExt, AsyncWriteExt};
     5	use tokio::net::TcpStream;
     6	
     7	use crate::buffer::BufferPool;
     8	use crate::generated::FileHeader;
     9	
    10	use super::payload::{prepared_payload_stream, PreparedPayload, TransferPayload};
    11	use super::stall_guard::{StallGuardWriter, TRANSFER_STALL_TIMEOUT};
    12	use crate::remote::transfer::source::TransferSource;
    13	use std::sync::Arc;
    14	
    15	pub const CONTROL_PLANE_CHUNK_SIZE: usize = 1024 * 1024;
    16	pub const DATA_PLANE_RECORD_FILE: u8 = 0;
    17	pub const DATA_PLANE_RECORD_TAR_SHARD: u8 = 1;
    18	pub const DATA_PLANE_RECORD_BLOCK: u8 = 2;
    19	pub const DATA_PLANE_RECORD_BLOCK_COMPLETE: u8 = 3;
    20	pub const DATA_PLANE_RECORD_END: u8 = 0xFF;
    21	
    22	pub struct DataPlaneSession {
    23	    // audit-h3b: writes go through StallGuardWriter so a stalled
    24	    // reader (TCP backpressure from a slow / wedged peer) trips after
    25	    // TRANSFER_STALL_TIMEOUT of no observable write progress instead
    26	    // of pinning the worker for OS-level TCP retransmit exhaustion
    27	    // (15+ minutes). All existing `self.stream.write_all/.flush`
    28	    // call sites in this file (~30 sites) compose against the
    29	    // AsyncWrite impl of StallGuardWriter, so no per-site change
    30	    // was needed.
    31	    stream: StallGuardWriter<TcpStream>,
    32	    pool: Arc<BufferPool>,
    33	    trace: bool,
    34	    chunk_bytes: usize,
    35	    payload_prefetch: usize,
    36	    bytes_sent: u64,
    37	}
    38	
    39	macro_rules! trace_client {
    40	    ($session:expr, $($arg:tt)*) => {
    41	        if $session.trace {
    42	            eprintln!("[data-plane-client] {}", format_args!($($arg)*));
    43	        }
    44	    };
    45	}
    46	
    47	impl DataPlaneSession {
    48	    /// Create a session from an existing stream with buffer pooling.
    49	    ///
    50	    /// audit-h3b: the stream is wrapped in [`StallGuardWriter`] so a
    51	    /// stalled peer (slow / wedged reader causing TCP backpressure)
    52	    /// trips after [`TRANSFER_STALL_TIMEOUT`] of no observable write
    53	    /// progress instead of pinning the worker for OS-level TCP
    54	    /// retransmit exhaustion. All three production call sites
    55	    /// (`daemon/service/pull.rs` regular pull,
    56	    /// `daemon/service/pull_sync.rs` regular pull-sync, same file's
    57	    /// resume mode) inherit the guard without code changes.
    58	    pub async fn from_stream(
    59	        stream: TcpStream,
    60	        trace: bool,
    61	        chunk_bytes: usize,
    62	        payload_prefetch: usize,
    63	        pool: Arc<BufferPool>,
    64	    ) -> Self {
    65	        let payload_prefetch = payload_prefetch.max(1);
    66	        let chunk_bytes = chunk_bytes.max(64 * 1024);
    67	        Self {
    68	            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
    69	            pool,
    70	            trace,
    71	            chunk_bytes,
    72	            payload_prefetch,
    73	            bytes_sent: 0,
    74	        }
    75	    }
    76	
    77	    /// Connect to a data plane endpoint with buffer pooling.
    78	    pub async fn connect(
    79	        host: &str,
    80	        port: u32,
    81	        token: &[u8],
    82	        chunk_bytes: usize,
    83	        payload_prefetch: usize,
    84	        trace: bool,
    85	        tcp_buffer_size: Option<usize>,
    86	        pool: Arc<BufferPool>,
    87	    ) -> Result<Self> {
    88	        let addr = format!("{}:{}", host, port);
    89	        if trace {
    90	            eprintln!("[data-plane-client] connecting to {}", addr);
    91	        }
    92	        let stream = TcpStream::connect(addr.clone())
    93	            .await
    94	            .with_context(|| format!("connecting to data plane {}", addr))?;
    95	
    96	        let std_stream = stream.into_std().context("converting to std stream")?;
    97	        let socket = Socket::from(std_stream);
    98	        socket
    99	            .set_tcp_nodelay(true)
   100	            .context("setting TCP_NODELAY")?;
   101	        // Keep idle connections alive during long transfers on
   102	        // other streams. Best-effort — kernel can refuse if the
   103	        // platform doesn't support keepalive on this socket type
   104	        // (uncommon but documented). Surface failures via log so a
   105	        // misconfigured run isn't silent. POST_REVIEW_FIXES §1.1.
   106	        if let Err(e) = socket.set_keepalive(true) {
   107	            log::warn!("set TCP keepalive on data-plane socket: {}", e);
   108	        }
   109	
   110	        if let Some(size) = tcp_buffer_size {
   111	            // Buffer-size knobs are advisory; the kernel can clamp.
   112	            // Log failures so operators can spot a sysctl/rlimit
   113	            // mismatch instead of wondering why throughput sat
   114	            // below the configured target.
   115	            if let Err(e) = socket.set_send_buffer_size(size) {
   116	                log::warn!("set TCP send buffer to {} bytes: {}", size, e);
   117	            }
   118	            if let Err(e) = socket.set_recv_buffer_size(size) {
   119	                log::warn!("set TCP recv buffer to {} bytes: {}", size, e);
   120	            }
   121	        }
   122	
   123	        let std_stream: std::net::TcpStream = socket.into();
   124	        let mut stream =
   125	            TcpStream::from_std(std_stream).context("converting back to tokio stream")?;
   126	
   127	        stream
   128	            .write_all(token)
   129	            .await
   130	            .context("writing negotiation token")?;
   131	
   132	        Ok(Self::from_stream(stream, trace, chunk_bytes, payload_prefetch, pool).await)
   133	    }
   134	
   135	    pub async fn send_payloads(
   136	        &mut self,
   137	        source: Arc<dyn TransferSource>,
   138	        payloads: Vec<TransferPayload>,
   139	    ) -> Result<()> {
   140	        self.send_payloads_with_progress(source, payloads, None)
   141	            .await
   142	    }
   143	
   144	    pub async fn send_payloads_with_progress(
   145	        &mut self,
   146	        source: Arc<dyn TransferSource>,
   147	        payloads: Vec<TransferPayload>,
   148	        progress: Option<&super::progress::RemoteTransferProgress>,
   149	    ) -> Result<()> {
   150	        let mut stream = prepared_payload_stream(payloads, source.clone(), self.payload_prefetch);
   151	        while let Some(prepared) = stream.next().await {
   152	            match prepared? {
   153	                PreparedPayload::File(header) => {
   154	                    if let Err(err) = self.send_file(source.clone(), &header).await {
   155	                        return Err(err.wrap_err(format!("sending {}", header.relative_path)));
   156	                    }
   157	                    self.bytes_sent = self.bytes_sent.saturating_add(header.size);
   158	                    if let Some(progress) = progress {
   159	                        progress.report_file_complete(header.relative_path.clone(), header.size);
   160	                    }
   161	                }
   162	                PreparedPayload::TarShard { headers, data } => {
   163	                    let shard_bytes: u64 = headers.iter().map(|h| h.size).sum();
   164	                    if let Err(err) = self.send_prepared_tar_shard(headers.clone(), &data).await {
   165	                        return Err(err.wrap_err("sending tar shard"));
   166	                    }
   167	                    self.bytes_sent = self.bytes_sent.saturating_add(shard_bytes);
   168	                    if let Some(progress) = progress {
   169	                        for header in &headers {
   170	                            progress
   171	                                .report_file_complete(header.relative_path.clone(), header.size);
   172	                        }
   173	                    }
   174	                }
   175	                PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   176	                    bail!("DataPlaneSession::send_payloads does not handle resume payloads");
   177	                }
   178	            }
   179	        }
   180	
   181	        Ok(())
   182	    }
   183	
   184	    pub async fn finish(&mut self) -> Result<()> {
   185	        self.stream
   186	            .write_all(&[DATA_PLANE_RECORD_END])
   187	            .await
   188	            .context("writing transfer terminator")?;
   189	        self.stream
   190	            .flush()
   191	            .await
   192	            .context("flushing data plane stream")
   193	    }
   194	
   195	    pub fn bytes_sent(&self) -> u64 {
   196	        self.bytes_sent
   197	    }
   198	
   199	    pub async fn send_file(
   200	        &mut self,
   201	        source: Arc<dyn TransferSource>,
   202	        header: &FileHeader,
   203	    ) -> Result<()> {
   204	        let rel = &header.relative_path;
   205	        let mut file = source
   206	            .open_file(header)
   207	            .await
   208	            .with_context(|| format!("opening {}", rel))?;
   209	        self.send_file_from_reader(header, &mut file).await
   210	    }
   211	
   212	    /// Send a file payload whose bytes come from an arbitrary async
   213	    /// reader (not a local file). Used by `DataPlaneSink` for the
   214	    /// remote→remote relay case, where bytes arrive from an inbound
   215	    /// `DataPlaneSource` and need to be forwarded to the next hop.
   216	    ///
   217	    /// Same wire format and double-buffered loop as `send_file`.
   218	    pub async fn send_file_from_reader(
   219	        &mut self,
   220	        header: &FileHeader,
   221	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   222	    ) -> Result<()> {
   223	        let rel = &header.relative_path;
   224	        trace_client!(self, "sending file '{}' ({} bytes)", rel, header.size);
   225	
   226	        let path_bytes = rel.as_bytes();
   227	        if path_bytes.len() > u32::MAX as usize {
   228	            bail!("relative path too long for transfer: {}", rel);
   229	        }
   230	
   231	        self.stream
   232	            .write_all(&[DATA_PLANE_RECORD_FILE])
   233	            .await
   234	            .context("writing data-plane record tag")?;
   235	        self.stream
   236	            .write_all(&(path_bytes.len() as u32).to_be_bytes())
   237	            .await
   238	            .context("writing path length")?;
   239	        self.stream
   240	            .write_all(path_bytes)
   241	            .await
   242	            .context("writing path bytes")?;
   243	
   244	        self.stream
   245	            .write_all(&header.size.to_be_bytes())
   246	            .await
   247	            .context("writing file size")?;
   248	        // Wire-format extension (2026-05-01): include mtime + permissions
   249	        // inline so push and pull data plane records carry the same
   250	        // information. Lets the receive pipeline apply metadata via
   251	        // FsTransferSink without consulting an out-of-band manifest cache.
   252	        self.stream
   253	            .write_all(&header.mtime_seconds.to_be_bytes())
   254	            .await
   255	            .context("writing mtime")?;
   256	        self.stream
   257	            .write_all(&header.permissions.to_be_bytes())
   258	            .await
   259	            .context("writing permissions")?;
   260	
   261	        // Double-buffered I/O: overlaps source reads with network writes
   262	        self.send_file_double_buffered(reader, header, rel).await?;
   263	
   264	        trace_client!(self, "file '{}' sent ({} bytes)", rel, header.size);
   265	
   266	        Ok(())
   267	    }
   268	
   269	    /// Double-buffered file sending: overlaps disk reads with network writes.
   270	    /// Uses two buffers from the pool to enable concurrent I/O operations.
   271	    ///
   272	    /// Pattern: While buffer A is being written to network, buffer B is filled from disk.
   273	    /// This hides disk latency behind network latency for improved throughput.
   274	    async fn send_file_double_buffered(
   275	        &mut self,
   276	        file: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   277	        header: &FileHeader,
   278	        rel: &str,
   279	    ) -> Result<()> {
   280	        let mut remaining = header.size;
   281	        if remaining == 0 {
   282	            return Ok(());
   283	        }
   284	
   285	        // Acquire two buffers for double-buffering
   286	        let mut buf_a = self.pool.acquire().await;
   287	        let mut buf_b = self.pool.acquire().await;
   288	
   289	        // Initial read into buf_a
   290	        let mut bytes_a = file
   291	            .read(buf_a.as_mut_slice())
   292	            .await
   293	            .with_context(|| format!("reading {}", rel))?;
   294	
   295	        if bytes_a == 0 {
   296	            bail!(
   297	                "unexpected EOF while reading {} ({} bytes remaining)",
   298	                rel,
   299	                remaining
   300	            );
   301	        }
   302	        // Clamp to the declared size before subtracting. A source that
   303	        // returns more bytes than `header.size` — a file that grew after
   304	        // the manifest was computed, or a lying `TransferSource` — would
   305	        // otherwise underflow `remaining` (debug: panic; release: wrap to
   306	        // u64::MAX → runaway loop) and push undeclared bytes onto the
   307	        // framed stream. We send exactly `header.size` and ignore excess.
   308	        bytes_a = (bytes_a as u64).min(remaining) as usize;
   309	        remaining -= bytes_a as u64;
   310	
   311	        // Main loop: write buf_a while reading into buf_b
   312	        while remaining > 0 {
   313	            // Overlap: write from buf_a, read into buf_b concurrently
   314	            let (write_result, read_result) = tokio::join!(
   315	                self.stream.write_all(&buf_a.as_slice()[..bytes_a]),
   316	                file.read(buf_b.as_mut_slice())
   317	            );
   318	
   319	            write_result.with_context(|| format!("sending {}", rel))?;
   320	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
   321	
   322	            let bytes_b = read_result.with_context(|| format!("reading {}", rel))?;
   323	
   324	            if bytes_b == 0 && remaining > 0 {
   325	                bail!(
   326	                    "unexpected EOF while reading {} ({} bytes remaining)",
   327	                    rel,
   328	                    remaining
   329	                );
   330	            }
   331	            // Same clamp as the initial read: never subtract more than
   332	            // `remaining`, so an over-returning reader can neither
   333	            // underflow the counter nor send undeclared bytes.
   334	            let bytes_b = (bytes_b as u64).min(remaining) as usize;
   335	            remaining -= bytes_b as u64;
   336	
   337	            // Swap roles: buf_b becomes the write buffer, buf_a becomes read buffer
   338	            std::mem::swap(&mut buf_a, &mut buf_b);
   339	            bytes_a = bytes_b;
   340	        }
   341	
   342	        // Final write: send the last chunk in buf_a
   343	        if bytes_a > 0 {
   344	            self.stream
   345	                .write_all(&buf_a.as_slice()[..bytes_a])
   346	                .await
   347	                .with_context(|| format!("sending {}", rel))?;
   348	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
   349	        }
   350	
   351	        // Buffers return to pool automatically on drop
   352	        Ok(())
   353	    }
   354	
   355	    pub async fn send_prepared_tar_shard(
   356	        &mut self,
   357	        headers: Vec<FileHeader>,
   358	        data: &[u8],
   359	    ) -> Result<()> {
   360	        let shard_len = headers.len();
   361	        let preview = headers
   362	            .first()
   363	            .map(|h| h.relative_path.as_str())
   364	            .unwrap_or("<empty>");
   365	        trace_client!(
   366	            self,
   367	            "sending tar shard with {} file(s), {} bytes (first='{}')",
   368	            shard_len,
   369	            data.len(),
   370	            preview
   371	        );
   372	        self.stream
   373	            .write_all(&[DATA_PLANE_RECORD_TAR_SHARD])
   374	            .await
   375	            .context("writing tar shard record tag")?;
   376	        self.stream
   377	            .write_all(&(headers.len() as u32).to_be_bytes())
   378	            .await
   379	            .context("writing tar shard count")?;
   380	
   381	        for header in headers {
   382	            let rel_bytes = header.relative_path.as_bytes();
   383	            if rel_bytes.len() > u32::MAX as usize {
   384	                bail!(
   385	                    "relative path too long for transfer: {}",
   386	                    header.relative_path
   387	                );
   388	            }
   389	            self.stream
   390	                .write_all(&(rel_bytes.len() as u32).to_be_bytes())
   391	                .await
   392	                .context("writing shard path length")?;
   393	            self.stream
   394	                .write_all(rel_bytes)
   395	                .await
   396	                .context("writing shard path bytes")?;
   397	            self.stream
   398	                .write_all(&header.size.to_be_bytes())
   399	                .await
   400	                .context("writing shard size")?;
   401	            self.stream
   402	                .write_all(&header.mtime_seconds.to_be_bytes())
   403	                .await
   404	                .context("writing shard mtime")?;
   405	            self.stream
   406	                .write_all(&header.permissions.to_be_bytes())
   407	                .await
   408	                .context("writing shard permissions")?;
   409	        }
   410	
   411	        self.stream
   412	            .write_all(&(data.len() as u64).to_be_bytes())
   413	            .await
   414	            .context("writing tar shard length")?;
   415	        for chunk in data.chunks(self.chunk_bytes.max(1)) {
   416	            self.stream
   417	                .write_all(chunk)
   418	                .await
   419	                .context("writing tar shard payload")?;
   420	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(chunk.len() as u64);
   421	        }
   422	        trace_client!(
   423	            self,
   424	            "tar shard payload sent ({} file(s), {} bytes)",
   425	            shard_len,
   426	            data.len()
   427	        );
   428	
   429	        Ok(())
   430	    }
   431	
   432	    /// Send a single block for block-level resume.
   433	    /// Format: [type:1][path_len:4][path][offset:8][block_len:4][content]
   434	    pub async fn send_block(
   435	        &mut self,
   436	        relative_path: &str,
   437	        offset: u64,
   438	        content: &[u8],
   439	    ) -> Result<()> {
   440	        let path_bytes = relative_path.as_bytes();
   441	        if path_bytes.len() > u32::MAX as usize {
   442	            bail!("relative path too long for transfer: {}", relative_path);
   443	        }
   444	
   445	        trace_client!(
   446	            self,
   447	            "sending block for '{}' at offset {} ({} bytes)",
   448	            relative_path,
   449	            offset,
   450	            content.len()
   451	        );
   452	
   453	        self.stream
   454	            .write_all(&[DATA_PLANE_RECORD_BLOCK])
   455	            .await
   456	            .context("writing block record tag")?;
   457	        self.stream
   458	            .write_all(&(path_bytes.len() as u32).to_be_bytes())
   459	            .await
   460	            .context("writing path length")?;
   461	        self.stream
   462	            .write_all(path_bytes)
   463	            .await
   464	            .context("writing path bytes")?;
   465	        self.stream
   466	            .write_all(&offset.to_be_bytes())
   467	            .await
   468	            .context("writing block offset")?;
   469	        self.stream
   470	            .write_all(&(content.len() as u32).to_be_bytes())
   471	            .await
   472	            .context("writing block length")?;
   473	        self.stream
   474	            .write_all(content)
   475	            .await
   476	            .context("writing block content")?;
   477	        crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(content.len() as u64);
   478	
   479	        self.bytes_sent += content.len() as u64;
   480	        Ok(())
   481	    }
   482	
   483	    /// Signal that block-level transfer for a file is complete.
   484	    /// Format: [type:1][path_len:4][path][total_size:8][mtime:8][perms:4]
   485	    ///
   486	    /// Carries mtime + perms inline so the receiver can stamp the
   487	    /// destination metadata even when zero blocks transferred (the
   488	    /// "mtime touched, content identical" case for mirror).
   489	    pub async fn send_block_complete(
   490	        &mut self,
   491	        relative_path: &str,
   492	        total_size: u64,
   493	        mtime_seconds: i64,
   494	        permissions: u32,
   495	    ) -> Result<()> {
   496	        let path_bytes = relative_path.as_bytes();
   497	        if path_bytes.len() > u32::MAX as usize {
   498	            bail!("relative path too long for transfer: {}", relative_path);
   499	        }
   500	
   501	        trace_client!(
   502	            self,
   503	            "sending block complete for '{}' ({} bytes total)",
   504	            relative_path,
   505	            total_size
   506	        );
   507	
   508	        self.stream
   509	            .write_all(&[DATA_PLANE_RECORD_BLOCK_COMPLETE])
   510	            .await
   511	            .context("writing block complete record tag")?;
   512	        self.stream
   513	            .write_all(&(path_bytes.len() as u32).to_be_bytes())
   514	            .await
   515	            .context("writing path length")?;
   516	        self.stream
   517	            .write_all(path_bytes)
   518	            .await
   519	            .context("writing path bytes")?;
   520	        self.stream

exec
/usr/bin/bash -lc "git show 515fb76:crates/blit-core/src/remote/transfer/pipeline.rs | nl -ba | sed -n '1,240p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
     1	//! Unified transfer pipeline: source → prepare → sink(s).
     2	//!
     3	//! All transfer paths (local→local, local→remote push, remote→local pull,
     4	//! remote→remote) route through the same executor. Payloads can be supplied
     5	//! either upfront ([`execute_sink_pipeline`]) or incrementally as they are
     6	//! produced ([`execute_sink_pipeline_streaming`]). The one-shot form is a
     7	//! thin wrapper that sends every payload on a channel and delegates.
     8	
     9	use std::sync::Arc;
    10	
    11	use eyre::{Context, Result};
    12	use tokio::sync::mpsc;
    13	
    14	use super::payload::{PreparedPayload, TransferPayload};
    15	use super::progress::RemoteTransferProgress;
    16	use super::sink::{SinkOutcome, TransferSink};
    17	use super::source::TransferSource;
    18	
    19	/// Execute a transfer pipeline with all payloads known upfront.
    20	///
    21	/// This is a convenience wrapper around [`execute_sink_pipeline_streaming`]
    22	/// that spawns a task to send every payload into the channel and then drops
    23	/// the sender, signalling end-of-stream.
    24	pub async fn execute_sink_pipeline(
    25	    source: Arc<dyn TransferSource>,
    26	    sinks: Vec<Arc<dyn TransferSink>>,
    27	    payloads: Vec<TransferPayload>,
    28	    prefetch: usize,
    29	    progress: Option<&RemoteTransferProgress>,
    30	) -> Result<SinkOutcome> {
    31	    if sinks.is_empty() {
    32	        return Ok(SinkOutcome::default());
    33	    }
    34	    if payloads.is_empty() {
    35	        for sink in &sinks {
    36	            sink.finish().await?;
    37	        }
    38	        return Ok(SinkOutcome::default());
    39	    }
    40	
    41	    let capacity = prefetch.max(1);
    42	    let (tx, rx) = mpsc::channel::<TransferPayload>(capacity);
    43	
    44	    // Feed payloads in a background task so the pipeline can start writing
    45	    // before the whole vec is queued (the channel provides back-pressure).
    46	    let feeder = tokio::spawn(async move {
    47	        for payload in payloads {
    48	            if tx.send(payload).await.is_err() {
    49	                break;
    50	            }
    51	        }
    52	        // Dropping tx closes the channel and signals end-of-stream.
    53	    });
    54	
    55	    let result = execute_sink_pipeline_streaming(source, sinks, rx, prefetch, progress).await;
    56	    let _ = feeder.await;
    57	    result
    58	}
    59	
    60	/// Execute a transfer pipeline with payloads arriving on a channel.
    61	///
    62	/// Distributes payloads round-robin across `sinks` as they arrive. Each sink
    63	/// runs as a separate tokio task: it reads payloads from its dedicated queue,
    64	/// prepares them via `source.prepare_payload()`, writes them via
    65	/// `sink.write_payload()`, and finally calls `sink.finish()`. Errors from any
    66	/// worker propagate up.
    67	///
    68	/// `prefetch` controls the per-sink channel capacity — effectively the
    69	/// preparation-in-flight limit per sink.
    70	pub async fn execute_sink_pipeline_streaming(
    71	    source: Arc<dyn TransferSource>,
    72	    sinks: Vec<Arc<dyn TransferSink>>,
    73	    mut payload_rx: mpsc::Receiver<TransferPayload>,
    74	    prefetch: usize,
    75	    progress: Option<&RemoteTransferProgress>,
    76	) -> Result<SinkOutcome> {
    77	    if sinks.is_empty() {
    78	        // Drain incoming channel so the producer isn't left dangling.
    79	        while payload_rx.recv().await.is_some() {}
    80	        return Ok(SinkOutcome::default());
    81	    }
    82	
    83	    let sink_count = sinks.len();
    84	    let per_sink_capacity = prefetch.max(1);
    85	    let total = Arc::new(std::sync::Mutex::new(SinkOutcome::default()));
    86	
    87	    // Per-sink payload channels; dispatcher forwards round-robin to these.
    88	    let mut sink_senders: Vec<mpsc::Sender<TransferPayload>> = Vec::with_capacity(sink_count);
    89	    let mut sink_handles: Vec<tokio::task::JoinHandle<Result<()>>> = Vec::with_capacity(sink_count);
    90	
    91	    for sink in sinks {
    92	        let (tx, mut rx) = mpsc::channel::<TransferPayload>(per_sink_capacity);
    93	        sink_senders.push(tx);
    94	        let source_clone = source.clone();
    95	        let progress_clone = progress.cloned();
    96	        let total_clone = total.clone();
    97	        sink_handles.push(tokio::spawn(async move {
    98	            while let Some(payload) = rx.recv().await {
    99	                let prepared = source_clone
   100	                    .prepare_payload(payload)
   101	                    .await
   102	                    .context("preparing payload")?;
   103	                let files: Vec<(String, u64)> = match &prepared {
   104	                    PreparedPayload::File(h) => vec![(h.relative_path.clone(), h.size)],
   105	                    PreparedPayload::TarShard { headers, .. } => headers
   106	                        .iter()
   107	                        .map(|h| (h.relative_path.clone(), h.size))
   108	                        .collect(),
   109	                    // Resume-block payloads patch existing files; no
   110	                    // file-completion event from one-block-at-a-time.
   111	                    PreparedPayload::FileBlock { .. }
   112	                    | PreparedPayload::FileBlockComplete { .. } => Vec::new(),
   113	                };
   114	                let outcome = sink
   115	                    .write_payload(prepared)
   116	                    .await
   117	                    .context("writing payload")?;
   118	                if let Some(p) = &progress_clone {
   119	                    for (name, size) in &files {
   120	                        p.report_file_complete(name.clone(), *size);
   121	                    }
   122	                }
   123	                let mut t = total_clone.lock().unwrap();
   124	                t.merge(&outcome);
   125	            }
   126	            sink.finish().await?;
   127	            Ok::<(), eyre::Report>(())
   128	        }));
   129	    }
   130	
   131	    // Dispatcher: pull from the incoming channel, round-robin to sinks.
   132	    // Uses async send (which applies backpressure) — if one sink is slower,
   133	    // the dispatcher naturally blocks on that sink until it drains.
   134	    let dispatcher = tokio::spawn(async move {
   135	        let mut next = 0usize;
   136	        while let Some(payload) = payload_rx.recv().await {
   137	            let idx = next % sink_count;
   138	            next = next.wrapping_add(1);
   139	            if sink_senders[idx].send(payload).await.is_err() {
   140	                // Sink worker dropped its receiver — treat as shutdown.
   141	                return;
   142	            }
   143	        }
   144	        // Drop senders so sink workers see end-of-stream and finish().
   145	        drop(sink_senders);
   146	    });
   147	
   148	    // Wait for all sinks to finish and aggregate errors.
   149	    let mut first_err: Option<eyre::Report> = None;
   150	    for h in sink_handles {
   151	        match h.await {
   152	            Ok(Ok(())) => {}
   153	            Ok(Err(e)) if first_err.is_none() => first_err = Some(e),
   154	            Ok(Err(_)) => {}
   155	            Err(join) if first_err.is_none() => {
   156	                first_err = Some(eyre::eyre!("sink worker panicked: {}", join));
   157	            }
   158	            Err(_) => {}
   159	        }
   160	    }
   161	    let _ = dispatcher.await;
   162	
   163	    if let Some(err) = first_err {
   164	        return Err(err);
   165	    }
   166	
   167	    let result = total.lock().unwrap().clone();
   168	    Ok(result)
   169	}
   170	
   171	// =====================================================================
   172	// Receive pipeline — symmetric counterpart of execute_sink_pipeline.
   173	// =====================================================================
   174	
   175	use crate::generated::FileHeader;
   176	use eyre::bail;
   177	use tokio::io::{AsyncRead, AsyncReadExt};
   178	
   179	use super::data_plane::{
   180	    DATA_PLANE_RECORD_BLOCK, DATA_PLANE_RECORD_BLOCK_COMPLETE, DATA_PLANE_RECORD_END,
   181	    DATA_PLANE_RECORD_FILE, DATA_PLANE_RECORD_TAR_SHARD,
   182	};
   183	
   184	/// Drive a `TransferSink` from a TCP wire stream.
   185	///
   186	/// This is the symmetric counterpart to [`execute_sink_pipeline_streaming`]:
   187	/// where the outbound executor takes a [`TransferSource`] and dispatches
   188	/// payloads round-robin across N sinks, this one consumes a single
   189	/// inbound wire (parsing record headers and producing
   190	/// [`PreparedPayload::FileStream`] / [`PreparedPayload::TarShard`] /
   191	/// [`PreparedPayload::FileBlock`] events) and feeds them to a single sink
   192	/// sequentially. Multi-stream parallelism comes from spawning N invocations,
   193	/// one per inbound TCP connection.
   194	///
   195	/// Both directions converge on `TransferSink::write_payload`: file data
   196	/// hits disk through `FsTransferSink::write_payload(FileStream { … })`,
   197	/// which uses the same `receive_stream_double_buffered` helper as the
   198	/// daemon's push receiver and the client's pull receiver — one path,
   199	/// one optimization surface.
   200	pub async fn execute_receive_pipeline<R: AsyncRead + Unpin + Send>(
   201	    socket: &mut R,
   202	    sink: Arc<dyn TransferSink>,
   203	    progress: Option<&RemoteTransferProgress>,
   204	) -> Result<SinkOutcome> {
   205	    let mut total = SinkOutcome::default();
   206	
   207	    loop {
   208	        let mut tag = [0u8; 1];
   209	        socket
   210	            .read_exact(&mut tag)
   211	            .await
   212	            .context("reading data-plane record tag")?;
   213	
   214	        match tag[0] {
   215	            DATA_PLANE_RECORD_END => break,
   216	            DATA_PLANE_RECORD_FILE => {
   217	                let mut header = read_file_header(socket).await?;
   218	                let file_size = read_u64(socket).await?;
   219	                let mtime = read_i64(socket).await?;
   220	                let perms = read_u32(socket).await?;
   221	                header.size = file_size;
   222	                header.mtime_seconds = mtime;
   223	                header.permissions = perms;
   224	                // Use AsyncReadExt::take to give the sink exactly
   225	                // file_size bytes of the wire. tokio's Take is the
   226	                // canonical way to limit a borrowed AsyncRead.
   227	                use tokio::io::AsyncReadExt;
   228	                let mut reader = (&mut *socket).take(file_size);
   229	                let outcome = sink
   230	                    .write_file_stream(&header, &mut reader)
   231	                    .await
   232	                    .with_context(|| format!("receiving {}", header.relative_path))?;
   233	                if let Some(p) = progress {
   234	                    p.report_payload(0, outcome.bytes_written);
   235	                    p.report_file_complete(header.relative_path.clone(), outcome.bytes_written);
   236	                }
   237	                total.merge(&outcome);
   238	            }
   239	            DATA_PLANE_RECORD_TAR_SHARD => {
   240	                let (headers, data) = read_tar_shard(socket).await?;

codex
The old version confirms `StallGuardWriter<TcpStream>` was the stream field before the slice. I’m narrowing now on exact `write_all` call sites and the worker/forwarder lifecycle, since those are the only places a real regression is likely to hide.
exec
/usr/bin/bash -lc "rg -n \"self\\.stream\\.(write_all|flush)|write_all\\(|flush\\(\" crates/blit-core/src/remote/transfer/data_plane.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:
38:/// (15+ minutes). All existing `self.stream.write_all/.flush` call
132:            .write_all(token)
217:            .write_all(&[DATA_PLANE_RECORD_END])
221:            .flush()
263:            .write_all(&[DATA_PLANE_RECORD_FILE])
267:            .write_all(&(path_bytes.len() as u32).to_be_bytes())
271:            .write_all(path_bytes)
276:            .write_all(&header.size.to_be_bytes())
284:            .write_all(&header.mtime_seconds.to_be_bytes())
288:            .write_all(&header.permissions.to_be_bytes())
355:                self.stream.write_all(&buf_a.as_slice()[..bytes_a]),
398:                .write_all(&buf_a.as_slice()[..bytes_a])
432:            .write_all(&[DATA_PLANE_RECORD_TAR_SHARD])
436:            .write_all(&(headers.len() as u32).to_be_bytes())
449:                .write_all(&(rel_bytes.len() as u32).to_be_bytes())
453:                .write_all(rel_bytes)
457:                .write_all(&header.size.to_be_bytes())
461:                .write_all(&header.mtime_seconds.to_be_bytes())
465:                .write_all(&header.permissions.to_be_bytes())
471:            .write_all(&(data.len() as u64).to_be_bytes())
476:                .write_all(chunk)
514:            .write_all(&[DATA_PLANE_RECORD_BLOCK])
518:            .write_all(&(path_bytes.len() as u32).to_be_bytes())
522:            .write_all(path_bytes)
526:            .write_all(&offset.to_be_bytes())
530:            .write_all(&(content.len() as u32).to_be_bytes())
534:            .write_all(content)
569:            .write_all(&[DATA_PLANE_RECORD_BLOCK_COMPLETE])
573:            .write_all(&(path_bytes.len() as u32).to_be_bytes())
577:            .write_all(path_bytes)
581:            .write_all(&total_size.to_be_bytes())
585:            .write_all(&mtime_seconds.to_be_bytes())
589:            .write_all(&permissions.to_be_bytes())
658:            dst.write_all(&buf_a[..bytes_a]),
684:        dst.write_all(&buf_a[..bytes_a])

exec
/usr/bin/bash -lc "sed -n '130,230p' crates/blit-core/src/remote/transfer/data_plane.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:

        stream
            .write_all(token)
            .await
            .context("writing negotiation token")?;

        Ok(Self::from_stream(stream, trace, chunk_bytes, payload_prefetch, pool).await)
    }
}

impl<P: Probe> DataPlaneSession<P> {
    /// Create a session carrying an arbitrary [`Probe`]. The generic
    /// primitive behind [`from_stream`](DataPlaneSession::from_stream);
    /// the adaptive controller calls this with a `LiveProbe` to enable
    /// per-stream telemetry.
    pub async fn from_stream_with_probe(
        stream: TcpStream,
        trace: bool,
        chunk_bytes: usize,
        payload_prefetch: usize,
        pool: Arc<BufferPool>,
        probe: P,
    ) -> Self {
        let payload_prefetch = payload_prefetch.max(1);
        let chunk_bytes = chunk_bytes.max(64 * 1024);
        Self {
            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
            pool,
            trace,
            chunk_bytes,
            payload_prefetch,
            bytes_sent: 0,
            probe,
        }
    }

    pub async fn send_payloads(
        &mut self,
        source: Arc<dyn TransferSource>,
        payloads: Vec<TransferPayload>,
    ) -> Result<()> {
        self.send_payloads_with_progress(source, payloads, None)
            .await
    }

    pub async fn send_payloads_with_progress(
        &mut self,
        source: Arc<dyn TransferSource>,
        payloads: Vec<TransferPayload>,
        progress: Option<&super::progress::RemoteTransferProgress>,
    ) -> Result<()> {
        let mut stream = prepared_payload_stream(payloads, source.clone(), self.payload_prefetch);
        while let Some(prepared) = stream.next().await {
            match prepared? {
                PreparedPayload::File(header) => {
                    if let Err(err) = self.send_file(source.clone(), &header).await {
                        return Err(err.wrap_err(format!("sending {}", header.relative_path)));
                    }
                    self.bytes_sent = self.bytes_sent.saturating_add(header.size);
                    if let Some(progress) = progress {
                        progress.report_file_complete(header.relative_path.clone(), header.size);
                    }
                }
                PreparedPayload::TarShard { headers, data } => {
                    let shard_bytes: u64 = headers.iter().map(|h| h.size).sum();
                    if let Err(err) = self.send_prepared_tar_shard(headers.clone(), &data).await {
                        return Err(err.wrap_err("sending tar shard"));
                    }
                    self.bytes_sent = self.bytes_sent.saturating_add(shard_bytes);
                    if let Some(progress) = progress {
                        for header in &headers {
                            progress
                                .report_file_complete(header.relative_path.clone(), header.size);
                        }
                    }
                }
                PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
                    bail!("DataPlaneSession::send_payloads does not handle resume payloads");
                }
            }
        }

        Ok(())
    }

    pub async fn finish(&mut self) -> Result<()> {
        self.stream
            .write_all(&[DATA_PLANE_RECORD_END])
            .await
            .context("writing transfer terminator")?;
        self.stream
            .flush()
            .await
            .context("flushing data plane stream")
    }

    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent
    }

    pub async fn send_file(

exec
/usr/bin/bash -lc "sed -n '430,620p' crates/blit-core/src/remote/transfer/data_plane.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:
        );
        self.stream
            .write_all(&[DATA_PLANE_RECORD_TAR_SHARD])
            .await
            .context("writing tar shard record tag")?;
        self.stream
            .write_all(&(headers.len() as u32).to_be_bytes())
            .await
            .context("writing tar shard count")?;

        for header in headers {
            let rel_bytes = header.relative_path.as_bytes();
            if rel_bytes.len() > u32::MAX as usize {
                bail!(
                    "relative path too long for transfer: {}",
                    header.relative_path
                );
            }
            self.stream
                .write_all(&(rel_bytes.len() as u32).to_be_bytes())
                .await
                .context("writing shard path length")?;
            self.stream
                .write_all(rel_bytes)
                .await
                .context("writing shard path bytes")?;
            self.stream
                .write_all(&header.size.to_be_bytes())
                .await
                .context("writing shard size")?;
            self.stream
                .write_all(&header.mtime_seconds.to_be_bytes())
                .await
                .context("writing shard mtime")?;
            self.stream
                .write_all(&header.permissions.to_be_bytes())
                .await
                .context("writing shard permissions")?;
        }

        self.stream
            .write_all(&(data.len() as u64).to_be_bytes())
            .await
            .context("writing tar shard length")?;
        for chunk in data.chunks(self.chunk_bytes.max(1)) {
            self.stream
                .write_all(chunk)
                .await
                .context("writing tar shard payload")?;
            self.probe.record_bytes(chunk.len() as u64);
            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(chunk.len() as u64);
        }
        trace_client!(
            self,
            "tar shard payload sent ({} file(s), {} bytes)",
            shard_len,
            data.len()
        );

        Ok(())
    }

    /// Send a single block for block-level resume.
    /// Format: [type:1][path_len:4][path][offset:8][block_len:4][content]
    pub async fn send_block(
        &mut self,
        relative_path: &str,
        offset: u64,
        content: &[u8],
    ) -> Result<()> {
        let path_bytes = relative_path.as_bytes();
        if path_bytes.len() > u32::MAX as usize {
            bail!("relative path too long for transfer: {}", relative_path);
        }

        trace_client!(
            self,
            "sending block for '{}' at offset {} ({} bytes)",
            relative_path,
            offset,
            content.len()
        );

        self.stream
            .write_all(&[DATA_PLANE_RECORD_BLOCK])
            .await
            .context("writing block record tag")?;
        self.stream
            .write_all(&(path_bytes.len() as u32).to_be_bytes())
            .await
            .context("writing path length")?;
        self.stream
            .write_all(path_bytes)
            .await
            .context("writing path bytes")?;
        self.stream
            .write_all(&offset.to_be_bytes())
            .await
            .context("writing block offset")?;
        self.stream
            .write_all(&(content.len() as u32).to_be_bytes())
            .await
            .context("writing block length")?;
        self.stream
            .write_all(content)
            .await
            .context("writing block content")?;
        crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(content.len() as u64);

        self.bytes_sent += content.len() as u64;
        Ok(())
    }

    /// Signal that block-level transfer for a file is complete.
    /// Format: [type:1][path_len:4][path][total_size:8][mtime:8][perms:4]
    ///
    /// Carries mtime + perms inline so the receiver can stamp the
    /// destination metadata even when zero blocks transferred (the
    /// "mtime touched, content identical" case for mirror).
    pub async fn send_block_complete(
        &mut self,
        relative_path: &str,
        total_size: u64,
        mtime_seconds: i64,
        permissions: u32,
    ) -> Result<()> {
        let path_bytes = relative_path.as_bytes();
        if path_bytes.len() > u32::MAX as usize {
            bail!("relative path too long for transfer: {}", relative_path);
        }

        trace_client!(
            self,
            "sending block complete for '{}' ({} bytes total)",
            relative_path,
            total_size
        );

        self.stream
            .write_all(&[DATA_PLANE_RECORD_BLOCK_COMPLETE])
            .await
            .context("writing block complete record tag")?;
        self.stream
            .write_all(&(path_bytes.len() as u32).to_be_bytes())
            .await
            .context("writing path length")?;
        self.stream
            .write_all(path_bytes)
            .await
            .context("writing path bytes")?;
        self.stream
            .write_all(&total_size.to_be_bytes())
            .await
            .context("writing total size")?;
        self.stream
            .write_all(&mtime_seconds.to_be_bytes())
            .await
            .context("writing mtime")?;
        self.stream
            .write_all(&permissions.to_be_bytes())
            .await
            .context("writing permissions")?;

        Ok(())
    }
}

/// Default buffer size for the symmetric receive path. Matches what the
/// send side's buffer pool uses for chunk_bytes; large enough that the
/// per-syscall overhead doesn't dominate at 10 GbE, and that ZFS-style
/// transactional filesystems can amortize per-write costs.
///
/// Empirically, 8 KiB caps push throughput at ~1 Gbps on EPYC/ZFS even
/// when the network can do 9.4 Gbps and the disk can do 14.76 Gbps.
/// 1 MiB lets the receiver keep up with the sender's double-buffered
/// pipeline.
pub const RECEIVE_CHUNK_SIZE: usize = 1024 * 1024;

/// Stream `expected` bytes from an async source into an async sink with
/// double-buffered I/O — while one buffer drains to disk, the other is
/// being filled from the wire. Symmetric counterpart of
/// `DataPlaneSession::send_file_double_buffered`.
///
/// Both the daemon's push receiver (writing to disk from a TCP socket)
/// and the client's pull receiver (same shape, opposite direction) call
/// this so the receive side has the same throughput characteristics as
/// the send side. Replacing this with `tokio::io::copy` (8 KiB internal
/// buffer) caps real-world transfers at ~1 Gbps regardless of network
/// or disk speed.
///
/// Returns the number of bytes copied. Errors on early EOF.

exec
/usr/bin/bash -lc "sed -n '250,430p' crates/blit-core/src/remote/transfer/data_plane.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:
        &mut self,
        header: &FileHeader,
        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
    ) -> Result<()> {
        let rel = &header.relative_path;
        trace_client!(self, "sending file '{}' ({} bytes)", rel, header.size);

        let path_bytes = rel.as_bytes();
        if path_bytes.len() > u32::MAX as usize {
            bail!("relative path too long for transfer: {}", rel);
        }

        self.stream
            .write_all(&[DATA_PLANE_RECORD_FILE])
            .await
            .context("writing data-plane record tag")?;
        self.stream
            .write_all(&(path_bytes.len() as u32).to_be_bytes())
            .await
            .context("writing path length")?;
        self.stream
            .write_all(path_bytes)
            .await
            .context("writing path bytes")?;

        self.stream
            .write_all(&header.size.to_be_bytes())
            .await
            .context("writing file size")?;
        // Wire-format extension (2026-05-01): include mtime + permissions
        // inline so push and pull data plane records carry the same
        // information. Lets the receive pipeline apply metadata via
        // FsTransferSink without consulting an out-of-band manifest cache.
        self.stream
            .write_all(&header.mtime_seconds.to_be_bytes())
            .await
            .context("writing mtime")?;
        self.stream
            .write_all(&header.permissions.to_be_bytes())
            .await
            .context("writing permissions")?;

        // Double-buffered I/O: overlaps source reads with network writes
        self.send_file_double_buffered(reader, header, rel).await?;

        trace_client!(self, "file '{}' sent ({} bytes)", rel, header.size);

        Ok(())
    }

    /// Double-buffered file sending: overlaps disk reads with network writes.
    /// Uses two buffers from the pool to enable concurrent I/O operations.
    ///
    /// Pattern: While buffer A is being written to network, buffer B is filled from disk.
    /// This hides disk latency behind network latency for improved throughput.
    async fn send_file_double_buffered(
        &mut self,
        file: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
        header: &FileHeader,
        rel: &str,
    ) -> Result<()> {
        let mut remaining = header.size;
        if remaining == 0 {
            return Ok(());
        }

        // Acquire two buffers for double-buffering
        let mut buf_a = self.pool.acquire().await;
        let mut buf_b = self.pool.acquire().await;

        // Initial read into buf_a
        let mut bytes_a = file
            .read(buf_a.as_mut_slice())
            .await
            .with_context(|| format!("reading {}", rel))?;

        if bytes_a == 0 {
            bail!(
                "unexpected EOF while reading {} ({} bytes remaining)",
                rel,
                remaining
            );
        }
        // Clamp to the declared size before subtracting. A source that
        // returns more bytes than `header.size` — a file that grew after
        // the manifest was computed, or a lying `TransferSource` — would
        // otherwise underflow `remaining` (debug: panic; release: wrap to
        // u64::MAX → runaway loop) and push undeclared bytes onto the
        // framed stream. We send exactly `header.size` and ignore excess.
        bytes_a = (bytes_a as u64).min(remaining) as usize;
        remaining -= bytes_a as u64;

        // Main loop: write buf_a while reading into buf_b
        while remaining > 0 {
            // Per-stream telemetry: time the overlapped write+read step
            // as a backpressure proxy. Gated on the compile-time
            // `P::ACTIVE` constant so `DataPlaneSession<NoProbe>` reads
            // no clock and folds this to nothing.
            let step_start = if P::ACTIVE {
                Some(std::time::Instant::now())
            } else {
                None
            };
            // Overlap: write from buf_a, read into buf_b concurrently
            let (write_result, read_result) = tokio::join!(
                self.stream.write_all(&buf_a.as_slice()[..bytes_a]),
                file.read(buf_b.as_mut_slice())
            );

            write_result.with_context(|| format!("sending {}", rel))?;
            if P::ACTIVE {
                if let Some(t) = step_start {
                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
                }
            }
            self.probe.record_bytes(bytes_a as u64);
            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);

            let bytes_b = read_result.with_context(|| format!("reading {}", rel))?;

            if bytes_b == 0 && remaining > 0 {
                bail!(
                    "unexpected EOF while reading {} ({} bytes remaining)",
                    rel,
                    remaining
                );
            }
            // Same clamp as the initial read: never subtract more than
            // `remaining`, so an over-returning reader can neither
            // underflow the counter nor send undeclared bytes.
            let bytes_b = (bytes_b as u64).min(remaining) as usize;
            remaining -= bytes_b as u64;

            // Swap roles: buf_b becomes the write buffer, buf_a becomes read buffer
            std::mem::swap(&mut buf_a, &mut buf_b);
            bytes_a = bytes_b;
        }

        // Final write: send the last chunk in buf_a. This is a pure
        // write (no overlapped read), so the timing is cleanly
        // attributable to socket-write backpressure.
        if bytes_a > 0 {
            let tail_start = if P::ACTIVE {
                Some(std::time::Instant::now())
            } else {
                None
            };
            self.stream
                .write_all(&buf_a.as_slice()[..bytes_a])
                .await
                .with_context(|| format!("sending {}", rel))?;
            if P::ACTIVE {
                if let Some(t) = tail_start {
                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
                }
            }
            self.probe.record_bytes(bytes_a as u64);
            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
        }

        // Buffers return to pool automatically on drop
        Ok(())
    }

    pub async fn send_prepared_tar_shard(
        &mut self,
        headers: Vec<FileHeader>,
        data: &[u8],
    ) -> Result<()> {
        let shard_len = headers.len();
        let preview = headers
            .first()
            .map(|h| h.relative_path.as_str())
            .unwrap_or("<empty>");
        trace_client!(
            self,
            "sending tar shard with {} file(s), {} bytes (first='{}')",
            shard_len,
            data.len(),
            preview
        );

exec
/usr/bin/bash -lc "sed -n '70,220p' crates/blit-core/src/remote/transfer/pipeline.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:
/// drains, at which point it calls `sink.finish()`. Errors from any
/// worker propagate up (first error wins).
///
/// `prefetch` controls the per-sink preparation-in-flight limit; the
/// shared queue is bounded at `prefetch * sinks.len()` so total
/// in-flight capacity matches the previous per-sink-channel design
/// (back-pressure preserved).
pub async fn execute_sink_pipeline_streaming(
    source: Arc<dyn TransferSource>,
    sinks: Vec<Arc<dyn TransferSink>>,
    mut payload_rx: mpsc::Receiver<TransferPayload>,
    prefetch: usize,
    progress: Option<&RemoteTransferProgress>,
) -> Result<SinkOutcome> {
    if sinks.is_empty() {
        // Drain incoming channel so the producer isn't left dangling.
        while payload_rx.recv().await.is_some() {}
        return Ok(SinkOutcome::default());
    }

    let sink_count = sinks.len();
    let capacity = prefetch.max(1) * sink_count;
    let total = Arc::new(std::sync::Mutex::new(SinkOutcome::default()));

    // Single shared work queue. Each worker owns exactly one sink but
    // pulls payloads from the common queue, so work is stolen by
    // whichever sink is free rather than pre-assigned round-robin.
    let (work_tx, work_rx) = flume::bounded::<TransferPayload>(capacity);
    let mut sink_handles: Vec<tokio::task::JoinHandle<Result<()>>> = Vec::with_capacity(sink_count);

    // Cancellation flag set by the first worker that errors. Without it,
    // one sink failing only drops that worker's `work_rx` clone; as long
    // as any other worker is alive `send_async` keeps succeeding, so the
    // forwarder would keep draining `payload_rx` and queueing payloads
    // that can never complete — delaying first-error-wins propagation
    // (Codex review, PR2). With it, the forwarder stops at the next
    // payload boundary and closes the queue so the survivors drain and
    // finish promptly.
    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));

    for sink in sinks {
        let work_rx = work_rx.clone();
        let source_clone = source.clone();
        let progress_clone = progress.cloned();
        let total_clone = total.clone();
        let cancelled_worker = cancelled.clone();
        sink_handles.push(tokio::spawn(async move {
            // Wrap the body so any early-return error trips the shared
            // cancel flag before the `?` unwinds the task.
            let run = async {
                while let Ok(payload) = work_rx.recv_async().await {
                    let prepared = source_clone
                        .prepare_payload(payload)
                        .await
                        .context("preparing payload")?;
                    let files: Vec<(String, u64)> = match &prepared {
                        PreparedPayload::File(h) => vec![(h.relative_path.clone(), h.size)],
                        PreparedPayload::TarShard { headers, .. } => headers
                            .iter()
                            .map(|h| (h.relative_path.clone(), h.size))
                            .collect(),
                        // Resume-block payloads patch existing files; no
                        // file-completion event from one-block-at-a-time.
                        PreparedPayload::FileBlock { .. }
                        | PreparedPayload::FileBlockComplete { .. } => Vec::new(),
                    };
                    let outcome = sink
                        .write_payload(prepared)
                        .await
                        .context("writing payload")?;
                    if let Some(p) = &progress_clone {
                        for (name, size) in &files {
                            p.report_file_complete(name.clone(), *size);
                        }
                    }
                    let mut t = total_clone.lock().unwrap();
                    t.merge(&outcome);
                }
                sink.finish().await?;
                Ok::<(), eyre::Report>(())
            }
            .await;
            if run.is_err() {
                // Signal the forwarder (and implicitly the other workers,
                // once the queue closes) to stop feeding new work.
                cancelled_worker.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            run
        }));
    }

    // Drop our own receiver handle so the channel closes once the
    // forwarder drops its sender and the workers' clones drain — without
    // this, `recv_async` would never see `Disconnected`.
    drop(work_rx);

    // Forwarder: move payloads from the incoming channel onto the shared
    // work queue. `send_async` applies back-pressure (bounded queue); if
    // every worker has gone away (e.g. all sinks errored) the send fails
    // and we stop. It also bails as soon as a worker sets `cancelled`, so
    // a single sink error halts intake promptly instead of waiting for
    // every worker to drop. Dropping `work_tx` on end-of-stream (or on
    // cancel) signals the workers.
    let cancelled_fwd = cancelled.clone();
    let forwarder = tokio::spawn(async move {
        while let Some(payload) = payload_rx.recv().await {
            if cancelled_fwd.load(std::sync::atomic::Ordering::Relaxed) {
                // A worker errored — stop draining the producer and let
                // the queue close so survivors finish and the error
                // surfaces without delay.
                return;
            }
            if work_tx.send_async(payload).await.is_err() {
                // All workers dropped their receivers — nothing left to
                // feed; treat as shutdown.
                return;
            }
        }
        // Dropping work_tx closes the queue → workers see Disconnected
        // after draining and run finish().
        drop(work_tx);
    });

    // Wait for all sinks to finish and aggregate errors (first wins).
    let mut first_err: Option<eyre::Report> = None;
    for h in sink_handles {
        match h.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) if first_err.is_none() => first_err = Some(e),
            Ok(Err(_)) => {}
            Err(join) if first_err.is_none() => {
                first_err = Some(eyre::eyre!("sink worker panicked: {}", join));
            }
            Err(_) => {}
        }
    }
    let _ = forwarder.await;

    if let Some(err) = first_err {
        return Err(err);
    }

    let result = total.lock().unwrap().clone();
    Ok(result)
}

// =====================================================================
// Receive pipeline — symmetric counterpart of execute_sink_pipeline.
// =====================================================================

use crate::generated::FileHeader;

exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '88,190p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
    88	    }
    89	
    90	    let sink_count = sinks.len();
    91	    let capacity = prefetch.max(1) * sink_count;
    92	    let total = Arc::new(std::sync::Mutex::new(SinkOutcome::default()));
    93	
    94	    // Single shared work queue. Each worker owns exactly one sink but
    95	    // pulls payloads from the common queue, so work is stolen by
    96	    // whichever sink is free rather than pre-assigned round-robin.
    97	    let (work_tx, work_rx) = flume::bounded::<TransferPayload>(capacity);
    98	    let mut sink_handles: Vec<tokio::task::JoinHandle<Result<()>>> = Vec::with_capacity(sink_count);
    99	
   100	    // Cancellation flag set by the first worker that errors. Without it,
   101	    // one sink failing only drops that worker's `work_rx` clone; as long
   102	    // as any other worker is alive `send_async` keeps succeeding, so the
   103	    // forwarder would keep draining `payload_rx` and queueing payloads
   104	    // that can never complete — delaying first-error-wins propagation
   105	    // (Codex review, PR2). With it, the forwarder stops at the next
   106	    // payload boundary and closes the queue so the survivors drain and
   107	    // finish promptly.
   108	    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
   109	
   110	    for sink in sinks {
   111	        let work_rx = work_rx.clone();
   112	        let source_clone = source.clone();
   113	        let progress_clone = progress.cloned();
   114	        let total_clone = total.clone();
   115	        let cancelled_worker = cancelled.clone();
   116	        sink_handles.push(tokio::spawn(async move {
   117	            // Wrap the body so any early-return error trips the shared
   118	            // cancel flag before the `?` unwinds the task.
   119	            let run = async {
   120	                while let Ok(payload) = work_rx.recv_async().await {
   121	                    let prepared = source_clone
   122	                        .prepare_payload(payload)
   123	                        .await
   124	                        .context("preparing payload")?;
   125	                    let files: Vec<(String, u64)> = match &prepared {
   126	                        PreparedPayload::File(h) => vec![(h.relative_path.clone(), h.size)],
   127	                        PreparedPayload::TarShard { headers, .. } => headers
   128	                            .iter()
   129	                            .map(|h| (h.relative_path.clone(), h.size))
   130	                            .collect(),
   131	                        // Resume-block payloads patch existing files; no
   132	                        // file-completion event from one-block-at-a-time.
   133	                        PreparedPayload::FileBlock { .. }
   134	                        | PreparedPayload::FileBlockComplete { .. } => Vec::new(),
   135	                    };
   136	                    let outcome = sink
   137	                        .write_payload(prepared)
   138	                        .await
   139	                        .context("writing payload")?;
   140	                    if let Some(p) = &progress_clone {
   141	                        for (name, size) in &files {
   142	                            p.report_file_complete(name.clone(), *size);
   143	                        }
   144	                    }
   145	                    let mut t = total_clone.lock().unwrap();
   146	                    t.merge(&outcome);
   147	                }
   148	                sink.finish().await?;
   149	                Ok::<(), eyre::Report>(())
   150	            }
   151	            .await;
   152	            if run.is_err() {
   153	                // Signal the forwarder (and implicitly the other workers,
   154	                // once the queue closes) to stop feeding new work.
   155	                cancelled_worker.store(true, std::sync::atomic::Ordering::Relaxed);
   156	            }
   157	            run
   158	        }));
   159	    }
   160	
   161	    // Drop our own receiver handle so the channel closes once the
   162	    // forwarder drops its sender and the workers' clones drain — without
   163	    // this, `recv_async` would never see `Disconnected`.
   164	    drop(work_rx);
   165	
   166	    // Forwarder: move payloads from the incoming channel onto the shared
   167	    // work queue. `send_async` applies back-pressure (bounded queue); if
   168	    // every worker has gone away (e.g. all sinks errored) the send fails
   169	    // and we stop. It also bails as soon as a worker sets `cancelled`, so
   170	    // a single sink error halts intake promptly instead of waiting for
   171	    // every worker to drop. Dropping `work_tx` on end-of-stream (or on
   172	    // cancel) signals the workers.
   173	    let cancelled_fwd = cancelled.clone();
   174	    let forwarder = tokio::spawn(async move {
   175	        while let Some(payload) = payload_rx.recv().await {
   176	            if cancelled_fwd.load(std::sync::atomic::Ordering::Relaxed) {
   177	                // A worker errored — stop draining the producer and let
   178	                // the queue close so survivors finish and the error
   179	                // surfaces without delay.
   180	                return;
   181	            }
   182	            if work_tx.send_async(payload).await.is_err() {
   183	                // All workers dropped their receivers — nothing left to
   184	                // feed; treat as shutdown.
   185	                return;
   186	            }
   187	        }
   188	        // Dropping work_tx closes the queue → workers see Disconnected
   189	        // after draining and run finish().
   190	        drop(work_tx);

exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '140,165p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   140	impl<P: Probe> DataPlaneSession<P> {
   141	    /// Create a session carrying an arbitrary [`Probe`]. The generic
   142	    /// primitive behind [`from_stream`](DataPlaneSession::from_stream);
   143	    /// the adaptive controller calls this with a `LiveProbe` to enable
   144	    /// per-stream telemetry.
   145	    pub async fn from_stream_with_probe(
   146	        stream: TcpStream,
   147	        trace: bool,
   148	        chunk_bytes: usize,
   149	        payload_prefetch: usize,
   150	        pool: Arc<BufferPool>,
   151	        probe: P,
   152	    ) -> Self {
   153	        let payload_prefetch = payload_prefetch.max(1);
   154	        let chunk_bytes = chunk_bytes.max(64 * 1024);
   155	        Self {
   156	            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
   157	            pool,
   158	            trace,
   159	            chunk_bytes,
   160	            payload_prefetch,
   161	            bytes_sent: 0,
   162	            probe,
   163	        }
   164	    }
   165	

exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '340,410p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   340	        remaining -= bytes_a as u64;
   341	
   342	        // Main loop: write buf_a while reading into buf_b
   343	        while remaining > 0 {
   344	            // Per-stream telemetry: time the overlapped write+read step
   345	            // as a backpressure proxy. Gated on the compile-time
   346	            // `P::ACTIVE` constant so `DataPlaneSession<NoProbe>` reads
   347	            // no clock and folds this to nothing.
   348	            let step_start = if P::ACTIVE {
   349	                Some(std::time::Instant::now())
   350	            } else {
   351	                None
   352	            };
   353	            // Overlap: write from buf_a, read into buf_b concurrently
   354	            let (write_result, read_result) = tokio::join!(
   355	                self.stream.write_all(&buf_a.as_slice()[..bytes_a]),
   356	                file.read(buf_b.as_mut_slice())
   357	            );
   358	
   359	            write_result.with_context(|| format!("sending {}", rel))?;
   360	            if P::ACTIVE {
   361	                if let Some(t) = step_start {
   362	                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
   363	                }
   364	            }
   365	            self.probe.record_bytes(bytes_a as u64);
   366	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
   367	
   368	            let bytes_b = read_result.with_context(|| format!("reading {}", rel))?;
   369	
   370	            if bytes_b == 0 && remaining > 0 {
   371	                bail!(
   372	                    "unexpected EOF while reading {} ({} bytes remaining)",
   373	                    rel,
   374	                    remaining
   375	                );
   376	            }
   377	            // Same clamp as the initial read: never subtract more than
   378	            // `remaining`, so an over-returning reader can neither
   379	            // underflow the counter nor send undeclared bytes.
   380	            let bytes_b = (bytes_b as u64).min(remaining) as usize;
   381	            remaining -= bytes_b as u64;
   382	
   383	            // Swap roles: buf_b becomes the write buffer, buf_a becomes read buffer
   384	            std::mem::swap(&mut buf_a, &mut buf_b);
   385	            bytes_a = bytes_b;
   386	        }
   387	
   388	        // Final write: send the last chunk in buf_a. This is a pure
   389	        // write (no overlapped read), so the timing is cleanly
   390	        // attributable to socket-write backpressure.
   391	        if bytes_a > 0 {
   392	            let tail_start = if P::ACTIVE {
   393	                Some(std::time::Instant::now())
   394	            } else {
   395	                None
   396	            };
   397	            self.stream
   398	                .write_all(&buf_a.as_slice()[..bytes_a])
   399	                .await
   400	                .with_context(|| format!("sending {}", rel))?;
   401	            if P::ACTIVE {
   402	                if let Some(t) = tail_start {
   403	                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
   404	                }
   405	            }
   406	            self.probe.record_bytes(bytes_a as u64);
   407	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
   408	        }
   409	
   410	        // Buffers return to pool automatically on drop

exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/progress.rs | sed -n '230,338p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   230	    }
   231	
   232	    pub fn from_telemetry(id: StreamId, inner: Arc<StreamTelemetry>) -> Self {
   233	        Self { id, inner }
   234	    }
   235	
   236	    pub fn id(&self) -> StreamId {
   237	        self.id
   238	    }
   239	
   240	    /// Shared `Arc` so a sampler can hold the telemetry independently of
   241	    /// the send loop's probe.
   242	    pub fn telemetry(&self) -> Arc<StreamTelemetry> {
   243	        Arc::clone(&self.inner)
   244	    }
   245	
   246	    /// Add `delta` bytes that just landed on the wire. `Relaxed` is
   247	    /// sufficient: the sampler only needs eventual visibility.
   248	    #[inline]
   249	    pub fn record_bytes(&self, delta: u64) {
   250	        self.inner.bytes_sent.fetch_add(delta, Ordering::Relaxed);
   251	    }
   252	
   253	    /// Add nanoseconds spent blocked on a socket write — the signal the
   254	    /// controller uses to tell "link-bound" from "source-bound".
   255	    #[inline]
   256	    pub fn add_write_blocked(&self, nanos: u64) {
   257	        self.inner
   258	            .write_blocked_nanos
   259	            .fetch_add(nanos, Ordering::Relaxed);
   260	    }
   261	
   262	    pub fn set_state(&self, state: StreamState) {
   263	        self.inner.state.store(state as u8, Ordering::Relaxed);
   264	    }
   265	
   266	    pub fn set_generation(&self, generation: u64) {
   267	        self.inner.generation.store(generation, Ordering::Relaxed);
   268	    }
   269	
   270	    pub fn snapshot(&self) -> StreamTelemetrySnapshot {
   271	        StreamTelemetrySnapshot {
   272	            id: self.id,
   273	            bytes_sent: self.inner.bytes_sent.load(Ordering::Relaxed),
   274	            write_blocked_nanos: self.inner.write_blocked_nanos.load(Ordering::Relaxed),
   275	            state: StreamState::from_u8(self.inner.state.load(Ordering::Relaxed)),
   276	            generation: self.inner.generation.load(Ordering::Relaxed),
   277	        }
   278	    }
   279	}
   280	
   281	/// Zero-cost telemetry abstraction for the byte-copy hot path.
   282	///
   283	/// The send loop is generic over `P: Probe`. The associated
   284	/// `const ACTIVE` lets the timing instrumentation (`Instant::now()`)
   285	/// be compile-time elided for the [`NoProbe`] monomorphization: an
   286	/// `if P::ACTIVE { … }` guarding the clock reads folds to nothing when
   287	/// `ACTIVE == false`, and the empty `#[inline(always)]` methods emit no
   288	/// code. The result is byte-identical codegen to the pre-telemetry hot
   289	/// loop — the hard "zero added cost on the byte-copy hot path"
   290	/// constraint, satisfied at compile time rather than via a runtime
   291	/// branch.
   292	pub trait Probe: Send + Sync + 'static {
   293	    /// When `false`, callers must skip all instrumentation work
   294	    /// (including clock reads) so the optimizer drops it entirely.
   295	    const ACTIVE: bool;
   296	    fn record_bytes(&self, delta: u64);
   297	    fn note_write_blocked(&self, nanos: u64);
   298	}
   299	
   300	/// The default probe: every method is an inlined no-op and `ACTIVE`
   301	/// is `false`, so a `DataPlaneSession<NoProbe>` send loop compiles to
   302	/// exactly today's code.
   303	#[derive(Clone, Copy, Debug, Default)]
   304	pub struct NoProbe;
   305	
   306	impl Probe for NoProbe {
   307	    const ACTIVE: bool = false;
   308	    #[inline(always)]
   309	    fn record_bytes(&self, _delta: u64) {}
   310	    #[inline(always)]
   311	    fn note_write_blocked(&self, _nanos: u64) {}
   312	}
   313	
   314	/// The instrumented probe, constructed only under adaptive mode. Wraps
   315	/// a [`StreamProbe`] and forwards into its lock-free counters.
   316	#[derive(Clone, Debug)]
   317	pub struct LiveProbe(pub StreamProbe);
   318	
   319	impl Probe for LiveProbe {
   320	    const ACTIVE: bool = true;
   321	    #[inline(always)]
   322	    fn record_bytes(&self, delta: u64) {
   323	        self.0.record_bytes(delta);
   324	    }
   325	    #[inline(always)]
   326	    fn note_write_blocked(&self, nanos: u64) {
   327	        self.0.add_write_blocked(nanos);
   328	    }
   329	}
   330	
   331	#[derive(Clone)]
   332	pub struct RemoteTransferProgress {
   333	    sender: UnboundedSender<ProgressEvent>,
   334	}
   335	
   336	impl RemoteTransferProgress {
   337	    pub fn new(sender: UnboundedSender<ProgressEvent>) -> Self {
   338	        Self { sender }

exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '494,590p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   494	    pub async fn send_block(
   495	        &mut self,
   496	        relative_path: &str,
   497	        offset: u64,
   498	        content: &[u8],
   499	    ) -> Result<()> {
   500	        let path_bytes = relative_path.as_bytes();
   501	        if path_bytes.len() > u32::MAX as usize {
   502	            bail!("relative path too long for transfer: {}", relative_path);
   503	        }
   504	
   505	        trace_client!(
   506	            self,
   507	            "sending block for '{}' at offset {} ({} bytes)",
   508	            relative_path,
   509	            offset,
   510	            content.len()
   511	        );
   512	
   513	        self.stream
   514	            .write_all(&[DATA_PLANE_RECORD_BLOCK])
   515	            .await
   516	            .context("writing block record tag")?;
   517	        self.stream
   518	            .write_all(&(path_bytes.len() as u32).to_be_bytes())
   519	            .await
   520	            .context("writing path length")?;
   521	        self.stream
   522	            .write_all(path_bytes)
   523	            .await
   524	            .context("writing path bytes")?;
   525	        self.stream
   526	            .write_all(&offset.to_be_bytes())
   527	            .await
   528	            .context("writing block offset")?;
   529	        self.stream
   530	            .write_all(&(content.len() as u32).to_be_bytes())
   531	            .await
   532	            .context("writing block length")?;
   533	        self.stream
   534	            .write_all(content)
   535	            .await
   536	            .context("writing block content")?;
   537	        crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(content.len() as u64);
   538	
   539	        self.bytes_sent += content.len() as u64;
   540	        Ok(())
   541	    }
   542	
   543	    /// Signal that block-level transfer for a file is complete.
   544	    /// Format: [type:1][path_len:4][path][total_size:8][mtime:8][perms:4]
   545	    ///
   546	    /// Carries mtime + perms inline so the receiver can stamp the
   547	    /// destination metadata even when zero blocks transferred (the
   548	    /// "mtime touched, content identical" case for mirror).
   549	    pub async fn send_block_complete(
   550	        &mut self,
   551	        relative_path: &str,
   552	        total_size: u64,
   553	        mtime_seconds: i64,
   554	        permissions: u32,
   555	    ) -> Result<()> {
   556	        let path_bytes = relative_path.as_bytes();
   557	        if path_bytes.len() > u32::MAX as usize {
   558	            bail!("relative path too long for transfer: {}", relative_path);
   559	        }
   560	
   561	        trace_client!(
   562	            self,
   563	            "sending block complete for '{}' ({} bytes total)",
   564	            relative_path,
   565	            total_size
   566	        );
   567	
   568	        self.stream
   569	            .write_all(&[DATA_PLANE_RECORD_BLOCK_COMPLETE])
   570	            .await
   571	            .context("writing block complete record tag")?;
   572	        self.stream
   573	            .write_all(&(path_bytes.len() as u32).to_be_bytes())
   574	            .await
   575	            .context("writing path length")?;
   576	        self.stream
   577	            .write_all(path_bytes)
   578	            .await
   579	            .context("writing path bytes")?;
   580	        self.stream
   581	            .write_all(&total_size.to_be_bytes())
   582	            .await
   583	            .context("writing total size")?;
   584	        self.stream
   585	            .write_all(&mtime_seconds.to_be_bytes())
   586	            .await
   587	            .context("writing mtime")?;
   588	        self.stream
   589	            .write_all(&permissions.to_be_bytes())
   590	            .await

exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/sink.rs | sed -n '730,830p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   730	            log::warn!("set permissions on {}: {}", dst.display(), e);
   731	        }
   732	    }
   733	    #[cfg(not(unix))]
   734	    let _ = permissions;
   735	    Ok(SinkOutcome {
   736	        files_written: 1,
   737	        bytes_written: 0,
   738	    })
   739	}
   740	
   741	// ---------------------------------------------------------------------------
   742	// DataPlaneSink — TCP data plane writer
   743	// ---------------------------------------------------------------------------
   744	
   745	/// Writes payloads to a remote daemon via the TCP data plane binary protocol.
   746	///
   747	/// Each instance wraps a single TCP stream (DataPlaneSession). For multi-stream
   748	/// transfers, the pipeline executor creates multiple DataPlaneSink instances.
   749	pub struct DataPlaneSink<P: Probe = NoProbe> {
   750	    session: tokio::sync::Mutex<DataPlaneSession<P>>,
   751	    source: Arc<dyn TransferSource>,
   752	    dst_root: PathBuf,
   753	}
   754	
   755	impl<P: Probe> DataPlaneSink<P> {
   756	    pub fn new(
   757	        session: DataPlaneSession<P>,
   758	        source: Arc<dyn TransferSource>,
   759	        dst_root: PathBuf,
   760	    ) -> Self {
   761	        Self {
   762	            session: tokio::sync::Mutex::new(session),
   763	            source,
   764	            dst_root,
   765	        }
   766	    }
   767	}
   768	
   769	#[async_trait]
   770	impl<P: Probe> TransferSink for DataPlaneSink<P> {
   771	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
   772	        let mut session = self.session.lock().await;
   773	        match payload {
   774	            PreparedPayload::File(header) => {
   775	                let size = header.size;
   776	                session
   777	                    .send_file(self.source.clone(), &header)
   778	                    .await
   779	                    .with_context(|| format!("sending {}", header.relative_path))?;
   780	                Ok(SinkOutcome {
   781	                    files_written: 1,
   782	                    bytes_written: size,
   783	                })
   784	            }
   785	            PreparedPayload::TarShard { headers, data } => {
   786	                let bytes: u64 = headers.iter().map(|h| h.size).sum();
   787	                let count = headers.len();
   788	                session
   789	                    .send_prepared_tar_shard(headers, &data)
   790	                    .await
   791	                    .context("sending tar shard")?;
   792	                Ok(SinkOutcome {
   793	                    files_written: count,
   794	                    bytes_written: bytes,
   795	                })
   796	            }
   797	            // Resume payloads can't be relayed without a reverse-resume
   798	            // protocol on the next hop. Reject explicitly.
   799	            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   800	                eyre::bail!("DataPlaneSink does not relay resume-block payloads")
   801	            }
   802	        }
   803	    }
   804	
   805	    /// Relay case: bytes arrive on `reader` (e.g. from a DataPlaneSource
   806	    /// during a remote→remote transfer) and forward to the next hop.
   807	    async fn write_file_stream(
   808	        &self,
   809	        header: &FileHeader,
   810	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   811	    ) -> Result<SinkOutcome> {
   812	        let size = header.size;
   813	        let mut session = self.session.lock().await;
   814	        session
   815	            .send_file_from_reader(header, reader)
   816	            .await
   817	            .with_context(|| format!("relaying {}", header.relative_path))?;
   818	        Ok(SinkOutcome {
   819	            files_written: 1,
   820	            bytes_written: size,
   821	        })
   822	    }
   823	
   824	    async fn finish(&self) -> Result<()> {
   825	        let mut session = self.session.lock().await;
   826	        session.finish().await
   827	    }
   828	
   829	    fn root(&self) -> &Path {
   830	        &self.dst_root

exec
/usr/bin/bash -lc "rg -n \"send_block\\(|send_block_complete\\(|FileBlock\" crates/blit-core/src/remote/transfer -g '*.rs'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
crates/blit-core/src/remote/transfer/pipeline.rs:133:                        PreparedPayload::FileBlock { .. }
crates/blit-core/src/remote/transfer/pipeline.rs:134:                        | PreparedPayload::FileBlockComplete { .. } => Vec::new(),
crates/blit-core/src/remote/transfer/pipeline.rs:236:/// [`PreparedPayload::FileBlock`] events) and feeds them to a single sink
crates/blit-core/src/remote/transfer/pipeline.rs:313:                let payload = PreparedPayload::FileBlock {
crates/blit-core/src/remote/transfer/pipeline.rs:329:                let payload = PreparedPayload::FileBlockComplete {
crates/blit-core/src/remote/transfer/payload.rs:31:    FileBlock {
crates/blit-core/src/remote/transfer/payload.rs:37:    FileBlockComplete {
crates/blit-core/src/remote/transfer/payload.rs:61:        TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => {
crates/blit-core/src/remote/transfer/payload.rs:62:            bail!("FileBlock payloads cannot be prepared from a filesystem source")
crates/blit-core/src/remote/transfer/payload.rs:71:/// uses `FileBlock` / `FileBlockComplete` for the resume protocol.
crates/blit-core/src/remote/transfer/payload.rs:90:    FileBlock {
crates/blit-core/src/remote/transfer/payload.rs:100:    FileBlockComplete {
crates/blit-core/src/remote/transfer/payload.rs:186:    // Resume variants (FileBlock / FileBlockComplete) are receive-only and
crates/blit-core/src/remote/transfer/payload.rs:191:        TransferPayload::FileBlock { size, .. } => (2, *size),
crates/blit-core/src/remote/transfer/payload.rs:192:        TransferPayload::FileBlockComplete { .. } => (3, 0),
crates/blit-core/src/remote/transfer/payload.rs:209:            TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => 0,
crates/blit-core/src/remote/transfer/payload.rs:334:            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
crates/blit-core/src/remote/transfer/payload.rs:335:                bail!("FileBlock payloads cannot traverse the gRPC control plane (outbound only)");
crates/blit-core/src/remote/transfer/data_plane.rs:206:                PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
crates/blit-core/src/remote/transfer/data_plane.rs:494:    pub async fn send_block(
crates/blit-core/src/remote/transfer/data_plane.rs:549:    pub async fn send_block_complete(
crates/blit-core/src/remote/transfer/source.rs:298:            TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => {
crates/blit-core/src/remote/transfer/source.rs:299:                bail!("FileBlock payloads cannot be prepared from a remote source")
crates/blit-core/src/remote/transfer/diff_planner.rs:11://!      block-level resume `FileBlock` + `FileBlockComplete` pairs).
crates/blit-core/src/remote/transfer/sink.rs:224:            PreparedPayload::FileBlock {
crates/blit-core/src/remote/transfer/sink.rs:238:            PreparedPayload::FileBlockComplete {
crates/blit-core/src/remote/transfer/sink.rs:799:            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
crates/blit-core/src/remote/transfer/sink.rs:873:            PreparedPayload::FileBlock { bytes, .. } => Ok(SinkOutcome {
crates/blit-core/src/remote/transfer/sink.rs:877:            PreparedPayload::FileBlockComplete { .. } => Ok(SinkOutcome::default()),
crates/blit-core/src/remote/transfer/sink.rs:1065:            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
crates/blit-core/src/remote/transfer/sink.rs:1066:                eyre::bail!("GrpcFallbackSink does not handle FileBlock payloads (outbound only)");
crates/blit-core/src/remote/transfer/sink.rs:1234:            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
crates/blit-core/src/remote/transfer/sink.rs:1236:                    "GrpcServerStreamingSink does not handle FileBlock payloads (resume \
crates/blit-core/src/remote/transfer/sink.rs:2281:    /// c-1b round 2 regression: resume `FileBlock` payloads
crates/blit-core/src/remote/transfer/sink.rs:2290:        // FileBlock writes seek into an existing destination file.
crates/blit-core/src/remote/transfer/sink.rs:2312:            .write_payload(PreparedPayload::FileBlock {
crates/blit-core/src/remote/transfer/sink.rs:2320:        // FileBlock's outcome.bytes_written reflects bytes
crates/blit-core/src/remote/transfer/sink.rs:2326:            "FileBlock byte progress must equal outcome.bytes_written"

exec
/usr/bin/bash -lc "rg -n \"from_stream_with_probe|LiveProbe|StreamProbe|DataPlaneSession<\" crates -g '*.rs'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
crates/blit-core/src/remote/transfer/progress.rs:219:pub struct StreamProbe {
crates/blit-core/src/remote/transfer/progress.rs:224:impl StreamProbe {
crates/blit-core/src/remote/transfer/progress.rs:301:/// is `false`, so a `DataPlaneSession<NoProbe>` send loop compiles to
crates/blit-core/src/remote/transfer/progress.rs:315:/// a [`StreamProbe`] and forwards into its lock-free counters.
crates/blit-core/src/remote/transfer/progress.rs:317:pub struct LiveProbe(pub StreamProbe);
crates/blit-core/src/remote/transfer/progress.rs:319:impl Probe for LiveProbe {
crates/blit-core/src/remote/transfer/progress.rs:362:        let probe = StreamProbe::new(StreamId(7));
crates/blit-core/src/remote/transfer/progress.rs:363:        let live = LiveProbe(probe.clone());
crates/blit-core/src/remote/transfer/progress.rs:379:    const _: () = assert!(<LiveProbe as Probe>::ACTIVE);
crates/blit-core/src/remote/transfer/progress.rs:393:        let probe = StreamProbe::new(StreamId(0));
crates/blit-core/src/remote/transfer/progress.rs:403:        let probe = StreamProbe::new(StreamId(1));
crates/blit-core/src/remote/transfer/data_plane.rs:29:/// entirely for `DataPlaneSession<NoProbe>`. Existing callers name the
crates/blit-core/src/remote/transfer/data_plane.rs:31:/// constructs `DataPlaneSession<LiveProbe>` via
crates/blit-core/src/remote/transfer/data_plane.rs:32:/// [`from_stream_with_probe`](DataPlaneSession::from_stream_with_probe).
crates/blit-core/src/remote/transfer/data_plane.rs:41:pub struct DataPlaneSession<P: Probe = NoProbe> {
crates/blit-core/src/remote/transfer/data_plane.rs:59:impl DataPlaneSession<NoProbe> {
crates/blit-core/src/remote/transfer/data_plane.rs:64:    /// [`StallGuardWriter`] (inside `from_stream_with_probe`) so a
crates/blit-core/src/remote/transfer/data_plane.rs:77:        Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, NoProbe)
crates/blit-core/src/remote/transfer/data_plane.rs:140:impl<P: Probe> DataPlaneSession<P> {
crates/blit-core/src/remote/transfer/data_plane.rs:143:    /// the adaptive controller calls this with a `LiveProbe` to enable
crates/blit-core/src/remote/transfer/data_plane.rs:145:    pub async fn from_stream_with_probe(
crates/blit-core/src/remote/transfer/data_plane.rs:346:            // `P::ACTIVE` constant so `DataPlaneSession<NoProbe>` reads
crates/blit-core/src/remote/transfer/mod.rs:26:    ByteProgressSink, LiveProbe, NoProbe, Probe, ProgressEvent, RemoteTransferProgress, StreamId,
crates/blit-core/src/remote/transfer/mod.rs:27:    StreamProbe, StreamState, StreamTelemetry, StreamTelemetrySnapshot,
crates/blit-core/src/remote/transfer/sink.rs:750:    session: tokio::sync::Mutex<DataPlaneSession<P>>,
crates/blit-core/src/remote/transfer/sink.rs:757:        session: DataPlaneSession<P>,

exec
/usr/bin/bash -lc "rg -n \"DataPlaneSession::connect|DataPlaneSession::from_stream|DataPlaneSink::new|DataPlaneSink<\" crates -g '*.rs'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
crates/blit-daemon/src/service/pull.rs:732:        let session = blit_core::remote::transfer::data_plane::DataPlaneSession::from_stream(
crates/blit-daemon/src/service/pull.rs:741:        sinks.push(Arc::new(DataPlaneSink::new(
crates/blit-daemon/src/service/pull_sync.rs:641:    let session = DataPlaneSession::from_stream(socket, false, tuning.chunk_bytes, 8, pool).await;
crates/blit-daemon/src/service/pull_sync.rs:645:    let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
crates/blit-daemon/src/service/pull_sync.rs:765:        DataPlaneSession::from_stream(socket, false, tuning.chunk_bytes, 8, pool).await;
crates/blit-core/src/remote/push/client/mod.rs:134:            let session = DataPlaneSession::connect(
crates/blit-core/src/remote/push/client/mod.rs:145:            sinks.push(Arc::new(DataPlaneSink::new(
crates/blit-core/src/remote/transfer/sink.rs:749:pub struct DataPlaneSink<P: Probe = NoProbe> {
crates/blit-core/src/remote/transfer/sink.rs:755:impl<P: Probe> DataPlaneSink<P> {
crates/blit-core/src/remote/transfer/sink.rs:770:impl<P: Probe> TransferSink for DataPlaneSink<P> {
crates/blit-core/src/remote/transfer/data_plane.rs:32:/// [`from_stream_with_probe`](DataPlaneSession::from_stream_with_probe).
crates/blit-core/src/remote/transfer/data_plane.rs:142:    /// primitive behind [`from_stream`](DataPlaneSession::from_stream);
crates/blit-core/src/remote/transfer/data_plane.rs:856:        let mut session = DataPlaneSession::from_stream(client, false, 64 * 1024, 1, pool).await;

codex
One possible telemetry gap is in block-resume payloads, so I’m checking whether those payloads ever traverse `DataPlaneSink`; if they are explicitly rejected there, it is not a shipping finding for this slice.
exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/sink.rs | sed -n '770,806p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   770	impl<P: Probe> TransferSink for DataPlaneSink<P> {
   771	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
   772	        let mut session = self.session.lock().await;
   773	        match payload {
   774	            PreparedPayload::File(header) => {
   775	                let size = header.size;
   776	                session
   777	                    .send_file(self.source.clone(), &header)
   778	                    .await
   779	                    .with_context(|| format!("sending {}", header.relative_path))?;
   780	                Ok(SinkOutcome {
   781	                    files_written: 1,
   782	                    bytes_written: size,
   783	                })
   784	            }
   785	            PreparedPayload::TarShard { headers, data } => {
   786	                let bytes: u64 = headers.iter().map(|h| h.size).sum();
   787	                let count = headers.len();
   788	                session
   789	                    .send_prepared_tar_shard(headers, &data)
   790	                    .await
   791	                    .context("sending tar shard")?;
   792	                Ok(SinkOutcome {
   793	                    files_written: count,
   794	                    bytes_written: bytes,
   795	                })
   796	            }
   797	            // Resume payloads can't be relayed without a reverse-resume
   798	            // protocol on the next hop. Reject explicitly.
   799	            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   800	                eyre::bail!("DataPlaneSink does not relay resume-block payloads")
   801	            }
   802	        }
   803	    }
   804	
   805	    /// Relay case: bytes arrive on `reader` (e.g. from a DataPlaneSource
   806	    /// during a remote→remote transfer) and forward to the next hop.

exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '224,340p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   224	use super::data_plane::{
   225	    DATA_PLANE_RECORD_BLOCK, DATA_PLANE_RECORD_BLOCK_COMPLETE, DATA_PLANE_RECORD_END,
   226	    DATA_PLANE_RECORD_FILE, DATA_PLANE_RECORD_TAR_SHARD,
   227	};
   228	
   229	/// Drive a `TransferSink` from a TCP wire stream.
   230	///
   231	/// This is the symmetric counterpart to [`execute_sink_pipeline_streaming`]:
   232	/// where the outbound executor takes a [`TransferSource`] and dispatches
   233	/// payloads round-robin across N sinks, this one consumes a single
   234	/// inbound wire (parsing record headers and producing
   235	/// [`PreparedPayload::FileStream`] / [`PreparedPayload::TarShard`] /
   236	/// [`PreparedPayload::FileBlock`] events) and feeds them to a single sink
   237	/// sequentially. Multi-stream parallelism comes from spawning N invocations,
   238	/// one per inbound TCP connection.
   239	///
   240	/// Both directions converge on `TransferSink::write_payload`: file data
   241	/// hits disk through `FsTransferSink::write_payload(FileStream { … })`,
   242	/// which uses the same `receive_stream_double_buffered` helper as the
   243	/// daemon's push receiver and the client's pull receiver — one path,
   244	/// one optimization surface.
   245	pub async fn execute_receive_pipeline<R: AsyncRead + Unpin + Send>(
   246	    socket: &mut R,
   247	    sink: Arc<dyn TransferSink>,
   248	    progress: Option<&RemoteTransferProgress>,
   249	) -> Result<SinkOutcome> {
   250	    let mut total = SinkOutcome::default();
   251	
   252	    loop {
   253	        let mut tag = [0u8; 1];
   254	        socket
   255	            .read_exact(&mut tag)
   256	            .await
   257	            .context("reading data-plane record tag")?;
   258	
   259	        match tag[0] {
   260	            DATA_PLANE_RECORD_END => break,
   261	            DATA_PLANE_RECORD_FILE => {
   262	                let mut header = read_file_header(socket).await?;
   263	                let file_size = read_u64(socket).await?;
   264	                let mtime = read_i64(socket).await?;
   265	                let perms = read_u32(socket).await?;
   266	                header.size = file_size;
   267	                header.mtime_seconds = mtime;
   268	                header.permissions = perms;
   269	                // Use AsyncReadExt::take to give the sink exactly
   270	                // file_size bytes of the wire. tokio's Take is the
   271	                // canonical way to limit a borrowed AsyncRead.
   272	                use tokio::io::AsyncReadExt;
   273	                let mut reader = (&mut *socket).take(file_size);
   274	                let outcome = sink
   275	                    .write_file_stream(&header, &mut reader)
   276	                    .await
   277	                    .with_context(|| format!("receiving {}", header.relative_path))?;
   278	                if let Some(p) = progress {
   279	                    p.report_payload(0, outcome.bytes_written);
   280	                    p.report_file_complete(header.relative_path.clone(), outcome.bytes_written);
   281	                }
   282	                total.merge(&outcome);
   283	            }
   284	            DATA_PLANE_RECORD_TAR_SHARD => {
   285	                let (headers, data) = read_tar_shard(socket).await?;
   286	                let bytes = data.len() as u64;
   287	                let payload = PreparedPayload::TarShard { headers, data };
   288	                let outcome = sink
   289	                    .write_payload(payload)
   290	                    .await
   291	                    .context("writing payload")?;
   292	                if let Some(p) = progress {
   293	                    p.report_payload(0, bytes);
   294	                }
   295	                total.merge(&outcome);
   296	            }
   297	            DATA_PLANE_RECORD_BLOCK => {
   298	                let path = read_string(socket).await?;
   299	                let offset = read_u64(socket).await?;
   300	                let len = read_u32(socket).await? as usize;
   301	                if len > MAX_WIRE_BLOCK_BYTES {
   302	                    bail!(
   303	                        "wire block payload {} bytes exceeds max {} (rejecting to avoid OOM)",
   304	                        len,
   305	                        MAX_WIRE_BLOCK_BYTES
   306	                    );
   307	                }
   308	                let mut bytes = vec![0u8; len];
   309	                socket
   310	                    .read_exact(&mut bytes)
   311	                    .await
   312	                    .context("reading block bytes")?;
   313	                let payload = PreparedPayload::FileBlock {
   314	                    relative_path: path,
   315	                    offset,
   316	                    bytes,
   317	                };
   318	                let outcome = sink
   319	                    .write_payload(payload)
   320	                    .await
   321	                    .context("writing payload")?;
   322	                total.merge(&outcome);
   323	            }
   324	            DATA_PLANE_RECORD_BLOCK_COMPLETE => {
   325	                let path = read_string(socket).await?;
   326	                let total_size = read_u64(socket).await?;
   327	                let mtime = read_i64(socket).await?;
   328	                let perms = read_u32(socket).await?;
   329	                let payload = PreparedPayload::FileBlockComplete {
   330	                    relative_path: path,
   331	                    total_size,
   332	                    mtime_seconds: mtime,
   333	                    permissions: perms,
   334	                };
   335	                let outcome = sink
   336	                    .write_payload(payload)
   337	                    .await
   338	                    .context("writing payload")?;
   339	                total.merge(&outcome);
   340	            }

exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/sink.rs | sed -n '850,900p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   850	            label: PathBuf::from("/dev/null"),
   851	        }
   852	    }
   853	}
   854	
   855	impl NullSink {
   856	    pub fn new() -> Self {
   857	        Self::default()
   858	    }
   859	}
   860	
   861	#[async_trait]
   862	impl TransferSink for NullSink {
   863	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
   864	        match payload {
   865	            PreparedPayload::File(header) => Ok(SinkOutcome {
   866	                files_written: 1,
   867	                bytes_written: header.size,
   868	            }),
   869	            PreparedPayload::TarShard { headers, data } => Ok(SinkOutcome {
   870	                files_written: headers.len(),
   871	                bytes_written: data.len() as u64,
   872	            }),
   873	            PreparedPayload::FileBlock { bytes, .. } => Ok(SinkOutcome {
   874	                files_written: 0,
   875	                bytes_written: bytes.len() as u64,
   876	            }),
   877	            PreparedPayload::FileBlockComplete { .. } => Ok(SinkOutcome::default()),
   878	        }
   879	    }
   880	
   881	    /// Drain the wire so the protocol stream stays aligned, then count
   882	    /// the bytes. Lets `--null` benchmark the receive path end-to-end
   883	    /// without paying for disk writes.
   884	    async fn write_file_stream(
   885	        &self,
   886	        header: &FileHeader,
   887	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   888	    ) -> Result<SinkOutcome> {
   889	        use crate::remote::transfer::data_plane::{
   890	            receive_stream_double_buffered, RECEIVE_CHUNK_SIZE,
   891	        };
   892	        let mut sink = tokio::io::sink();
   893	        // --null benchmark: bytes never land on user disk; do
   894	        // not advance a daemon-side progress counter for these
   895	        // drains. Same reasoning as the dry-run path on
   896	        // FsTransferSink.
   897	        let n = receive_stream_double_buffered(
   898	            reader,
   899	            &mut sink,
   900	            header.size,

exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/payload.rs | sed -n '280,350p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   280	                        bail!(
   281	                            "unexpected EOF while reading {} ({} bytes remaining)",
   282	                            header.relative_path,
   283	                            remaining
   284	                        );
   285	                    }
   286	
   287	                    send_payload(
   288	                        tx,
   289	                        ClientPayload::FileData(FileData {
   290	                            content: buffer[..chunk].to_vec(),
   291	                        }),
   292	                    )
   293	                    .await?;
   294	                    if let Some(progress) = progress {
   295	                        progress.report_payload(0, chunk as u64);
   296	                    }
   297	                    remaining -= chunk as u64;
   298	                }
   299	                if let Some(progress) = progress {
   300	                    progress.report_file_complete(header.relative_path.clone(), header.size);
   301	                }
   302	            }
   303	            PreparedPayload::TarShard { headers, data } => {
   304	                send_payload(
   305	                    tx,
   306	                    ClientPayload::TarShardHeader(TarShardHeader {
   307	                        files: headers.clone(),
   308	                        archive_size: data.len() as u64,
   309	                    }),
   310	                )
   311	                .await?;
   312	
   313	                for chunk in data.chunks(chunk_size) {
   314	                    send_payload(
   315	                        tx,
   316	                        ClientPayload::TarShardChunk(TarShardChunk {
   317	                            content: chunk.to_vec(),
   318	                        }),
   319	                    )
   320	                    .await?;
   321	                    if let Some(progress) = progress {
   322	                        progress.report_payload(0, chunk.len() as u64);
   323	                    }
   324	                }
   325	
   326	                send_payload(tx, ClientPayload::TarShardComplete(TarShardComplete {})).await?;
   327	                if let Some(progress) = progress {
   328	                    for header in &headers {
   329	                        progress.report_file_complete(header.relative_path.clone(), header.size);
   330	                    }
   331	                }
   332	            }
   333	            // Resume variants are receive-only — gRPC control plane is outbound only.
   334	            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   335	                bail!("FileBlock payloads cannot traverse the gRPC control plane (outbound only)");
   336	            }
   337	        }
   338	    }
   339	
   340	    if finish {
   341	        send_payload(tx, ClientPayload::UploadComplete(UploadComplete {})).await?;
   342	    }
   343	
   344	    Ok(())
   345	}
   346	
   347	async fn send_payload(tx: &mpsc::Sender<ClientPushRequest>, payload: ClientPayload) -> Result<()> {
   348	    tx.send(ClientPushRequest {
   349	        payload: Some(payload),
   350	    })

exec
/usr/bin/bash -lc "rg -n \"DataPlaneSession::connect|connect\\(["'^'"\\n]*pool|from_stream_with_probe\" crates/blit-core/src crates/blit-daemon/src -g '*.rs'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
crates/blit-core/src/remote/transfer/data_plane.rs:32:/// [`from_stream_with_probe`](DataPlaneSession::from_stream_with_probe).
crates/blit-core/src/remote/transfer/data_plane.rs:64:    /// [`StallGuardWriter`] (inside `from_stream_with_probe`) so a
crates/blit-core/src/remote/transfer/data_plane.rs:77:        Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, NoProbe)
crates/blit-core/src/remote/transfer/data_plane.rs:145:    pub async fn from_stream_with_probe(
crates/blit-core/src/remote/push/client/mod.rs:134:            let session = DataPlaneSession::connect(

exec
/usr/bin/bash -lc "rg -n \"\\.send_block\\(|send_block_complete\\(\" crates -g '*.rs'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
crates/blit-core/src/remote/transfer/data_plane.rs:549:    pub async fn send_block_complete(
crates/blit-daemon/src/service/pull_sync.rs:868:                    .send_block(relative_path, offset as u64, block_content)
crates/blit-daemon/src/service/pull_sync.rs:883:            .send_block_complete(

exec
/usr/bin/bash -lc "rg -n \"execute_sink_pipeline_streaming\\(\" crates -g '*.rs'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
crates/blit-daemon/src/service/pull.rs:880:    execute_sink_pipeline_streaming(source, sinks, payload_rx, payload_prefetch, None)
crates/blit-core/src/remote/push/client/mod.rs:157:            execute_sink_pipeline_streaming(
crates/blit-core/src/remote/transfer/pipeline.rs:55:    let result = execute_sink_pipeline_streaming(source, sinks, rx, prefetch, progress).await;
crates/blit-core/src/remote/transfer/pipeline.rs:77:pub async fn execute_sink_pipeline_streaming(
crates/blit-core/src/remote/transfer/pipeline.rs:593:        let outcome = execute_sink_pipeline_streaming(source, vec![sink], rx, 2, None)
crates/blit-core/src/remote/transfer/pipeline.rs:944:            execute_sink_pipeline_streaming(source_clone, vec![failing], payload_rx, 4, None).await
crates/blit-core/src/remote/transfer/pipeline.rs:1089:        let outcome = execute_sink_pipeline_streaming(source, vec![fast, slow], rx, 2, None)
crates/blit-core/src/remote/transfer/pipeline.rs:1172:        let result = execute_sink_pipeline_streaming(source, vec![sink], rx, 1, None).await;
crates/blit-core/src/remote/transfer/pipeline.rs:1258:        let outcome = execute_sink_pipeline_streaming(source, vec![a, b], rx, 2, None)
crates/blit-core/src/remote/transfer/pipeline.rs:1323:            execute_sink_pipeline_streaming(source, vec![sink], rx, 2, None),

exec
/usr/bin/bash -lc "nl -ba crates/blit-daemon/src/service/pull_sync.rs | sed -n '820,895p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   820	            }
   821	        } else {
   822	            None
   823	        };
   824	
   825	        // Open file for streaming read
   826	        let mut file = tokio::fs::File::open(&abs_path).await.map_err(|e| {
   827	            Status::internal(format!("failed to open {}: {}", abs_path.display(), e))
   828	        })?;
   829	
   830	        let file_size = file
   831	            .metadata()
   832	            .await
   833	            .map_err(|e| {
   834	                Status::internal(format!(
   835	                    "failed to get metadata for {}: {}",
   836	                    abs_path.display(),
   837	                    e
   838	                ))
   839	            })?
   840	            .len() as usize;
   841	
   842	        // Process blocks by streaming
   843	        let mut block_idx = 0usize;
   844	        let mut offset = 0usize;
   845	
   846	        loop {
   847	            let bytes_read = file.read(&mut buffer).await.map_err(|e| {
   848	                Status::internal(format!("reading block from {}: {}", abs_path.display(), e))
   849	            })?;
   850	
   851	            if bytes_read == 0 {
   852	                break;
   853	            }
   854	
   855	            let block_content = &buffer[..bytes_read];
   856	            let server_hash = blake3::hash(block_content);
   857	
   858	            // Check if this block needs transfer
   859	            let needs_transfer = match &file_client_hashes {
   860	                Some(hashes) if block_idx < hashes.len() => {
   861	                    server_hash.as_bytes() != hashes[block_idx].as_slice()
   862	                }
   863	                _ => true,
   864	            };
   865	
   866	            if needs_transfer {
   867	                session
   868	                    .send_block(relative_path, offset as u64, block_content)
   869	                    .await
   870	                    .map_err(|err| Status::internal(format!("sending block: {}", err)))?;
   871	
   872	                stats.bytes_transferred += block_content.len() as u64;
   873	            }
   874	
   875	            offset += bytes_read;
   876	            block_idx += 1;
   877	        }
   878	
   879	        // Signal file complete via data plane. Send mtime + perms with
   880	        // the terminator so the receiver can stamp metadata even when
   881	        // zero blocks transferred (mtime-only touch + auto-promote case).
   882	        session
   883	            .send_block_complete(
   884	                relative_path,
   885	                file_size as u64,
   886	                entry.header.mtime_seconds,
   887	                entry.header.permissions,
   888	            )
   889	            .await
   890	            .map_err(|err| Status::internal(format!("sending block complete: {}", err)))?;
   891	
   892	        stats.files_transferred += 1;
   893	    }
   894	
   895	    // Finish data plane session

exec
/usr/bin/bash -lc "nl -ba crates/blit-daemon/src/service/pull_sync.rs | sed -n '620,675p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   620	        Ok(Ok(_)) => {}
   621	        Ok(Err(e)) => {
   622	            return Err(Status::internal(format!("failed to read token: {}", e)));
   623	        }
   624	        Err(_elapsed) => {
   625	            return Err(Status::deadline_exceeded(format!(
   626	                "pull-sync token read timed out after {:?}",
   627	                PULL_SYNC_TOKEN_TIMEOUT
   628	            )));
   629	        }
   630	    }
   631	    if token_buf != expected_token {
   632	        return Err(Status::unauthenticated("invalid data plane token"));
   633	    }
   634	
   635	    // Wrap the session as a TransferSink and route through the unified pipeline.
   636	    let buffer_size = tuning.chunk_bytes.max(64 * 1024);
   637	    let pool_size = 4;
   638	    let memory_budget = buffer_size * pool_size * 2;
   639	    let pool = Arc::new(BufferPool::new(buffer_size, pool_size, Some(memory_budget)));
   640	
   641	    let session = DataPlaneSession::from_stream(socket, false, tuning.chunk_bytes, 8, pool).await;
   642	
   643	    let source: Arc<dyn TransferSource> =
   644	        Arc::new(FsTransferSource::new(source_root.to_path_buf()));
   645	    let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
   646	        session,
   647	        source.clone(),
   648	        source_root.to_path_buf(),
   649	    ));
   650	
   651	    execute_sink_pipeline(source, vec![sink], planned.payloads, 8, None)
   652	        .await
   653	        .map_err(|err| Status::internal(format!("pull sync data plane pipeline: {err:#}")))?;
   654	
   655	    Ok(TransferStats {
   656	        files_transferred: file_count as u64,
   657	        bytes_transferred: total_bytes,
   658	        bytes_zero_copy: 0,
   659	    })
   660	}
   661	
   662	/// Stream files using block-level resume via data plane (primary path).
   663	///
   664	/// Uses gRPC for block hash exchange, then sends blocks via TCP data plane.
   665	/// Pipelines block hash requests to avoid per-file RTT penalty.
   666	async fn stream_via_data_plane_resume(
   667	    module: &ModuleConfig,
   668	    entries: Vec<PullEntry>,
   669	    total_bytes: u64,
   670	    block_size_param: u32,
   671	    tx: &PullSyncSender,
   672	    stream: &mut Streaming<ClientPullMessage>,
   673	    effective_resume: &std::collections::HashSet<String>,
   674	) -> Result<TransferStats, Status> {
   675	    use blit_core::buffer::BufferPool;

exec
/usr/bin/bash -lc "nl -ba crates/blit-daemon/src/service/pull.rs | sed -n '720,745p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   720	            Err(_elapsed) => {
   721	                return Err(Status::deadline_exceeded(format!(
   722	                    "pull token read timed out after {:?}",
   723	                    PULL_TOKEN_TIMEOUT
   724	                )));
   725	            }
   726	        }
   727	        if token_buf != expected_token {
   728	            log::warn!("pull data plane: invalid token");
   729	            return Err(Status::permission_denied("invalid pull data plane token"));
   730	        }
   731	
   732	        let session = blit_core::remote::transfer::data_plane::DataPlaneSession::from_stream(
   733	            socket,
   734	            false,
   735	            chunk_bytes,
   736	            payload_prefetch,
   737	            Arc::clone(&pool),
   738	        )
   739	        .await;
   740	
   741	        sinks.push(Arc::new(DataPlaneSink::new(
   742	            session,
   743	            source.clone(),
   744	            dst_root.clone(),
   745	        )));

exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/push/client/mod.rs | sed -n '126,165p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   126	        let buffer_size = chunk_bytes.max(64 * 1024);
   127	        let memory_budget = buffer_size * pool_size * 2;
   128	        let pool = Arc::new(BufferPool::new(buffer_size, pool_size, Some(memory_budget)));
   129	
   130	        let dst_root = PathBuf::from(format!("{}:{}", host, port));
   131	
   132	        let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(streams);
   133	        for _ in 0..streams {
   134	            let session = DataPlaneSession::connect(
   135	                host,
   136	                port,
   137	                token,
   138	                chunk_bytes,
   139	                payload_prefetch,
   140	                trace,
   141	                tcp_buffer_size,
   142	                Arc::clone(&pool),
   143	            )
   144	            .await?;
   145	            sinks.push(Arc::new(DataPlaneSink::new(
   146	                session,
   147	                source.clone(),
   148	                dst_root.clone(),
   149	            )));
   150	        }
   151	
   152	        let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(payload_prefetch.max(1));
   153	
   154	        let source_clone = source.clone();
   155	        let prefetch = payload_prefetch.max(1);
   156	        let pipeline_handle = tokio::spawn(async move {
   157	            execute_sink_pipeline_streaming(
   158	                source_clone,
   159	                sinks,
   160	                payload_rx,
   161	                prefetch,
   162	                progress.as_ref(),
   163	            )
   164	            .await
   165	        });

exec
/usr/bin/bash -lc "nl -ba crates/blit-daemon/src/service/pull_sync.rs | sed -n '879,890p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   879	        // Signal file complete via data plane. Send mtime + perms with
   880	        // the terminator so the receiver can stamp metadata even when
   881	        // zero blocks transferred (mtime-only touch + auto-promote case).
   882	        session
   883	            .send_block_complete(
   884	                relative_path,
   885	                file_size as u64,
   886	                entry.header.mtime_seconds,
   887	                entry.header.permissions,
   888	            )
   889	            .await
   890	            .map_err(|err| Status::internal(format!("sending block complete: {}", err)))?;

codex
The cancellation flag only gates before a send; it does not wake a forwarder already parked in `send_async`. I’m checking `flume` capabilities and existing tests to decide whether that is a real finding or just an acceptable payload-boundary delay.
exec
/usr/bin/bash -lc 'rg -n "flume" Cargo.toml crates/*/Cargo.toml Cargo.lock' in /home/michael/dev/blit_v2
 succeeded in 0ms:
Cargo.lock:315: "flume",
Cargo.lock:993:name = "flume"
Cargo.lock:1711: "flume",
crates/blit-core/Cargo.toml:29:flume = "0.11"

exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '1110,1182p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
  1110	
  1111	    /// Codex-review (PR2) regression: when the only sink errors, the
  1112	    /// forwarder must stop draining the producer promptly rather than
  1113	    /// continuing to pull every remaining payload. We feed a large
  1114	    /// payload set through a single always-failing sink and assert that
  1115	    /// (a) the pipeline surfaces the error, and (b) the forwarder
  1116	    /// consumed far fewer than all payloads before halting — proving the
  1117	    /// cancel flag short-circuits intake instead of draining to the end.
  1118	    struct ErrSink {
  1119	        root: PathBuf,
  1120	    }
  1121	
  1122	    #[async_trait::async_trait]
  1123	    impl TransferSink for ErrSink {
  1124	        async fn write_payload(&self, _payload: PreparedPayload) -> Result<SinkOutcome> {
  1125	            eyre::bail!("synthetic immediate failure")
  1126	        }
  1127	        fn root(&self) -> &Path {
  1128	            &self.root
  1129	        }
  1130	    }
  1131	
  1132	    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
  1133	    async fn forwarder_stops_promptly_on_worker_error() {
  1134	        let tmp = tempdir().unwrap();
  1135	        let src = tmp.path().join("src");
  1136	        std::fs::create_dir_all(&src).unwrap();
  1137	        let n = 200usize;
  1138	        for i in 0..n {
  1139	            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
  1140	        }
  1141	        let source = Arc::new(FsTransferSource::new(src.clone()));
  1142	        let unreadable = Arc::new(Mutex::new(Vec::new()));
  1143	        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
  1144	        let mut headers = Vec::new();
  1145	        while let Some(h) = header_rx.recv().await {
  1146	            headers.push(h);
  1147	        }
  1148	        let _ = scan_handle.await.unwrap().unwrap();
  1149	        assert_eq!(headers.len(), n);
  1150	
  1151	        let sink: Arc<dyn TransferSink> = Arc::new(ErrSink {
  1152	            root: PathBuf::from("/err"),
  1153	        });
  1154	
  1155	        // Count how many payloads the forwarder actually pulled from the
  1156	        // producer. With prefetch=1 and a single sink, the bounded queue
  1157	        // holds 1; once the sink errors and trips `cancelled`, the
  1158	        // forwarder must stop, so `sent` stays a tiny constant rather
  1159	        // than reaching n.
  1160	        let sent = Arc::new(AtomicU64::new(0));
  1161	        let (tx, rx) = mpsc::channel::<TransferPayload>(1);
  1162	        let sent_feeder = sent.clone();
  1163	        let feeder = tokio::spawn(async move {
  1164	            for h in headers {
  1165	                if tx.send(TransferPayload::File(h)).await.is_err() {
  1166	                    break;
  1167	                }
  1168	                sent_feeder.fetch_add(1, Ordering::Relaxed);
  1169	            }
  1170	        });
  1171	
  1172	        let result = execute_sink_pipeline_streaming(source, vec![sink], rx, 1, None).await;
  1173	        let _ = feeder.await;
  1174	
  1175	        assert!(result.is_err(), "pipeline must surface the sink error");
  1176	        let pulled = sent.load(Ordering::Relaxed);
  1177	        assert!(
  1178	            pulled < (n as u64) / 2,
  1179	            "forwarder should halt soon after the error, not drain all {n} payloads; pulled={pulled}"
  1180	        );
  1181	    }
  1182	

exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '1210,1336p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
  1210	
  1211	    /// REV4 ue-r2-1a (work-stealing as behaviour): byte and file totals
  1212	    /// stay correct when two sinks pull from the shared queue. Distinct
  1213	    /// per-file sizes mean any double-count or dropped payload shifts the
  1214	    /// totals, and the per-sink sum pins that every byte lands on exactly
  1215	    /// one sink.
  1216	    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  1217	    async fn byte_and_file_totals_correct_under_work_stealing() {
  1218	        let tmp = tempdir().unwrap();
  1219	        let src = tmp.path().join("src");
  1220	        std::fs::create_dir_all(&src).unwrap();
  1221	        let n = 30usize;
  1222	        let mut expected_bytes = 0u64;
  1223	        for i in 0..n {
  1224	            // Distinct sizes so a miscount (double-add / drop) is visible.
  1225	            let body = vec![b'x'; i + 1];
  1226	            expected_bytes += body.len() as u64;
  1227	            std::fs::write(src.join(format!("f{i}.dat")), &body).unwrap();
  1228	        }
  1229	        let source = Arc::new(FsTransferSource::new(src.clone()));
  1230	        let unreadable = Arc::new(Mutex::new(Vec::new()));
  1231	        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
  1232	        let mut headers = Vec::new();
  1233	        while let Some(h) = header_rx.recv().await {
  1234	            headers.push(h);
  1235	        }
  1236	        let _ = scan_handle.await.unwrap().unwrap();
  1237	        assert_eq!(headers.len(), n, "one header per file");
  1238	
  1239	        let bytes_a = Arc::new(AtomicU64::new(0));
  1240	        let bytes_b = Arc::new(AtomicU64::new(0));
  1241	        let a: Arc<dyn TransferSink> = Arc::new(ByteSink {
  1242	            bytes: Arc::clone(&bytes_a),
  1243	            root: PathBuf::from("/a"),
  1244	        });
  1245	        let b: Arc<dyn TransferSink> = Arc::new(ByteSink {
  1246	            bytes: Arc::clone(&bytes_b),
  1247	            root: PathBuf::from("/b"),
  1248	        });
  1249	
  1250	        let (tx, rx) = mpsc::channel::<TransferPayload>(4);
  1251	        let feeder = tokio::spawn(async move {
  1252	            for h in headers {
  1253	                if tx.send(TransferPayload::File(h)).await.is_err() {
  1254	                    break;
  1255	                }
  1256	            }
  1257	        });
  1258	        let outcome = execute_sink_pipeline_streaming(source, vec![a, b], rx, 2, None)
  1259	            .await
  1260	            .expect("pipeline ok");
  1261	        let _ = feeder.await;
  1262	
  1263	        assert_eq!(outcome.files_written, n, "file total");
  1264	        assert_eq!(outcome.bytes_written, expected_bytes, "byte total");
  1265	        assert_eq!(
  1266	            bytes_a.load(Ordering::Relaxed) + bytes_b.load(Ordering::Relaxed),
  1267	            expected_bytes,
  1268	            "every byte accounted to exactly one sink, none double-counted"
  1269	        );
  1270	    }
  1271	
  1272	    /// REV4 ue-r2-1a (cancellation): when the producer stops feeding and
  1273	    /// drops the channel mid-stream, the work-stealing executor winds
  1274	    /// down promptly — workers drain what was queued, run `finish`, and
  1275	    /// the call returns without hanging (the timeout is the no-hang
  1276	    /// assertion). Only the fed payloads complete; nothing past the
  1277	    /// cancellation point is invented.
  1278	    ///
  1279	    /// Hard-abort of in-flight workers on dropping the pipeline future
  1280	    /// itself is out of scope here: the workers are bare `tokio::spawn`
  1281	    /// (a `JoinHandle` drop does not abort the task), which is the
  1282	    /// AbortOnDrop family tracked under w4-1.
  1283	    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  1284	    async fn producer_cancel_winds_down_pipeline_promptly() {
  1285	        let tmp = tempdir().unwrap();
  1286	        let src = tmp.path().join("src");
  1287	        std::fs::create_dir_all(&src).unwrap();
  1288	        let n = 50usize;
  1289	        for i in 0..n {
  1290	            std::fs::write(src.join(format!("f{i}.txt")), b"x").unwrap();
  1291	        }
  1292	        let source = Arc::new(FsTransferSource::new(src.clone()));
  1293	        let unreadable = Arc::new(Mutex::new(Vec::new()));
  1294	        let (mut header_rx, scan_handle) = source.scan(None, unreadable);
  1295	        let mut headers = Vec::new();
  1296	        while let Some(h) = header_rx.recv().await {
  1297	            headers.push(h);
  1298	        }
  1299	        let _ = scan_handle.await.unwrap().unwrap();
  1300	        assert_eq!(headers.len(), n);
  1301	
  1302	        let count = Arc::new(AtomicU64::new(0));
  1303	        let sink: Arc<dyn TransferSink> = Arc::new(CountingSink {
  1304	            delay: Duration::ZERO,
  1305	            count: Arc::clone(&count),
  1306	            root: PathBuf::from("/c"),
  1307	        });
  1308	
  1309	        // Feed only the first 5 headers, then drop the sender to model a
  1310	        // cancelled / aborted producer.
  1311	        let (tx, rx) = mpsc::channel::<TransferPayload>(2);
  1312	        let feeder = tokio::spawn(async move {
  1313	            for h in headers.into_iter().take(5) {
  1314	                if tx.send(TransferPayload::File(h)).await.is_err() {
  1315	                    break;
  1316	                }
  1317	            }
  1318	            // `tx` dropped here → channel closes → pipeline must wind down.
  1319	        });
  1320	
  1321	        let outcome = tokio::time::timeout(
  1322	            Duration::from_secs(5),
  1323	            execute_sink_pipeline_streaming(source, vec![sink], rx, 2, None),
  1324	        )
  1325	        .await
  1326	        .expect("pipeline must wind down promptly after producer cancels, not hang")
  1327	        .expect("graceful shutdown is not an error");
  1328	        let _ = feeder.await;
  1329	
  1330	        assert_eq!(
  1331	            outcome.files_written, 5,
  1332	            "only the fed payloads are written"
  1333	        );
  1334	        assert_eq!(count.load(Ordering::Relaxed), 5);
  1335	    }
  1336	}

exec
/usr/bin/bash -lc "rg -n \"disconnect\\(|close\\(|Sender<|Receiver<\" ~/.cargo/registry/src -g 'lib.rs' -g '*.rs' | rg 'flume' | head -80" in /home/michael/dev/blit_v2
 succeeded in 3867ms:
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/async-task-4.7.1/src/runnable.rs:388:    ///     static QUEUE: (Sender<Runnable>, Receiver<Runnable>) = flume::unbounded();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/async-task-4.7.1/src/runnable.rs:592:///     static QUEUE: (Sender<Runnable>, Receiver<Runnable>) = flume::unbounded();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/async-task-4.7.1/src/runnable.rs:676:/// static QUEUE: Lazy<flume::Sender<Runnable>> = Lazy::new(|| {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/async-task-4.7.1/examples/spawn-local.rs:11:    static QUEUE: (flume::Sender<Runnable>, flume::Receiver<Runnable>) = flume::unbounded();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/async-task-4.7.1/examples/spawn.rs:18:    static QUEUE: Lazy<flume::Sender<Runnable>> = Lazy::new(|| {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/zero.rs:549:                    .downcast_mut::<Option<Receiver<T>>>()
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/stream.rs:33:fn stream_recv_disconnect() {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/stream.rs:85:fn r#stream_drop_send_disconnect() {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/basic.rs:410:fn weak_close() {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/select_macro.rs:466://     fn get() -> Sender<i32> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/select_macro.rs:481://     fn get() -> Receiver<i32> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/select_macro.rs:718://                                 .downcast_mut::<Option<Receiver<T>>>()
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/select_macro.rs:1151://     struct Sender<T>(cc::Sender<T>);
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/select_macro.rs:1152://     struct Receiver<T>(cc::Receiver<T>);
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/select_macro.rs:1154://     impl<T> Deref for Receiver<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/select_macro.rs:1155://         type Target = cc::Receiver<T>;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/select_macro.rs:1162://     impl<T> Deref for Sender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/select_macro.rs:1163://         type Target = cc::Sender<T>;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/async.rs:46:fn r#async_recv_disconnect() {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/async.rs:63:fn r#async_send_disconnect() {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/async.rs:260:    async fn producer(tx: flume::Sender<usize>) {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/async.rs:266:    async fn consumer(rx: flume::Receiver<usize>) {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/array.rs:223:fn send_after_disconnect() {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/array.rs:241:fn recv_after_disconnect() {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/array.rs:649:                    .downcast_mut::<Option<Receiver<T>>>()
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/select.rs:989://                                 .downcast_mut::<Option<Receiver<T>>>()
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/select.rs:118:        sender: &'a Sender<U>,
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/select.rs:123:            sender: &'a Sender<U>,
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/select.rs:222:        receiver: &'a Receiver<U>,
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/select.rs:226:            receiver: &'a Receiver<U>,
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/ready.rs:713://                                 .downcast_mut::<Option<Receiver<T>>>()
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:703:pub struct Sender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:707:impl<T> Sender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:795:    pub fn downgrade(&self) -> WeakSender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:802:    pub fn same_channel(&self, other: &Sender<T>) -> bool {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:807:impl<T> Clone for Sender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:816:impl<T> fmt::Debug for Sender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:822:impl<T> Drop for Sender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:839:pub struct WeakSender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:843:impl<T> WeakSender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:848:    pub fn upgrade(&self) -> Option<Sender<T>> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:870:impl<T> fmt::Debug for WeakSender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:876:impl<T> Clone for WeakSender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:888:pub struct Receiver<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:892:impl<T> Receiver<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:991:    pub fn same_channel(&self, other: &Receiver<T>) -> bool {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:996:impl<T> Clone for Receiver<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:1010:impl<T> fmt::Debug for Receiver<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:1016:impl<T> Drop for Receiver<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:1027:impl<'a, T> IntoIterator for &'a Receiver<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:1036:impl<T> IntoIterator for Receiver<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:1048:    receiver: &'a Receiver<T>,
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:1067:    receiver: &'a Receiver<T>,
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:1110:    receiver: Receiver<T>,
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:1141:pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs:1176:pub fn bounded<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:31:pub struct Sender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:32:    pub inner: cc::Sender<T>,
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:35:impl<T> Sender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:41:impl<T> Clone for Sender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:42:    fn clone(&self) -> Sender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:49:pub struct SyncSender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:50:    pub inner: cc::Sender<T>,
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:53:impl<T> SyncSender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:66:impl<T> Clone for SyncSender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:67:    fn clone(&self) -> SyncSender<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:74:pub struct Receiver<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:75:    pub inner: cc::Receiver<T>,
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:78:impl<T> Receiver<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:106:impl<'a, T> IntoIterator for &'a Receiver<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:115:impl<T> IntoIterator for Receiver<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:125:    inner: &'a Receiver<T>,
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:137:    inner: &'a Receiver<T>,
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:149:    inner: Receiver<T>,
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:160:pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:167:pub fn sync_channel<T>(bound: usize) -> (SyncSender<T>, Receiver<T>) {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:430:    fn oneshot_single_thread_send_port_close() {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:438:    fn oneshot_single_thread_recv_chan_close() {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:488:    fn oneshot_single_thread_peek_close() {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/mpsc.rs:513:    fn oneshot_multi_task_recv_then_close() {

exec
/usr/bin/bash -lc "sed -n '700,820p' /home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs" in /home/michael/dev/blit_v2
exec
/usr/bin/bash -lc "sed -n '410,455p' /home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/basic.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:
}

/// A transmitting end of a channel.
pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Sender<T> {
    /// Attempt to send a value into the channel. If the channel is bounded and full, or all
    /// receivers have been dropped, an error is returned. If the channel associated with this
    /// sender is unbounded, this method has the same behaviour as [`Sender::send`].
    pub fn try_send(&self, msg: T) -> Result<(), TrySendError<T>> {
        self.shared.send_sync(msg, None).map_err(|err| match err {
            TrySendTimeoutError::Full(msg) => TrySendError::Full(msg),
            TrySendTimeoutError::Disconnected(msg) => TrySendError::Disconnected(msg),
            _ => unreachable!(),
        })
    }

    /// Send a value into the channel, returning an error if all receivers have been dropped.
    /// If the channel is bounded and is full, this method will block until space is available
    /// or all receivers have been dropped. If the channel is unbounded, this method will not
    /// block.
    pub fn send(&self, msg: T) -> Result<(), SendError<T>> {
        self.shared.send_sync(msg, Some(None)).map_err(|err| match err {
            TrySendTimeoutError::Disconnected(msg) => SendError(msg),
            _ => unreachable!(),
        })
    }

    /// Send a value into the channel, returning an error if all receivers have been dropped
    /// or the deadline has passed. If the channel is bounded and is full, this method will
    /// block until space is available, the deadline is reached, or all receivers have been
    /// dropped.
    pub fn send_deadline(&self, msg: T, deadline: Instant) -> Result<(), SendTimeoutError<T>> {
        self.shared.send_sync(msg, Some(Some(deadline))).map_err(|err| match err {
            TrySendTimeoutError::Disconnected(msg) => SendTimeoutError::Disconnected(msg),
            TrySendTimeoutError::Timeout(msg) => SendTimeoutError::Timeout(msg),
            _ => unreachable!(),
        })
    }

    /// Send a value into the channel, returning an error if all receivers have been dropped
    /// or the timeout has expired. If the channel is bounded and is full, this method will
    /// block until space is available, the timeout has expired, or all receivers have been
    /// dropped.
    pub fn send_timeout(&self, msg: T, dur: Duration) -> Result<(), SendTimeoutError<T>> {
        self.send_deadline(msg, Instant::now().checked_add(dur).unwrap())
    }

    /// Returns true if all receivers for this channel have been dropped.
    pub fn is_disconnected(&self) -> bool {
        self.shared.is_disconnected()
    }

    /// Returns true if the channel is empty.
    /// Note: Zero-capacity channels are always empty.
    pub fn is_empty(&self) -> bool {
        self.shared.is_empty()
    }

    /// Returns true if the channel is full.
    /// Note: Zero-capacity channels are always full.
    pub fn is_full(&self) -> bool {
        self.shared.is_full()
    }

    /// Returns the number of messages in the channel
    pub fn len(&self) -> usize {
        self.shared.len()
    }

    /// If the channel is bounded, returns its capacity.
    pub fn capacity(&self) -> Option<usize> {
        self.shared.capacity()
    }

    /// Get the number of senders that currently exist, including this one.
    pub fn sender_count(&self) -> usize {
        self.shared.sender_count()
    }

    /// Get the number of receivers that currently exist.
    ///
    /// Note that this method makes no guarantees that a subsequent send will succeed; it's
    /// possible that between `receiver_count()` being called and a `send()`, all open receivers
    /// could drop.
    pub fn receiver_count(&self) -> usize {
        self.shared.receiver_count()
    }

    /// Creates a [`WeakSender`] that does not keep the channel open.
    ///
    /// The channel is closed once all `Sender`s are dropped, even if there
    /// are still active `WeakSender`s.
    pub fn downgrade(&self) -> WeakSender<T> {
        WeakSender {
            shared: Arc::downgrade(&self.shared),
        }
    }

    /// Returns whether the senders are belong to the same channel.
    pub fn same_channel(&self, other: &Sender<T>) -> bool {
        Arc::ptr_eq(&self.shared, &other.shared)
    }
}

impl<T> Clone for Sender<T> {
    /// Clone this sender. [`Sender`] acts as a handle to the ending a channel. Remaining channel
    /// contents will only be cleaned up when all senders and the receiver have been dropped.
    fn clone(&self) -> Self {
        self.shared.sender_count.fetch_add(1, Ordering::Relaxed);
        Self { shared: self.shared.clone() }
    }
}

impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Sender").finish()
    }
}

 succeeded in 0ms:
fn weak_close() {
    let (tx, rx) = unbounded::<()>();
    let weak = tx.downgrade();
    drop(tx);
    assert!(weak.upgrade().is_none());
    assert!(rx.is_disconnected());
    assert!(rx.try_recv().is_err());
}

#[test]
fn weak_upgrade() {
    let (tx, rx) = unbounded();
    let weak = tx.downgrade();
    let tx2 = weak.upgrade().unwrap();
    drop(tx);
    assert!(!rx.is_disconnected());
    tx2.send(()).unwrap();
    assert!(rx.try_recv().is_ok());
}

exec
/usr/bin/bash -lc "rg -n \"pub fn (disconnect|close|is_disconnected|is_empty|len|capacity)|disconnect_all|shared\\.disconnect\" /home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:
312:    pub fn is_empty(&self) -> bool {
656:    fn disconnect_all(&self) {
751:    pub fn is_disconnected(&self) -> bool {
757:    pub fn is_empty(&self) -> bool {
768:    pub fn len(&self) -> usize {
773:    pub fn capacity(&self) -> Option<usize> {
826:            self.shared.disconnect_all();
954:    pub fn is_disconnected(&self) -> bool {
960:    pub fn is_empty(&self) -> bool {
971:    pub fn len(&self) -> usize {
976:    pub fn capacity(&self) -> Option<usize> {
1021:            self.shared.disconnect_all();

exec
/usr/bin/bash -lc "sed -n '50,75p' /home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/tests/async.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:
        std::thread::sleep(std::time::Duration::from_millis(250));
        drop(tx)
    });

    async_std::task::block_on(async {
        assert_eq!(rx.recv_async().await, Err(RecvError::Disconnected));
    });

    t.join().unwrap();
}

#[cfg(all(feature = "async", not(target_os = "unknown")))]
#[test]
fn r#async_send_disconnect() {
    let (tx, rx) = bounded(0);

    let t = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        drop(rx)
    });

    async_std::task::block_on(async {
        assert_eq!(tx.send_async(42u32).await, Err(SendError(42)));
    });

    t.join().unwrap();

exec
/usr/bin/bash -lc "sed -n '1008,1028p' /home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:
}

impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Receiver").finish()
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        // Notify senders that all receivers have been dropped if the number of receivers drops
        // to 0.
        if self.shared.receiver_count.fetch_sub(1, Ordering::Relaxed) == 1 {
            self.shared.disconnect_all();
        }
    }
}

/// This exists as a shorthand for [`Receiver::iter`].
impl<'a, T> IntoIterator for &'a Receiver<T> {
    type Item = T;

exec
/usr/bin/bash -lc "sed -n '815,835p' /home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:

impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Sender").finish()
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        // Notify receivers that all senders have been dropped if the number of senders drops to 0.
        if self.shared.sender_count.fetch_sub(1, Ordering::Relaxed) == 1 {
            self.shared.disconnect_all();
        }
    }
}

/// A sender that does not prevent the channel from being closed.
///
/// Weak senders do not count towards the number of active senders on the channel. As soon as
/// all normal [`Sender`]s are dropped, the channel is closed, even if there is still a
/// `WeakSender`.

exec
/usr/bin/bash -lc "sed -n '620,665p' /home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flume-0.11.1/src/lib.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:
            block.is_some(),
            // make_signal
            || Hook::slot(None, SyncSignal::default()),
            // do_block
            |hook| if let Some(deadline) = block.unwrap() {
                hook.wait_deadline_recv(&self.disconnected, deadline)
                    .or_else(|timed_out| {
                        if timed_out { // Remove our signal
                            let hook: Arc<Hook<T, dyn Signal>> = hook.clone();
                            wait_lock(&self.chan).waiting
                                .retain(|s| s.signal().as_ptr() != hook.signal().as_ptr());
                        }
                        match hook.try_take() {
                            Some(msg) => Ok(msg),
                            None => {
                                let disconnected = self.is_disconnected(); // Check disconnect *before* msg
                                if let Some(msg) = wait_lock(&self.chan).queue.pop_front() {
                                    Ok(msg)
                                } else if disconnected {
                                    Err(TryRecvTimeoutError::Disconnected)
                                } else {
                                    Err(TryRecvTimeoutError::Timeout)
                                }
                            },
                        }
                    })
            } else {
                hook.wait_recv(&self.disconnected)
                    .or_else(|| wait_lock(&self.chan).queue.pop_front())
                    .ok_or(TryRecvTimeoutError::Disconnected)
            },
        )
    }

    /// Disconnect anything listening on this channel (this will not prevent receivers receiving
    /// msgs that have already been sent)
    fn disconnect_all(&self) {
        self.disconnected.store(true, Ordering::Relaxed);

        let mut chan = wait_lock(&self.chan);
        chan.pull_pending(false);
        if let Some((_, sending)) = chan.sending.as_ref() {
            sending.iter().for_each(|hook| {
                hook.signal().fire();
            })
        }

codex
The `flume` sender only wakes blocked sends when receivers disconnect; setting the atomic flag alone does not wake an already-blocked send. That is looking like a real cancellation regression in the PR2 fix path.
codex
I’m also checking the one-shot wrapper and producer side so the finding does not overstate it: producer back-pressure is preserved, but first-error return can still wait on queued/survivor work because workers never observe cancellation before taking more items.
exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '19,60p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
    19	/// Execute a transfer pipeline with all payloads known upfront.
    20	///
    21	/// This is a convenience wrapper around [`execute_sink_pipeline_streaming`]
    22	/// that spawns a task to send every payload into the channel and then drops
    23	/// the sender, signalling end-of-stream.
    24	pub async fn execute_sink_pipeline(
    25	    source: Arc<dyn TransferSource>,
    26	    sinks: Vec<Arc<dyn TransferSink>>,
    27	    payloads: Vec<TransferPayload>,
    28	    prefetch: usize,
    29	    progress: Option<&RemoteTransferProgress>,
    30	) -> Result<SinkOutcome> {
    31	    if sinks.is_empty() {
    32	        return Ok(SinkOutcome::default());
    33	    }
    34	    if payloads.is_empty() {
    35	        for sink in &sinks {
    36	            sink.finish().await?;
    37	        }
    38	        return Ok(SinkOutcome::default());
    39	    }
    40	
    41	    let capacity = prefetch.max(1);
    42	    let (tx, rx) = mpsc::channel::<TransferPayload>(capacity);
    43	
    44	    // Feed payloads in a background task so the pipeline can start writing
    45	    // before the whole vec is queued (the channel provides back-pressure).
    46	    let feeder = tokio::spawn(async move {
    47	        for payload in payloads {
    48	            if tx.send(payload).await.is_err() {
    49	                break;
    50	            }
    51	        }
    52	        // Dropping tx closes the channel and signals end-of-stream.
    53	    });
    54	
    55	    let result = execute_sink_pipeline_streaming(source, sinks, rx, prefetch, progress).await;
    56	    let _ = feeder.await;
    57	    result
    58	}
    59	
    60	/// Execute a transfer pipeline with payloads arriving on a channel.

exec
/usr/bin/bash -lc "git show 515fb76:crates/blit-core/src/remote/transfer/pipeline.rs | nl -ba | sed -n '19,170p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
    19	/// Execute a transfer pipeline with all payloads known upfront.
    20	///
    21	/// This is a convenience wrapper around [`execute_sink_pipeline_streaming`]
    22	/// that spawns a task to send every payload into the channel and then drops
    23	/// the sender, signalling end-of-stream.
    24	pub async fn execute_sink_pipeline(
    25	    source: Arc<dyn TransferSource>,
    26	    sinks: Vec<Arc<dyn TransferSink>>,
    27	    payloads: Vec<TransferPayload>,
    28	    prefetch: usize,
    29	    progress: Option<&RemoteTransferProgress>,
    30	) -> Result<SinkOutcome> {
    31	    if sinks.is_empty() {
    32	        return Ok(SinkOutcome::default());
    33	    }
    34	    if payloads.is_empty() {
    35	        for sink in &sinks {
    36	            sink.finish().await?;
    37	        }
    38	        return Ok(SinkOutcome::default());
    39	    }
    40	
    41	    let capacity = prefetch.max(1);
    42	    let (tx, rx) = mpsc::channel::<TransferPayload>(capacity);
    43	
    44	    // Feed payloads in a background task so the pipeline can start writing
    45	    // before the whole vec is queued (the channel provides back-pressure).
    46	    let feeder = tokio::spawn(async move {
    47	        for payload in payloads {
    48	            if tx.send(payload).await.is_err() {
    49	                break;
    50	            }
    51	        }
    52	        // Dropping tx closes the channel and signals end-of-stream.
    53	    });
    54	
    55	    let result = execute_sink_pipeline_streaming(source, sinks, rx, prefetch, progress).await;
    56	    let _ = feeder.await;
    57	    result
    58	}
    59	
    60	/// Execute a transfer pipeline with payloads arriving on a channel.
    61	///
    62	/// Distributes payloads round-robin across `sinks` as they arrive. Each sink
    63	/// runs as a separate tokio task: it reads payloads from its dedicated queue,
    64	/// prepares them via `source.prepare_payload()`, writes them via
    65	/// `sink.write_payload()`, and finally calls `sink.finish()`. Errors from any
    66	/// worker propagate up.
    67	///
    68	/// `prefetch` controls the per-sink channel capacity — effectively the
    69	/// preparation-in-flight limit per sink.
    70	pub async fn execute_sink_pipeline_streaming(
    71	    source: Arc<dyn TransferSource>,
    72	    sinks: Vec<Arc<dyn TransferSink>>,
    73	    mut payload_rx: mpsc::Receiver<TransferPayload>,
    74	    prefetch: usize,
    75	    progress: Option<&RemoteTransferProgress>,
    76	) -> Result<SinkOutcome> {
    77	    if sinks.is_empty() {
    78	        // Drain incoming channel so the producer isn't left dangling.
    79	        while payload_rx.recv().await.is_some() {}
    80	        return Ok(SinkOutcome::default());
    81	    }
    82	
    83	    let sink_count = sinks.len();
    84	    let per_sink_capacity = prefetch.max(1);
    85	    let total = Arc::new(std::sync::Mutex::new(SinkOutcome::default()));
    86	
    87	    // Per-sink payload channels; dispatcher forwards round-robin to these.
    88	    let mut sink_senders: Vec<mpsc::Sender<TransferPayload>> = Vec::with_capacity(sink_count);
    89	    let mut sink_handles: Vec<tokio::task::JoinHandle<Result<()>>> = Vec::with_capacity(sink_count);
    90	
    91	    for sink in sinks {
    92	        let (tx, mut rx) = mpsc::channel::<TransferPayload>(per_sink_capacity);
    93	        sink_senders.push(tx);
    94	        let source_clone = source.clone();
    95	        let progress_clone = progress.cloned();
    96	        let total_clone = total.clone();
    97	        sink_handles.push(tokio::spawn(async move {
    98	            while let Some(payload) = rx.recv().await {
    99	                let prepared = source_clone
   100	                    .prepare_payload(payload)
   101	                    .await
   102	                    .context("preparing payload")?;
   103	                let files: Vec<(String, u64)> = match &prepared {
   104	                    PreparedPayload::File(h) => vec![(h.relative_path.clone(), h.size)],
   105	                    PreparedPayload::TarShard { headers, .. } => headers
   106	                        .iter()
   107	                        .map(|h| (h.relative_path.clone(), h.size))
   108	                        .collect(),
   109	                    // Resume-block payloads patch existing files; no
   110	                    // file-completion event from one-block-at-a-time.
   111	                    PreparedPayload::FileBlock { .. }
   112	                    | PreparedPayload::FileBlockComplete { .. } => Vec::new(),
   113	                };
   114	                let outcome = sink
   115	                    .write_payload(prepared)
   116	                    .await
   117	                    .context("writing payload")?;
   118	                if let Some(p) = &progress_clone {
   119	                    for (name, size) in &files {
   120	                        p.report_file_complete(name.clone(), *size);
   121	                    }
   122	                }
   123	                let mut t = total_clone.lock().unwrap();
   124	                t.merge(&outcome);
   125	            }
   126	            sink.finish().await?;
   127	            Ok::<(), eyre::Report>(())
   128	        }));
   129	    }
   130	
   131	    // Dispatcher: pull from the incoming channel, round-robin to sinks.
   132	    // Uses async send (which applies backpressure) — if one sink is slower,
   133	    // the dispatcher naturally blocks on that sink until it drains.
   134	    let dispatcher = tokio::spawn(async move {
   135	        let mut next = 0usize;
   136	        while let Some(payload) = payload_rx.recv().await {
   137	            let idx = next % sink_count;
   138	            next = next.wrapping_add(1);
   139	            if sink_senders[idx].send(payload).await.is_err() {
   140	                // Sink worker dropped its receiver — treat as shutdown.
   141	                return;
   142	            }
   143	        }
   144	        // Drop senders so sink workers see end-of-stream and finish().
   145	        drop(sink_senders);
   146	    });
   147	
   148	    // Wait for all sinks to finish and aggregate errors.
   149	    let mut first_err: Option<eyre::Report> = None;
   150	    for h in sink_handles {
   151	        match h.await {
   152	            Ok(Ok(())) => {}
   153	            Ok(Err(e)) if first_err.is_none() => first_err = Some(e),
   154	            Ok(Err(_)) => {}
   155	            Err(join) if first_err.is_none() => {
   156	                first_err = Some(eyre::eyre!("sink worker panicked: {}", join));
   157	            }
   158	            Err(_) => {}
   159	        }
   160	    }
   161	    let _ = dispatcher.await;
   162	
   163	    if let Some(err) = first_err {
   164	        return Err(err);
   165	    }
   166	
   167	    let result = total.lock().unwrap().clone();
   168	    Ok(result)
   169	}
   170	

exec
/usr/bin/bash -lc 'git show 515fb76:crates/blit-core/src/remote/transfer/data_plane.rs | rg -n "StallGuardWriter|stream:|from_stream|write_all|flush"' in /home/michael/dev/blit_v2
 succeeded in 0ms:
11:use super::stall_guard::{StallGuardWriter, TRANSFER_STALL_TIMEOUT};
23:    // audit-h3b: writes go through StallGuardWriter so a stalled
27:    // (15+ minutes). All existing `self.stream.write_all/.flush`
29:    // AsyncWrite impl of StallGuardWriter, so no per-site change
31:    stream: StallGuardWriter<TcpStream>,
50:    /// audit-h3b: the stream is wrapped in [`StallGuardWriter`] so a
58:    pub async fn from_stream(
59:        stream: TcpStream,
68:            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
123:        let std_stream: std::net::TcpStream = socket.into();
128:            .write_all(token)
132:        Ok(Self::from_stream(stream, trace, chunk_bytes, payload_prefetch, pool).await)
186:            .write_all(&[DATA_PLANE_RECORD_END])
190:            .flush()
192:            .context("flushing data plane stream")
232:            .write_all(&[DATA_PLANE_RECORD_FILE])
236:            .write_all(&(path_bytes.len() as u32).to_be_bytes())
240:            .write_all(path_bytes)
245:            .write_all(&header.size.to_be_bytes())
253:            .write_all(&header.mtime_seconds.to_be_bytes())
257:            .write_all(&header.permissions.to_be_bytes())
315:                self.stream.write_all(&buf_a.as_slice()[..bytes_a]),
345:                .write_all(&buf_a.as_slice()[..bytes_a])
373:            .write_all(&[DATA_PLANE_RECORD_TAR_SHARD])
377:            .write_all(&(headers.len() as u32).to_be_bytes())
390:                .write_all(&(rel_bytes.len() as u32).to_be_bytes())
394:                .write_all(rel_bytes)
398:                .write_all(&header.size.to_be_bytes())
402:                .write_all(&header.mtime_seconds.to_be_bytes())
406:                .write_all(&header.permissions.to_be_bytes())
412:            .write_all(&(data.len() as u64).to_be_bytes())
417:                .write_all(chunk)
454:            .write_all(&[DATA_PLANE_RECORD_BLOCK])
458:            .write_all(&(path_bytes.len() as u32).to_be_bytes())
462:            .write_all(path_bytes)
466:            .write_all(&offset.to_be_bytes())
470:            .write_all(&(content.len() as u32).to_be_bytes())
474:            .write_all(content)
509:            .write_all(&[DATA_PLANE_RECORD_BLOCK_COMPLETE])
513:            .write_all(&(path_bytes.len() as u32).to_be_bytes())
517:            .write_all(path_bytes)
521:            .write_all(&total_size.to_be_bytes())
525:            .write_all(&mtime_seconds.to_be_bytes())
529:            .write_all(&permissions.to_be_bytes())
598:            dst.write_all(&buf_a[..bytes_a]),
603:        // AFTER `write_all` succeeds so a `bytes_completed`
624:        dst.write_all(&buf_a[..bytes_a])
796:        let mut session = DataPlaneSession::from_stream(client, false, 64 * 1024, 1, pool).await;

exec
/usr/bin/bash -lc "rg -n \"sender_count|receiver_count|is_disconnected|cancelled|send_async|recv_async|finish\\(\\)\" crates/blit-core/src/remote/transfer/pipeline.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:
36:            sink.finish().await?;
65:/// `recv_async().await`, so a slow sink can never head-of-line-block the
70:/// drains, at which point it calls `sink.finish()`. Errors from any
102:    // as any other worker is alive `send_async` keeps succeeding, so the
108:    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
115:        let cancelled_worker = cancelled.clone();
120:                while let Ok(payload) = work_rx.recv_async().await {
148:                sink.finish().await?;
155:                cancelled_worker.store(true, std::sync::atomic::Ordering::Relaxed);
163:    // this, `recv_async` would never see `Disconnected`.
167:    // work queue. `send_async` applies back-pressure (bounded queue); if
169:    // and we stop. It also bails as soon as a worker sets `cancelled`, so
173:    let cancelled_fwd = cancelled.clone();
176:            if cancelled_fwd.load(std::sync::atomic::Ordering::Relaxed) {
182:            if work_tx.send_async(payload).await.is_err() {
189:        // after draining and run finish().
345:    sink.finish().await.context("finalising sink")?;
1157:        // holds 1; once the sink errors and trips `cancelled`, the
1310:        // cancelled / aborted producer.

exec
/usr/bin/bash -lc "rg -n \"while let Ok\\(payload\\)|cancelled_fwd\\.load|send_async\\(payload\\)|sink\\.finish\\(\\)\\.await|first_err|for h in sink_handles\" crates/blit-core/src/remote/transfer/pipeline.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:
36:            sink.finish().await?;
120:                while let Ok(payload) = work_rx.recv_async().await {
148:                sink.finish().await?;
176:            if cancelled_fwd.load(std::sync::atomic::Ordering::Relaxed) {
182:            if work_tx.send_async(payload).await.is_err() {
194:    let mut first_err: Option<eyre::Report> = None;
195:    for h in sink_handles {
198:            Ok(Err(e)) if first_err.is_none() => first_err = Some(e),
200:            Err(join) if first_err.is_none() => {
201:                first_err = Some(eyre::eyre!("sink worker panicked: {}", join));
208:    if let Some(err) = first_err {
345:    sink.finish().await.context("finalising sink")?;

exec
/usr/bin/bash -lc "rg -n \"byte_and_file_totals_correct_under_work_stealing|struct ByteSink|bytes_a\\.load|files_written, n|expected_bytes|root: PathBuf::from\\(\\\"/b\\\"\" crates/blit-core/src/remote/transfer/pipeline.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:
1096:        assert_eq!(outcome.files_written, n, "every payload written once");
1185:    struct ByteSink {
1217:    async fn byte_and_file_totals_correct_under_work_stealing() {
1222:        let mut expected_bytes = 0u64;
1226:            expected_bytes += body.len() as u64;
1247:            root: PathBuf::from("/b"),
1263:        assert_eq!(outcome.files_written, n, "file total");
1264:        assert_eq!(outcome.bytes_written, expected_bytes, "byte total");
1266:            bytes_a.load(Ordering::Relaxed) + bytes_b.load(Ordering::Relaxed),
1267:            expected_bytes,

exec
/usr/bin/bash -lc "rg -n \"producer_cancel_winds_down_pipeline_promptly|timeout\\(|take\\(5\\)|vec"'!'"\\[sink\\]\" crates/blit-core/src/remote/transfer/pipeline.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:
535:        let outcome = execute_sink_pipeline(source, vec![sink], planned.payloads, 4, None)
593:        let outcome = execute_sink_pipeline_streaming(source, vec![sink], rx, 2, None)
1172:        let result = execute_sink_pipeline_streaming(source, vec![sink], rx, 1, None).await;
1284:    async fn producer_cancel_winds_down_pipeline_promptly() {
1313:            for h in headers.into_iter().take(5) {
1321:        let outcome = tokio::time::timeout(
1323:            execute_sink_pipeline_streaming(source, vec![sink], rx, 2, None),

exec
/usr/bin/bash -lc "rg -n \"from_stream_with_probe|StallGuardWriter::new|self\\.probe\\.record_bytes|DATA_PLANE_RECORD_BLOCK|writing block content\" crates/blit-core/src/remote/transfer/data_plane.rs" in /home/michael/dev/blit_v2
 succeeded in 0ms:
19:pub const DATA_PLANE_RECORD_BLOCK: u8 = 2;
20:pub const DATA_PLANE_RECORD_BLOCK_COMPLETE: u8 = 3;
32:/// [`from_stream_with_probe`](DataPlaneSession::from_stream_with_probe).
64:    /// [`StallGuardWriter`] (inside `from_stream_with_probe`) so a
77:        Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, NoProbe)
145:    pub async fn from_stream_with_probe(
156:            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
365:            self.probe.record_bytes(bytes_a as u64);
406:            self.probe.record_bytes(bytes_a as u64);
479:            self.probe.record_bytes(chunk.len() as u64);
514:            .write_all(&[DATA_PLANE_RECORD_BLOCK])
536:            .context("writing block content")?;
569:            .write_all(&[DATA_PLANE_RECORD_BLOCK_COMPLETE])

codex
I found one likely production issue in the PR2 cancellation path and a couple of test/telemetry gaps. I’m doing a quick pass over `StallGuardWriter` and the raw stream access now so I don’t miss a merge artifact around the guarded stream.
exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/transfer/stall_guard.rs | sed -n '1,220p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
     1	//! audit-1c / audit-h3: a `StallGuard<R>` `AsyncRead` adapter that turns
     2	//! a *stalled* transfer — no bytes received for `timeout` — into a clean
     3	//! `io::ErrorKind::TimedOut`, while leaving a steadily-progressing
     4	//! transfer untouched.
     5	//!
     6	//! Why an `AsyncRead` adapter and not a `tokio::time::timeout` around the
     7	//! receive call: the receive pipeline reads each wire frame through many
     8	//! separate socket awaits (record tag, file header, length-prefixed
     9	//! fields, file-data streaming, tar shards). A stall can happen at *any*
    10	//! of them, mid-frame. Sitting at the `AsyncRead` layer catches a stall
    11	//! at every read without touching the parsing logic, and — crucially —
    12	//! it is an **idle** timeout (re-armed on every read that makes progress)
    13	//! NOT a total-duration deadline, so a legitimate large transfer that
    14	//! keeps making progress is never aborted. (Owner decision, memory
    15	//! `audit-owner-decisions`: no-bytes-for-30s.)
    16	//!
    17	//! Scope:
    18	//! - audit-1c shipped [`StallGuard`] on the CLI pull-receive TCP path
    19	//!   (the original AsyncRead idle adapter).
    20	//! - audit-h3a extended [`StallGuard`] to the daemon push-receive socket
    21	//!   — another receive path.
    22	//! - audit-h3b adds [`StallGuardWriter`] (this slice), an AsyncWrite
    23	//!   adapter mirroring [`StallGuard`] for **write** progress. The
    24	//!   daemon-side pull data plane is a SENDER (daemon writes bytes to
    25	//!   the puller), so the stall surface is a slow / wedged reader
    26	//!   causing TCP write backpressure on the daemon. `StallGuardWriter`
    27	//!   trips after `TRANSFER_STALL_TIMEOUT` of no successful write
    28	//!   progress, with the same idle-vs-total-deadline semantics as the
    29	//!   read side. The earlier R2/R3 wording for h3b ("daemon pull-data-
    30	//!   plane accepts") was imprecise — the accept + token phases are
    31	//!   already bounded by `PULL_ACCEPT_TIMEOUT` / `PULL_TOKEN_TIMEOUT`;
    32	//!   the missing guard is daemon pull-data-plane **write progress
    33	//!   after token acceptance**, addressed here by wiring this writer
    34	//!   inside `DataPlaneSession`.
    35	//! - audit-h3c is the gRPC-fallback class, re-scoped 2026-06-05 to a
    36	//!   two-slice contract because message-granular timeouts can't be
    37	//!   reused from `StallGuard`'s byte-level model. **Slice 1 shipped**
    38	//!   (structural frame cap + unified receive helper at
    39	//!   `crates/blit-core/src/remote/transfer/grpc_fallback.rs`); **slice
    40	//!   2 pending** (dynamic progress watchdog + retryable `TimedOut`
    41	//!   error). See that module for details.
    42	
    43	use std::future::Future;
    44	use std::io;
    45	use std::pin::Pin;
    46	use std::task::{Context, Poll};
    47	use std::time::Duration;
    48	
    49	use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    50	use tokio::time::{Instant, Sleep};
    51	
    52	/// Idle/stall timeout applied to every data-plane transfer path: if no
    53	/// data-plane progress (read or write) is observable for this long, the
    54	/// transfer is aborted with `TimedOut` rather than pinning resources
    55	/// forever. Owner-decided 30s.
    56	///
    57	/// Applied by:
    58	/// - CLI pull-receive TCP (`remote::pull` — audit-1c) via [`StallGuard`].
    59	/// - Daemon push-receive TCP (`daemon::service::push::data_plane`
    60	///   — audit-h3a) via [`StallGuard`].
    61	/// - Daemon pull-data-plane **write progress after token acceptance**
    62	///   (`daemon::service::{pull, pull_sync}` — audit-h3b) via
    63	///   [`StallGuardWriter`] inside `DataPlaneSession`. The accept + token
    64	///   phases on those paths are separately bounded by
    65	///   `PULL_ACCEPT_TIMEOUT` / `PULL_TOKEN_TIMEOUT`.
    66	///
    67	/// The gRPC-fallback paths sit below `tonic::Streaming<T>` rather than
    68	/// `AsyncRead` / `AsyncWrite` and are covered separately (audit-h3c).
    69	pub const TRANSFER_STALL_TIMEOUT: Duration = Duration::from_secs(30);
    70	
    71	/// Wraps an `AsyncRead` so a read that makes no progress within `timeout`
    72	/// resolves to `io::ErrorKind::TimedOut`. The deadline is re-armed on
    73	/// every read that returns (data or clean EOF), so it is a per-gap idle
    74	/// timeout, not a cap on the whole transfer.
    75	pub struct StallGuard<R> {
    76	    inner: R,
    77	    timeout: Duration,
    78	    deadline: Pin<Box<Sleep>>,
    79	}
    80	
    81	impl<R> StallGuard<R> {
    82	    pub fn new(inner: R, timeout: Duration) -> Self {
    83	        Self {
    84	            inner,
    85	            timeout,
    86	            deadline: Box::pin(tokio::time::sleep(timeout)),
    87	        }
    88	    }
    89	}
    90	
    91	impl<R: AsyncRead + Unpin> AsyncRead for StallGuard<R> {
    92	    fn poll_read(
    93	        self: Pin<&mut Self>,
    94	        cx: &mut Context<'_>,
    95	        buf: &mut ReadBuf<'_>,
    96	    ) -> Poll<io::Result<()>> {
    97	        let this = self.get_mut();
    98	        match Pin::new(&mut this.inner).poll_read(cx, buf) {
    99	            Poll::Ready(Ok(())) => {
   100	                // The read completed (delivered bytes, or a clean EOF) —
   101	                // that's progress, so re-arm the idle deadline.
   102	                this.deadline.as_mut().reset(Instant::now() + this.timeout);
   103	                Poll::Ready(Ok(()))
   104	            }
   105	            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
   106	            Poll::Pending => {
   107	                // No data available yet. Trip only if the whole idle
   108	                // window has elapsed since the last progress; otherwise
   109	                // stay pending (the deadline poll registers our waker).
   110	                match this.deadline.as_mut().poll(cx) {
   111	                    Poll::Ready(()) => Poll::Ready(Err(io::Error::new(
   112	                        io::ErrorKind::TimedOut,
   113	                        format!("transfer stalled: no data received for {:?}", this.timeout),
   114	                    ))),
   115	                    Poll::Pending => Poll::Pending,
   116	                }
   117	            }
   118	        }
   119	    }
   120	}
   121	
   122	/// Wraps an `AsyncWrite` so a write that makes no progress within `timeout`
   123	/// resolves to `io::ErrorKind::TimedOut`. The deadline is re-armed on every
   124	/// successful `poll_write` (any byte count > 0 counts as progress), so it
   125	/// is a per-gap idle timeout, not a cap on the whole transfer.
   126	///
   127	/// audit-h3b: the daemon-side pull data plane writes bytes to the puller.
   128	/// If the puller stops reading mid-stream, TCP flow control fills the
   129	/// kernel send buffer and `write_all` blocks indefinitely (until OS-level
   130	/// TCP retransmit exhaustion, often 15+ minutes). Wrapping the inner
   131	/// stream in this adapter turns that into a clean
   132	/// `io::ErrorKind::TimedOut` after `TRANSFER_STALL_TIMEOUT` of no
   133	/// observable write progress.
   134	///
   135	/// Symmetric in spirit with [`StallGuard`] on the read side: same idle-
   136	/// timeout semantics, same load-bearing property that a steadily-
   137	/// progressing transfer (any non-trivial network at all) is never
   138	/// aborted.
   139	pub struct StallGuardWriter<W> {
   140	    inner: W,
   141	    timeout: Duration,
   142	    deadline: Pin<Box<Sleep>>,
   143	}
   144	
   145	impl<W> StallGuardWriter<W> {
   146	    pub fn new(inner: W, timeout: Duration) -> Self {
   147	        Self {
   148	            inner,
   149	            timeout,
   150	            deadline: Box::pin(tokio::time::sleep(timeout)),
   151	        }
   152	    }
   153	
   154	    pub fn into_inner(self) -> W {
   155	        self.inner
   156	    }
   157	}
   158	
   159	impl<W: AsyncWrite + Unpin> AsyncWrite for StallGuardWriter<W> {
   160	    fn poll_write(
   161	        self: Pin<&mut Self>,
   162	        cx: &mut Context<'_>,
   163	        buf: &[u8],
   164	    ) -> Poll<io::Result<usize>> {
   165	        let this = self.get_mut();
   166	        match Pin::new(&mut this.inner).poll_write(cx, buf) {
   167	            Poll::Ready(Ok(0)) => {
   168	                // Per the doc contract above, "no progress" means zero
   169	                // bytes accepted. A 0-byte poll_write does NOT reset
   170	                // the deadline — otherwise a peer that accepts zero
   171	                // bytes per poll would never trip the guard. The
   172	                // caller (write_all loop) will keep polling; if real
   173	                // progress doesn't show up within the window the
   174	                // Pending arm below trips. (h3b round 2: GPT review
   175	                // flagged Ok(0) as a doc/code mismatch.)
   176	                Poll::Ready(Ok(0))
   177	            }
   178	            Poll::Ready(Ok(n)) => {
   179	                // n > 0: real progress. Reset the idle deadline so a
   180	                // steadily-progressing transfer is never aborted.
   181	                this.deadline.as_mut().reset(Instant::now() + this.timeout);
   182	                Poll::Ready(Ok(n))
   183	            }
   184	            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
   185	            Poll::Pending => {
   186	                // The inner stream is back-pressuring (kernel send
   187	                // buffer full → no kernel space available). Trip only
   188	                // if the whole idle window has elapsed since the last
   189	                // progress; otherwise stay pending (the deadline poll
   190	                // registers our waker).
   191	                match this.deadline.as_mut().poll(cx) {
   192	                    Poll::Ready(()) => Poll::Ready(Err(io::Error::new(
   193	                        io::ErrorKind::TimedOut,
   194	                        format!("transfer stalled: no write progress for {:?}", this.timeout),
   195	                    ))),
   196	                    Poll::Pending => Poll::Pending,
   197	                }
   198	            }
   199	        }
   200	    }
   201	
   202	    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
   203	        // Flush is a no-op for most AsyncWrite impls; we don't gate it
   204	        // on the deadline because a stuck flush manifests as a stuck
   205	        // poll_write upstream, which IS gated. Pass through cleanly.
   206	        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
   207	    }
   208	
   209	    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
   210	        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
   211	    }
   212	}
   213	
   214	#[cfg(test)]
   215	mod tests {
   216	    use super::*;
   217	    use tokio::io::{AsyncReadExt, AsyncWriteExt};
   218	
   219	    #[tokio::test]
   220	    async fn times_out_when_reader_stalls() {

exec
/usr/bin/bash -lc "git diff 515fb76..HEAD -- Cargo.toml crates/blit-core/Cargo.toml Cargo.lock | sed -n '1,180p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
exec
/usr/bin/bash -lc "rg -n \"struct StallGuardWriter|impl.*StallGuardWriter|Deref|get_ref|get_mut|into_inner|AsyncWrite\" crates/blit-core/src/remote/transfer/stall_guard.rs crates/blit-core/src/remote/transfer -g '*.rs'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
crates/blit-core/src/remote/transfer/stall_guard.rs:22://! - audit-h3b adds [`StallGuardWriter`] (this slice), an AsyncWrite
crates/blit-core/src/remote/transfer/stall_guard.rs:49:use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
crates/blit-core/src/remote/transfer/stall_guard.rs:68:/// `AsyncRead` / `AsyncWrite` and are covered separately (audit-h3c).
crates/blit-core/src/remote/transfer/stall_guard.rs:97:        let this = self.get_mut();
crates/blit-core/src/remote/transfer/stall_guard.rs:122:/// Wraps an `AsyncWrite` so a write that makes no progress within `timeout`
crates/blit-core/src/remote/transfer/stall_guard.rs:139:pub struct StallGuardWriter<W> {
crates/blit-core/src/remote/transfer/stall_guard.rs:145:impl<W> StallGuardWriter<W> {
crates/blit-core/src/remote/transfer/stall_guard.rs:154:    pub fn into_inner(self) -> W {
crates/blit-core/src/remote/transfer/stall_guard.rs:159:impl<W: AsyncWrite + Unpin> AsyncWrite for StallGuardWriter<W> {
crates/blit-core/src/remote/transfer/stall_guard.rs:165:        let this = self.get_mut();
crates/blit-core/src/remote/transfer/stall_guard.rs:203:        // Flush is a no-op for most AsyncWrite impls; we don't gate it
crates/blit-core/src/remote/transfer/stall_guard.rs:206:        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
crates/blit-core/src/remote/transfer/stall_guard.rs:210:        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
crates/blit-core/src/remote/transfer/stall_guard.rs:217:    use tokio::io::{AsyncReadExt, AsyncWriteExt};
crates/blit-core/src/remote/transfer/pipeline.rs:834:                use tokio::io::AsyncWriteExt;
crates/blit-core/src/remote/transfer/progress.rs:131:impl<T> std::ops::Deref for CachePadded<T> {
crates/blit-core/src/remote/transfer/source.rs:294:                let data = builder.into_inner()?;
crates/blit-core/src/remote/transfer/payload.rs:393:    builder.into_inner().context("finalizing tar shard")
crates/blit-core/src/remote/transfer/data_plane.rs:4:use tokio::io::{AsyncReadExt, AsyncWriteExt};
crates/blit-core/src/remote/transfer/data_plane.rs:39:/// sites compose against the `AsyncWrite` impl of `StallGuardWriter`,
crates/blit-core/src/remote/transfer/data_plane.rs:638:    W: tokio::io::AsyncWrite + Unpin + ?Sized,
crates/blit-core/src/remote/transfer/tar_safety.rs:285:        builder.into_inner().unwrap()
crates/blit-core/src/remote/transfer/tar_safety.rs:295:        builder.into_inner().unwrap()
crates/blit-core/src/remote/transfer/stall_guard.rs:22://! - audit-h3b adds [`StallGuardWriter`] (this slice), an AsyncWrite
crates/blit-core/src/remote/transfer/stall_guard.rs:49:use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
crates/blit-core/src/remote/transfer/stall_guard.rs:68:/// `AsyncRead` / `AsyncWrite` and are covered separately (audit-h3c).
crates/blit-core/src/remote/transfer/stall_guard.rs:97:        let this = self.get_mut();
crates/blit-core/src/remote/transfer/stall_guard.rs:122:/// Wraps an `AsyncWrite` so a write that makes no progress within `timeout`
crates/blit-core/src/remote/transfer/stall_guard.rs:139:pub struct StallGuardWriter<W> {
crates/blit-core/src/remote/transfer/stall_guard.rs:145:impl<W> StallGuardWriter<W> {
crates/blit-core/src/remote/transfer/stall_guard.rs:154:    pub fn into_inner(self) -> W {
crates/blit-core/src/remote/transfer/stall_guard.rs:159:impl<W: AsyncWrite + Unpin> AsyncWrite for StallGuardWriter<W> {
crates/blit-core/src/remote/transfer/stall_guard.rs:165:        let this = self.get_mut();
crates/blit-core/src/remote/transfer/stall_guard.rs:203:        // Flush is a no-op for most AsyncWrite impls; we don't gate it
crates/blit-core/src/remote/transfer/stall_guard.rs:206:        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
crates/blit-core/src/remote/transfer/stall_guard.rs:210:        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
crates/blit-core/src/remote/transfer/stall_guard.rs:217:    use tokio::io::{AsyncReadExt, AsyncWriteExt};
crates/blit-core/src/remote/transfer/sink.rs:373:            use tokio::io::AsyncWriteExt as _;
crates/blit-core/src/remote/transfer/sink.rs:648:    use tokio::io::{AsyncSeekExt, AsyncWriteExt};
crates/blit-core/src/remote/transfer/sink.rs:1472:        let tar_data = builder.into_inner().unwrap();
crates/blit-core/src/remote/transfer/sink.rs:2235:        let tar_data = builder.into_inner().unwrap();

exec
/usr/bin/bash -lc "rg -n \"tcp_info|sample_tcp_info|TcpInfoSample|sample_stream\" crates docs proto -g '*.rs' -g '*.md' -g '*.proto'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
crates/blit-core/src/remote/transfer/mod.rs:12:pub mod tcp_info;
crates/blit-core/src/remote/transfer/mod.rs:33:pub use tcp_info::{sample_stream as sample_tcp_info, TcpInfoSample};
crates/blit-core/src/remote/transfer/tcp_info.rs:7://! else the syscall has no portable equivalent, so [`sample_stream`]
crates/blit-core/src/remote/transfer/tcp_info.rs:17:pub struct TcpInfoSample {
crates/blit-core/src/remote/transfer/tcp_info.rs:29:pub fn sample_stream(stream: &tokio::net::TcpStream) -> Option<TcpInfoSample> {
crates/blit-core/src/remote/transfer/tcp_info.rs:33:    // is a fully-owned zeroed `tcp_info`; `len` is initialised to its
crates/blit-core/src/remote/transfer/tcp_info.rs:37:    let mut info: libc::tcp_info = unsafe { std::mem::zeroed() };
crates/blit-core/src/remote/transfer/tcp_info.rs:38:    let mut len = std::mem::size_of::<libc::tcp_info>() as libc::socklen_t;
crates/blit-core/src/remote/transfer/tcp_info.rs:44:            &mut info as *mut libc::tcp_info as *mut libc::c_void,
crates/blit-core/src/remote/transfer/tcp_info.rs:51:    Some(TcpInfoSample {
crates/blit-core/src/remote/transfer/tcp_info.rs:60:pub fn sample_stream(_stream: &tokio::net::TcpStream) -> Option<TcpInfoSample> {
crates/blit-core/src/remote/transfer/tcp_info.rs:79:        let sample = sample_stream(&client).expect("TCP_INFO available on loopback");

exec
/usr/bin/bash -lc "nl -ba crates/blit-core/src/remote/tuning.rs | sed -n '1,120p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
     1	use crate::auto_tune::TuningParams;
     2	
     3	/// Determine remote transfer tuning from the estimated byte count.
     4	///
     5	/// One honest static table (w2-1): chunk size, stream counts, and
     6	/// socket sizing are keyed on `total_bytes` only — there is no
     7	/// runtime bandwidth adaptation (a real warmup probe is H10b-class
     8	/// future work). Note the stream ladder here is the *client's*
     9	/// authority; the daemon's push negotiation currently runs its own
    10	/// ladder and wins (single-owner consolidation is w2-2).
    11	pub fn determine_remote_tuning(total_bytes: u64) -> TuningParams {
    12	    let chunk_bytes = if total_bytes >= 8 * 1024 * 1024 * 1024 {
    13	        64 * 1024 * 1024
    14	    } else if total_bytes >= 512 * 1024 * 1024 {
    15	        32 * 1024 * 1024
    16	    } else {
    17	        16 * 1024 * 1024
    18	    };
    19	
    20	    let (initial_streams, max_streams) = if total_bytes >= 32 * 1024 * 1024 * 1024 {
    21	        (24, 32)
    22	    } else if total_bytes >= 8 * 1024 * 1024 * 1024 {
    23	        (16, 24)
    24	    } else if total_bytes >= 2 * 1024 * 1024 * 1024 {
    25	        (12, 16)
    26	    } else if total_bytes >= 512 * 1024 * 1024 {
    27	        (8, 12)
    28	    } else if total_bytes >= 128 * 1024 * 1024 {
    29	        (6, 10)
    30	    } else {
    31	        (4, 8)
    32	    };
    33	
    34	    let (tcp_buffer_size, prefetch_count) = if total_bytes >= 8 * 1024 * 1024 * 1024 {
    35	        (Some(8 * 1024 * 1024), Some(32))
    36	    } else if total_bytes >= 512 * 1024 * 1024 {
    37	        (Some(4 * 1024 * 1024), Some(16))
    38	    } else {
    39	        (None, None)
    40	    };
    41	
    42	    TuningParams {
    43	        chunk_bytes,
    44	        initial_streams,
    45	        max_streams,
    46	        tcp_buffer_size,
    47	        prefetch_count,
    48	    }
    49	}
    50	
    51	#[cfg(test)]
    52	mod tests {
    53	    use super::*;
    54	
    55	    const MIB: u64 = 1024 * 1024;
    56	    const GIB: u64 = 1024 * MIB;
    57	
    58	    #[test]
    59	    fn small_transfers_get_the_floor_tier() {
    60	        let t = determine_remote_tuning(10 * MIB);
    61	        assert_eq!(t.chunk_bytes, 16 * 1024 * 1024);
    62	        assert_eq!((t.initial_streams, t.max_streams), (4, 8));
    63	        assert_eq!(t.tcp_buffer_size, None);
    64	        assert_eq!(t.prefetch_count, None);
    65	    }
    66	
    67	    #[test]
    68	    fn mid_tier_scales_chunk_streams_and_buffers_together() {
    69	        let t = determine_remote_tuning(GIB);
    70	        assert_eq!(t.chunk_bytes, 32 * 1024 * 1024);
    71	        assert_eq!((t.initial_streams, t.max_streams), (8, 12));
    72	        assert_eq!(t.tcp_buffer_size, Some(4 * 1024 * 1024));
    73	        assert_eq!(t.prefetch_count, Some(16));
    74	    }
    75	
    76	    #[test]
    77	    fn large_transfers_get_the_64mib_chunk_tier() {
    78	        let t = determine_remote_tuning(10 * GIB);
    79	        assert_eq!(t.chunk_bytes, 64 * 1024 * 1024);
    80	        assert_eq!((t.initial_streams, t.max_streams), (16, 24));
    81	        assert_eq!(t.tcp_buffer_size, Some(8 * 1024 * 1024));
    82	        assert_eq!(t.prefetch_count, Some(32));
    83	    }
    84	
    85	    #[test]
    86	    fn top_tier_engages_at_32_gib() {
    87	        let below = determine_remote_tuning(32 * GIB - 1);
    88	        assert_eq!((below.initial_streams, below.max_streams), (16, 24));
    89	        let at = determine_remote_tuning(32 * GIB);
    90	        assert_eq!((at.initial_streams, at.max_streams), (24, 32));
    91	    }
    92	}

exec
/usr/bin/bash -lc "rg -n \"payload_prefetch|prefetch_count|prefetch\" crates/blit-core/src crates/blit-daemon/src -g '*.rs'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
crates/blit-daemon/src/service/pull.rs:620:    payload_prefetch: usize,
crates/blit-daemon/src/service/pull.rs:633:        payload_prefetch,
crates/blit-daemon/src/service/pull.rs:643:    execute_sink_pipeline(source, sinks, payloads, payload_prefetch, None)
crates/blit-daemon/src/service/pull.rs:667:    payload_prefetch: usize,
crates/blit-daemon/src/service/pull.rs:736:            payload_prefetch,
crates/blit-daemon/src/service/pull.rs:836:    payload_prefetch: usize,
crates/blit-daemon/src/service/pull.rs:848:        payload_prefetch,
crates/blit-daemon/src/service/pull.rs:855:    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(payload_prefetch.max(1));
crates/blit-daemon/src/service/pull.rs:880:    execute_sink_pipeline_streaming(source, sinks, payload_rx, payload_prefetch, None)
crates/blit-core/src/auto_tune/mod.rs:25:    /// Number of payloads to prefetch
crates/blit-core/src/auto_tune/mod.rs:26:    pub prefetch_count: Option<usize>,
crates/blit-core/src/remote/tuning.rs:34:    let (tcp_buffer_size, prefetch_count) = if total_bytes >= 8 * 1024 * 1024 * 1024 {
crates/blit-core/src/remote/tuning.rs:47:        prefetch_count,
crates/blit-core/src/remote/tuning.rs:64:        assert_eq!(t.prefetch_count, None);
crates/blit-core/src/remote/tuning.rs:73:        assert_eq!(t.prefetch_count, Some(16));
crates/blit-core/src/remote/tuning.rs:82:        assert_eq!(t.prefetch_count, Some(32));
crates/blit-core/src/remote/transfer/payload.rs:224:    prefetch: usize,
crates/blit-core/src/remote/transfer/payload.rs:226:    let capacity = prefetch.max(1);
crates/blit-core/src/remote/transfer/payload.rs:241:    payload_prefetch: usize,
crates/blit-core/src/remote/transfer/payload.rs:253:    let mut prepared_stream = prepared_payload_stream(payloads, source.clone(), payload_prefetch);
crates/blit-core/src/remote/transfer/pipeline.rs:28:    prefetch: usize,
crates/blit-core/src/remote/transfer/pipeline.rs:41:    let capacity = prefetch.max(1);
crates/blit-core/src/remote/transfer/pipeline.rs:55:    let result = execute_sink_pipeline_streaming(source, sinks, rx, prefetch, progress).await;
crates/blit-core/src/remote/transfer/pipeline.rs:73:/// `prefetch` controls the per-sink preparation-in-flight limit; the
crates/blit-core/src/remote/transfer/pipeline.rs:74:/// shared queue is bounded at `prefetch * sinks.len()` so total
crates/blit-core/src/remote/transfer/pipeline.rs:81:    prefetch: usize,
crates/blit-core/src/remote/transfer/pipeline.rs:91:    let capacity = prefetch.max(1) * sink_count;
crates/blit-core/src/remote/transfer/pipeline.rs:1156:        // producer. With prefetch=1 and a single sink, the bounded queue
crates/blit-core/src/remote/transfer/data_plane.rs:46:    payload_prefetch: usize,
crates/blit-core/src/remote/transfer/data_plane.rs:74:        payload_prefetch: usize,
crates/blit-core/src/remote/transfer/data_plane.rs:77:        Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, NoProbe)
crates/blit-core/src/remote/transfer/data_plane.rs:87:        payload_prefetch: usize,
crates/blit-core/src/remote/transfer/data_plane.rs:136:        Ok(Self::from_stream(stream, trace, chunk_bytes, payload_prefetch, pool).await)
crates/blit-core/src/remote/transfer/data_plane.rs:149:        payload_prefetch: usize,
crates/blit-core/src/remote/transfer/data_plane.rs:153:        let payload_prefetch = payload_prefetch.max(1);
crates/blit-core/src/remote/transfer/data_plane.rs:160:            payload_prefetch,
crates/blit-core/src/remote/transfer/data_plane.rs:181:        let mut stream = prepared_payload_stream(payloads, source.clone(), self.payload_prefetch);
crates/blit-core/src/remote/push/client/mod.rs:115:        payload_prefetch: usize,
crates/blit-core/src/remote/push/client/mod.rs:139:                payload_prefetch,
crates/blit-core/src/remote/push/client/mod.rs:152:        let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(payload_prefetch.max(1));
crates/blit-core/src/remote/push/client/mod.rs:155:        let prefetch = payload_prefetch.max(1);
crates/blit-core/src/remote/push/client/mod.rs:161:                prefetch,
crates/blit-core/src/remote/push/client/mod.rs:633:                                                tuning.prefetch_count.unwrap_or_else(|| tuning.initial_streams.max(1)),
crates/blit-core/src/remote/push/client/mod.rs:671:                                            let payload_prefetch = tuning
crates/blit-core/src/remote/push/client/mod.rs:672:                                                .prefetch_count
crates/blit-core/src/remote/push/client/mod.rs:679:                                                payload_prefetch,
crates/blit-core/src/remote/push/client/mod.rs:992:    payload_prefetch: usize,
crates/blit-core/src/remote/push/client/mod.rs:1028:        payload_prefetch,

exec
/usr/bin/bash -lc "nl -ba crates/blit-daemon/src/service/pull.rs | sed -n '850,890p'" in /home/michael/dev/blit_v2
 succeeded in 0ms:
   850	    )
   851	    .await?;
   852	
   853	    // Bridge channel: flatten Vec<TransferPayload> → individual payloads
   854	    // with byte/file counting along the way.
   855	    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(payload_prefetch.max(1));
   856	
   857	    let flatten = tokio::spawn(async move {
   858	        let mut total_bytes = 0u64;
   859	        let mut total_files = 0u64;
   860	        while let Some(batch) = batch_rx.recv().await {
   861	            for payload in batch {
   862	                total_bytes += payload_bytes(&payload);
   863	                total_files += match &payload {
   864	                    TransferPayload::File(_) => 1,
   865	                    TransferPayload::TarShard { headers } => headers.len() as u64,
   866	                    // Daemon's pull source (filesystem) never produces resume payloads.
   867	                    TransferPayload::FileBlock { .. }
   868	                    | TransferPayload::FileBlockComplete { .. } => 0,
   869	                };
   870	                if payload_tx.send(payload).await.is_err() {
   871	                    return (total_files, total_bytes);
   872	                }
   873	            }
   874	        }
   875	        (total_files, total_bytes)
   876	    });
   877	
   878	    let source: Arc<dyn blit_core::remote::transfer::source::TransferSource> =
   879	        Arc::new(FsTransferSource::new(source_root));
   880	    execute_sink_pipeline_streaming(source, sinks, payload_rx, payload_prefetch, None)
   881	        .await
   882	        .map_err(|err| Status::internal(format!("pull streaming pipeline: {err}")))?;
   883	
   884	    let (total_files, total_bytes) = flatten
   885	        .await
   886	        .map_err(|err| Status::internal(format!("pull flatten task panicked: {err}")))?;
   887	
   888	    let elapsed = start.elapsed().as_secs_f64().max(1e-6);
   889	    if total_bytes > 0 {
   890	        let gbps = (total_bytes as f64 * 8.0) / elapsed / 1e9;

codex
- `crates/blit-core/src/remote/transfer/pipeline.rs:120` — High — Workers never observe `cancelled` before taking more queued work, and the forwarder only checks it before `send_async`, so a sink error can still wait for queued/backpressured survivor work before surfacing; fix with a cancellation token/watch used in `tokio::select!` around worker receive and forwarder send/recv, and stop workers from draining queued payloads after first error.
- `crates/blit-core/src/remote/transfer/data_plane.rs:534` — Low — `send_block` records CLI outbound bytes but not `probe.record_bytes`, so `LiveProbe` undercounts block-resume data-plane bytes; add `self.probe.record_bytes(content.len() as u64)` after successful block-content writes.
- `crates/blit-core/src/remote/transfer/pipeline.rs:1217` — Low — The byte/file total test can pass balanced duplicate/drop cases because it asserts only aggregate counts and sums; record relative paths/IDs in the test sink and assert the exact expected set appears once.
- `crates/blit-core/src/remote/transfer/pipeline.rs:1284` — Low — `producer_cancel_winds_down_pipeline_promptly` covers clean producer EOF with one fast sink, not the shared `cancelled` error path under backpressure; add a multi-sink test with one failing sink, one slow sink, and a full queue.

VERDICT: fix-then-ship
tokens used
180,046
- `crates/blit-core/src/remote/transfer/pipeline.rs:120` — High — Workers never observe `cancelled` before taking more queued work, and the forwarder only checks it before `send_async`, so a sink error can still wait for queued/backpressured survivor work before surfacing; fix with a cancellation token/watch used in `tokio::select!` around worker receive and forwarder send/recv, and stop workers from draining queued payloads after first error.
- `crates/blit-core/src/remote/transfer/data_plane.rs:534` — Low — `send_block` records CLI outbound bytes but not `probe.record_bytes`, so `LiveProbe` undercounts block-resume data-plane bytes; add `self.probe.record_bytes(content.len() as u64)` after successful block-content writes.
- `crates/blit-core/src/remote/transfer/pipeline.rs:1217` — Low — The byte/file total test can pass balanced duplicate/drop cases because it asserts only aggregate counts and sums; record relative paths/IDs in the test sink and assert the exact expected set appears once.
- `crates/blit-core/src/remote/transfer/pipeline.rs:1284` — Low — `producer_cancel_winds_down_pipeline_promptly` covers clean producer EOF with one fast sink, not the shared `cancelled` error path under backpressure; add a multi-sink test with one failing sink, one slow sink, and a full queue.

VERDICT: fix-then-ship
