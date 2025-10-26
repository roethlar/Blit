Nova -

**1. Harness stderr logging**
- Updated `scripts/windows/bench-local-mirror.ps1` so both STDOUT and STDERR from each tool line are fed through `Write-Log`. The saved log now keeps robocopy warnings and any blit fast-path prints.
- Reran the 4 GiB ReFS bench with `TEMP/TMP` pinned to `D:\blit_bench`. Log: `logs/windows/bench_local_windows_4gb_clone_20251026T183749Z.log`.
  - The log captures stderr (e.g., `robocopy` privilege errors), but there is still no `block clone …` info line because `FSCTL_DUPLICATE_EXTENTS_TO_FILE` is refusing to clone without `SeManageVolumePrivilege` (we run unprivileged). The bench therefore falls back to CopyFileEx + metadata and lands at ~0.68 s per run.

**2. Clone-only metadata experiment**
- Added a prototype toggle (`BLIT_SKIP_METADATA_ON_CLONE`) that skips `preserve_metadata` when a block clone succeeds.
- Compared averages (both runs on D:\):
  | Mode | Env | Log | blit avg | robocopy avg |
  | --- | --- | --- | --- | --- |
  | Baseline (metadata) | none | `bench_local_windows_4gb_clone_20251026T183749Z.log` | **0.681 s** | 0.189 s |
  | Metadata skipped | `BLIT_SKIP_METADATA_ON_CLONE=1` | `bench_local_windows_4gb_clone_nometa_20251026T184100Z.log` | **0.698 s** | 0.189 s |
- Because the clone ioctl is still failing privilege checks, both paths fall back to a full CopyFileEx copy, so skipping metadata has no measurable effect. Once we can acquire `SeManageVolumePrivilege`, this toggle will let us isolate metadata overhead.

**3. ETW summary**
- With WPA unavailable in this environment, I used `tracerpt` to pull a textual summary (`logs/windows/refs_clone_profile_20251026T022401Z_summary.txt`). The trace spans ~4.6 M events across 221 s; the heaviest providers are `PerfInfo` (stack sampling) and `StackWalk`. No block-clone activity shows up, again because the ioctl never succeeds under the current token. We still need WPA/Windows Performance Analyzer to drill into call stacks (expected hot spots: `NtSetInformationFile` and friends). Happy to re-run the analysis once we have WPT on the box or a privileged shell.

Let me know if you can supply elevated creds or WPT binaries; that would let us confirm the clone fast path properly and evaluate the metadata toggle versus robocopy.
