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

## 4. Current Snapshot (2025-10-20)

- **Large sequential workloads (✅ complete)**  
  - Linux: `logs/linux/bench_local_size512_20251020T222233Z/bench.log` (512 MiB average 3.85 s vs rsync 6.61 s) and `logs/linux/bench_local_size2048_20251020T222342Z/bench.log` (2 GiB average 9.37 s vs rsync 10.19 s).  
  - macOS: `logs/macos/bench-local-mirror-size512-20251020T220548Z/bench.log` (0.397 s vs rsync 1.234 s) and `logs/macos/bench-local-mirror-size2048-20251020T220703Z/bench.log` (1.597 s vs 5.009 s).  
  - Windows: `logs/wingpt/bench-512mb-20251020.log` (0.775 s vs robocopy 0.727 s) and `logs/wingpt/bench-2048mb-20251020.log` (4.10 s vs 4.19 s).  
  - 2025-10-25 rerun (post journal fast-path integration):  
    - Linux 1 GiB: `logs/linux/bench_local_linux_20251025T230101Z.log` → blit 1.427 s vs rsync 3.206 s (~225 %).  
    - macOS 1 GiB/4 GiB: `logs/macos/bench_local_20251025T231137Z/bench.log` (0.712 s vs 2.427 s) and `logs/macos/bench_local_20251025T235415Z/bench.log` (2.823 s vs 9.721 s).  
    - Windows 1 GiB NTFS: `logs/windows/bench_local_windows_20251025T233442Z.log` (1.619 s vs robocopy 1.516 s, ~107 %).  
    - Windows 4 GiB ReFS: `logs/windows/bench_local_windows_4gb_20251025T235715Z.log` (0.374 s vs robocopy 0.155 s, ~41 %); gap tracked via TODO item (`Investigate ReFS mirror throughput`).
- **Tiny manifest sanity checks (✅ complete)**  
  - Linux/macOS: `logs/linux/bench_local_size000_20251020T221948Z/bench.log`, `logs/macos/bench-local-mirror-size0-20251020T220501Z/bench.log` (rsync still faster, expected).  
  - Windows: `logs/wingpt/bench-0mb-20251020.log` (robocopy faster; planner overhead dominates).
- **Many small files (✅ complete)**  
  - Linux: `logs/linux/bench_smallfiles_tar_20251021T024313Z/bench.log` (100 k × 4 KiB) → blit 2.90 ± 0.02 s vs tuned rsync 8.56 ± 0.14 s (~295 %).  
  - macOS: `logs/macos/bench_smallfiles_tar_20251021T021418Z/bench.log` → blit 10.53 s vs rsync 11.62 s (~109 %).  
  - Windows: `logs/wingpt/bench-100k-smallfiles-20251021.log` → blit 60.63 s vs robocopy 218.48 s (~360 %).
- **Mixed workload (✅ complete)**  
  - Linux: `logs/linux/bench_mixed_tar_20251022T015203Z/bench.log` (512 MiB payload + 50 k × 2 KiB files) → blit 2.24 ± 0.07 s vs rsync 6.95 ± 0.32 s (~310 %).  
  - macOS: `logs/macos/bench_mixed_tar_20251022T014611Z/bench.log` → blit 6.32 s vs rsync 6.56 s (~104 %).  
  - Windows: `logs/wingpt/bench_mixed_incremental_20251021T230000Z/bench.log` (summarised from WingPT harness) → blit 31.26 s avg vs robocopy 110.51 s (~353 %).
- **Incremental mirror (✅ complete)**  
  - Linux: `logs/linux/bench_incremental_base_tar_20251022T015347Z/bench.log` (baseline) and `logs/linux/bench_incremental_update_tar_20251022T015347Z/bench.log` (touch 2 k, delete 1 k, add 1 k). Mutation averages: blit 0.61 ± 0.01 s vs rsync 1.23 ± 0.05 s (~202 %).  
  - macOS: `logs/macos/bench_incremental_base_tar_20251022T014812Z/bench.log` / `bench_incremental_update_tar_20251022T014823Z/bench.log` → blit 0.65 s vs rsync 0.69 s.  
  - Windows: `logs/wingpt/bench_mixed_incremental_20251021T230000Z/bench.log` (same run: baseline 7.10 s vs robocopy 20.72 s; mutation average 6.45 s vs 6.94 s).

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
