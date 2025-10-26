# Third NTFS Journal Fast-Path Test Results

nova,

Ran the third NTFS pass as requested. Destination still shows `state=Changes` - details below.

## Test Results

**Command:** `pwsh scripts/windows/run-journal-fastpath.ps1 -Volume NTFS`

**Initial Sync:**
- 5000 files, 57.51 KiB in 29.14s
- Workers: 12
- Planner: 5 tar shards

**Zero-Change Sync:**
- 0 files, 0 B in 3.77s ✅
- Workers: 12

## Journal Probe Details

### First Run (Initial Sync)

**Source:**
```
Journal probe src state=Unknown snapshot=true path=C:\temp\blit_journal_fastpath\src
  src windows: volume=C: journal_id=134057928158440641 next_usn=1117670280 mtime=Some(1761357516612)
```

**Destination:**
```
Journal probe dest state=Unknown snapshot=true path=C:\temp\blit_journal_fastpath\dst
  dest windows: volume=C: journal_id=134057928158440641 next_usn=1117672240 mtime=Some(1761357500397)
```

**USN Delta (Initial):** 1,960 (dest was ahead by 1960 USN)

### Second Run (Zero-Change Sync)

**Source:**
```
Journal probe src state=NoChanges snapshot=true path=C:\temp\blit_journal_fastpath\src
  src windows: volume=C: journal_id=134057928158440641 next_usn=1126730336 mtime=Some(1761357516612)
```

**Destination:**
```
Journal probe dest state=Changes snapshot=true path=C:\temp\blit_journal_fastpath\dst
  dest windows: volume=C: journal_id=134057928158440641 next_usn=1126730336 mtime=Some(1761357545714)
```

**USN Delta (Zero-Change):** 0 (both at 1126730336) ✅

## Analysis

**Good News:**
- ✅ Source correctly shows `state=NoChanges` on zero-change run
- ✅ USN values are now synchronized (both at 1126730336)
- ✅ Transfer completes successfully with 0 files

**Issue:**
- ❌ Destination still shows `state=Changes` despite:
  - USN values matching exactly
  - No actual file changes
  - mtime updated to 1761357545714 (45 seconds after initial 1761357500397)

**USN Progression:**
```
Initial → Zero-Change
Src:   1117670280 → 1126730336 (delta: +9,060,056)
Dest:  1117672240 → 1126730336 (delta: +9,058,096)
```

**Observation:** The destination's mtime changed from the initial run (1761357500397) to the zero-change run (1761357545714), which is likely triggering the `Changes` state even though USNs are synchronized.

## Hypothesis

The checkpoint logic may be comparing mtime in addition to USN. Since the zero-change sync updates the destination directory's mtime (even though no files changed), it's flagged as `Changes`.

Possible fixes:
1. Compare USN values only (ignore mtime if USNs match)
2. Update checkpoint after sync to capture the new mtime
3. Consider a USN-only fast path when mtimes differ but USNs are equal

Let me know if you need any additional diagnostics or if I should test a specific scenario.

— WinGPT
