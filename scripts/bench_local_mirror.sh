#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

SIZE_MB=${SIZE_MB:-256}
RUNS=${RUNS:-5}
WARMUP=${WARMUP:-1}
KEEP_WORK=${KEEP_BENCH_DIR:-1}
WORK_ROOT=${BENCH_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/blit_v2_bench.XXXXXX")}

SRC_DIR="$WORK_ROOT/src"
DST_DIR="$WORK_ROOT/dst_blit"
LOG_FILE="$WORK_ROOT/bench.log"

mkdir -p "$SRC_DIR" "$DST_DIR"
: >"$LOG_FILE"
set -o pipefail

if [[ "$KEEP_WORK" != "0" ]]; then
  CLEANUP_LABEL="preserved"
else
  CLEANUP_LABEL="removed"
  trap 'rm -rf "$WORK_ROOT"' EXIT
fi

log() {
  echo "$*" | tee -a "$LOG_FILE"
}

log "Workspace: $WORK_ROOT (will be $CLEANUP_LABEL on exit)"
log "Generating ${SIZE_MB} MiB synthetic payload..."

if command -v python3 >/dev/null 2>&1; then
  python3 - "$SRC_DIR" "$SIZE_MB" <<'PY'
import os, sys
root = sys.argv[1]
size_mb = int(sys.argv[2])
os.makedirs(root, exist_ok=True)
with open(os.path.join(root, "payload.bin"), "wb") as fh:
    for _ in range(size_mb):
        fh.write(os.urandom(1024 * 1024))
for idx in range(32):
    subdir = os.path.join(root, f"dir_{idx:02d}")
    os.makedirs(subdir, exist_ok=True)
    with open(os.path.join(subdir, "file.txt"), "w", encoding="utf-8") as fh:
        fh.write(("hello world\n") * (idx + 1))
PY
else
  dd if=/dev/urandom of="$SRC_DIR/payload.bin" bs=1M count="$SIZE_MB" status=none
  for idx in $(seq 0 31); do
    subdir=$(printf "%s/dir_%02d" "$SRC_DIR" "$idx")
    mkdir -p "$subdir"
    printf 'hello world\n' >"$subdir/file.txt"
  done
fi

if command -v du >/dev/null 2>&1; then
  log "Payload size: $(du -sh "$SRC_DIR" | cut -f1)"
fi

log "Building blit-cli (release)..."
(
  cd "$REPO_ROOT"
  cargo build --release --package blit-cli --bin blit-cli
) >>"$LOG_FILE" 2>&1

BLIT_BIN="$REPO_ROOT/target/release/blit-cli"
if [[ ! -x "$BLIT_BIN" ]]; then
  log "error: expected binary not found at $BLIT_BIN"
  exit 1
fi
log "Binary ready: $BLIT_BIN"

if ! [[ "$RUNS" =~ ^[0-9]+$ && "$WARMUP" =~ ^[0-9]+$ ]]; then
  log "error: RUNS and WARMUP must be non-negative integers"
  exit 1
fi

if [[ -z "${BLIT_DISABLE_PERF_HISTORY+x}" ]]; then
  export BLIT_DISABLE_PERF_HISTORY=1
  log "Perf history disabled for benchmark runs (set BLIT_DISABLE_PERF_HISTORY=0 to keep history)."
else
  log "Perf history env already set to '$BLIT_DISABLE_PERF_HISTORY'."
fi

prepare_dest() {
  rm -rf "$DST_DIR"
  mkdir -p "$DST_DIR"
}

RUN_ONCE_LAST_NS=0

run_once() {
  local phase=$1
  local index=$2
  prepare_dest
  log "$phase run $index: mirror -> $DST_DIR"
  local start_ns end_ns elapsed_ns elapsed_s status
  start_ns=$(date +%s%N)
  "$BLIT_BIN" mirror --no-progress "$SRC_DIR" "$DST_DIR" 2>&1 | tee -a "$LOG_FILE"
  status=${PIPESTATUS[0]}
  end_ns=$(date +%s%N)
  if [[ $status -ne 0 ]]; then
    log "error: blit-cli exited with status $status"
    exit $status
  fi
  elapsed_ns=$((end_ns - start_ns))
  elapsed_s=$(awk "BEGIN { printf \"%.3f\", $elapsed_ns/1e9 }")
  log "$phase run $index completed in ${elapsed_s}s"
  RUN_ONCE_LAST_NS=$elapsed_ns
}

declare -a MEASURED_NS=()

if command -v hyperfine >/dev/null 2>&1; then
  log "Running hyperfine benchmark (runs=$RUNS, warmup=$WARMUP)..."
  prepare_cmd=$(printf "rm -rf '%s' && mkdir -p '%s'" "$DST_DIR" "$DST_DIR")
  blit_cmd=$(printf "env BLIT_DISABLE_PERF_HISTORY=%q %q mirror --no-progress %q %q" \
    "$BLIT_DISABLE_PERF_HISTORY" "$BLIT_BIN" "$SRC_DIR" "$DST_DIR")
  hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --prepare "$prepare_cmd" \
    "$blit_cmd" | tee -a "$LOG_FILE"
else
  log "hyperfine not found; running sequential timings (runs=$RUNS, warmup=$WARMUP)"
  for ((i = 1; i <= WARMUP; i++)); do
    run_once "Warmup" "$i/$WARMUP" >/dev/null
  done
  for ((i = 1; i <= RUNS; i++)); do
    run_once "Measured" "$i/$RUNS"
    MEASURED_NS+=("$RUN_ONCE_LAST_NS")
  done
fi

if (( ${#MEASURED_NS[@]} > 0 )); then
  total_ns=0
  for ns in "${MEASURED_NS[@]}"; do
    total_ns=$((total_ns + ns))
  done
  avg_s=$(awk "BEGIN { printf \"%.3f\", $total_ns/${#MEASURED_NS[@]}/1e9 }")
  log "Average over ${#MEASURED_NS[@]} measured run(s): ${avg_s}s"
fi

log "Benchmark complete. Full log: $LOG_FILE"
