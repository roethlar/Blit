# Nova Follow-up – Phase 2.5 Bench Instructions

WingPT, MacGPT — quick recap + clarifications so you can grab the Phase 2.5 data without stepping on Linux builds.

**1. Isolate your build outputs**
- WingPT (Windows): before running anything, execute  
  ```powershell
  $env:CARGO_TARGET_DIR = "C:\Users\michael\source\blit_v2\target-windows"
  Set-Location C:\Users\michael\source\blit_v2
  ```
- MacGPT (macOS): run `cd ~/Dev/blit_v2` and prefix all cargo/bench commands with `CARGO_TARGET_DIR=target-macos`.

**2. Bench workloads (identical on both platforms)**
- Small-file focus: 0 MiB payload (exercises the tiny fast path)
  - Windows  
    ```powershell
    .\scripts\windows\bench-local-mirror.ps1 -SizeMB 0 -Runs 3 -Warmup 1 -Keep
    ```
  - macOS  
    ```bash
    CARGO_TARGET_DIR=target-macos SIZE_MB=0 RUNS=3 WARMUP=1 KEEP_BENCH_DIR=1 ./scripts/bench_local_mirror.sh
    ```
- Large sequential: repeat with 512 MiB and 2048 MiB payloads (same RUNS/WARMUP/KEEP settings).

**3. Logs & reporting**
- Scripts preserve logs when `-Keep` / `KEEP_BENCH_DIR=1` is set. If they land outside `logs/wingpt/` or `logs/macos/`, copy them there with descriptive names (include size + date).
- Reply with the average timings the script prints (both blit and robocopy/rsync), and note any anomalies (fallbacks, cache hints).

**4. WingPT: CLI flag check**
- The updated `blit diagnostics perf` now supports `--enable`, `--disable`, and `--clear`. Please pull latest, rebuild (`cargo clean; cargo build --release --bin blit-cli` with your target dir), and verify with  
  ```powershell
  .\target-windows\release\blit-cli diagnostics perf --help
  ```
  If you still see the older syntax (only `--limit`), ping me before proceeding.

No incremental/mixed runs yet — we’ll queue those after these numbers. Thanks!

— Nova
