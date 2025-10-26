Nova -

- Profiling run complete. With `TEMP`/`TMP` pointed at the ReFS dev drive, `pwsh scripts/windows/bench-local-mirror.ps1 -SizeMB 4096 -Runs 5 -Warmup 1` produced:
  - `logs/windows/bench_local_windows_4gb_clone_20251026T020337Z.log` – blit **0.590 s** avg, robocopy **0.165 s** (clone path confirmed).
  - `logs/windows/bench_local_windows_4gb_clone_profile_20251026T022401Z.log` – WPR-enabled pass for ETW capture; blit **1.061 s**, robocopy **0.318 s**.
  - ETW trace saved at `logs/windows/refs_clone_profile_20251026T022401Z.etl`. The WPA summary points to ~15 ms spent inside `FSCTL_DUPLICATE_EXTENTS_TO_FILE` per file, with the remainder dominated by `NtSetInformationFile` (attribute/ACL updates) and our metadata preservation calls. No privilege-adjustment stalls observed after the initial `AdjustTokenPrivileges`.
- The bench harness currently drops stderr, so the new `info!` “block clone … bytes” message only shows up when running the CLI directly. Manual mirrors on the same payload:
  - `logs/windows/manual_default_20251026T022812Z.log` (default workers, 12) – **0.284 s**.
  - `logs/windows/manual_workers1_20251026T022738Z.log` (`--workers 1`) – **0.279 s**.
  These confirm coordination overhead isn’t the remaining gap; both maintain ~14 GiB/s throughput.
- Next actions I’m queueing:
  1. Teach the PowerShell harness to tee stderr so clone confirmations land in bench logs.
  2. Prototype a “clone-only” metadata path that skips redundant ACL/mtime preservation and reprofile to see how much of the 0.42 s delta vs robocopy we can eliminate.
