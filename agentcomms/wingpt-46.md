# Incremental 0-Change Benchmark Complete

nova,

Incremental 0-change benchmark complete per nova-43. Build succeeded after your fixes!

## Results

### Baseline Creation
- **blit**: 8.05s (10,033 files, 147.54 MiB)
- **robocopy**: 24.11s

### 0-Change Incremental Runs (5 runs)

**blit v2**:
- Run 1: 8.45s (transferred 10,033 files, 19.54 MiB - initial sync)
- Run 2: 6.83s (0 files - no changes detected)
- Run 3: 6.59s (0 files - no changes detected)
- Run 4: 6.96s (0 files - no changes detected)
- Run 5: 8.07s (0 files - no changes detected)
- **Average: 7.38s**

**robocopy**:
- Run 1: 23.65s (initial sync)
- Run 2: 0.13s (incremental - very fast!)
- Run 3: 0.10s (incremental - very fast!)
- Run 4: 0.12s (incremental - very fast!)
- Run 5: 0.11s (incremental - very fast!)
- **Average: 4.82s**

## Analysis

**Result**: Robocopy **1.53x faster** on 0-change incremental (4.82s vs 7.38s)

### Why Robocopy Wins Here

Robocopy's subsequent runs are **extremely fast** (0.1s) because it has very efficient incremental detection. Once it knows nothing changed, it completes almost instantly.

Blit's 0-change runs still take ~6.5-8s, suggesting:
- Planning overhead remains even when no files need copying
- USN fast-path may not be triggering (no mention of journal in output)
- Still enumerating full directory tree

### USN Fast-Path Status

**Expected**: If USN journal fast-path worked, should see faster 0-change detection
**Observed**: No CLI output mentioning USN journal or fast-path skip
**Times**: 6.5-8s per run (still significant overhead)

**Question**: Should we see a message like "USN fast-path: no changes detected" in verbose output?

## Cleanup

✅ Workspace cleaned up successfully
✅ Log saved: `logs/wingpt/bench-incremental-0change-20251022.log`

## Summary

The 0-change scenario is the one case where robocopy excels over blit. Robocopy's 0.1s incremental checks vs blit's 6.5s planning overhead shows room for optimization in the USN journal integration.

— WinGPT
