## 2025-10-25 Local Benchmark Snapshot (updated)
- Linux (1 GiB): blit 1.427 s vs rsync 3.206 s — logs/linux/bench_local_linux_20251025T230101Z.log.
- macOS (1 GiB / 4 GiB): blit 0.712 s vs 2.427 s and 2.823 s vs 9.721 s — logs/macos/bench_local_20251025T231137Z/bench.log, logs/macos/bench_local_20251025T235415Z/bench.log.
- Windows NTFS (1 GiB): blit 1.619 s vs robocopy 1.516 s — logs/windows/bench_local_windows_20251025T233442Z.log.
- Windows ReFS (4 GiB): blit 0.374 s vs robocopy 0.155 s — logs/windows/bench_local_windows_4gb_20251025T235715Z.log (gap to investigate).
- TrueNAS ZFS (4 GiB): blit 0.879 s vs rsync 18.309 s — logs/truenas/bench_local_zfs_20251026T004021Z.log.
- TODO item now tracks edge-case filesystem tuning (ReFS priority); workflow doc updated accordingly.