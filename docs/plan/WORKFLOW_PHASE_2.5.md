# Phase 2.5: Performance & Validation Checkpoint

**Goal**: Confirm Blit v2 meets the plan v6 local-performance targets (≥95 % of baseline workloads) before proceeding to remote work.
**Duration**: 1-2 days
**Prerequisites**: Phase 2 complete; predictor/telemetry updates integrated
**Status**: In progress
**Type**: QUALITY GATE (mandatory)

---

## 1. Overview

Phase 2.5 is the hard stop/go gate before Phase 3. Re-run the existing benchmark harnesses, capture results, and evaluate against the acceptance criteria. The harnesses already live in the repo:

- macOS/Linux: `scripts/bench_local_mirror.sh` (runs `blit-cli` vs `rsync -a --delete`)
- Windows: `scripts/windows/bench-local-mirror.ps1` (runs `blit-cli` vs `robocopy /MIR`)

All benchmark outputs should be saved under `logs/` with timestamped filenames (e.g., `logs/bench_local_20251020T0300Z.log`). Summaries go into `DEVLOG.md` and this workflow document.

### Workloads to Measure
1. Large sequential copy (≥512 MiB, 1 GiB, 2 GiB, 4 GiB)
2. Many small files (100k files, 1–10 KiB)
3. Mixed workload (blend of large + small files)
4. Mirror incremental update (delete/update subset of files)

For each workload record:
- `blit-cli` timing + throughput
- Comparator timing (rsync/robocopy)
- System notes (disk type, CPU, RAM)
- Benchmark logs saved in `logs/` (link in DEVLOG)

---

## 2. Gate Criteria

| Metric | Target | Notes |
|--------|--------|-------|
| Large file throughput | ≥95 % of comparator | Use rsync (macOS/Linux) or robocopy (Windows) |
| Small-file throughput | ≥95 % of comparator | Watch for predictor/heuristic regressions |
| Mixed workload | ≥95 % of comparator | Ensure planner picks optimal path |
| Incremental mirror | ≥95 % of comparator | Verify delete/skip logic |
| Memory/CPU usage | Within expected bounds | Compare to previous logged runs |

**Decision:**
- ✅ GO – All workloads meet or exceed targets → proceed to Phase 3.
- ❌ NO-GO – Any workload falls short → investigate and fix before moving on.

If NO-GO, typical remediation includes tuning CopyFileEx heuristics, adjusting predictor thresholds, or profiling the hot path indicated by the benchmark logs.

---

## 3. Task Checklist

1. **Run macOS/Linux benchmarks** via `scripts/bench_local_mirror.sh` (with appropriate environment variables to disable telemetry if comparing cold starts). Save output under `logs/` and note results in DEVLOG.
2. **Run Windows benchmarks** via `scripts/windows/bench-local-mirror.ps1`; capture robocopy comparisons and stash logs under `logs/wingpt/...`.
3. **Update DEVLOG.md** with a summary table (throughput, comparator, GO/NO-GO call) and link to log files.
4. **Update this workflow** with key metrics so future agents don’t re-run without cause.
5. **If heuristics changed during Phase 2**, repeat the relevant workloads to confirm improvements.

---

## 4. Current Snapshot (2025-10-19)

Recent benchmark runs (macOS + Windows) already show blit-cli meeting/exceeding comparators for large files (see `logs/bench_local_mirror*.log`, `logs/wingpt/windows_bench_extract/`). Mixed/small-file suites still need fresh data before declaring GO for Phase 3.

---

## 5. Reporting Template

When logging results in DEVLOG/workflow:

```
Platform: macOS 14.3 (APFS NVMe)
Workload: 512 MiB (1 warmup, 5 runs)
  blit-cli mirror: 0.275 s avg (1.86 GB/s)
  rsync -a --delete: 0.605 s avg (0.85 GB/s)
Decision: ✅ GO (blit-cli 218% of comparator)
Logs: logs/bench_local_20251020T0300Z.log
```

Replicate for other workloads/platforms so the decision trail is preserved for future agents.

---

## 6. Exit Criteria
- Benchmark results captured for all workloads listed above.
- DEVLOG / `logs/` updated with raw data and summaries.
- Any regressions investigated and resolved (or documented with plan).

Meeting these items satisfies the Phase 2.5 gate and unlocks Phase 3 work.
