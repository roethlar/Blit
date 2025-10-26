Nova -

- Added an `info!` log to `windows_copyfile` when `FSCTL_DUPLICATE_EXTENTS_TO_FILE` returns success; benches can set `RUST_LOG=info,blit_core::copy::windows=info` to surface it.
- Re-ran the 4?GiB ReFS mirror with TEMP/TMP redirected to `D:\blit_bench`; log `logs/windows/bench_local_windows_4gb_clone_20251026T020337Z.log` shows blit **0.590?s** avg vs robocopy **0.165?s**. Keeping the workspace on `C:\Temp` hit the slow path: `bench_local_windows_4gb_clone_20251026T015438Z.log` blit **7.806?s**, robocopy **10.274?s**.
- Captured ETW trace `logs/windows/refs_clone_profile_20251026T022401Z.etl` plus bench log `bench_local_windows_4gb_clone_profile_20251026T022401Z.log`. WPR adds overhead (blit ?1.06?s, robocopy ?0.318?s), but the trace is ready for WPA to inspect privilege/metadata overhead.
- Manual mirrors on the same payload: default workers (12) `manual_default_20251026T022812Z.log` ?0.284?s; `--workers 1` `manual_workers1_20251026T022738Z.log` ?0.279?s. Throughput stays ~14?GiB/s in both cases, so fan-out isn?t the bottleneck; remaining gap is likely metadata/manifest prep + DeviceIoControl plumbing.
- Planning to diff the ETW trace for token/ACL churn next and evaluate whether clone mode can skip redundant metadata syncs.
