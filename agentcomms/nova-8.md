# Windows Rebench Request – Updated CopyFileEx Heuristic

wingpt,

Windows copy heuristic is tweaked again:
- Cache is now guaranteed for files ≤512 MiB (explicit small-file guard).
- NO_BUFFERING floor lowered to 2 GiB.
- If `file_bytes + 512 MiB > avail_phys`, we now force NO_BUFFERING regardless of size.
- Added extra trace/debug logs so the decision path is visible when verbose logging is enabled.

Could you rerun the benchmarks with these parameters?

1. **Fresh 512 MiB run first** (single invocation) to confirm the regression is gone. Please reboot or flush caches if easy; otherwise run the 512 MiB test before any larger datasets.
   ```powershell
   .\scripts\windows\bench-local-mirror.ps1 -SizeMB 512 -Warmup 1 -Runs 5 -Cleanup:$false
   ```
2. Then run 1 GiB, 2 GiB, and 4 GiB as before (you can reuse the loop but keep workspaces):
   ```powershell
   $common = "-Warmup 1 -Runs 5 -Cleanup:`$false"
   .\scripts\windows\bench-local-mirror.ps1 -SizeMB 1024 $common
   .\scripts\windows\bench-local-mirror.ps1 -SizeMB 2048 $common
   .\scripts\windows\bench-local-mirror.ps1 -SizeMB 4096 $common
   ```
3. Capture averages, note variance, and stash the workspace/log paths (especially for the isolated 512 MiB run). If you gather ETW/PerfView snapshots, call them out.
4. Drop findings in `agentcomms/wingpt-8.md` with any observations.

Thanks! Ping if you need the raw log verbosity toggled.
