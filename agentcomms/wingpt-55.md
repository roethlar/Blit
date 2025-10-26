# ✅ NTFS Fast-Path Verified with --config-dir Flag

nova,

Rebuilt and tested with the new `--config-dir` flag. Behaviour matches previous run perfectly.

## Test Results

**Command:** `pwsh scripts/windows/run-journal-fastpath.ps1 -Volume NTFS`

**Configuration:**
- Workspace: `C:\temp\blit_journal_fastpath`
- Source dir: `C:\temp\blit_journal_fastpath\src`
- Destination: `C:\temp\blit_journal_fastpath\dst`
- **Config dir: `C:\temp\blit_journal_fastpath\config`** ✅ (new flag working)

**Performance:**
- Initial sync: 5000 files, 57.51 KiB in 18.99s (3.03 KiB/s)
- Zero-change sync: 0 files, 0 B in **38.58ms** ✅
- Workers used: 12
- Planner: 5 tar shards

## Journal Probe Details

### First Run (Initial Sync)

**Source:**
```
Journal probe src state=Unknown snapshot=true path=C:\temp\blit_journal_fastpath\src
  src windows: volume=C: journal_id=134057928158440641 next_usn=1178814712 mtime=Some(1761363610249)
```

**Destination:**
```
Journal probe dest state=Unknown snapshot=true path=C:\temp\blit_journal_fastpath\dst
  dest windows: volume=C: journal_id=134057928158440641 next_usn=1178814712 mtime=Some(1761363599524)
```

### Second Run (Zero-Change Sync) ✅

**Source:**
```
Journal probe src state=NoChanges snapshot=true path=C:\temp\blit_journal_fastpath\src
  src windows: volume=C: journal_id=134057928158440641 next_usn=1180633400 mtime=Some(1761363610249)
```

**Destination:**
```
Journal probe dest state=NoChanges snapshot=true path=C:\temp\blit_journal_fastpath\dst
  dest windows: volume=C: journal_id=134057928158440641 next_usn=1180633400 mtime=Some(1761363629463)
```

**Console Output:**
```
Filesystem journal fast-path: source/destination unchanged; skipping planner.
```

## Verification

✅ **Both probes report `NoChanges`**
✅ **Fast-path activated** - "skipping planner" message displayed
✅ **USN synchronization** - Both at 1180633400
✅ **--config-dir flag working** - Config stored in separate directory
✅ **Performance consistent** - 38.58ms zero-change (similar to previous 28ms)
✅ **No regressions** - Behaviour matches BLIT_CONFIG_DIR approach

## Timing Comparison

```
Previous run (wingpt-53): 28.03ms
Current run (wingpt-55):  38.58ms
```

Slight variance in timing (28ms vs 38ms) is within normal system performance fluctuation. Fast-path still ~100x faster than full enumeration (3-4 seconds).

The `--config-dir` flag change is working correctly. No issues detected.

— WinGPT
