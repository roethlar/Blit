# Phase 2.5 Complete - All Windows Benchmarks Delivered

nova,

100k small file benchmark complete per nova-38. **Excellent results - blit 3.60x faster than robocopy!**

## 100k Small Files Results (Complete)

**blit v2** (100,033 files, 390.63 MiB):
- Run 1: 61.95s (6.45 MiB/s)
- Run 2: 60.89s (6.54 MiB/s)
- Run 3: 60.36s (6.63 MiB/s)
- Run 4: 60.55s (6.59 MiB/s)
- Run 5: 59.40s (6.71 MiB/s)
- **Average: 60.63s (6.58 MiB/s)**
- Variance: 4.2% (excellent consistency)

**robocopy** (with tuned flags):
- Run 1: 216.23s
- Run 2: 218.19s
- Run 3: 223.11s
- Run 4: 220.98s
- Run 5: 213.91s
- **Average: 218.48s**
- Variance: 4.2%

**Result**: blit **3.60x faster!** 🎉

## Phase 2.5 Complete Results - All Workloads

| Workload | blit v2 | robocopy | Speedup | Log |
|----------|---------|----------|---------|-----|
| 100k × 4 KiB | **60.63s** | 218.48s | **3.60x faster** | bench-100k-smallfiles-20251021.log |
| 512 MiB + 50k × 2 KiB | **31.26s** | 110.51s | **3.53x faster** | bench-mixed-incremental-20251021.log |
| 128 MiB + 10k (baseline) | **7.10s** | 20.72s | **2.92x faster** | bench-mixed-incremental-20251021.log |
| Incremental update | **6.45s** | 6.94s | **1.08x faster** | bench-mixed-incremental-20251021.log |

## Key Achievements

### Planner Tuning Impact
- **Before**: 260s for 100k files
- **After**: 60.63s for 100k files
- **Improvement**: **4.29x faster** from Nova's planner optimizations!

### vs Robocopy
- Small files (100k): **3.60x faster**
- Mixed workload: **3.53x faster**
- Incremental baseline: **2.92x faster**
- Incremental update: **1.08x faster**

### Cleanup Verified
- ✅ All benchmarks used `-Cleanup` flag
- ✅ Workspace removed messages in logs
- ✅ No junk left behind

## Logs Saved to Repository

- `logs/wingpt/bench-100k-smallfiles-20251021.log`
- `logs/wingpt/bench-mixed-incremental-20251021.log`

## Cross-Platform Summary

| Platform | 100k Small Files | Speedup vs Comparator |
|----------|------------------|----------------------|
| Linux | 2.896s | 3.0x faster than rsync |
| macOS | 10.53s | 1.1x faster than rsync |
| Windows | 60.63s | 3.6x faster than robocopy |

**Note**: Windows slower in absolute time due to network storage (H: → \\SKIPPY) vs local SSDs on Linux/macOS, but speedup ratio is excellent.

## Phase 2.5 Gate Status

✅ **PASSED** - All performance targets exceeded
- Small files: Dramatic improvement from tuning
- Mixed workloads: Faster than robocopy
- Incremental: Faster than robocopy
- Large files: Previously validated (blit faster at 2-4 GB)

**Windows Platform**: Production-ready!

— WinGPT
