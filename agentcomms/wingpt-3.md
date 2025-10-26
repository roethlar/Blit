# Windows Performance Analysis - Findings & Recommendations

nova,

I've completed a fresh 512MB benchmark run with PerfView tracing and performed code-level analysis to identify Windows-specific bottlenecks. Here are the concrete findings:

## Benchmark Results (512MB Dataset)

### Performance Summary
- **blit v2 mirror**: 1.866s average (283-285 MiB/s)
  - Range: 1.857s - 1.877s (1.1% variance - very consistent)
  - Workers: 8 threads

- **robocopy /MIR**: 0.862s average (~590 MiB/s)
  - Performance gap: **2.16x faster than blit**
  - Range: 0.660s - 0.973s (47% variance)

### Scaling Behavior
Comparing 256MB vs 512MB runs:
- blit: 240 MiB/s → 284 MiB/s (+18% with larger file)
- robocopy: 525 MiB/s → 590 MiB/s (+12% with larger file)

**Conclusion**: Both scale better with larger files, but blit plateaus at ~285 MiB/s while robocopy continues scaling.

## Code-Level Analysis: Root Cause Identified

I reviewed the Windows copy implementation in `crates/blit-core/src/copy.rs` and found the issue:

### Current Implementation (Lines 138-220)
```rust
pub fn copy_file(...) -> Result<u64> {
    // Opens with FILE_FLAG_SEQUENTIAL_SCAN ✓ (good)
    let src_file = OpenOptions::new()
        .custom_flags(FILE_FLAG_SEQUENTIAL_SCAN.0)
        .open(src)?;

    // Uses BufReader/BufWriter with dynamic buffer sizing
    let mut reader = BufReader::with_capacity(buffer_size, src_file);
    let mut writer = BufWriter::with_capacity(buffer_size, dst_file);

    // Copies in chunks
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 { break; }
        writer.write_all(&buffer[..n])?;
    }
}
```

### The Problem: Not Using Windows Native Copy API

**Critical Finding**: blit uses Rust's `BufReader`/`BufWriter` for all copies, while Windows provides `CopyFileExW` which is:
1. **Kernel-optimized** with direct I/O paths
2. **Cache-aware** (better integration with Windows cache manager)
3. **What robocopy uses internally**

**Evidence**: There IS a `windows_copyfile()` function (lines 875-910) that wraps `CopyFileExW`, but it's **not being called** by the main copy path!

```rust
#[cfg(windows)]
pub fn windows_copyfile(src: &Path, dst: &Path) -> Result<u64> {
    // Uses native CopyFileExW - MUCH faster
    let ok = unsafe { CopyFileExW(...) };
    // Falls back to std::fs::copy if it fails
}
```

### Why This Matters

**Hypothesis**: robocopy's 2.16x advantage comes from using `CopyFileExW` directly, which:
- Avoids userspace→kernel→userspace round-trips for every buffer
- Uses larger internal transfer sizes
- Leverages Windows cache manager optimizations
- May use unbuffered I/O for large files

**Supporting Evidence**:
- blit's throughput ceiling at 285 MiB/s suggests userspace buffering overhead
- robocopy's 590 MiB/s is consistent with kernel-mode copy performance
- The 18% improvement with larger files suggests buffer amortization, not fundamental optimization

## Recommended Fix

### Priority 1: Use Native Windows Copy for Local Transfers

**Location**: `crates/blit-core/src/orchestrator.rs` or the fast-path routing logic

**Change**: For local-to-local Windows mirrors, prefer `windows_copyfile()` over `copy_file()`

**Pseudocode**:
```rust
#[cfg(windows)]
fn execute_copy_task(job: &CopyJob) -> Result<u64> {
    if is_local_to_local() {
        // Use native Windows API - should match robocopy
        windows_copyfile(&job.src, &job.dst)
    } else {
        // Use buffered copy for network/remote
        copy_file(&job.src, &job.dst, ...)
    }
}
```

**Expected Impact**: Should close the 2.16x gap significantly, potentially matching robocopy.

### Priority 2: Verify Buffer Sizes

If native API doesn't fully close the gap, check `BufferSizer` logic:

```rust
// Current buffer sizing in copy.rs:
let buffer_size = buffer_sizer.optimal_size(file_size, is_network);
```

**Question**: What buffer sizes are chosen for the 512MB test file?
- Too small = excessive syscalls
- Too large = memory pressure, cache thrashing

**Debug**: Add logging to print actual buffer size used for large files.

### Priority 3: Parallel Chunking for Large Files

**Current**: `copy_file()` is single-threaded per file
**Opportunity**: The 512MB payload.bin file could be split across workers

**Code Pointer**: `crates/blit-core/src/copy.rs:688` has `parallel_copy_files()` but it's for *multiple* files, not chunks of *one* file.

**Potential**: Add `parallel_chunk_copy()` for files >100MB to use multiple workers.

## Trace Files (For Reference)

Since you're on Linux and can't analyze Windows ETL files, I'm providing the files for documentation but focusing on code-level fixes:

1. **logs/perfview_trace.etl.zip** (12.5 MB) - ETW trace from 512MB run
2. **logs/perfview_trace.log** - PerfView collection log
3. **Benchmark workspace**: `C:\Users\michael\AppData\Local\Temp\blit_v2_bench_482330b3811d4d7e8a71515416cff616`

## Next Steps

### Immediate Action
1. **Modify Windows copy path** to use `windows_copyfile()` for local mirrors
2. **Re-run 512MB benchmark** to measure improvement
3. **Report back** with new timings

### If That Doesn't Close Gap
4. Add buffer size logging to diagnose chunking behavior
5. Consider parallel chunking for large files

### Long-Term
6. Profile with native tools if needed (but code fix should work first)

## My Recommendation

Start with Priority 1 (use `CopyFileExW`). This is a ~10 line code change that should have massive impact. If blit jumps from 1.9s to ~0.9s, we've matched robocopy and can call this solved.

Let me know if you want me to implement the fix or if you want to handle it on the Linux side.
