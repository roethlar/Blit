Nova -

- Added `log::info!` in `windows_copyfile` when `FSCTL_DUPLICATE_EXTENTS_TO_FILE` succeeds; `cargo fmt` run. The message targets the full path and byte count so future benches can set `RUST_LOG=info,blit_core::copy::windows=info` and spot the fast path. (The harness pipes tool output through `Write-Log`, so stderr still needs capturing to surface the new line.)
- Bench reruns:
  - `logs/windows/bench_local_windows_4gb_clone_20251026T020337Z.log` (TEMP/TMP → `D:\blit_bench`, ReFS) → blit **0.590 s** avg, robocopy **0.165 s**.
  - Keeping the workspace on `C:\Temp` repeats the slow path: blit **7.806 s**, robocopy **10.274 s** (`bench_local_windows_4gb_clone_20251026T015438Z.log`).
- Profiling pass captured ETW trace `logs/windows/refs_clone_profile_20251026T022401Z.etl` with companion log `bench_local_windows_4gb_clone_profile_20251026T022401Z.log` (blit **1.061 s**, robocopy **0.318 s** - WPR adds noticeable overhead). Trace is ready for WPA drill-down on token adjustments and DeviceIoControl latency.
- Manual spot checks on the same payload:
  - Default workers (12): `logs/windows/manual_default_20251026T022812Z.log`, runtime ≈**0.284 s**.
  - `--workers 1`: `logs/windows/manual_workers1_20251026T022738Z.log`, runtime ≈**0.279 s**.
  - Throughputs stay ~14 GiB/s in both cases, so scheduler fan-out is not the primary delta versus robocopy (~0.17 s).
- Observation: the remaining ~1.6–1.7× gap likely comes from start-up bookkeeping (config/manifest prep, metadata preservation) and extra DeviceIoControl calls. Next steps could include diffing the ETW trace for token/ACL churn and checking whether we can skip redundant metadata sync when the clone path runs.
