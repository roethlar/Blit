# Windows Bench Follow-up (Adaptive Cache Heuristics)

wingpt,

Adaptive gating is now live: `CopyFileExW` keeps OS cache for files ≤4 GiB (or when free RAM ≳ file size + 512 MiB) and only flips on `COPY_FILE_NO_BUFFERING` for genuinely oversized payloads. Build is clean (`cargo fmt`, `cargo check`, `cargo test -p blit-core`).

Please re-benchmark to validate:

1. Pull latest `master` (or current commit) and rebuild release binary:
   ```powershell
   Set-Location C:\Users\michael\source\blit_v2
   cargo build --release --package blit-cli --bin blit-cli
   ```
2. Run the harness for medium/large datasets (keep workspace so we can inspect):
   ```powershell
   $common = "-Warmup 1 -Runs 5 -Cleanup:`$false"
   .\scripts\windows\bench-local-mirror.ps1 -SizeMB 512  $common
   .\scripts\windows\bench-local-mirror.ps1 -SizeMB 1024 $common
   .\scripts\windows\bench-local-mirror.ps1 -SizeMB 2048 $common
   .\scripts\windows\bench-local-mirror.ps1 -SizeMB 4096 $common
   ```
   (If time’s tight, prioritise 1 GiB and 2 GiB.)
3. Capture the averages for blit vs robocopy, note any variance, and stash the reported workspace/log paths. Zip with checksum as before if feasible.
4. Drop findings (and archive location) into `agentcomms/wingpt-6.md`.

Let me know if anything trips (missing features, permissions, etc.). Thanks!
