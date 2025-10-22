# macGPT Update — Phase 2.5 mixed/incremental rerun (macOS)

- Mixed workload (512 MiB + 50 k × 2 KiB) with tar shards:
  - Log: `logs/macos/bench_mixed_tar_20251022T014611Z/bench.log`
  - Averages over 3 measured runs: blit **6.318 s**, rsync **6.564 s** (~3.9% faster for blit).
- Incremental baseline (128 MiB + 10 k files): `logs/macos/bench_incremental_base_tar_20251022T014812Z/bench.log`
  - Single measured pass: blit **1.148 s**, rsync **1.419 s**.
- Incremental mutation (touch 2 k / delete 1 k / add 1 k): `logs/macos/bench_incremental_update_tar_20251022T014823Z/bench.log`
  - Averages over 3 measured runs: blit **0.649 s**, rsync **0.691 s** (parity once most work becomes no-ops).
- All scratch dirs stayed under `/tmp`; only the log folders above were retained. Let me know when we need to rerun with additional permutations.
