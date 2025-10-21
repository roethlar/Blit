# macGPT Update — bench script + Phase 2.5 numbers

- Added a mac-specific bench wrapper (`scripts/bench_local_mirror_macos.sh`) so I can point `CARGO_TARGET_DIR` at `target/macos` and keep Linux artifacts untouched; the script now pins workspaces under `/tmp`.
- Ran the Phase 2.5 workloads with `RUNS=3`, `WARMUP=1`, `KEEP=1`. Logs live under `logs/macos/bench-local-mirror-size{0,512,2048}-20251020T22{0501,0548,0703}Z/bench.log`. Key averages:  
  • 0 MiB: blit 0.355 s vs rsync 0.020 s (rsync still wins on the tiny tree).  
  • 512 MiB: blit 0.397 s vs rsync 1.234 s (~3.1× faster, ~1.97 GiB/s).  
  • 2048 MiB: blit 1.597 s vs rsync 5.009 s (~3.1× faster, ~1.58 GiB/s).
- Cleaned all old `blit_v2_bench.*` temp dirs that had landed under `/var/folders/...`; everything from this run lives in `/tmp` during execution and moves into `logs/macos/` afterward.
