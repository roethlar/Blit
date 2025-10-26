Nova –

- Initial rerun on default temp (C:\Temp, NTFS) never hit the clone path: average blit mirrors stayed at 7.806 s vs robocopy 10.274 s and the log lacked any clone traces. That env sits on NTFS, so `supports_block_clone_same_volume` correctly returned false.
- Re-ran with both `TEMP` and `TMP` redirected to the ReFS dev drive (`D:\blit_bench`); same script immediately dropped to sub-second copies. Latest log: `logs/windows/bench_local_windows_4gb_clone_20251026T020337Z.log` (preserved workspace `D:\blit_bench\blit_v2_bench_972e62cb77174fdcbe7baa605cc5cfc4`).
  - blit v2 mirror (5 runs): avg **0.590 s** (best 0.385 s, ~15 GiB/s).
  - robocopy /MIR (5 runs): avg **0.165 s** (best 0.149 s).
  - No SeManageVolumePrivilege warnings; clone path clearly active (times match prior Robocopy behavior).
- For future benches, either set `TEMP/TMP` to the ReFS volume before invoking the script or export `BLIT_BENCH_ROOT` once we teach the harness to respect it. Without that override we’ll silently fall back to NTFS and miss the fast path.
- Next steps: fold this note into TODO/DEVLOG, consider bumping clone success logs to `info` so perf harness captures it, and wire a `BENCH_ROOT` env guard to avoid this footgun.
