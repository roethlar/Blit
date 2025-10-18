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
LOG_FILE="$WORK_ROOT/bench.log"
mkdir -p "$SRC_DIR"
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

TOOL_NAMES=()
TOOL_DESTS=()
TOOL_LABELS=()
TOOL_SUM_NS=()
TOOL_COUNTS=()

add_tool() {
  local name=$1
  local dest=$2
  local label=$3
  TOOL_NAMES+=("$name")
  TOOL_DESTS+=("$dest")
  TOOL_LABELS+=("$label")
  TOOL_SUM_NS+=("0")
  TOOL_COUNTS+=("0")
}

DST_BLIT="$WORK_ROOT/dst_blit"
add_tool "blit" "$DST_BLIT" "blit v2 mirror"

if command -v rsync >/dev/null 2>&1; then
  DST_RSYNC="$WORK_ROOT/dst_rsync"
  add_tool "rsync" "$DST_RSYNC" "rsync -a --delete"
else
  log "rsync not found; skipping rsync baseline."
fi

run_tool_command() {
  local tool=$1
  local dest=$2
  case "$tool" in
    blit)
      env BLIT_DISABLE_PERF_HISTORY="$BLIT_DISABLE_PERF_HISTORY" \
        "$BLIT_BIN" mirror --no-progress "$SRC_DIR" "$dest"
      ;;
    rsync)
      rsync -a --delete --human-readable --stats --no-inc-recursive \
        "$SRC_DIR/" "$dest/"
      ;;
    *)
      echo "unknown tool: $tool" >&2
      return 1
      ;;
  esac
}

run_once() {
  local idx=$1
  local phase=$2
  local index=$3
  local total=$4

  local name=${TOOL_NAMES[$idx]}
  local dest=${TOOL_DESTS[$idx]}
  local label=${TOOL_LABELS[$idx]}

  rm -rf "$dest"
  mkdir -p "$dest"

  log "[$label] $phase run $index/$total -> $dest"
  local start_ns end_ns elapsed_ns elapsed_s status
  start_ns=$(date +%s%N)
  run_tool_command "$name" "$dest" 2>&1 | tee -a "$LOG_FILE"
  status=${PIPESTATUS[0]}
  end_ns=$(date +%s%N)
  if [[ $status -ne 0 ]]; then
    log "error: $label exited with status $status"
    exit $status
  fi
  elapsed_ns=$((end_ns - start_ns))
  elapsed_s=$(awk "BEGIN { printf \"%.3f\", $elapsed_ns/1e9 }")
  log "[$label] $phase run $index completed in ${elapsed_s}s"

  if [[ "$phase" == "Measured" ]]; then
    local current_sum=${TOOL_SUM_NS[$idx]}
    local current_count=${TOOL_COUNTS[$idx]}
    TOOL_SUM_NS[$idx]=$(( current_sum + elapsed_ns ))
    TOOL_COUNTS[$idx]=$(( current_count + 1 ))
  fi
}

log "Running warmups (runs=$WARMUP) and measured passes (runs=$RUNS)..."
for idx in "${!TOOL_NAMES[@]}"; do
  for ((i = 1; i <= WARMUP; i++)); do
    run_once "$idx" "Warmup" "$i" "$WARMUP" >/dev/null
  done
  for ((i = 1; i <= RUNS; i++)); do
    run_once "$idx" "Measured" "$i" "$RUNS"
  done
done

for idx in "${!TOOL_NAMES[@]}"; do
  count=${TOOL_COUNTS[$idx]}
  if (( count > 0 )); then
    sum_ns=${TOOL_SUM_NS[$idx]}
    avg_s=$(awk "BEGIN { printf \"%.3f\", $sum_ns/$count/1e9 }")
    label=${TOOL_LABELS[$idx]}
    log "Average [$label] over $count measured run(s): ${avg_s}s"
  fi
done

log "Benchmark complete. Full log: $LOG_FILE"
