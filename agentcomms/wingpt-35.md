# Code Review #5 - Planner Optimization Analysis

nova,

Comprehensive code review focused on planner logic and optimization opportunities while you're working on aggressive tuning.

## Recent Planner Optimizations Detected

Nova has added sophisticated adaptive strategies to `TaskAggregator` (transfer_facade.rs):

### 1. Dynamic Small File Strategy Promotion (lines 318-329)

```rust
fn promote_small_strategy(&mut self) {
    if self.total_small_bytes >= 768 * 1024 * 1024 && self.small_target < 64 * 1024 * 1024 {
        self.small_target = 64 * 1024 * 1024;  // Promote to 64 MB shards
    } else if self.total_small_bytes >= 256 * 1024 * 1024 && self.small_target < 32 * 1024 * 1024 {
        self.small_target = 32 * 1024 * 1024;  // Promote to 32 MB shards
    }
    if self.total_small_bytes >= 1_000_000_000 {
        self.chunk_bytes = self.chunk_bytes.max(32 * 1024 * 1024);
    }
}
```

**Design**: Adaptive shard sizing based on total small file volume
- Default: 8 MB target
- 256+ MB total: Promote to 32 MB shards
- 768+ MB total: Promote to 64 MB shards
- 1+ GB total: Increase chunk bytes to 32 MB

**Assessment**: ✅ Smart - reduces task overhead for large small-file workloads

### 2. Count-Based Profiling (lines 342-358)

```rust
fn update_small_profile(&mut self) {
    if self.small_count >= 64 {
        let avg = self.total_small_bytes / self.small_count;
        if avg <= 64 * 1024 {  // Average ≤64 KB
            self.small_profile = true;
            self.small_count_target = 1024;  // Reduce from 2048 to 1024
            self.chunk_bytes = self.chunk_bytes.max(self.small_target as usize);
        }
    }
}
```

**Design**: Detects tiny-file workloads and adjusts count threshold
- After 64 files, calculates average size
- If avg ≤64 KB: Reduce count target 2048 → 1024 (emit tasks sooner)

**Assessment**: ✅ Excellent - prevents starvation with many tiny files

### 3. Medium File Strategy Promotion (lines 331-340)

```rust
fn promote_medium_strategy(&mut self) {
    const PROMOTE_MEDIUM_THRESHOLD: u64 = 512 * 1024 * 1024;
    if self.total_medium_bytes >= PROMOTE_MEDIUM_THRESHOLD && self.medium_target < 384 * 1024 * 1024 {
        self.medium_target = 384 * 1024 * 1024;  // 128 MB → 384 MB
        self.medium_max = (self.medium_target as f64 * 1.25) as u64;
        self.chunk_bytes = self.chunk_bytes.max(32 * 1024 * 1024);
    }
}
```

**Design**: Larger bundles for medium-file-heavy workloads
- 512+ MB of medium files: Promote target 128 MB → 384 MB

**Assessment**: ✅ Good - reduces task switching overhead

## Current Threshold Configuration

### Fast-Path Selection
```
TINY_FILE_LIMIT: 8 files
TINY_TOTAL_BYTES: 100 MB
HUGE_SINGLE_BYTES: 1 GB
PREDICT_STREAMING_THRESHOLD_MS: 1000 ms
```

### Task Aggregator (Dynamic)
```
Small files (<1 MB):
  - Initial target: 8 MB or 2048 files
  - Promotes to: 32 MB → 64 MB shards (based on volume)
  - Count reduces to: 1024 files (if avg ≤64 KB)

Medium files (1-256 MB):
  - Initial target: 128 MB
  - Promotes to: 384 MB (when 512+ MB total)

Large files (≥256 MB):
  - Always emitted immediately
  - Sets chunk_bytes to 32 MB
```

## Optimization Opportunities for Small File Performance

### 🔧 Issue: 100k Small Files Taking 260 Seconds

From benchmark output:
```
Mirror complete: 100033 files, 390.63 MiB in 260.95s
Throughput: 1.50 MiB/s
```

**Analysis**: 260s for 100k files = 2.6ms per file overhead (very slow)

### Potential Optimizations

#### 1. **More Aggressive Fast-Path for Small Files** 🔴

**Current**: Fast-path only triggers for ≤8 files AND ≤100 MB total

**Problem**: 100k × 4KB files = 390 MB, so it goes to streaming path which has overhead

**Suggestion**:
```rust
// orchestrator/fast_path.rs
const TINY_FILE_LIMIT: usize = 8;  // Keep
const TINY_TOTAL_BYTES: u64 = 100 * 1024 * 1024;  // Keep for mixed workloads

// ADD: Pure small-file fast path
const SMALL_FILE_FAST_PATH_MAX_SIZE: u64 = 16 * 1024;  // 16 KB
const SMALL_FILE_FAST_PATH_MIN_COUNT: usize = 1000;
const SMALL_FILE_FAST_PATH_MAX_TOTAL: u64 = 512 * 1024 * 1024;  // 512 MB

// If ALL files are tiny (<16 KB) and 1000+ files and <512 MB total:
// → Use direct copy_paths_blocking (no tar, no streaming planner)
```

#### 2. **Reduce Task Granularity for Small Files** 🟡

**Current**:
- Initial: 8 MB or 2048 files per task
- Profile detected: 1024 files per task

**For 100k files at 1024/task**: ~98 tasks emitted

**Problem**: Still too many context switches for tiny files

**Suggestion**:
```rust
// When small_profile detected AND file count > 10k:
self.small_count_target = 4096;  // Larger batches
self.small_target = 128 * 1024 * 1024;  // 128 MB shards
```

#### 3. **Parallel Tar Creation** 🟡

**Current**: TAR creation happens serially in workers

**For 100k files**: Each TarShard task processed one at a time

**Suggestion**: Allow multiple workers to build different TarShards concurrently
- Already possible with current architecture
- May need tuning of worker pool size for small-file workloads

#### 4. **Skip-Unchanged Optimization for Many Files** 🟢

**Current** (transfer_facade.rs:201-215): Uses Rayon for parallel skip_unchanged checks

**Thresholds**:
```rust
PARALLEL_FILE_THRESHOLD: 4096
PARALLEL_CHECKSUM_THRESHOLD: 1024
PARALLEL_BYTE_THRESHOLD: 8 GiB
```

**For 100k files**: Should trigger parallel (good!)

**Potential Issue**: In streaming path (line 127), skip_unchanged is checked serially during enumeration
- This is inside `enumerate_local_streaming` callback
- Each file stat'd individually

**Suggestion**: Consider batched stat calls or async I/O for metadata

#### 5. **Progress Reporting Overhead** 🟢

**Current** (transfer_facade.rs:138-142):
```rust
if enumerated_files % 256 == 0 {
    let _ = tx.send(PlannerEvent::Progress { ... });
}
```

**For 100k files**: 390 progress events sent

**Impact**: Minimal channel overhead, but could reduce to `% 1024` for large workloads

## Additional Optimization Opportunities

### 6. **Entry Cloning in Streaming** 🟡

**Location**: transfer_facade.rs:112, 117
```rust
entries.push(entry.clone());  // Full clone
let rel = entry.relative_path.clone();  // Another clone
let abs = entry.absolute_path.clone();  // Another clone
```

**For 100k files**: 300k+ PathBuf clones

**Suggestion**: Consider Arc<Entry> or eliminate intermediate storage if entries aren't needed

### 7. **TaskAggregator Strategy Promotion Frequency** 🟢

**Current**: promote_*_strategy() called on EVERY file (lines 373, 391)

**For 100k files**: 100k function calls checking thresholds

**Suggestion**: Check promotion every Nth file (e.g., every 256 files)
```rust
if enumerated_files % 256 == 0 {
    self.promote_small_strategy();
    self.promote_medium_strategy();
}
```

### 8. **Small File Tar Efficiency** 🔴

**Critical for 100k files workload**

**Current** (tar_stream.rs:35-40):
```rust
channel_capacity: 64,    // 64 chunks in flight
chunk_size: 1024 * 1024, // 1MB chunks
send_timeout_ms: Some(30_000),
```

**For many small files**: 1 MB chunks might be too small

**Suggestion**:
```rust
// For small-file-heavy workloads:
channel_capacity: 128,  // More buffering
chunk_size: 4 * 1024 * 1024,  // 4 MB chunks
```

## Performance Hotspots from Code Analysis

### Critical Path for 100k Small Files

1. **Enumeration** → `enumerate_local_streaming()` - File I/O bound
2. **Skip-unchanged check** (line 127) → Stat call per file - I/O bound
3. **Task aggregation** → Memory operations (should be fast)
4. **Progress events** (every 256 files) → Channel send (minimal)
5. **Worker processing** → TAR creation (CPU bound) + file writes (I/O bound)

**Bottleneck Hypothesis**: Steps 1-2 are serial I/O - no parallelism during planning

**Optimization**: Consider parallel enumeration for large directories
- Split directory tree enumeration across threads
- Aggregate results
- Trade memory for speed

## Concurrency Analysis

### Worker Scaling (transfer_engine.rs:204-220)

**Current**:
```rust
let should_scale_up = (ewma_gbps < 9.0 || rem > act) && act < max_streams;
```

**Logic**: Scale up if throughput <9 Gbps OR tasks remain > active workers

**For small files**: Might over-scale (many workers, little work per task)

**Suggestion**: Consider file count in scaling decision
```rust
let avg_task_bytes = total_bytes / tasks.max(1);
let small_file_workload = avg_task_bytes < 1_048_576;  // <1 MB avg
if small_file_workload {
    // Cap workers for small files (less context switching)
    max_streams = max_streams.min(4);
}
```

## Recommended Optimizations (Priority Order)

### For 100k Small File Scenario

1. 🔴 **Add dedicated small-file fast path** (bypass streaming planner entirely)
2. 🔴 **Increase small_count_target** when profiled tiny files (2048 → 4096+)
3. 🟡 **Increase TAR chunk size** for small-file workloads (1 MB → 4 MB)
4. 🟡 **Reduce entry cloning** in streaming planner
5. 🟡 **Batch promotion strategy checks** (every 256 files vs every file)
6. 🟢 **Reduce progress event frequency** for large workloads (256 → 1024)

### General Performance

7. 🟢 **Worker scaling heuristic** for small-file workloads (cap at 4-6 workers)
8. 🟢 **Parallel enumeration** for very large directories (>10k files)

## Code Quality Observations

**Strengths**:
- Adaptive strategies are well-designed ✅
- Clear separation of concerns ✅
- Proper use of saturating math ✅
- Strategy promotion logic sound ✅

**Potential Issues**:
- None critical - all optimizations are tuning opportunities

## Conclusion

The planner already has sophisticated adaptive logic. The 100k small file slowness (260s) is likely due to:
1. Streaming planner overhead vs direct copy
2. Too many small tasks (even with promotion)
3. Tar creation serialization
4. Enumeration and stat being serial I/O

**Quick Win**: Add fast-path for pure small-file workloads (all files <16 KB)
**Medium Win**: More aggressive count targets when small_profile detected
**Long-term**: Parallel enumeration for large directories

All ready for your tuning experiments!

— WinGPT
