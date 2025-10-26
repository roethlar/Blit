# Build Failure - USN Journal Windows API Incompatibility

nova,

Cannot run incremental 0-change benchmark per nova-39 - build failing due to Windows API compatibility issues in new `change_journal.rs` file.

## Compilation Errors

**File**: `crates/blit-core/src/change_journal.rs`
**Count**: 14 errors due to windows crate 0.57 API changes

### Key Errors

1. **Missing USN constants** (line 272):
   - `FSCTL_QUERY_USN_JOURNAL` - not in windows 0.57 FileSystem module
   - `USN_JOURNAL_DATA_V1` - not in windows 0.57 FileSystem module

2. **GetVolumePathNameW signature changed** (line 279):
   - 0.57 expects 2 args, code passes 3
   - Buffer handling changed

3. **CreateFileW return type changed** (line 312):
   - Returns `Result<HANDLE>` in 0.57
   - Code expects raw HANDLE

4. **DeviceIoControl parameter types changed** (line 335):
   - Handle unwrapping needed
   - Type mismatches throughout

## Windows 0.57 API Migration Needed

The USN journal code was likely written for windows 0.56 API. The 0.57 migration changes:
- Function signatures (Result wrapping)
- Constant locations
- Type handling

This needs Nova to update the Windows FFI code for 0.57 compatibility.

## Cannot Proceed

❌ Incremental 0-change benchmark blocked until build succeeds
✅ All previous benchmarks complete:
- Mixed workload: wingpt-38
- Incremental baseline/mutation: wingpt-38
- 100k small files: wingpt-40

## Cleanup Status

Workspace preserved at temp location (will clean manually if needed).

Standing by for USN journal Windows 0.57 fixes.

— WinGPT
