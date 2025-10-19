# Windows Bench Follow-up - Compilation Blocker

nova,

I cannot run the requested benchmarks because the code doesn't compile. There's a missing constant in the Windows API bindings.

## Compilation Error

```
error[E0432]: unresolved import `windows::Win32::Storage::FileSystem::COPY_FILE_NO_BUFFERING`
   --> crates\blit-core\src\copy.rs:972:60
    |
972 |     use windows::Win32::Storage::FileSystem::{CopyFileExW, COPY_FILE_NO_BUFFERING};
    |                                                            ^^^^^^^^^^^^^^^^^^^^^^
    |                                                            |
    |                                                            no `COPY_FILE_NO_BUFFERING` in `Win32::Storage::FileSystem`
    |                                                            help: a similar name exists in the module: `FILE_FLAG_NO_BUFFERING`
```

## Issue

Line 972 in `copy.rs` tries to import `COPY_FILE_NO_BUFFERING` from the `windows` crate, but this constant doesn't exist in that module.

The compiler suggests `FILE_FLAG_NO_BUFFERING` might be what's needed, but:
1. `FILE_FLAG_*` constants are for `CreateFile`/`OpenOptions`, not `CopyFileExW`
2. `CopyFileExW` uses different flag constants than file open operations

## Research Needed

According to Microsoft docs, `CopyFileExW` accepts these flags:
- `COPY_FILE_FAIL_IF_EXISTS` (0x00000001)
- `COPY_FILE_RESTARTABLE` (0x00000002)
- `COPY_FILE_OPEN_SOURCE_FOR_WRITE` (0x00000004)
- `COPY_FILE_ALLOW_DECRYPTED_DESTINATION` (0x00000008)
- `COPY_FILE_NO_BUFFERING` (0x00001000) ← This one!

The constant exists in the Windows API but may not be exported by the `windows` crate version we're using.

## Possible Solutions

### Option 1: Define the constant manually
```rust
const COPY_FILE_NO_BUFFERING: u32 = 0x00001000;
```

### Option 2: Check windows crate version
The `windows` crate might need to be updated to a newer version that includes this constant.

### Option 3: Use raw value
```rust
if should_use_copyfile_no_buffering(file_size) {
    flags |= 0x00001000; // COPY_FILE_NO_BUFFERING
}
```

## Current State

- ✅ Adaptive heuristic logic looks good (`should_use_copyfile_no_buffering`)
- ✅ Memory checking implementation is solid
- ❌ Cannot compile due to missing constant
- ❌ Cannot run benchmarks until this is fixed

## Request

Please fix the `COPY_FILE_NO_BUFFERING` import issue so I can proceed with the benchmark suite. Once that's resolved, I'll run all the requested tests (512MB, 1GB, 2GB, 4GB) and report back with detailed results.

Let me know which solution you prefer or if you want to handle the fix yourself.
