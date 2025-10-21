# macGPT Update — Phase 2.5 tuned runs (macOS)

- Updated `scripts/bench_local_mirror.sh` so it honours `CARGO_TARGET_DIR` (and filters `--no-compress` when the platform rsync lacks it). Linux script stays untouched.
- Phase 2.5 workloads executed with the tuned comparator settings; workspaces copied under `logs/macos/` (blit binaries from `target/macos`):
  * Small files (100 k × 4 KiB): `logs/macos/bench_smallfiles_20251021T012306Z` → blit avg **12.278 s** (≈33 MiB/s), rsync avg **11.220 s** (≈36 MiB/s). Both hammer metadata; rsync still edges out on this worst‑case tree.
  * Mixed workload (512 MiB + 50 k × 2 KiB): `logs/macos/bench_mixed_20251021T012604Z` → blit avg **6.162 s**, rsync avg **6.405 s** (~4% faster for blit; ~105 MiB/s during runs).
  * Incremental baseline (128 MiB + 10 k files): `logs/macos/bench_incremental_base_20251021T012744Z` → single blit pass **1.130 s** vs rsync **1.423 s**.
  * Incremental mutation (touch 2 k / delete 1 k / add 1 k): `logs/macos/bench_incremental_update_20251021T012754Z` → blit avg **0.772 s**, rsync avg **0.716 s** (rsync slightly ahead once most work is deletions/no-ops). No timeouts or fallback warnings observed.
- All scratch trees were allocated under `/tmp` and removed after each run (only the log directories remain in-repo).
