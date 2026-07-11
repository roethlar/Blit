#!/usr/bin/env bash
# otp-11 local perf gate (docs/plan/OTP11_LOCAL_SESSION.md, 11a step 4):
# A/B the OLD local-orchestration binary against the NEW session-route
# binary over the local cells, interleaved (old,new per round) so cache
# state is levelled. Gate: per cell, NEW median <= OLD median + 10%.
#
# Usage:
#   OLD_BIN=/path/to/pre-otp11/blit NEW_BIN=/path/to/otp11/blit \
#     [RUNS=3] [BENCH_ROOT=/path] scripts/bench_otp11_local_ab.sh
#
# Cells:
#   huge  — 1 GiB single file, copy into a fresh dest each run
#           (clonefile/block-clone sensitivity: the reason pure byte
#           relay was rejected in the slice doc's D1).
#   tree  — 256 MiB payload + 32 small dirs (bench_local_mirror.sh's
#           default shape), copy into a fresh dest each run.
#   small — 10,000 x 4 KiB files, copy into a fresh dest each run.
#   noop  — mirror over an already-synced tree (the no-op mirror pin;
#           dest pre-synced once per binary with that same binary).
set -euo pipefail

OLD_BIN=${OLD_BIN:?set OLD_BIN to the pre-otp-11 blit binary}
NEW_BIN=${NEW_BIN:?set NEW_BIN to the otp-11 blit binary}
RUNS=${RUNS:-3}
ROOT=${BENCH_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/otp11_ab.XXXXXX")}
mkdir -p "$ROOT"
CFG_OLD="$ROOT/cfg_old"
CFG_NEW="$ROOT/cfg_new"
mkdir -p "$CFG_OLD" "$CFG_NEW"

log() { echo "$*" >&2; }

log "workspace: $ROOT (preserved)"
log "old: $OLD_BIN"
log "new: $NEW_BIN"

gen_fixtures() {
  python3 - "$ROOT" <<'PY'
import os, sys
root = sys.argv[1]

huge = os.path.join(root, "src_huge")
os.makedirs(huge, exist_ok=True)
p = os.path.join(huge, "payload.bin")
if not os.path.exists(p):
    with open(p, "wb") as fh:
        for _ in range(1024):
            fh.write(os.urandom(1024 * 1024))

tree = os.path.join(root, "src_tree")
os.makedirs(tree, exist_ok=True)
p = os.path.join(tree, "payload.bin")
if not os.path.exists(p):
    with open(p, "wb") as fh:
        for _ in range(256):
            fh.write(os.urandom(1024 * 1024))
    for idx in range(32):
        sub = os.path.join(tree, f"dir_{idx:02d}")
        os.makedirs(sub, exist_ok=True)
        with open(os.path.join(sub, "file.txt"), "w") as fh:
            fh.write("hello world\n" * (idx + 1))

small = os.path.join(root, "src_small")
if not os.path.isdir(small):
    payload = b"X" * 4096
    for idx in range(10_000):
        sub = os.path.join(small, f"d{idx // 1000:02d}")
        os.makedirs(sub, exist_ok=True)
        with open(os.path.join(sub, f"f{idx:05d}.bin"), "wb") as fh:
            fh.write(payload)
print("fixtures ready")
PY
}

# run_cell BIN CFG VERB SRC DST  -> prints elapsed ms
run_cell() {
  local bin=$1 cfg=$2 verb=$3 src=$4 dst=$5
  local t0 t1
  t0=$(python3 -c 'import time; print(int(time.time()*1000))')
  "$bin" --config-dir "$cfg" "$verb" "$src" "$dst" --yes >/dev/null 2>&1
  t1=$(python3 -c 'import time; print(int(time.time()*1000))')
  echo $((t1 - t0))
}

median() {
  printf '%s\n' "$@" | sort -n | awk '{a[NR]=$1} END {print (NR%2) ? a[(NR+1)/2] : int((a[NR/2]+a[NR/2+1])/2)}'
}

gen_fixtures

declare -a CELLS=("huge" "tree" "small" "noop")
declare -a RESULTS=()

# noop cell: pre-sync one dest per binary with that binary.
NOOP_OLD="$ROOT/noop_dst_old"
NOOP_NEW="$ROOT/noop_dst_new"
"$OLD_BIN" --config-dir "$CFG_OLD" mirror "$ROOT/src_tree" "$NOOP_OLD" --yes >/dev/null 2>&1
"$NEW_BIN" --config-dir "$CFG_NEW" mirror "$ROOT/src_tree" "$NOOP_NEW" --yes >/dev/null 2>&1

for cell in "${CELLS[@]}"; do
  declare -a old_ms=() new_ms=()
  for ((run = 1; run <= RUNS; run++)); do
    for side in old new; do
      if [[ $side == old ]]; then bin=$OLD_BIN; cfg=$CFG_OLD; else bin=$NEW_BIN; cfg=$CFG_NEW; fi
      case $cell in
        huge)
          dst="$ROOT/dst_huge_$side"
          rm -rf "$dst"
          ms=$(run_cell "$bin" "$cfg" copy "$ROOT/src_huge" "$dst")
          ;;
        tree)
          dst="$ROOT/dst_tree_$side"
          rm -rf "$dst"
          ms=$(run_cell "$bin" "$cfg" copy "$ROOT/src_tree" "$dst")
          ;;
        small)
          dst="$ROOT/dst_small_$side"
          rm -rf "$dst"
          ms=$(run_cell "$bin" "$cfg" copy "$ROOT/src_small" "$dst")
          ;;
        noop)
          if [[ $side == old ]]; then dst=$NOOP_OLD; else dst=$NOOP_NEW; fi
          ms=$(run_cell "$bin" "$cfg" mirror "$ROOT/src_tree" "$dst")
          ;;
      esac
      if [[ $side == old ]]; then old_ms+=("$ms"); else new_ms+=("$ms"); fi
      log "cell=$cell run=$run $side=${ms}ms"
    done
  done
  om=$(median "${old_ms[@]}")
  nm=$(median "${new_ms[@]}")
  verdict=PASS
  # Gate: new <= old * 1.10 (integer math: new*10 <= old*11).
  if ((nm * 10 > om * 11)); then verdict=FAIL; fi
  RESULTS+=("$cell old_median=${om}ms new_median=${nm}ms verdict=$verdict")
done

echo
echo "== otp-11 local A/B (RUNS=$RUNS, workspace $ROOT) =="
overall=PASS
for row in "${RESULTS[@]}"; do
  echo "$row"
  [[ $row == *FAIL* ]] && overall=FAIL
done
echo "overall: $overall"
[[ $overall == PASS ]]
