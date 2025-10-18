# Blit v2 – Phase 2 Review Snapshot  
**Date:** 2025‑10‑18  
**Reviewer:** Internal follow-up (post-Challenge fixes)  
**Scope:** `orchestrator.rs`, `perf_history.rs`, `perf_predictor.rs`, `local_worker.rs`, supporting docs

---

## Executive Summary

The original external review surfaced several real issues alongside a number of misunderstandings. We have validated the accurate findings, landed fixes, and captured the remaining follow‑ups below. The code now aligns with the documented Phase 2 architecture; no blocking defects remain, but a couple of diagnostics niceties are still open.

---

## Verified Issues & Resolutions

| Area | Finding | Resolution / Status |
|------|---------|---------------------|
| Performance history rotation | `enforce_size_cap` rewrote the log without protecting against concurrent appends (possible record loss, O(n²) trimming). | **Fixed** in `perf_history.rs`: switch to `VecDeque`, only rewrite when we actually trim, and skip the rewrite if the file grew after we sampled its size. |
| Predictor granularity | Profiles ignored `skip_unchanged` / `checksum`, mixing radically different planner costs. | **Fixed** in `perf_predictor.rs` (`ProfileKey` now captures both flags; orchestrator supplies them). |
| Worker observability | Per-file copy failures were suppressed unless the progress UI was enabled. | **Fixed** in `local_worker.rs`: workers always log `[wX] error` lines. |
| Mirror deletion failures | Deletion errors were hidden unless `--verbose` was set. | **Fixed** in `orchestrator.rs`: deletion failures always emit warnings. |
| Documentation drift | DEVLOG/Phase 2.5 docs lacked the latest benchmark results. | **Updated** (macOS vs rsync passes, Windows vs robocopy currently fails the ≥95 % gate). |

---

## Items To Consider (Non-blocking)

1. **Stall detector messaging** – Condition is correct, but splitting the error into “planner idle” vs “workers idle” would aid troubleshooting.
2. **Predictor state upgrades** – We still drop the JSON state on version mismatch; logging a warning before reset would help users understand why the predictor “forgot” past runs.
3. **History corruption logging** – `read_recent_records` quietly skips malformed lines; adding a debug/trace warning would improve transparency.

These are quality-of-life improvements; none affect correctness.

---

## Notes on Discarded Findings

The earlier review also raised concerns about the predictor learning rate and fast-path thresholds. After examining the code and specifications:

- The gradient-descent update (`LEARNING_RATE = 0.0005`) is intentionally scaled by the error magnitude and file/byte counts; it converges quickly in practice (dozens of observations), so no change is required.
- Fast-path limits (≤8 files, ≤100 MiB, ≥1 GiB) match `LOCAL_TRANSFER_HEURISTICS.md` and the current tiny/huge heuristics.

---

## Next Steps

1. Profile the Windows copy path (perf history now clean; benchmark gap tracked in Phase 2.5 doc).
2. Implement the optional diagnostics enhancements listed above if time allows.
3. Re-run the Windows benchmark after performance work and update the Phase 2.5 gate status.

