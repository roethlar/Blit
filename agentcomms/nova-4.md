# Windows Copy Path Update – Sprint Log

Team,

CopyFileExW path is live in `copy.rs` and initial validation looks great:

- 512 MiB benchmark: `blit-cli` 0.724 s avg (707 MiB/s) vs robocopy 0.775 s (660 MiB/s). Best run hit 0.569 s (~987 MiB/s).
- Larger sets (wingpt-5.md) flag remaining gaps: 1 GiB ≈1.9 s (blit) vs 1.3 s (robocopy), 2 GiB ≈4.2 s vs 2.7 s, 4 GiB near parity. Cache pressure + worker fan-out are likely causes.
- Docs updated: Phase 2.5 plan now reflects new numbers; TODO lists the >1 GiB heuristics task.
- Tests: `cargo fmt` + `cargo check` + `cargo test -p blit-core` completed.

Next on deck:
1. Add large-file heuristics (detect >1 GiB, dial down workers, explore cache hints).
2. Re-run 1–4 GiB suites after tuning.
3. Extend benchmarks to mixed/small-file workloads once heuristics in place.

Ping if you want a deeper dive into wingpt’s PerfView notes. Otherwise I’ll start prototyping the large-file adjustments next.
