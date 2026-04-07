#!/usr/bin/env bash
# Benchmark blit over 10GbE: local, remote push/pull, TCP vs gRPC, NFS vs native.
#
# Usage:
#   # Set these before running:
#   export REMOTE_HOST=truenas.local        # TrueNAS hostname/IP
#   export NFS_MOUNT=/mnt/truenas           # Local NFS mount point (optional)
#   export SMB_MOUNT=/mnt/truenas_smb       # Local SMB mount point (optional)
#   export REMOTE_MODULE=bench              # blit-daemon module name on remote
#   export REMOTE_PORT=9031                 # blit-daemon port on remote
#
#   # Then run:
#   ./scripts/bench_10gbe.sh
#
# Prerequisites:
#   - Release binaries built: cargo build --release
#   - For remote tests: blit-daemon running on REMOTE_HOST with a module configured
#   - For NFS tests: NFS share mounted at NFS_MOUNT
#   - For SMB tests: SMB share mounted at SMB_MOUNT

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
BLIT="$REPO_ROOT/target/release/blit-cli"
BLIT_DAEMON="$REPO_ROOT/target/release/blit-daemon"

# --- Configuration ---
SIZE_MB=${SIZE_MB:-1024}
SMALL_COUNT=${SMALL_COUNT:-10000}
SMALL_SIZE=${SMALL_SIZE:-4096}
RUNS=${RUNS:-3}
REMOTE_HOST=${REMOTE_HOST:-}
REMOTE_PORT=${REMOTE_PORT:-9031}
REMOTE_MODULE=${REMOTE_MODULE:-bench}
NFS_MOUNT=${NFS_MOUNT:-}
SMB_MOUNT=${SMB_MOUNT:-}

WORK=$(mktemp -d /tmp/blit_10gbe_bench.XXXXXX)
LOG_DIR="$REPO_ROOT/logs/bench_10gbe_$(date +%Y%m%dT%H%M%S)"
mkdir -p "$LOG_DIR"

trap 'echo "Cleaning up..."; rm -rf "$WORK"' EXIT

# --- Helpers ---
log() { echo "$(date +%H:%M:%S) $*" | tee -a "$LOG_DIR/bench.log"; }

generate_large_file() {
    local dir="$1"
    mkdir -p "$dir"
    dd if=/dev/urandom of="$dir/large_${SIZE_MB}M.bin" bs=1M count="$SIZE_MB" 2>/dev/null
    log "Generated ${SIZE_MB}M large file in $dir"
}

generate_small_files() {
    local dir="$1"
    mkdir -p "$dir"
    for i in $(seq 1 "$SMALL_COUNT"); do
        local subdir="$dir/d$(( i / 1000 ))"
        mkdir -p "$subdir"
        dd if=/dev/urandom of="$subdir/f${i}.dat" bs="$SMALL_SIZE" count=1 2>/dev/null
    done
    log "Generated $SMALL_COUNT × ${SMALL_SIZE}B small files in $dir"
}

generate_mixed() {
    local dir="$1"
    mkdir -p "$dir"
    # One large file
    dd if=/dev/urandom of="$dir/big.bin" bs=1M count=512 2>/dev/null
    # Many small files
    for i in $(seq 1 5000); do
        local subdir="$dir/d$(( i / 500 ))"
        mkdir -p "$subdir"
        dd if=/dev/urandom of="$subdir/f${i}.dat" bs=2048 count=1 2>/dev/null
    done
    log "Generated mixed workload in $dir (512M + 5000×2K)"
}

run_timed() {
    local label="$1"
    shift
    local total=0
    local best=999999
    for run in $(seq 1 "$RUNS"); do
        local start=$(date +%s%N)
        "$@" 2>/dev/null
        local end=$(date +%s%N)
        local ms=$(( (end - start) / 1000000 ))
        total=$(( total + ms ))
        if (( ms < best )); then best=$ms; fi
        log "  $label run $run: ${ms}ms"
    done
    local avg=$(( total / RUNS ))
    log "  $label avg: ${avg}ms  best: ${best}ms"
    echo "$label,$avg,$best" >> "$LOG_DIR/results.csv"
}

cleanup_dest() {
    rm -rf "$1" 2>/dev/null || true
    mkdir -p "$1"
}

# --- Generate test data ---
log "=== Generating test data ==="
SRC_LARGE="$WORK/src_large"
SRC_SMALL="$WORK/src_small"
SRC_MIXED="$WORK/src_mixed"

generate_large_file "$SRC_LARGE"
generate_small_files "$SRC_SMALL"
generate_mixed "$SRC_MIXED"

echo "test,avg_ms,best_ms" > "$LOG_DIR/results.csv"

# ============================================================
# 1. LOCAL → LOCAL (baseline)
# ============================================================
log ""
log "=== LOCAL → LOCAL ==="

for workload in large small mixed; do
    eval "src=\$SRC_$(echo $workload | tr a-z A-Z)"
    dest="$WORK/dst_local_$workload"

    log "--- $workload ---"
    for run_label in "local_${workload}_copy" ; do
        cleanup_dest "$dest"
        run_timed "$run_label" "$BLIT" copy "$src" "$dest" --yes
    done

    # Incremental (no-change) run
    run_timed "local_${workload}_noop" "$BLIT" mirror "$src" "$dest" --yes
done

# ============================================================
# 2. LOCAL → NFS MOUNT (if available)
# ============================================================
if [[ -n "$NFS_MOUNT" && -d "$NFS_MOUNT" ]]; then
    log ""
    log "=== LOCAL → NFS ==="
    for workload in large small mixed; do
        eval "src=\$SRC_$(echo $workload | tr a-z A-Z)"
        dest="$NFS_MOUNT/blit_bench_$workload"

        log "--- $workload (NFS) ---"
        cleanup_dest "$dest"
        run_timed "nfs_${workload}_copy" "$BLIT" copy "$src" "$dest" --yes
        run_timed "nfs_${workload}_noop" "$BLIT" mirror "$src" "$dest" --yes
        rm -rf "$dest"
    done
fi

# ============================================================
# 3. LOCAL → SMB MOUNT (if available)
# ============================================================
if [[ -n "$SMB_MOUNT" && -d "$SMB_MOUNT" ]]; then
    log ""
    log "=== LOCAL → SMB ==="
    for workload in large small mixed; do
        eval "src=\$SRC_$(echo $workload | tr a-z A-Z)"
        dest="$SMB_MOUNT/blit_bench_$workload"

        log "--- $workload (SMB) ---"
        cleanup_dest "$dest"
        run_timed "smb_${workload}_copy" "$BLIT" copy "$src" "$dest" --yes
        run_timed "smb_${workload}_noop" "$BLIT" mirror "$src" "$dest" --yes
        rm -rf "$dest"
    done
fi

# ============================================================
# 4. LOCAL → REMOTE PUSH (TCP data plane)
# ============================================================
if [[ -n "$REMOTE_HOST" ]]; then
    REMOTE="$REMOTE_HOST:$REMOTE_PORT:/$REMOTE_MODULE"

    log ""
    log "=== LOCAL → REMOTE PUSH (TCP) ==="
    for workload in large small mixed; do
        eval "src=\$SRC_$(echo $workload | tr a-z A-Z)"

        log "--- $workload (TCP push) ---"
        run_timed "push_tcp_${workload}" "$BLIT" copy "$src" "$REMOTE" --yes -v
    done

    log ""
    log "=== LOCAL → REMOTE PUSH (gRPC fallback) ==="
    for workload in large small mixed; do
        eval "src=\$SRC_$(echo $workload | tr a-z A-Z)"

        log "--- $workload (gRPC push) ---"
        run_timed "push_grpc_${workload}" "$BLIT" copy "$src" "$REMOTE" --yes -v --force-grpc-data
    done

    # ============================================================
    # 5. REMOTE → LOCAL PULL
    # ============================================================
    log ""
    log "=== REMOTE → LOCAL PULL (TCP) ==="
    for workload in large small mixed; do
        dest="$WORK/dst_pull_tcp_$workload"

        log "--- $workload (TCP pull) ---"
        cleanup_dest "$dest"
        run_timed "pull_tcp_${workload}" "$BLIT" copy "$REMOTE" "$dest" --yes -v
    done

    log ""
    log "=== REMOTE → LOCAL PULL (gRPC fallback) ==="
    for workload in large small mixed; do
        dest="$WORK/dst_pull_grpc_$workload"

        log "--- $workload (gRPC pull) ---"
        cleanup_dest "$dest"
        run_timed "pull_grpc_${workload}" "$BLIT" copy "$REMOTE" "$dest" --yes -v --force-grpc-data
    done
fi

# ============================================================
# 6. RSYNC COMPARISON (local baseline)
# ============================================================
if command -v rsync &>/dev/null; then
    log ""
    log "=== RSYNC COMPARISON (local) ==="
    for workload in large small mixed; do
        eval "src=\$SRC_$(echo $workload | tr a-z A-Z)"
        dest="$WORK/dst_rsync_$workload"

        log "--- $workload (rsync) ---"
        cleanup_dest "$dest"
        run_timed "rsync_${workload}" rsync -a --delete --whole-file --inplace --no-compress "$src/" "$dest/"
        run_timed "rsync_${workload}_noop" rsync -a --delete --whole-file --inplace --no-compress "$src/" "$dest/"
    done
fi

# ============================================================
# Summary
# ============================================================
log ""
log "=== RESULTS ==="
log "Results CSV: $LOG_DIR/results.csv"
log "Full log: $LOG_DIR/bench.log"
log ""
column -t -s, "$LOG_DIR/results.csv" | tee -a "$LOG_DIR/bench.log"
