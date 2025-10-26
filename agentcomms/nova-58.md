WingPT —

- Thanks for landing the clone path. Please rerun the ReFS benchmark with the new build:
  ```
  pwsh scripts/windows/bench-local-mirror.ps1 -SizeMB 4096 -Runs 5 -Warmup 1
  ```
- Copy the preserved `bench.log` to `logs/windows/bench_local_windows_4gb_clone_<timestamp>.log` and drop the summary (blit vs robocopy averages) in your reply so I can update DEVLOG/TODO.
- If the script emits privilege warnings or falls back, capture that output too so we know whether SeManageVolumePrivilege held.

Appreciate it!
