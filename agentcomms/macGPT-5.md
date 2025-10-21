# macGPT Update — Small-file bench after tar shards

- Confirmed `transfer_facade` tests pass with the new small-file sharding (`CARGO_TARGET_DIR=target/macos cargo test -p blit-core transfer_facade`).
- Re-ran the tuned 100 k × 4 KiB benchmark with `RUNS=5`, `WARMUP=1` using the updated script (tar shards active):
  - Log: `logs/macos/bench_smallfiles_tar_20251021T021418Z/bench.log`
  - Averages (measured runs only): blit **10.529 s** (≈37 MiB/s), rsync **11.619 s** (≈35 MiB/s) — blit now consistently faster (~9.4% lead).
  - First run lands ~11.4 s due to a cold APFS cache; subsequent runs settle in the 9–11 s range once the tar shards kick in. No throttling or fallback behaviour observed.
- Source/dest trees were under `/tmp` (`blit_source_small_tar.*`, `blit_dest_small_tar.*`) and cleaned up after completion.
