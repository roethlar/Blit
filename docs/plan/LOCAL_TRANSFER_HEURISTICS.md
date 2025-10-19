# Local Transfer Heuristics

**Goal:** Deliver the fastest possible local `copy`/`mirror` experience across *all* workloads while preserving the SIMPLE and RELIABLE guarantees. This document captures the full design we intend to implement in Phase 2 for Blit v2. There is no staged rollout—every mechanism described here will ship together once complete.

---

## 1. Design Tenets

- **FAST:** bytes start moving immediately, planner overhead stays invisible unless something genuinely stalls.
- **SIMPLE:** no user-facing tuning flags; the orchestrator decides the optimal path. Debug-only overrides may exist but are not required for normal operation.
- **RELIABLE:** mirror deletions, checksum parity, filesystem safety checks always take precedence over raw speed.
- **Privacy:** performance history is strictly local; nothing leaves the user’s machine.

---

## 2. Current Pipeline (Baseline)

1. Enumerate source tree → collect `EnumeratedEntry` objects.
2. `MirrorPlanner` determines skip/deletion decisions.
3. `TransferFacade` classifies work into tar shards, raw bundles, large-file tasks.
4. `TransferEngine` schedules worker goroutines; `LocalWorkerFactory` pushes bytes.

Strengths: shared logic with remote pipelines, zero-copy for large files, skip-unchanged behaviour.
Weakness: small workloads pay the planning cost before any data is moved (perceived lag).

---

## 3. Planner Latency Handling

### 3.1 Streaming Planner Architecture

- Convert `TransferFacade::build_local_plan` into a streaming producer that classifies entries as they appear.
- Batches (large-file, raw bundle, tar shard) are emitted incrementally to the `TransferEngine`.
- Workers start as soon as the first batch is emitted; there is no requirement to wait for whole-manifest planning.

### 3.2 Heartbeat & Progress

- The orchestrator maintains a 1 s heartbeat timer. At each tick it flushes whatever batches are ready to the worker queue.
- Flush cadence dynamically adjusts based on queue saturation: 1000 ms while the queue is empty, tightening to 500 ms if workers are draining results quickly, relaxing back when the queue fills.
- CLI progress spinner (indicatif) tracks elapsed time and surfaces final throughput; `--no-progress` disables the spinner, while `--verbose` keeps raw planner logs.

### 3.3 Stall Detection & Timeouts

- As long as either the planner emits new batches or workers report progress, no timeout triggers. Planning is free to exceed 1 s when necessary (e.g., large manifests).
- If **both** planner output *and* worker progress are idle for 10 s, the orchestrator aborts with a precise message: e.g., `Planner stalled while reading /path/to/dir; aborting after 10s without progress.`
- The prior 30 s timeout concept is dropped; 10 s is the hard limit for no-progress scenarios.

### 3.4 User Feedback

- `--verbose` shows real-time heartbeat messages (`Planning… 12,345 entries classified; queue depth 8`).
- Default mode remains quiet unless a stall occurs, in which case the error message includes the path being processed and suggested action (e.g., check filesystem accessibility).

---

## 4. Immediate Fast-Paths

These are deterministic decisions made before streaming begins:

| Trigger | Action | Notes |
|---------|--------|-------|
| Mirror deletions enabled | Full planner mandatory | Reliability > speed |
| Checksum run (`--checksum`) | Full planner mandatory | skip decisions depend on hashes |
| ≤ 8 files AND ≤ 100 MB total AND no mirror/checksum/tar | Direct sequential copy | Planner overhead dominates; reuse `copy::copy_file` path |
| Single file ≥ 1 GiB | Dispatch immediately to large-file (zero-copy) worker | Planner continues streaming remaining entries |

These heuristics are internal; users continue to run `blit copy` / `blit mirror` with no additional flags.

*Tiny manifest fast path is additionally gated by the predictor: once a profile has observations, we only bypass streaming when predicted planning time exceeds 1 s; with no history we default to fast-path for the initial runs.*

---

## 5. Performance History (Local Only)

### 5.1 Purpose

- Measure actual planning vs. copy durations to validate our 1 s perceived latency goal.
- Track fast-path hit rate and stall events for debugging.

### 5.2 Storage & Privacy

- Metrics are stored locally as a capped JSON Lines file (e.g., `~/.config/blit/perf_local.jsonl`, max ~1 MiB).
- Each entry includes: timestamp, workload signature (file count, total bytes, flags), planning_ms, copy_ms, stall_count, filesystem profile.
- No data is sent off-machine. Set `BLIT_DISABLE_PERF_HISTORY=1` to disable recording entirely.

### 5.3 Usage

- Metrics feed diagnostic tooling (`blit diagnostics perf`) for support. 
- Adaptive logic reads only the most recent N entries (default 50) to inform planning-time predictions.

---

## 6. Adaptive Planning Prediction

The orchestrator maintains a simple predictor to estimate planning overhead and decide whether to go straight into the streaming planner or use an immediate direct-copy fast path.

### 6.1 Predictor Model

- Linear combination of factors: `planning_ms ≈ α * files + β * total_bytes + γ`.
- Coefficients updated via exponential moving average after each run. Separate coefficients maintained per filesystem profile (e.g., SSD, HDD, network share).
- Initialized with conservative defaults (e.g., α=0.05 ms/file, β=0.01 ms/MB) so predictions err on the side of using the planner.

### 6.2 Routing Decisions

- If predicted planning_ms ≤ 1000 ms: enter streaming planner immediately.
- If predicted planning_ms > 1000 ms *and* fast-path conditions are met (Section 4), use fast-path.
- If predicted planning_ms > 1000 ms and no fast-path applies, still enter streaming planner but emit a verbose warning (`expected planning time >1s; continuing due to mirror/checksum requirements`).
- When verbose mode is enabled, report the predictor estimate before entering the planner; defaults remain quiet unless a stall occurs.

### 6.3 Self-Correction

- After each run, record actual planning_ms. If prediction error exceeds 25%, adjust coefficients more aggressively.
- 
### 6.4 Performance History Opt-Out

- When performance history capture is disabled, prediction falls back to conservative defaults and no updates occur.

---

## 7. Worker & Buffer Tuning

- The planner automatically selects aggressive buffer sizes, tar shard targets, and worker counts based on workload and available CPU (no manual speed flags).
- Default worker count = `num_cpus::get()` (with safeguards for hyper-threaded vs. physical cores). Upper bound clamps to 16 by default but adapts if the machine proves capable.
- Optional debug limiters (`--workers`, `--max-threads`, `BLIT_MAX_THREADS`) cap worker count for diagnostics. Using them must surface a clear “DEBUG MODE” indicator so operators know FAST heuristics are constrained.
- CLI stays quiet during transfers; progress events are emitted for verbose/log subscribers and GUI surfaces.
- Buffer sizing logic evaluates run-time conditions (e.g., detect memory pressure via `sysinfo`) to avoid over-allocating on small systems.

---

## 8. Implementation Checklist

1. **Streaming Planner Refactor**
   - Transform `TransferFacade::build_local_plan` into an iterator/async stream.
   - Introduce batch descriptors with partial completion semantics for tar shards.
2. **Heartbeat Scheduler**
   - Implement orchestrator component that flushes planner output at 1 s/0.5 s intervals.
   - Integrate with `TransferEngine` queue depth monitoring.
3. **Fast-Path Integration**
   - Wire pre-checks for tiny manifests and giant single files.
   - Ensure direct copy path reuses existing copy primitives (preserve timestamps, symlinks).
4. **Telemetry Store**
   - JSONL writer with size cap + optional disable flag.
   - Prediction model persistence (e.g., store coefficients alongside performance history log).
5. **Timeout & Messaging**
   - Implement 10 s stall detection with clear user-facing errors.
   - Add progress messages under `--verbose`.
6. **CLI Cleanup**
   - Remove `--ludicrous-speed`; no compatibility shim required prior to CLI v2.
   - Document optional debug overrides.
7. **Testing & Benchmarks**
   - Unit tests for fast-path routing, predictor updates, stall detection.
   - Integration tests covering tiny, medium, and large workloads.
   - Phase 2.5 benchmark suite expanded to include 1-file, 8-file, 100 k-file cases and cross-FS copies.

---

## 9. Safety & Reliability Guarantees

- **Mirror Deletions:** Always require completed planner output; direct copy path is disabled when deletions are requested.
- **Checksums:** Planner mandatory; skip logic depends on hash comparisons.
- **Error Handling:** Any planner or worker error aborts the run with accumulation of all errors (existing behaviour).
- **Platform Variance:** Predictor maintains separate models for Windows vs. Unix, and case-insensitive vs. case-sensitive filesystems.

---

## 10. Open Questions & Resolutions

| Question | Resolution |
|----------|------------|
| Do we expose performance summaries? | Yes, via `blit diagnostics perf` (local only). |
| Different thresholds for low-powered hardware? | Managed automatically by adaptive predictor; no hard-coded per-hardware rules. |
| Cross-filesystem performance differences? | Predictor coefficients segmented by source/dest FS profile; transfer engine monitors backpressure to throttle. |
| Cache mirror deletion plans? | No; correctness risk outweighs gain. |
| OS-specific event logs (USN journal)? | Future optimization. Use opportunistically as fast-path once baseline is stable. |

---

## 11. Future Work (Post v2 Launch)

- Investigate FSEvents/USN journal integration for incremental planning.
- Explore GPU/accelerated hashing for checksum mode.
- Consider remote performance history opt-in to improve heuristics globally (opt-in only).
- Revisit `--max-threads` flag usage; deprecate if unused.

---

**Status:** Updated design approved for immediate implementation. No further staged phases; work continues until the entire orchestration stack meets performance goals.
