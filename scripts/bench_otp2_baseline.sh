#!/usr/bin/env bash
# otp-2: symmetric-fs disk-to-disk baseline of the OLD transfer paths
# (ONE_TRANSFER_PATH plan, slice 2). This is the converge-up reference
# the otp-12 acceptance run compares against, per cell, per direction.
#
# Methodology (corrects the sf-1 harness — see
# docs/bench/10gbe-2026-07-05/DIAGNOSIS.md for what it replaces):
#   * VERDICT CELLS are symmetric-fs disk-to-disk: the client end's
#     data lives on the client machine's real disk (never /tmp — on
#     Linux that is tmpfs), the daemon end's module root lives on the
#     daemon machine's real pool. Both directions of a cell use the
#     SAME two storage ends, so push vs pull is a fair comparison.
#   * COLD CACHES before every timed run, both ends: `purge` on the
#     macOS client (needs a NOPASSWD sudoers rule), drop_caches on the
#     Linux daemon host.
#   * DURABLE-AT-DESTINATION timing: the timed window is the transfer
#     PLUS a destination flush — remote `sync` for pushes (Linux sync
#     waits for writeback), a per-file fsync walk for pulls (macOS
#     sync(2) only SCHEDULES writes, so a bare local sync would
#     under-time pulls relative to pushes). Without durable windows a
#     run's number is a write-cache lottery — probe 1 showed up to 8x
#     spread on push cells purely from how much of the payload the
#     pool absorbed into cache before writeback throttled.
#   * POOL DRAIN before every timed run, AFTER flushing dirty pages
#     (sync first, then wait quiet): the daemon host's write path has
#     state (an NVMe tier destaging to the spinning RAID); pushes
#     timed against a partially-full tier ascend 2.7s -> 13.4s for
#     identical work (probe run 2). Quiet = three consecutive 2s
#     windows under 2 MiB written; a drain TIMEOUT is recorded against
#     the run's label, never silent.
#   * MEDIAN is the cell statistic (robust to the residual one-in-four
#     outlier drained pushes still show); avg and best recorded too.
#     All times integer ms; an even-count median is the floor of the
#     mean of the middle two.
#   * FRESH destination every run (blit no-ops onto delivered
#     content), unique per invocation (an interrupted run cannot
#     leave content a rerun would no-op onto).
#   * Prerequisite: python3 on the client (monotonic timing + the
#     fsync walk).
#   * No competitor rows (D-2026-07-04-4: ceiling-driven, never
#     competitor-relative). The July tmpfs/warm rows remain in
#     docs/bench/10gbe-2026-07-05/ as explicitly-labeled
#     wire-reference data only.
#
# Cells: {large, small, mixed} x {push, pull} x {tcp, grpc} = 12.
# Fixture shapes match sf-1 for continuity: large = 1 GiB single file,
# small = 10,000 x 4 KiB, mixed = 512 MiB + 5,000 x 2 KiB.
#
# Usage (from the client Mac):
#   export ZOEY_SSH=root@zoey
#   export ZOEY_TEMP=/volume/<pool>/.srv/.unifi-drive/michael/.data/blit-temp
#   export ZOEY_HOST=10.1.10.206        # pin the 10GbE path by IP
#   ./scripts/bench_otp2_baseline.sh
#
# The daemon binary must already be staged at $ZOEY_TEMP/blit-daemon
# (static aarch64-musl build of the SAME commit as the local client).
# Everything on the daemon host stays inside $ZOEY_TEMP (owner rule).

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
BLIT="$REPO_ROOT/target/release/blit"

ZOEY_SSH=${ZOEY_SSH:-root@zoey}
ZOEY_TEMP=${ZOEY_TEMP:?set ZOEY_TEMP to the blit-temp folder on the daemon host}
ZOEY_HOST=${ZOEY_HOST:-10.1.10.206}
PORT=${PORT:-9031}
RUNS=${RUNS:-3}
# Real-disk client workdir. NOT /tmp: keep the client end on APFS SSD.
MAC_WORK=${MAC_WORK:-$HOME/blit-bench-work}

OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp2_baseline_$(date +%Y%m%dT%H%M%S)}
mkdir -p "$OUT_DIR" "$MAC_WORK"

MODULE_ROOT="$ZOEY_TEMP/bench-module"
REMOTE="$ZOEY_HOST:$PORT:/bench/"

log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log"; }
zssh() { ssh -o BatchMode=yes "$ZOEY_SSH" "$@"; }
# Wall-clock ms. Deliberately NOT time.monotonic(): its reference
# point is per-process-undefined, and start/end here are two separate
# python3 processes — a monotonic attempt produced 0/negative windows
# while the daemon log showed multi-second transfers. Wall clock is
# correct across processes; the windows are seconds long and the
# median absorbs the (rare) NTP-step outlier. python3 is a documented
# prerequisite (preflight-checked).
now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }
# Durable pull window (codex otp-2 F2): macOS sync(2) SCHEDULES writes
# and may return early, unlike Linux sync(2) which waits — so a bare
# `sync` under-times the pull cells relative to the push cells' remote
# sync. fsync every file in the dest tree instead: on macOS fsync
# flushes to the drive, the closest equivalent of Linux sync's
# wait-for-writeback depth (F_FULLFSYNC-to-media is deliberately NOT
# used — the Linux side does not pay media-flush either).
fsync_tree() {
    python3 - "$1" <<'PYEOF'
import os, sys
for root, dirs, files in os.walk(sys.argv[1]):
    for name in files:
        fd = os.open(os.path.join(root, name), os.O_RDONLY)
        os.fsync(fd)
        os.close(fd)
PYEOF
}

# --- Preflight -------------------------------------------------------
[[ -x "$BLIT" ]] || { echo "missing $BLIT (cargo build --release first)"; exit 1; }
command -v python3 >/dev/null || { echo "python3 required (timing + fsync_tree)"; exit 1; }
sudo -n /usr/sbin/purge || {
    echo "cold-cache purge needs a NOPASSWD sudoers rule for /usr/sbin/purge"; exit 1; }
zssh "test -x '$ZOEY_TEMP/blit-daemon'" || {
    echo "daemon binary not staged at $ZOEY_TEMP/blit-daemon"; exit 1; }
BUILD_SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD)
SESSION_TAG=$(date +%H%M%S).$$
log "build sha: $BUILD_SHA  client: $(uname -m) macOS  daemon: $ZOEY_HOST  session: $SESSION_TAG"

# --- Daemon lifecycle (everything inside ZOEY_TEMP) ------------------
start_daemon() {
    zssh "mkdir -p '$MODULE_ROOT' && cat > '$ZOEY_TEMP/bench-config.toml' <<EOF
[daemon]
bind = \"0.0.0.0\"
port = $PORT
no_mdns = true

[[module]]
name = \"bench\"
path = \"$MODULE_ROOT\"
EOF
nohup '$ZOEY_TEMP/blit-daemon' --config '$ZOEY_TEMP/bench-config.toml' \
  > '$ZOEY_TEMP/bench-daemon.log' 2>&1 &
echo \$! > '$ZOEY_TEMP/bench-daemon.pid'"
    sleep 1
    zssh "kill -0 \$(cat '$ZOEY_TEMP/bench-daemon.pid')" \
        || { zssh "cat '$ZOEY_TEMP/bench-daemon.log'"; exit 1; }
    log "daemon up on $ZOEY_HOST:$PORT (module bench -> $MODULE_ROOT)"
}

stop_daemon() {
    zssh "kill \$(cat '$ZOEY_TEMP/bench-daemon.pid' 2>/dev/null) 2>/dev/null; \
          rm -f '$ZOEY_TEMP/bench-daemon.pid'" || true
}
# Sweep this invocation's push destinations even on an interrupted run
# (F5) — never leave content a rerun could no-op onto. Staged pull
# sources are kept for re-runs by design.
sweep_push_dirs() {
    zssh "cd '$MODULE_ROOT' 2>/dev/null && rm -rf push_${SESSION_TAG}_*" || true
}
trap 'stop_daemon; sweep_push_dirs' EXIT

# --- Pool drain + cold caches, both ends ------------------------------
# Order matters (codex otp-2 F4): FIRST flush the daemon host's dirty
# pages into the pool (`sync` — Linux sync waits), THEN wait for the
# tier to destage until quiet (three consecutive 2s windows with
# < 2 MiB written across all physical disks; timeout 240s), then cold
# the caches. A drain timeout is recorded against the run's label in
# drain.log AND bench.log so an undrained row is identifiable, never
# silent.
drain_pool() {
    zssh 'sync
quiet=0
for i in $(seq 1 120); do
  a=$(awk "\$3 ~ /^sd[a-z]\$|^nvme[01]n1\$/ {s+=\$10} END {printf \"%.0f\", s}" /proc/diskstats)
  sleep 2
  b=$(awk "\$3 ~ /^sd[a-z]\$|^nvme[01]n1\$/ {s+=\$10} END {printf \"%.0f\", s}" /proc/diskstats)
  if [ $((b-a)) -lt 4096 ]; then quiet=$((quiet+1)); else quiet=0; fi
  if [ $quiet -ge 3 ]; then echo "drained ${i}x2s"; exit 0; fi
done
echo "DRAIN-TIMEOUT"'
}

drop_caches() {   # $1 = run label for the drain record
    local outcome
    outcome=$(drain_pool)
    echo "$1: $outcome" >> "$OUT_DIR/drain.log"
    if [[ "$outcome" == *DRAIN-TIMEOUT* ]]; then
        log "  WARNING: $1 ran UNDRAINED (pool never went quiet)"
    fi
    sync
    sudo -n /usr/sbin/purge
    zssh "echo 3 > /proc/sys/vm/drop_caches"
}

# --- Fixtures (client disk; generated once) --------------------------
gen_fixtures() {
    if [[ ! -d "$MAC_WORK/src_large" ]]; then
        mkdir -p "$MAC_WORK/src_large"
        dd if=/dev/urandom of="$MAC_WORK/src_large/large_1024M.bin" bs=1m count=1024 2>/dev/null
        log "generated large fixture (1 GiB)"
    fi
    if [[ ! -d "$MAC_WORK/src_small" ]]; then
        mkdir -p "$MAC_WORK/src_small"
        for i in $(seq 1 10000); do
            d="$MAC_WORK/src_small/d$(( i / 1000 ))"; mkdir -p "$d"
            dd if=/dev/urandom of="$d/f${i}.dat" bs=4096 count=1 2>/dev/null
        done
        log "generated small fixture (10000 x 4 KiB)"
    fi
    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
        mkdir -p "$MAC_WORK/src_mixed"
        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
        for i in $(seq 1 5000); do
            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
            dd if=/dev/urandom of="$d/f${i}.dat" bs=2048 count=1 2>/dev/null
        done
        log "generated mixed fixture (512 MiB + 5000 x 2 KiB)"
    fi
}

# --- Timing core ------------------------------------------------------
CSV="$OUT_DIR/results.csv"
echo "cell,run,ms" > "$CSV"
SUMMARY="$OUT_DIR/summary.csv"
echo "cell,median_ms,avg_ms,best_ms" > "$SUMMARY"

finish_cell() {  # label total best  (per-run times read back from CSV)
    local label="$1" total="$2" best="$3"
    local median
    median=$(grep "^$label," "$CSV" | cut -d, -f3 | sort -n | awk '
        { v[NR] = $1 }
        END { if (NR % 2) print v[(NR+1)/2];
              else print int((v[NR/2] + v[NR/2+1]) / 2) }')
    echo "$label,$median,$(( total / RUNS )),$best" >> "$SUMMARY"
    log "  $label median: ${median}ms avg: $(( total / RUNS ))ms best: ${best}ms"
}

# push: client fixture -> fresh, never-seen module subdir per run.
# SESSION_TAG makes destinations unique per INVOCATION too (codex otp-2
# F5): an interrupted run's leftovers can never turn a rerun's copy
# into a partial no-op; the EXIT trap also sweeps them.
push_cell() {    # label src flag(optional)
    local label="$1" src="$2" flag="${3:-}"
    local total=0 best=999999999 run start end ms
    for run in $(seq 1 "$RUNS"); do
        drop_caches "$label-r$run"
        start=$(now_ms)
        # shellcheck disable=SC2086
        "$BLIT" copy "$src" "${REMOTE}push_${SESSION_TAG}_${label}_r${run}/" --yes $flag >/dev/null 2>&1
        zssh sync   # durable at the destination (zoey pool; Linux sync waits)
        end=$(now_ms)
        ms=$(( end - start ))
        total=$(( total + ms )); (( ms < best )) && best=$ms
        log "  $label run $run: ${ms}ms"
        echo "$label,$run,$ms" >> "$CSV"
    done
    finish_cell "$label" "$total" "$best"
}

# pull: staged module subdir -> fresh local dest per run.
pull_cell() {    # label remote_src flag(optional)
    local label="$1" remote_src="$2" flag="${3:-}"
    local total=0 best=999999999 run start end ms
    for run in $(seq 1 "$RUNS"); do
        rm -rf "$MAC_WORK/dst_pull"
        mkdir -p "$MAC_WORK/dst_pull"
        drop_caches "$label-r$run"
        start=$(now_ms)
        # shellcheck disable=SC2086
        "$BLIT" copy "$remote_src" "$MAC_WORK/dst_pull" --yes $flag >/dev/null 2>&1
        fsync_tree "$MAC_WORK/dst_pull"   # durable at the destination (see fsync_tree)
        end=$(now_ms)
        ms=$(( end - start ))
        total=$(( total + ms )); (( ms < best )) && best=$ms
        log "  $label run $run: ${ms}ms"
        echo "$label,$run,$ms" >> "$CSV"
    done
    finish_cell "$label" "$total" "$best"
}

# --- Matrix ------------------------------------------------------------
main() {
    gen_fixtures
    start_daemon

    # Stage pull sources once (untimed): each fixture into its own
    # module subdir. Caches are dropped before every timed pull, so
    # the staging write does not warm anything that matters.
    log "staging pull sources (untimed)"
    local w
    for w in large small mixed; do
        "$BLIT" copy "$MAC_WORK/src_$w" "${REMOTE}pull_src_$w/" --yes >/dev/null 2>&1
    done

    for w in large small mixed; do
        push_cell "push_tcp_${w}" "$MAC_WORK/src_$w"
        push_cell "push_grpc_${w}" "$MAC_WORK/src_$w" --force-grpc
        pull_cell "pull_tcp_${w}" "${REMOTE}pull_src_$w/src_$w/"
        pull_cell "pull_grpc_${w}" "${REMOTE}pull_src_$w/src_$w/" --force-grpc
    done

    stop_daemon

    log ""
    log "=== SUMMARY (cold-cache, disk-to-disk, $RUNS runs/cell) ==="
    column -t -s, "$SUMMARY" | tee -a "$OUT_DIR/bench.log"
    log "results: $CSV"
}

main "$@"
