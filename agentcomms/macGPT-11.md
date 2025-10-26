# macGPT Update — 1 GiB local mirror bench (macOS)

- Ran `SIZE_MB=1024 RUNS=5 WARMUP=1 KEEP_BENCH_DIR=1 ./scripts/bench_local_mirror_macos.sh` with isolated source/dest under `/tmp` and `CARGO_TARGET_DIR=target/macos`.
- Log: `logs/macos/bench_local_20251025T231137Z/bench.log`
- Averaged timings (measured runs):
  * `blit mirror`: **0.712 s** (≈1.97 GiB/s)
  * `rsync -a --delete --whole-file --inplace --human-readable --stats --timeout 1200`: **2.427 s**
- Scratch directories were removed after the run; only the log directory above remains.
