#!/usr/bin/env bash
set -euo pipefail

SIZE_MB=${SIZE_MB:-256}
WORK_ROOT=${BENCH_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/blit_v2_bench.XXXXXX")}
KEEP_WORK=${KEEP_BENCH_DIR:-0}

REPO_ROOT=$(pwd)
V1_ROOT=$(realpath "$REPO_ROOT/..")
V2_ROOT=$(realpath "$REPO_ROOT")

SRC_DIR="$WORK_ROOT/src"
DST_V1="$WORK_ROOT/dst_v1"
DST_V2="$WORK_ROOT/dst_v2"
LOG_FILE="$WORK_ROOT/bench.log"

mkdir -p "$SRC_DIR" "$DST_V1" "$DST_V2"

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
        fh.write("hello world\n" * (idx + 1))
PY
else
  dd if=/dev/urandom of="$SRC_DIR/payload.bin" bs=1M count="$SIZE_MB" status=none
  for idx in $(seq 0 31); do
    subdir=$(printf "%s/dir_%02d" "$SRC_DIR" "$idx")
    mkdir -p "$subdir"
    printf 'hello world\n' >"$subdir/file.txt"
  done
fi

(
  cd "$V1_ROOT"
  cargo build --release --quiet --bin blit
) >>"$LOG_FILE" 2>&1
(
  cd "$V2_ROOT"
  cargo build --release --quiet --bin blit-cli
) >>"$LOG_FILE" 2>&1

V1_BIN="$V1_ROOT/target/release/blit"
V2_BIN="$V2_ROOT/target/release/blit-cli"

measure() {
  local label=$1
  shift
  local start_ns=$(date +%s%N)
  "$@"
  local end_ns=$(date +%s%N)
  local elapsed_ns=$((end_ns - start_ns))
  python3 - "$label" "$elapsed_ns" <<'PY'
import sys
label, elapsed_ns = sys.argv[1], int(sys.argv[2])
print(f"{label}: {elapsed_ns / 1e9:.3f} s")
PY
}

if ! command -v hyperfine >/dev/null 2>&1; then
  echo "hyperfine not found; running sequential timings" | tee -a "$LOG_FILE"
  rm -rf "$DST_V1" && mkdir -p "$DST_V1"
  measure "v1 mirror" "$V1_BIN" mirror "$SRC_DIR" "$DST_V1" --ludicrous-speed
  rm -rf "$DST_V2" && mkdir -p "$DST_V2"
  measure "v2 mirror" "$V2_BIN" mirror "$SRC_DIR" "$DST_V2"
else
  hyperfine \
    --warmup 1 \
    --prepare "rm -rf '$DST_V1' && mkdir -p '$DST_V1'" \
    "$V1_BIN mirror '$SRC_DIR' '$DST_V1' --ludicrous-speed" \
    --prepare "rm -rf '$DST_V2' && mkdir -p '$DST_V2'" \
    "$V2_BIN mirror '$SRC_DIR' '$DST_V2'" | tee -a "$LOG_FILE"
fi

echo
echo "Benchmark artefacts stored in: $WORK_ROOT"
if [ "$KEEP_WORK" != "1" ]; then
  trap 'rm -rf "$WORK_ROOT"' EXIT
fi
