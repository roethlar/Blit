# ue-r2-1d: Streaming plan foundation (partial-scan InitialPlan/PlanUpdate)

**Slice**: ue-r2-1d — fourth slice of `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`
**Status**: Coded; under GPT review (`docs/agent/GPT_REVIEW_LOOP.md`)
**Branch**: master (no agent branches — AGENTS.md §8)
**Commits**: slice commit (this finding committed with it)

## What

Make the engine's streaming leg plan from a **partial** header stream
(REV4 Design §3): define `InitialPlan`/`PlanUpdate`, plan in batches as
enumeration proceeds, and feed the already-existing streaming pipeline
entry (`execute_sink_pipeline_streaming`, the seam push uses) so first
useful work no longer waits for full enumeration. Prove
first-work-before-scan-complete structurally (not by wall clock), and
document the RELIABLE exceptions that genuinely need full knowledge.

Today the local leg collects ALL headers, then plans once, then runs
the pipeline (`engine/mod.rs`, "2. Collect all headers"). Push already
streams work as it arrives (REV4 Current Code Reality) — its proof
obligation is covered by existing push wiring; local is the path this
slice converts.

## Design

- **`engine/streaming_plan.rs`** (new):
  - `InitialPlan { strategy: InitialPlanStrategy, plan_options }`,
    `InitialPlanStrategy::{Novel, Known{window_records}}` — the Design
    §3 novel/known split made first-class: Known when the perf-history
    tuning window produced records (plan options derived from history),
    Novel otherwise (conservative defaults). Full plan *replay* stays
    with the dial slice (`ue-r2-1e`) — see Known gaps.
  - `PlanUpdate { payloads, headers_planned, bytes_planned }` — one per
    planned batch.
  - `run_streaming_plan(header_rx, batch inputs, payload_tx, …)`:
    buffers headers; flushes a batch to `plan_local_mirror` (unchanged,
    now called per batch on `spawn_blocking`) when **any** of: batch
    full (`STREAMING_PLAN_BATCH_HEADERS = 512`), **time-based flush**
    (`STREAMING_PLAN_FLUSH_AFTER = 250ms` since the batch's first
    header — the REV4 "pathological slow enumeration" mitigation; a
    slow walker cannot stall first work past the flush window), or
    channel close. Sends resulting payloads into the pipeline channel;
    accumulates scanned totals, mirror source paths, and
    `first_payload_elapsed`.
- **`engine/mod.rs` streaming leg**: replace collect-all→plan→
  `execute_sink_pipeline` with: spawn scan → `tokio::join!` the
  streaming planner and `execute_sink_pipeline_streaming` → await the
  scan handle for error propagation → then mirror-deletion pass,
  journal checkpoints, predictor query, history record (unchanged
  order, with the diffs below).
- **Diff-behavior invariants preserved**: `plan_local_mirror` per batch
  stats the destination per header — batch-independent, so
  skip_unchanged/ignore_existing/compare-mode semantics are unchanged.
  Tar-shard/raw-bundle grouping happens within a batch (shards never
  span batches); grouping quality at 512-header batches is equivalent
  in practice, and grouping is a planning shape, not a correctness
  contract.
- **RELIABLE exceptions (documented + preserved, per the acceptance
  criteria)**:
  - **mirror/delete**: the deletion pass still runs only after the scan
    completed cleanly (R46-F2 refusal on unreadables unchanged); only
    the *copy* phase streams. Source-path set accumulates during the
    scan.
  - **resume / checksum-refusal**: remote-path concepts (block-resume
    negotiation, PullSync checksum handshake) — not exercised by the
    local leg this slice touches; noted for `1f`/`1g`.
- **Failure envelope change (documented)**: a hard scan error can now
  surface after some files were already written (previously scan
  completed before any byte moved). Written files are complete, correct
  copies; the operation still returns the scan error; mirror deletion
  never runs on an incomplete scan. This is the standard
  streaming-tool contract (rsync-like) and the price of the 1s start.
- **Predictor timing**: the estimate query needs final scan totals, so
  it moves after the join (still before `observe()` — train/query
  hygiene intact, R44-F1 feature alignment unchanged). With overlapped
  phases the planner/transfer split is redefined:
  `planner_duration_ms` = time to first payload handed to the pipeline
  (serial planning latency the user actually waits); transfer = the
  remainder. Regime change for predictor history noted below.

## Behavior pins / tests

- Structural first-work proof: a gated test `TransferSource` (custom
  `scan` emits wave 1, then WAITS until the sink observed a payload,
  then emits wave 2; `prepare/open` delegate to a real temp-dir
  `FsTransferSource`). If planning still waited for full enumeration
  this deadlocks (fails fast via test timeout); with streaming it
  passes and pins first-work-before-scan-complete with zero wall-clock
  flakiness. (The time-based flush is what makes wave 1 plannable
  below the 512 batch size.)
- Batch-boundary correctness: 600-file copy (2 batches + remainder)
  byte-complete, `copied_files == 600`.
- Existing pins keep passing unchanged: incremental mirror second run
  writes 0 bytes; mirror-delete positive/refusal tests; NoWork pins;
  tiny/streaming history-tag tests.

## Files changed

- `crates/blit-core/src/engine/streaming_plan.rs` — new:
  `InitialPlan`/`InitialPlanStrategy`/`PlanUpdate`, the batcher
  (`run_streaming_plan`: size/timer/close flush), per-batch
  `plan_local_mirror` on `spawn_blocking`; 2 in-module tests
  (timer-flush-while-open, size-flush + remainder).
- `crates/blit-core/src/engine/mod.rs` — streaming leg rewired:
  explicit Novel/Known strategy from the tuning window (+ verbose
  line), `tokio::join!` of planner + `execute_sink_pipeline_streaming`,
  error precedence pipeline→planner→scan, phase-split redefinition
  (planner = time-to-first-payload), predictor query moved after the
  join (still before observe), mirror deletions read the accumulated
  path set.
- `crates/blit-core/tests/engine_streaming_plan.rs` — new: the gated
  structural proof (`first_work_lands_before_enumeration_completes`).
- `crates/blit-core/tests/local_transfers.rs` — cross-batch-boundary
  pin (600 files).

## Tests

Baseline entering the slice: 1394 / 0 / 2 → after: **1398 / 0 / 2**
(+2 batcher unit tests, +1 gated structural proof, +1 cross-batch
pin). Structural guard proven non-vacuous: with the engine reverted to
collect-all (stash), the gated test deadlocks and fails at its 30s
timeout; restored, it passes in 0.25s. All pre-existing pins
(incremental-mirror zero-write, mirror-delete positive/refusal, NoWork,
history tags) pass unchanged.

## Known gaps

- **Known-workload full plan replay** (Design §3 "reproduce that plan
  immediately") is represented only as history-derived plan options
  (the tuning window); replaying a concrete plan and the live-tune
  ramp arrive with the dial (`ue-r2-1e`).
- **Push/pull 1s-start measurement**: push already streams
  structurally; a measured first-byte figure for push/pull shapes
  belongs to the 10 GbE session (`ue-1`) and `1f`/`1g` convergence.
- **Discovered pre-existing gap (not fixed here, surfaced to owner)**:
  streaming-path summaries have never populated
  `tar_shard_*`/`raw_bundle_*` bucket stats (`SinkOutcome` carries only
  files/bytes), so `select_tuning_window`'s `tar_shard_tasks > 0 ||
  raw_bundle_tasks > 0` filter can never admit a streaming record and
  `derive_local_plan_tuning` never fires at HEAD — the Known/Novel
  split therefore currently always resolves Novel in practice.
  Populating buckets would newly *activate* dormant auto-tuning —
  behavior change deliberately not smuggled into this slice; the dial
  slice (`1e`) subsumes or retires this mechanism (w2-2 territory).
- Predictor history regime change: planner/transfer split redefined
  under overlap (time-to-first-work vs remainder); per-profile
  coefficients re-train within the 20-record window.
