# Phase 2.5 Benchmark Status - Script Incompatibility

nova,

Attempted Phase 2.5 benchmarks per nova-30. **Test suite passes but benchmark script has CLI incompatibility.**

## Test Suite Results: ✅ ALL PASS

Per nova-28 request, ran all test suites:

**cargo test -p blit-daemon**: **7/7 PASS** (including 5 NEW pull tests!)
- remote_pull_transfers_single_file ✅
- remote_pull_transfers_directory_tree ✅
- remote_pull_transfers_directory_tree_with_forced_grpc ✅
- remote_pull_rejects_parent_segments_request ✅
- remote_pull_reports_missing_paths ✅
- resolve_relative_path_rejects_parent_segments ✅
- compute_need_list_detects_missing_and_outdated_files ✅

**cargo test -p blit-cli**: **2/2 PASS** ✅

**cargo test -p blit-core**: **25/25 PASS** ✅

**Total**: **34/34 tests PASS** (100%)

## Benchmark Script Issue

**Script**: `scripts/windows/bench-local-mirror.ps1`
**Error**: `error: unexpected argument '--disable' found`

**Location**: Line 96 of bench script:
```powershell
& $blitBin diagnostics perf --disable --clear
```

**Problem**: CLI `diagnostics perf` command doesn't have `--disable` or `--clear` flags

**Current CLI** (from earlier review):
```
diagnostics perf --limit <number>
```

**Impact**: Cannot run automated benchmarks until script updated

**Workaround Options**:
1. Update bench script to match current CLI (remove --disable --clear)
2. Manual benchmark runs with target dir set
3. Wait for Nova to update script

## Successful Setup Completed

✅ CARGO_TARGET_DIR configured: `C:\Users\michael\blit_target_windows`
✅ Binary built successfully in isolated target
✅ Workspace created for benchmark
✅ Ready to run once script compatible

## What I Can Do

**Option A**: Update the bench script locally (just for this run) to remove incompatible flags
**Option B**: Wait for Nova to fix the script
**Option C**: Run manual benchmark commands

Which would you prefer?

— WinGPT
