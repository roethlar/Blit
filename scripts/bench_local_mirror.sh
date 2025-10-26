#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

SIZE_MB=${SIZE_MB:-256}
RUNS=${RUNS:-5}
WARMUP=${WARMUP:-1}
KEEP_WORK=${KEEP_BENCH_DIR:-1}
WORK_ROOT=${BENCH_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/blit_v2_bench.XXXXXX")}

TARGET_DIR=${CARGO_TARGET_DIR:-"$REPO_ROOT/target"}
export CARGO_TARGET_DIR="$TARGET_DIR"

if [[ ! -d "$WORK_ROOT" ]]; then
  mkdir -p "$WORK_ROOT"
fi

SMALL_FILE_COUNT=${SMALL_FILE_COUNT:-0}
SMALL_FILE_BYTES=${SMALL_FILE_BYTES:-4096}
SMALL_FILE_DIR_SIZE=${SMALL_FILE_DIR_SIZE:-1000}

RSYNC_TIMEOUT=${RSYNC_TIMEOUT:-0}
RSYNC_ARGS_DEFAULT="-a --delete --whole-file --inplace --no-compress --human-readable --stats"
RSYNC_ARGS=${RSYNC_ARGS:-$RSYNC_ARGS_DEFAULT}

PRESERVE_DEST=${PRESERVE_DEST:-0}
INCREMENTAL_TOUCH_COUNT=${INCREMENTAL_TOUCH_COUNT:-0}
INCREMENTAL_DELETE_COUNT=${INCREMENTAL_DELETE_COUNT:-0}
INCREMENTAL_ADD_COUNT=${INCREMENTAL_ADD_COUNT:-0}
INCREMENTAL_ADD_BYTES=${INCREMENTAL_ADD_BYTES:-1024}
_INCREMENTAL_APPLIED=0

SRC_DIR=${SOURCE_DIR:?env SOURCE_DIR must point to the benchmark source directory}
DEST_ROOT=${DEST_DIR:?env DEST_DIR must point to the benchmark destination root}
LOG_FILE="$WORK_ROOT/bench.log"
: >"$LOG_FILE"
set -o pipefail

mkdir -p "$SRC_DIR"
mkdir -p "$DEST_ROOT"

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

if [[ "${SKIP_BASE_GENERATION:-0}" != "1" ]]; then
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
else
  log "Skipping base payload generation (SKIP_BASE_GENERATION=1)."
fi

generate_small_files() {
  local count=$1
  local bytes=$2
  local dir_size=$3
  if (( count <= 0 )); then
    return
  fi
  log "Generating ${count} small files (${bytes} bytes each)..."
  python3 - "$SRC_DIR" "$count" "$bytes" "$dir_size" <<'PY'
import os
import sys
from pathlib import Path

root = Path(sys.argv[1])
count = int(sys.argv[2])
bytes_per_file = int(sys.argv[3])
dir_fan = max(1, int(sys.argv[4]))

small_root = root / "small"
small_root.mkdir(parents=True, exist_ok=True)

payload = b"X" * bytes_per_file

for idx in range(count):
    bucket = idx // dir_fan
    directory = small_root / f"grp_{bucket:04d}"
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / f"file_{idx:06d}.dat"
    with open(path, "wb") as fh:
        fh.write(payload)

PY
  log "Small-file payload generated."
  if command -v du >/dev/null 2>&1; then
    log "Total source size: $(du -sh "$SRC_DIR" | cut -f1)"
  fi
}

if [[ "${SKIP_BASE_GENERATION:-0}" != "1" ]]; then
  generate_small_files "$SMALL_FILE_COUNT" "$SMALL_FILE_BYTES" "$SMALL_FILE_DIR_SIZE"
else
  log "Skipping small-file generation (SKIP_BASE_GENERATION=1)."
fi

apply_incremental_changes() {
  if (( _INCREMENTAL_APPLIED == 1 )); then
    return
  fi
  if (( INCREMENTAL_TOUCH_COUNT <= 0 && INCREMENTAL_DELETE_COUNT <= 0 && INCREMENTAL_ADD_COUNT <= 0 )); then
    return
  fi
  log "Applying incremental changes to source tree (touch=${INCREMENTAL_TOUCH_COUNT}, delete=${INCREMENTAL_DELETE_COUNT}, add=${INCREMENTAL_ADD_COUNT})..."
  python3 - "$SRC_DIR" "$INCREMENTAL_TOUCH_COUNT" "$INCREMENTAL_DELETE_COUNT" "$INCREMENTAL_ADD_COUNT" "$INCREMENTAL_ADD_BYTES" <<'PY'
import os
import sys
from pathlib import Path
import time

root = Path(sys.argv[1])
touch_count = int(sys.argv[2])
delete_count = int(sys.argv[3])
add_count = int(sys.argv[4])
add_bytes = int(sys.argv[5])

files = [p for p in root.rglob('*') if p.is_file()]
files.sort()

def safe_slice(seq, length):
    return seq[: min(length, len(seq))]

now = int(time.time())

touch_targets = safe_slice(files, touch_count)
remaining = files[len(touch_targets):]
delete_targets = safe_slice(remaining, delete_count)

for path in touch_targets:
    with open(path, 'ab') as fh:
        fh.write(f"\nupdated {now}\n".encode())

for path in delete_targets:
    try:
        path.unlink()
    except FileNotFoundError:
        pass

if add_count > 0:
    payload = os.urandom(add_bytes)
    add_root = root / "incremental_new"
    add_root.mkdir(parents=True, exist_ok=True)
    width = len(str(add_count))
    for idx in range(add_count):
        new_path = add_root / f"new_{idx:0{width}}.dat"
        with open(new_path, 'wb') as fh:
            fh.write(payload)

PY
  _INCREMENTAL_APPLIED=1
  if command -v du >/dev/null 2>&1; then
    log "Source size after incremental changes: $(du -sh "$SRC_DIR" | cut -f1)"
  fi
}

if [[ -n "${BLIT_BIN:-}" ]]; then
  if [[ ! -x "$BLIT_BIN" ]]; then
    log "error: BLIT_BIN is set to '$BLIT_BIN' but the file is not executable"
    exit 1
  fi
  log "Using prebuilt blit-cli at $BLIT_BIN"
else
  log "Building blit-cli (release) into $TARGET_DIR..."
  (
    cd "$REPO_ROOT"
    cargo build --release --package blit-cli --bin blit-cli
  ) >>"$LOG_FILE" 2>&1

  BLIT_BIN="$TARGET_DIR/release/blit-cli"
  if [[ ! -x "$BLIT_BIN" ]]; then
    log "error: expected binary not found at $BLIT_BIN"
    exit 1
  fi
  log "Binary ready: $BLIT_BIN"
fi

if ! [[ "$RUNS" =~ ^[0-9]+$ && "$WARMUP" =~ ^[0-9]+$ ]]; then
  log "error: RUNS and WARMUP must be non-negative integers"
  exit 1
fi

BENCH_CONFIG_DIR="$WORK_ROOT/blit_config"
mkdir -p "$BENCH_CONFIG_DIR"
log "Using isolated config dir at $BENCH_CONFIG_DIR"
CONFIG_ARGS=(--config-dir "$BENCH_CONFIG_DIR")
"$BLIT_BIN" "${CONFIG_ARGS[@]}" diagnostics perf --disable --clear \
  >>"$LOG_FILE" 2>&1 || true

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

DST_BLIT="$DEST_ROOT/dst_blit"
add_tool "blit" "$DST_BLIT" "blit v2 mirror"

if command -v rsync >/dev/null 2>&1; then
  DST_RSYNC="$DEST_ROOT/dst_rsync"
  add_tool "rsync" "$DST_RSYNC" "rsync -a --delete"
else
  log "rsync not found; skipping rsync baseline."
fi

run_tool_command() {
  local tool=$1
  local dest=$2
  case "$tool" in
    blit)
      "$BLIT_BIN" "${CONFIG_ARGS[@]}" mirror "$SRC_DIR" "$dest"
      ;;
    rsync)
      local args=()
      local filtered=()
      local opt

      read -r -a args <<<"$RSYNC_ARGS"
      if ! rsync --help 2>&1 | grep -q -- '--no-compress'; then
        for opt in "${args[@]}"; do
          if [[ "$opt" == "--no-compress" ]]; then
            continue
          fi
          filtered+=("$opt")
        done
        args=("${filtered[@]}")
      fi
      if rsync --help 2>&1 | grep -q -- '--no-inc-recursive'; then
        args+=(--no-inc-recursive)
      fi
      if [[ "$RSYNC_TIMEOUT" != "0" ]]; then
        args+=(--timeout "$RSYNC_TIMEOUT")
      fi
      log "[rsync] args: ${args[*]}"
      rsync "${args[@]}" "$SRC_DIR/" "$dest/"
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

  if [[ "$PRESERVE_DEST" == "1" ]]; then
    mkdir -p "$dest"
  else
    rm -rf "$dest"
    mkdir -p "$dest"
  fi

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
done

apply_incremental_changes

for ((i = 1; i <= RUNS; i++)); do
  tool_count=${#TOOL_NAMES[@]}
  for offset in "${!TOOL_NAMES[@]}"; do
    idx=$(( (offset + i - 1) % tool_count ))
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

"$BLIT_BIN" "${CONFIG_ARGS[@]}" diagnostics perf --enable \
  >>"$LOG_FILE" 2>&1 || true
log "Benchmark complete. Full log: $LOG_FILE"
