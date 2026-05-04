#!/usr/bin/env bash
# Benchmark direct remote→remote delegation against the explicit CLI relay path.
#
# Required environment:
#   SRC_REMOTE=server-a:/bench/   # source module/directory endpoint
#   DST_REMOTE=server-b:/bench/   # destination module/directory endpoint
#
# Optional environment:
#   BLIT=target/release/blit
#   SIZE_MB=512
#   RUNS=3
#   LOG_DIR=logs/bench_remote_remote_<timestamp>
#
# The destination daemon must allow delegation for the direct path:
#
#   [delegation]
#   allow_delegated_pull = true
#   allowed_source_hosts = ["server-a.lan"]
#
# The script uses BLIT_TEST_COUNTER_FILE, the same env-gated instrumentation
# used by integration tests, to record CLI outbound data-plane payload bytes.
# Direct runs should report 0. Relay runs should report roughly payload-sized
# byte counts.

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

BLIT=${BLIT:-"$REPO_ROOT/target/release/blit"}
SRC_REMOTE=${SRC_REMOTE:?set SRC_REMOTE, e.g. server-a:/bench/}
DST_REMOTE=${DST_REMOTE:?set DST_REMOTE, e.g. server-b:/bench/}
SIZE_MB=${SIZE_MB:-512}
RUNS=${RUNS:-3}
LOG_DIR=${LOG_DIR:-"$REPO_ROOT/logs/bench_remote_remote_$(date +%Y%m%dT%H%M%S)"}

if [[ ! -x "$BLIT" ]]; then
    echo "blit binary not executable: $BLIT" >&2
    echo "build first, e.g. cargo build --release" >&2
    exit 2
fi

WORK=$(mktemp -d /tmp/blit_remote_remote_bench.XXXXXX)
mkdir -p "$LOG_DIR"

cleanup() {
    rm -rf "$WORK"
}
trap cleanup EXIT

log() {
    echo "$(date +%H:%M:%S) $*" | tee -a "$LOG_DIR/bench.log"
}

remote_join() {
    local base="${1%/}"
    local name="$2"
    printf '%s/%s' "$base" "$name"
}

now_ms() {
    perl -MTime::HiRes=time -e 'printf "%.0f\n", time() * 1000'
}

counter_bytes() {
    local file="$1"
    awk '$1 == "cli_data_plane_outbound_bytes" { sum += $2 } END { print sum + 0 }' "$file" 2>/dev/null || echo 0
}

run_copy() {
    local mode="$1"
    local run="$2"
    local src="$3"
    local dst="$4"
    shift 4

    local counter="$LOG_DIR/${mode}_${run}.counter"
    local start
    local end
    start=$(now_ms)
    BLIT_TEST_COUNTER_FILE="$counter" "$BLIT" copy "$src" "$dst" "$@"
    end=$(now_ms)

    local ms=$((end - start))
    local bytes=$((SIZE_MB * 1024 * 1024))
    local mib_s
    mib_s=$(awk -v size="$SIZE_MB" -v ms="$ms" 'BEGIN { if (ms <= 0) print "inf"; else printf "%.2f", size / (ms / 1000.0) }')
    local cli_bytes
    cli_bytes=$(counter_bytes "$counter")

    log "$mode run $run: ${ms}ms, ${mib_s} MiB/s, cli_data_plane_outbound_bytes=${cli_bytes}"
    echo "$mode,$run,$ms,$bytes,$mib_s,$cli_bytes,$counter" >> "$LOG_DIR/results.csv"
}

PAYLOAD="$WORK/payload_${SIZE_MB}MiB.bin"
BENCH_ID="blit-remote-remote-$(date +%Y%m%dT%H%M%S)"
SRC_FILE=$(remote_join "$SRC_REMOTE" "$BENCH_ID/payload.bin")

log "Generating ${SIZE_MB}MiB payload at $PAYLOAD"
dd if=/dev/urandom of="$PAYLOAD" bs=1m count="$SIZE_MB" 2>/dev/null

echo "mode,run,elapsed_ms,payload_bytes,mib_per_sec,cli_data_plane_outbound_bytes,counter_file" > "$LOG_DIR/results.csv"

log "Staging payload to source: $SRC_FILE"
"$BLIT" copy "$PAYLOAD" "$SRC_FILE" --yes >/dev/null

for run in $(seq 1 "$RUNS"); do
    DIRECT_DST=$(remote_join "$DST_REMOTE" "$BENCH_ID/direct_${run}.bin")
    RELAY_DST=$(remote_join "$DST_REMOTE" "$BENCH_ID/relay_${run}.bin")

    run_copy direct "$run" "$SRC_FILE" "$DIRECT_DST" --yes
    run_copy relay "$run" "$SRC_FILE" "$RELAY_DST" --yes --relay-via-cli
done

log "Results written to $LOG_DIR/results.csv"
log "Cleanup remote benchmark paths manually if desired:"
log "  $BLIT rm $(remote_join "$SRC_REMOTE" "$BENCH_ID") --yes"
log "  $BLIT rm $(remote_join "$DST_REMOTE" "$BENCH_ID") --yes"
