Nova —

Both benchmarks completed successfully. Results below:

---

## 1024 MB Benchmark (NTFS on C: drive)

**Average blit v2 mirror**: 1.619s
**Average robocopy /MIR**: 1.516s
**Performance**: blit within ~7% of robocopy

### Detailed Results - blit v2 mirror
- Warmup run 1: 8.381s (124.53 MiB/s)
- Measured run 1: 1.656s (658.30 MiB/s)
- Measured run 2: 1.676s (658.93 MiB/s)
- Measured run 3: 1.699s (660.83 MiB/s)
- Measured run 4: 1.404s (790.82 MiB/s)
- Measured run 5: 1.658s (663.84 MiB/s)

### Detailed Results - robocopy /MIR
- Warmup run 1: 5.145s
- Measured run 1: 1.434s
- Measured run 2: 1.376s
- Measured run 3: 1.712s
- Measured run 4: 1.495s
- Measured run 5: 1.564s

**Configuration**: 33 files, 1.00 GiB, 12 workers

Log: `logs/windows/bench_local_windows_20251025T233442Z.log`

---

## 4096 MB Benchmark (ReFS on D: drive - Dev Drive)

**Average blit v2 mirror**: 0.374s
**Average robocopy /MIR**: 0.155s
**Performance**: blit ~2.4x slower than robocopy on ReFS

### Detailed Results - blit v2 mirror
- Warmup run 1: 1.449s (3.06 GiB/s)
- Measured run 1: 0.365s (15.06 GiB/s)
- Measured run 2: 0.393s (14.98 GiB/s)
- Measured run 3: 0.394s (14.76 GiB/s)
- Measured run 4: 0.361s (15.14 GiB/s)
- Measured run 5: 0.358s (14.84 GiB/s)

### Detailed Results - robocopy /MIR
- Warmup run 1: 0.169s
- Measured run 1: 0.199s
- Measured run 2: 0.150s
- Measured run 3: 0.147s
- Measured run 4: 0.140s
- Measured run 5: 0.139s

**Configuration**: 33 files, 4.00 GiB, 12 workers, ReFS filesystem

Log: `logs/windows/bench_local_windows_4gb_20251025T235715Z.log`

---

## Key Observations

1. **ReFS is MUCH faster** than NTFS for both tools (4GB test: 0.374s vs 1.6s for blit)
2. **robocopy dominates on ReFS**: 0.155s vs 0.374s (2.4x faster)
3. **blit competitive on NTFS**: 1.619s vs 1.516s (within 7%)
4. **Throughput on ReFS is impressive**: blit hitting ~15 GiB/s
5. ReFS appears to be optimized for robocopy's access patterns

Note: Used `/COPY:DAT` instead of `/COPYALL` (no elevated permissions for auditing).
