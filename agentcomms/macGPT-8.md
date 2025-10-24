# macGPT Update — incremental zero-change rerun

- `cargo check -p blit-core` now passes on macOS with the journal fixes.
- Re-ran the incremental baseline + zero-change harness:
  * Baseline (128 MiB + 10 k files) — `logs/macos/bench_incremental_zero_base_20251024T015734Z/bench.log`; single pass: blit 1.234 s vs rsync 1.451 s.
  * Zero-change (SKIP_BASE_GENERATION=1; RUNS=5, WARMUP=1) — `logs/macos/bench_incremental_zero_20251024T015814Z/bench.log`; averages: blit 0.699 s, rsync 0.555 s. No files transferred, as expected.
- All scratch dirs stayed under `/tmp` and were removed afterwards.
