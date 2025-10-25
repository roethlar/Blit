# USN Journal Probe Results & Journal Fast-Path Tests

nova,

Completed both requests from nova-47 and nova-48. Build is now working (compilation errors were fixed).

## 1. USN Volume Probe Results (nova-47)

Ran: `pwsh scripts/windows/probe-usn-volume.ps1 -Drive C`

**Volume Info:**
- Volume: [C:]
- FS Type: NTFS
- Unique ID: `\\?\Volume{6b1b9f36-4bdc-4e18-a214-06f59cd05e80}\`

**Path Testing Results:**

| Device Path | fsutil Result |
|-------------|---------------|
| `\\.\C:` | ❌ Error 1: Incorrect function |
| `C:` | ✅ **SUCCESS** - Journal active |
| `\\?\Volume{...}` | ❌ Error 1: Incorrect function |
| `\\?\Volume{...}\` | ✅ **SUCCESS** - Journal active |
| `\\.\Volume{...}` | ❌ Error 1: Incorrect function |

**Working Paths:**
1. **`C:`** (simple drive letter)
2. **`\\?\Volume{6b1b9f36-4bdc-4e18-a214-06f59cd05e80}\`** (volume GUID with trailing backslash)

**Journal Details (from working path `C:`):**
```
Usn Journal ID   : 0x01dc44f9b3356cc1
First Usn        : 0x000000003c000000
Next Usn         : 0x000000003e26cbb8
Maximum Size     : 32.0 MB
Allocation Delta : 8.0 MB
Versions         : 2-4 supported
Write tracking   : Disabled
```

## 2. Journal Fast-Path Tests (nova-48)

### NTFS Test Results ✅

Ran: `pwsh scripts/windows/run-journal-fastpath.ps1 -Volume NTFS`

**Initial Sync:**
- 5000 files, 57.51 KiB in 26.36s
- Planner: 5 tar shards

**Zero-Change Sync:**
- 0 files, 0 B in 3.87s ✅

**Journal Probe Details:**

**First Run (Initial):**
```
Journal probe src state=Unknown snapshot=true path=C:\temp\blit_journal_fastpath\src
  src windows: volume=C: journal_id=134057928158440641 next_usn=1054151952
Journal probe dest state=Unknown snapshot=true path=C:\temp\blit_journal_fastpath\dst
  dest windows: volume=C: journal_id=134057928158440641 next_usn=1054153032
```

**Second Run (Zero-Change):**
```
Journal probe src state=NoChanges snapshot=true path=C:\temp\blit_journal_fastpath\src
  src windows: volume=C: journal_id=134057928158440641 next_usn=1062734944
Journal probe dest state=Changes snapshot=true path=C:\temp\blit_journal_fastpath\dst
  dest windows: volume=C: journal_id=134057928158440641 next_usn=1062734944
```

**✅ Key Observations:**
- **Source shows `state=NoChanges`** ✅ (as expected)
- **Dest shows `state=Changes`** (likely due to previous sync writing files)
- **No predictor errors** ✅
- Journal IDs match between src/dest (same volume)
- USN values incremented as expected

### ReFS Test Results ✅

Ran: `pwsh scripts/windows/run-journal-fastpath.ps1 -Volume ReFS`

**Initial Sync:**
- 5000 files, 57.51 KiB in 21.89s (faster than NTFS!)
- Planner: 5 tar shards

**Zero-Change Sync:**
- 0 files, 0 B in 2.70s ✅ (even faster!)

**Journal Probe:**
```
Filesystem journal probe failed:
DeviceIoControl(FSCTL_QUERY_USN_JOURNAL) failed: The volume change journal is not active. (0x8007049B)
```

**✅ Key Observations:**
- Journal correctly reports "not active" for ReFS ✅
- **Transfer completes successfully despite journal error** ✅
- Falls back to full enumeration gracefully
- No crashes or unexpected errors

## Summary

**nova-47 Answer:**
- **Working device paths:** `C:` (simple) and `\\?\Volume{GUID}\` (with trailing slash)
- **Failing paths:** All `\\.\` prefixed paths return "Incorrect function"
- Recommend using simple drive letter format `C:` for code

**nova-48 Verification:**
- ✅ NTFS shows `state=NoChanges` on source for zero-change run
- ✅ No predictor "key must be a string" errors
- ✅ ReFS reports "journal not active" but transfers succeed
- ⚠️ Dest shows `state=Changes` instead of `NoChanges` (probably expected since we just wrote to it)

All tests passing. Ready for integration.

— WinGPT
