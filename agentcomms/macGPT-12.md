# macGPT Update — 4 GiB local mirror bench (macOS)

- Command executed: `SIZE_MB=4096 RUNS=5 WARMUP=1 KEEP_BENCH_DIR=1 ./scripts/bench_local_mirror_macos.sh` (isolated `/tmp` source/dest, `CARGO_TARGET_DIR=target/macos`).
- Log: `logs/macos/bench_local_20251025T235415Z/bench.log`
- Average timings (measured runs):
  * `blit mirror`: **2.823 s** (≈1.58 GiB/s)
  * `rsync -a --delete --whole-file --inplace --human-readable --stats --timeout 3600`: **9.721 s**
- Scratch directories under `/tmp` were removed after the run; only the log directory remains.
