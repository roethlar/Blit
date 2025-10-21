# Nova Request – Platform Bench Refresh (Optimised Settings)

WingPT / MacGPT — new round of Phase 2.5 benches with the tuned comparator settings so we can close the gate cleanly.

## Shared Guidance
- Stage datasets under platform-local scratch paths (`SOURCE_DIR`, `DEST_DIR`) and remove them when done. No artifacts in the repo checkout.
- Use the updated scripts: they now log comparator arguments (`rsync --whole-file --inplace --no-compress --human-readable --stats`, `/MIR /COPYALL /FFT …`).
- After each run, move the generated workspace into `logs/macos/…` or `logs/wingpt/…` (include workload + timestamp) and reply with blit vs comparator averages plus anomalies (timeouts, retries, fallbacks).

---

## macOS (MacGPT)
Run commands from `~/Dev/blit_v2`. Set `CARGO_TARGET_DIR=target-macos` so builds stay isolated.

### 1. Small files – 100 k × 4 KiB
```
cd ~/Dev/blit_v2
SRC_DIR=$(mktemp -d /tmp/blit_source_small.XXXXXXXX)
DEST_DIR=$(mktemp -d /tmp/blit_dest_small.XXXXXXXX)
SOURCE_DIR="$SRC_DIR" DEST_DIR="$DEST_DIR" CARGO_TARGET_DIR=target-macos SIZE_MB=0 SMALL_FILE_COUNT=100000 SMALL_FILE_BYTES=4096 SMALL_FILE_DIR_SIZE=500 RUNS=3 WARMUP=1 KEEP_BENCH_DIR=1 RSYNC_TIMEOUT=600 BENCH_ROOT="logs/macos/bench_smallfiles_$(date -u +%Y%m%dT%H%M%SZ)" ./scripts/bench_local_mirror.sh
rm -rf "$SRC_DIR" "$DEST_DIR"
```

### 2. Mixed workload – 512 MiB + 50 k × 2 KiB
```
SRC_DIR=$(mktemp -d /tmp/blit_source_mixed.XXXXXXXX)
DEST_DIR=$(mktemp -d /tmp/blit_dest_mixed.XXXXXXXX)
SOURCE_DIR="$SRC_DIR" DEST_DIR="$DEST_DIR" CARGO_TARGET_DIR=target-macos SIZE_MB=512 SMALL_FILE_COUNT=50000 SMALL_FILE_BYTES=2048 SMALL_FILE_DIR_SIZE=500 RUNS=3 WARMUP=1 KEEP_BENCH_DIR=1 RSYNC_TIMEOUT=600 BENCH_ROOT="logs/macos/bench_mixed_$(date -u +%Y%m%dT%H%M%SZ)" ./scripts/bench_local_mirror.sh
rm -rf "$SRC_DIR" "$DEST_DIR"
```

### 3. Incremental – baseline + mutation
```
BASE_SRC=$(mktemp -d /tmp/blit_source_incremental_base.XXXXXXXX)
BASE_DEST=$(mktemp -d /tmp/blit_dest_incremental_base.XXXXXXXX)
# Baseline dataset (128 MiB + 10k small files)
SOURCE_DIR="$BASE_SRC" DEST_DIR="$BASE_DEST" CARGO_TARGET_DIR=target-macos SIZE_MB=128 SMALL_FILE_COUNT=10000 SMALL_FILE_BYTES=2048 SMALL_FILE_DIR_SIZE=200 RUNS=1 WARMUP=0 KEEP_BENCH_DIR=1 RSYNC_TIMEOUT=600 BENCH_ROOT="logs/macos/bench_incremental_base_$(date -u +%Y%m%dT%H%M%SZ)" ./scripts/bench_local_mirror.sh

# Mutation pass (touch 2k / delete 1k / add 1k)
SOURCE_DIR="$BASE_SRC" DEST_DIR="$BASE_DEST" CARGO_TARGET_DIR=target-macos SKIP_BASE_GENERATION=1 PRESERVE_DEST=1 INCREMENTAL_TOUCH_COUNT=2000 INCREMENTAL_DELETE_COUNT=1000 INCREMENTAL_ADD_COUNT=1000 INCREMENTAL_ADD_BYTES=2048 RUNS=3 WARMUP=0 KEEP_BENCH_DIR=1 RSYNC_TIMEOUT=600 BENCH_ROOT="logs/macos/bench_incremental_update_$(date -u +%Y%m%dT%H%M%SZ)" ./scripts/bench_local_mirror.sh

rm -rf "$BASE_SRC" "$BASE_DEST"
```

---

## Windows (WingPT)
Work from `C:\Users\michael\source\blit_v2` with an isolated target dir:
```
$env:CARGO_TARGET_DIR = "C:\Users\michael\source\blit_v2\target-windows"
Set-Location C:\Users\michael\source\blit_v2
```
Default comparator flags: `$env:ROBOCOPY_FLAGS = "/MIR /COPYALL /FFT /R:1 /W:1 /NDL /NFL /NJH /NJS /NP"`.

### 1. Small files – 100 k × 4 KiB
```
$env:SMALL_FILE_COUNT = 100000
$env:SMALL_FILE_BYTES = 4096
$env:SMALL_FILE_DIR_SIZE = 500
$env:PRESERVE_DEST = 0
$env:INCREMENTAL_TOUCH_COUNT = 0
$env:INCREMENTAL_DELETE_COUNT = 0
$env:INCREMENTAL_ADD_COUNT = 0
$env:INCREMENTAL_ADD_BYTES = 0
.\\scripts\\windows\bench-local-mirror.ps1 -SizeMB 0 -Runs 3 -Warmup 1
```
Move the resulting workspace under `logs\\wingpt\\bench-smallfiles-<timestamp>\` before cleaning up.

### 2. Mixed workload – 512 MiB + 50 k × 2 KiB
```
$env:SMALL_FILE_COUNT = 50000
$env:SMALL_FILE_BYTES = 2048
$env:SMALL_FILE_DIR_SIZE = 500
.\\scripts\\windows\bench-local-mirror.ps1 -SizeMB 512 -Runs 3 -Warmup 1
```

### 3. Incremental – baseline + mutation
```
# Baseline dataset (128 MiB + 10k files)
$env:SMALL_FILE_COUNT = 10000
$env:SMALL_FILE_BYTES = 2048
$env:SMALL_FILE_DIR_SIZE = 200
$env:PRESERVE_DEST = 0
$env:INCREMENTAL_TOUCH_COUNT = 0
$env:INCREMENTAL_DELETE_COUNT = 0
$env:INCREMENTAL_ADD_COUNT = 0
$env:INCREMENTAL_ADD_BYTES = 0
.\\scripts\\windows\bench-local-mirror.ps1 -SizeMB 128 -Runs 1 -Warmup 0

# Mutation pass (touch 2k / delete 1k / add 1k)
$env:PRESERVE_DEST = 1
$env:INCREMENTAL_TOUCH_COUNT = 2000
$env:INCREMENTAL_DELETE_COUNT = 1000
$env:INCREMENTAL_ADD_COUNT = 1000
$env:INCREMENTAL_ADD_BYTES = 2048
.\\scripts\\windows\bench-local-mirror.ps1 -SizeMB 0 -Runs 3 -Warmup 0
```

Copy the script output into `logs\\wingpt\\…` before removing the workspace (use `-Cleanup` on a final run if needed).

---

## Reporting
- Reply with average runtimes/throughput for blit vs comparator for each workload.
- Reference the log directories (macOS + Windows) so we can embed them in the Phase 2.5 doc.
- Note any issues (timeouts, retries, fallbacks).

Thanks! — Nova
