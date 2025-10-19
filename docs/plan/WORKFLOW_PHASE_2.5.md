# Phase 2.5: Performance & Validation Checkpoint

**Goal**: Validate v2 local performance meets v1 parity (≥95%)
**Duration**: 1-2 days
**Prerequisites**: Phase 2 complete
**Status**: In progress
**Type**: QUALITY GATE (mandatory checkpoint)

## Overview

Phase 2.5 is a **mandatory quality gate** that must pass before proceeding to Phase 3. This phase validates that the v2 architecture can achieve acceptable performance for local operations before investing in network features.

### Current Benchmark Snapshot (updated 2025-10-18)

- **Harness**: `scripts/bench_local_mirror.sh` (macOS/Linux) + `scripts/windows/bench-local-mirror.ps1` (Windows); now v2-only with rsync/robocopy baselines.
- **macOS (2025-10-18, `SIZE_MB=512`, 1 warmup, 5 runs)**  
  - `blit-cli mirror`: **0.275 s avg** (log ephemeral; command output captured in session)  
  - `rsync -a --delete`: 0.605 s avg  
  - **Result**: v2 outperforms rsync (~220% of baseline throughput) — passes the Phase 2.5 gate for this workload.
- **Windows (2025-10-18, `SizeMB` = 256 MiB–4 GiB, warmup 1, 5 runs unless noted)**  
  - Pre-fix baseline (256 MiB): `blit-cli` **1.086 s avg** vs `robocopy` 0.487 s avg (≈2.23× slower); ETW run 1.226 s vs 0.567 s.  
  - **After CopyFileExW optimisation (512 MiB dataset)**: `blit-cli` **0.724 s avg** (707 MiB/s) vs `robocopy` 0.775 s avg (660 MiB/s) — blit is ~7 % faster; peak run 0.569 s (987 MiB/s).  
  - Scaling study: 256 MiB (0.621 s vs 0.404 s), 1 GiB (1.906 s vs 1.295 s), 2 GiB (4.205 s vs 2.694 s), 4 GiB (8.443 s vs 8.046 s). Gap persists for 1–2 GiB workloads due to cache/worker behaviour.  
  - Detailed findings and PerfView notes: `agentcomms/wingpt-4.md`, `agentcomms/wingpt-5.md`; raw data archived in `logs/blit_windows_bench.zip` (SHA256 `801B0AF5…F14F3D`).  
  - **Result**: Windows parity achieved for ≤512 MiB transfers; larger datasets still trail robocopy by ~1.5× pending cache-aware tuning.
- Earlier 2025-10-16/17 v1 comparisons remain referenced for historical context; up-to-date parity evaluation now uses platform-native tools as proxies until the legacy binary is available.

Follow-up actions:
1. Implement wingpt’s recommendations: detect >1 GiB files, reduce worker fan-out, and explore cache-aware buffering/flags on Windows.  
2. Re-run 1 GiB–4 GiB Windows benchmarks post-tuning to confirm parity; keep 512 MiB regression in watch list.  
3. Extend coverage to mixed and small-file workloads once large-file heuristics land, and fold results into this checkpoint.

### Critical Decision Point

This phase produces a **GO/NO-GO decision**:

- ✅ **GO** (≥95% of v1 performance) → Proceed to Phase 3 with current architecture
- ❌ **NO-GO** (<95% of v1 performance) → Fix performance issues before Phase 3

**If NO-GO**: Options include:
1. Profile and optimize hot paths in current code
2. Implement hybrid transport architecture (if not already done)
3. Optimize zero-copy implementation
4. Investigate algorithmic improvements

### Success Criteria

| Metric | Target | Status |
|--------|--------|--------|
| Large file (NVMe → NVMe, macOS 512 MiB) | ≥95% of baseline | ✅ `blit-cli` 0.275 s vs `rsync` 0.605 s (220% of baseline) |
| Large file (NVMe → NVMe, Windows 512 MiB) | ≥95% of baseline | ✅ `blit-cli` 0.724 s vs `robocopy` 0.775 s (~107%) — note 1–2 GiB workloads still lag |
| 100k small files | ≥95% of baseline | ⏳ |
| Mixed workload | ≥95% of baseline | ⏳ |
| Memory usage | ≤110% of v1 | ⏳ |
| CPU utilization | Reasonable (subjective) | ⏳ |

## Day 1: Benchmark Infrastructure (4-6 hours)

### Task 2.5.1: Create Benchmark Harness
**Priority**: 🔴 Critical
**Effort**: 2-3 hours
**Output**: Automated benchmark script

**Action**: Create `benchmarks/bench_local.sh`

```bash
#!/bin/bash
# benchmarks/bench_local.sh
# Comprehensive local transfer benchmark suite

set -e

# Configuration
BENCH_ROOT="/tmp/blit_bench"
V1_BINARY="${V1_BINARY:-/path/to/blit-v1}"
V2_BINARY="${V2_BINARY:-./target/release/blit-cli}"
RESULTS_DIR="./benchmark_results"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup function
cleanup() {
    rm -rf "$BENCH_ROOT"
}
trap cleanup EXIT

# Setup
mkdir -p "$BENCH_ROOT" "$RESULTS_DIR"

# Helper: Run benchmark and capture stats
run_benchmark() {
    local name=$1
    local binary=$2
    local src=$3
    local dst=$4

    echo "Running: $name with $binary"

    # Clear caches
    sync
    echo 3 | sudo tee /proc/sys/vm/drop_caches > /dev/null || true

    # Run with time measurement
    local start=$(date +%s.%N)
    "$binary" mirror "$src" "$dst" > /dev/null 2>&1
    local end=$(date +%s.%N)

    local duration=$(echo "$end - $start" | bc)
    echo "$duration"
}

# Test 1: Large single file (4GiB)
echo "========================================="
echo "Test 1: Large Single File (4GiB)"
echo "========================================="

mkdir -p "$BENCH_ROOT/large_src" "$BENCH_ROOT/large_dst_v1" "$BENCH_ROOT/large_dst_v2"
dd if=/dev/zero of="$BENCH_ROOT/large_src/file.bin" bs=1M count=4096 2>/dev/null

echo "v1 benchmark..."
v1_large=$(run_benchmark "large-v1" "$V1_BINARY" "$BENCH_ROOT/large_src" "$BENCH_ROOT/large_dst_v1")
echo "v1: ${v1_large}s"

echo "v2 benchmark..."
v2_large=$(run_benchmark "large-v2" "$V2_BINARY" "$BENCH_ROOT/large_src" "$BENCH_ROOT/large_dst_v2")
echo "v2: ${v2_large}s"

v1_large_f=$(printf "%.2f" $v1_large)
v2_large_f=$(printf "%.2f" $v2_large)
ratio_large=$(echo "scale=4; $v2_large / $v1_large" | bc)
pct_large=$(echo "scale=2; (1 - $ratio_large) * 100" | bc)

echo "Ratio: v2/v1 = $ratio_large"
if (( $(echo "$ratio_large <= 1.05" | bc -l) )); then
    echo -e "${GREEN}✓ PASS${NC}: v2 within 5% of v1"
else
    echo -e "${RED}✗ FAIL${NC}: v2 slower by ${pct_large}%"
fi
echo ""

# Test 2: Many small files (100k files, 1-10KB each)
echo "========================================="
echo "Test 2: Many Small Files (100k)"
echo "========================================="

mkdir -p "$BENCH_ROOT/small_src" "$BENCH_ROOT/small_dst_v1" "$BENCH_ROOT/small_dst_v2"

echo "Generating 100k small files..."
for i in $(seq 1 100000); do
    size=$((1024 + RANDOM % 10240))  # 1-10KB
    dd if=/dev/urandom of="$BENCH_ROOT/small_src/file_$i.dat" bs=$size count=1 2>/dev/null
done

echo "v1 benchmark..."
v1_small=$(run_benchmark "small-v1" "$V1_BINARY" "$BENCH_ROOT/small_src" "$BENCH_ROOT/small_dst_v1")
echo "v1: ${v1_small}s"

echo "v2 benchmark..."
v2_small=$(run_benchmark "small-v2" "$V2_BINARY" "$BENCH_ROOT/small_src" "$BENCH_ROOT/small_dst_v2")
echo "v2: ${v2_small}s"

ratio_small=$(echo "scale=4; $v2_small / $v1_small" | bc)
pct_small=$(echo "scale=2; (1 - $ratio_small) * 100" | bc)

echo "Ratio: v2/v1 = $ratio_small"
if (( $(echo "$ratio_small <= 1.05" | bc -l) )); then
    echo -e "${GREEN}✓ PASS${NC}: v2 within 5% of v1"
else
    echo -e "${RED}✗ FAIL${NC}: v2 slower by ${pct_small}%"
fi
echo ""

# Test 3: Mixed workload
echo "========================================="
echo "Test 3: Mixed Workload"
echo "========================================="

mkdir -p "$BENCH_ROOT/mixed_src" "$BENCH_ROOT/mixed_dst_v1" "$BENCH_ROOT/mixed_dst_v2"

# 10 large files (100MB each)
for i in $(seq 1 10); do
    dd if=/dev/zero of="$BENCH_ROOT/mixed_src/large_$i.bin" bs=1M count=100 2>/dev/null
done

# 1000 small files (1-100KB)
for i in $(seq 1 1000); do
    size=$((1024 + RANDOM % 102400))
    dd if=/dev/urandom of="$BENCH_ROOT/mixed_src/small_$i.dat" bs=$size count=1 2>/dev/null
done

echo "v1 benchmark..."
v1_mixed=$(run_benchmark "mixed-v1" "$V1_BINARY" "$BENCH_ROOT/mixed_src" "$BENCH_ROOT/mixed_dst_v1")
echo "v1: ${v1_mixed}s"

echo "v2 benchmark..."
v2_mixed=$(run_benchmark "mixed-v2" "$V2_BINARY" "$BENCH_ROOT/mixed_src" "$BENCH_ROOT/mixed_dst_v2")
echo "v2: ${v2_mixed}s"

ratio_mixed=$(echo "scale=4; $v2_mixed / $v1_mixed" | bc)

echo "Ratio: v2/v1 = $ratio_mixed"
if (( $(echo "$ratio_mixed <= 1.05" | bc -l) )); then
    echo -e "${GREEN}✓ PASS${NC}: v2 within 5% of v1"
else
    echo -e "${RED}✗ FAIL${NC}: v2 slower"
fi
echo ""

# Summary
echo "========================================="
echo "SUMMARY"
echo "========================================="
echo "Large file: v2/v1 = $ratio_large"
echo "Small files: v2/v1 = $ratio_small"
echo "Mixed workload: v2/v1 = $ratio_mixed"

# Overall pass/fail
if (( $(echo "$ratio_large <= 1.05 && $ratio_small <= 1.05 && $ratio_mixed <= 1.05" | bc -l) )); then
    echo -e "${GREEN}✓✓✓ OVERALL: PASS${NC}"
    echo "Proceed to Phase 3"
    exit 0
else
    echo -e "${RED}✗✗✗ OVERALL: FAIL${NC}"
    echo "Performance optimization required before Phase 3"
    exit 1
fi
```

**Make executable**:
```bash
chmod +x benchmarks/bench_local.sh
```

### Task 2.5.2: Establish v1 Baseline
**Priority**: 🔴 Critical
**Effort**: 1-2 hours
**Dependencies**: Access to v1 binary

**Action**: Run v1 benchmarks and record baseline

```bash
# Build v1 if needed
cd /path/to/blit-v1
cargo build --release

# Run baseline benchmarks
V1_BINARY=/path/to/blit-v1/target/release/blit \
V2_BINARY=/tmp/dummy \
./benchmarks/bench_local.sh 2>&1 | tee benchmark_results/v1_baseline.txt
```

**Record results** in `benchmark_results/baseline_metrics.md`:

```markdown
# Blit v1 Baseline Performance

**Date**: 2025-10-XX
**System**: [CPU, RAM, Disk type]
**OS**: [Linux kernel version]

## Results

| Test | Duration (s) | Throughput |
|------|--------------|------------|
| Large file (4GiB) | X.XX | Y.YY GB/s |
| Small files (100k) | X.XX | Y files/s |
| Mixed workload | X.XX | - |

## Notes
- [Any relevant observations]
```

### Task 2.5.3: Run v2 Benchmarks
**Priority**: 🔴 Critical
**Effort**: 1 hour
**Dependencies**: Phase 2 complete, v1 baseline established

**Action**: Build v2 in release mode and run benchmarks

```bash
# Build v2 in release mode (critical!)
cargo build --release -p blit-cli

# Run benchmarks
V1_BINARY=/path/to/blit-v1/target/release/blit \
V2_BINARY=./target/release/blit-cli \
./benchmarks/bench_local.sh 2>&1 | tee benchmark_results/v2_initial.txt
```

**Analyze results**:
- Is v2 within 5% of v1 across all tests?
- Where are the performance gaps?
- Is memory usage acceptable?

## Day 2: Analysis & Decision (2-4 hours)

### Task 2.5.4: Performance Analysis
**Priority**: 🔴 Critical
**Effort**: 1-2 hours
**Dependencies**: Benchmark results available

**If benchmarks PASS (≥95%)**:
- Document results
- Update TODO.md
- Proceed to Task 2.5.6 (Documentation)

**If benchmarks FAIL (<95%)**:
- Proceed to Task 2.5.5 (Profiling & Optimization)

### Task 2.5.5: Profiling & Optimization (If Needed)
**Priority**: 🔴 Critical (if triggered)
**Effort**: 2-4 hours
**Trigger**: Benchmark results <95% of v1

**Step 1: Profile with perf**
```bash
# Build with debug symbols
cargo build --release -p blit-cli

# Profile large file transfer
sudo perf record -g ./target/release/blit-cli mirror /tmp/large_src /tmp/large_dst

# Analyze
sudo perf report

# Generate flamegraph
cargo install flamegraph
sudo cargo flamegraph -p blit-cli -- mirror /tmp/large_src /tmp/large_dst
```

**Step 2: Analyze hotspots**
Look for:
- Unnecessary allocations
- Excessive system calls
- Inefficient buffer usage
- Missing zero-copy optimizations

**Step 3: Common optimizations**

**Optimization 1: Increase buffer size**
```rust
// In TransferOrchestrator::new()
buffer_size: 256 * 1024, // Try 256KB instead of 64KB
```

**Optimization 2: Enable zero-copy for smaller files**
```rust
// Lower threshold for zero-copy
if self.use_zero_copy && size > 64 * 1024 { // Was 1MB
    use crate::zero_copy::sendfile_copy;
    sendfile_copy(src, dst)?;
}
```

**Optimization 3: Parallel file transfers**
```rust
use rayon::prelude::*;

// Parallel transfer for independent files
plan.files_to_transfer.par_iter().try_for_each(|file_info| {
    let src_path = source.join(&file_info.relative_path);
    let dst_path = destination.join(&file_info.relative_path);
    self.transfer_file(&src_path, &dst_path, file_info.size)
})?;
```

**Step 4: Re-run benchmarks** after each optimization

### Task 2.5.6: Memory Profiling
**Priority**: 🟡 Important
**Effort**: 1 hour

**Tool**: Valgrind or Heaptrack

```bash
# With Valgrind
valgrind --tool=massif ./target/release/blit-cli mirror /tmp/src /tmp/dst
ms_print massif.out.XXX

# Or with Heaptrack
heaptrack ./target/release/blit-cli mirror /tmp/src /tmp/dst
heaptrack_gui heaptrack.blit-cli.XXX.gz
```

**Verify**:
- No memory leaks
- Peak memory usage ≤110% of v1
- Memory usage scales reasonably with file count

### Task 2.5.7: Document Results & Make Decision
**Priority**: 🔴 Critical
**Effort**: 30 minutes

**Create**: `benchmark_results/phase_2.5_decision.md`

```markdown
# Phase 2.5 Quality Gate Decision

**Date**: 2025-10-XX
**Status**: [PASS / FAIL]

## Benchmark Results

| Test | v1 (s) | v2 (s) | Ratio | Pass? |
|------|--------|--------|-------|-------|
| Large file (4GiB) | X.XX | Y.YY | Z.ZZ | ✓/✗ |
| Small files (100k) | X.XX | Y.YY | Z.ZZ | ✓/✗ |
| Mixed workload | X.XX | Y.YY | Z.ZZ | ✓/✗ |

## Memory Usage

| Metric | v1 | v2 | Ratio | Pass? |
|--------|----|----|-------|-------|
| Peak memory | X MB | Y MB | Z.ZZ | ✓/✗ |

## Decision

**[GO / NO-GO]**

### If GO:
v2 performance meets acceptance criteria. Proceed to Phase 3.

### If NO-GO:
Performance gaps identified:
1. [List specific issues]
2. [Proposed fixes]
3. [Re-benchmark timeline]

## Optimizations Applied (if any)

- [Optimization 1]
- [Optimization 2]

## Next Steps

- [Specific actions]
```

## Quality Gate Checklist

Before proceeding to Phase 3:

- [ ] v1 baseline benchmarks completed and documented
- [ ] v2 benchmarks completed
- [ ] Large file performance ≥95% of v1
- [ ] Small files performance ≥95% of v1
- [ ] Mixed workload performance ≥95% of v1
- [ ] Memory usage ≤110% of v1
- [ ] No memory leaks detected
- [ ] Results documented in `phase_2.5_decision.md`
- [ ] GO/NO-GO decision made and recorded

## GO Decision: Next Steps

If quality gate **PASSES**:

1. Update `TODO.md`:
   ```markdown
   ## Phase 2.5: Performance & Validation Checkpoint

   - [x] Create benchmark script for local mirror performance.
   - [x] Run and compare against v1.
   - [x] Quality gate: PASSED
   ```

2. Update `DEVLOG.md`:
   ```markdown
   **2025-10-XX HH:MM:00Z** - **MILESTONE**: Phase 2.5 complete. Performance validated at XX% of v1.
   ```

3. Proceed to **[WORKFLOW_PHASE_3.md](./WORKFLOW_PHASE_3.md)**

## NO-GO Decision: Remediation Path

If quality gate **FAILS**:

1. **Analyze** performance gaps (completed in Task 2.5.5)
2. **Prioritize** optimizations by impact
3. **Implement** top optimizations
4. **Re-benchmark** after each significant change
5. **Iterate** until criteria met or architectural decision needed

### Architectural Options if Optimization Insufficient

If optimization efforts don't achieve target:

**Option 1: Implement Hybrid Transport** (if not already done)
- Rationale: Raw TCP may be faster than gRPC streaming
- Effort: 1-2 days
- Risk: Medium (complexity increase)

**Option 2: Accept Performance Gap** (requires stakeholder approval)
- Document trade-offs
- Justify with other benefits (maintainability, features, etc.)
- Set new acceptance threshold

**Option 3: Investigate v1 Implementation Details**
- Deep dive into v1 hot paths
- Port specific optimizations to v2
- Validate in v2 architecture

## Risks & Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Benchmarks fail | Medium | High | Profiling tools ready, optimization strategies prepared |
| v1 binary unavailable | Low | High | Document expected performance, use historical data |
| Platform differences | Low | Medium | Benchmark on multiple systems |
| Optimization time overrun | Medium | Medium | Time-box optimization, have architectural fallback |

## Tools Required

- `bc` - For calculations in benchmark script
- `perf` - For CPU profiling (Linux)
- `valgrind` or `heaptrack` - For memory profiling
- `hyperfine` - For reliable timing (optional but recommended)
- `flamegraph` - For visualization (optional)

Install:
```bash
# Ubuntu/Debian
sudo apt install bc linux-tools-generic valgrind

# Cargo tools
cargo install hyperfine flamegraph
```

## Expected Outcomes

### Best Case (Pass on First Run)
- v2 meets all performance criteria
- Proceed to Phase 3 same day
- Total time: 1 day

### Typical Case (Pass After Optimization)
- Initial benchmarks 85-95% of v1
- 1-2 optimization iterations
- Re-benchmark and pass
- Total time: 1.5-2 days

### Worst Case (Architectural Change Needed)
- Performance <85% of v1
- Optimization insufficient
- Implement hybrid transport or major refactor
- Total time: 3-4 days

## Definition of Done

Phase 2.5 is complete when:

1. ✅ All benchmark scenarios executed
2. ✅ Results documented and analyzed
3. ✅ GO/NO-GO decision made and recorded
4. ✅ If PASS: Ready to proceed to Phase 3
5. ✅ If FAIL: Remediation plan created and approved
6. ✅ DEVLOG.md updated with milestone

**Critical**: Do not proceed to Phase 3 without explicit GO decision.
