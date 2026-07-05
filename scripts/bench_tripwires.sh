#!/usr/bin/env bash
# Tripwire + stream-scaling harness (SMALL_FILE_CEILING sf-1).
#
# Re-runs the 2026-07-05 tool-comparison matrix against any daemon host
# in one command, plus a stream-scaling probe (files/s vs negotiated
# stream count). Derived from scripts/bench_10gbe.sh and the ad-hoc
# session runner behind docs/bench/10gbe-2026-07-05/tool_comparison.csv
# (same CSV schema, so runs are directly comparable to the committed
# baseline).
#
# Tripwire semantics (docs/plan/SMALL_FILE_CEILING.md, Principle):
# the tools here are NOT targets — any cell where any tool measures
# faster than blit is proof blit is off its hardware ceiling and is a
# finding to fix. The harness matrix and the plan's tripwire list are
# the same set by construction.
#
# Usage (one command against a daemon host):
#   DAEMON_HOST=skippy \
#   REMOTE_ROOT=/mnt/generic-pool/video/blit-bin/bench-data \
#   REMOTE_BLIT_DAEMON=/mnt/generic-pool/video/blit-bin/blit-daemon \
#   ./scripts/bench_tripwires.sh [matrix|scale|all]     # default: all
#
#   Local-only tripwires (no DAEMON_HOST): blit vs rsync/rclone/cp on
#   this machine's ${TMPDIR:-/tmp}.
#
# Environment:
#   DAEMON_HOST        network + ssh name of the daemon host (remote cells)
#   SSH_HOST           ssh alias if it differs from DAEMON_HOST
#   REMOTE_ROOT        writable dir on the daemon host; a per-invocation
#                      session dir is created (and removed) under it.
#                      NOTE: must be exec-friendly for SPIN_DAEMONS — on
#                      TrueNAS /tmp and /home are noexec (session lesson).
#   REMOTE_BLIT_DAEMON path to blit-daemon ON the daemon host
#   SPIN_DAEMONS=1     spin blitd (--root, module "default") + rsyncd on
#                      the daemon host over ssh; 0 = daemons already run
#                      (then set BLIT_PORT/BLIT_MODULE/RSYNCD_PORT and
#                      optionally BLITD_LOG for scale-mode stream counts)
#   BLIT_PORT=9031  BLIT_MODULE=default  RSYNCD_PORT=8730
#   BLITD_LOG          remote path of blitd's stderr log (scale mode
#                      stream counting when SPIN_DAEMONS=0)
#   RUNS=2             timed runs per cell (baseline was best-of-2)
#   TIMEOUT_S=600      per-run cap (a wedged tool records status 124)
#   RCLONE_TRANSFERS=16  rclone best-config concurrency (fairness flags
#                      --ignore-checksum + tuned --transfers per
#                      docs/bench/10gbe-2026-07-05/DIAGNOSIS.md)
#   SIZE_MB=1024 SMALL_COUNT=10000 SMALL_SIZE=4096   workload knobs
#   SCALE_COUNTS="200 1000 5000 10000 25000 50000"   probe file counts
#                      (chosen to cross engine::initial_stream_proposal
#                      tiers: expected proposals 1/2/4/8/8/10)
#   BASELINE_CSV       committed baseline to diff blit cells against
#                      (default docs/bench/10gbe-2026-07-05/tool_comparison.csv)
#
# Requirements: ssh key access to the host (rsync-over-ssh and
# rclone-sftp cells deliberately pay the cipher tax — that is their
# datapoint); rsync on both ends; rclone on the client. Missing tools
# skip their cells with a note instead of failing the run.
#
# Methodology (matches the committed baseline): local ends on
# ${TMPDIR:-/tmp} (tmpfs on the rig), fresh never-seen target dirs for
# EVERY timed run (blit and rsync both no-op onto delivered content),
# pull sources seeded once per workload (write path leaves ZFS ARC
# warm, so pulls are warm re-reads), async writes, no sync between
# runs, wall-clock ms.
#
# Exit codes: 0 = ran and no tripwire tripped; 3 = at least one tool
# beat blit somewhere (the summary names the cells); 1 = harness error.

set -euo pipefail

MODE=${1:-all}
case "$MODE" in matrix|scale|all) ;; *) echo "usage: $0 [matrix|scale|all]" >&2; exit 1;; esac

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
BLIT=${BLIT:-"$REPO_ROOT/target/release/blit"}

DAEMON_HOST=${DAEMON_HOST:-}
SSH_HOST=${SSH_HOST:-$DAEMON_HOST}
REMOTE_ROOT=${REMOTE_ROOT:-}
REMOTE_BLIT_DAEMON=${REMOTE_BLIT_DAEMON:-}
SPIN_DAEMONS=${SPIN_DAEMONS:-1}
BLIT_PORT=${BLIT_PORT:-9031}
BLIT_MODULE=${BLIT_MODULE:-default}
RSYNCD_PORT=${RSYNCD_PORT:-8730}
BLITD_LOG=${BLITD_LOG:-}
RUNS=${RUNS:-2}
TIMEOUT_S=${TIMEOUT_S:-600}
RCLONE_TRANSFERS=${RCLONE_TRANSFERS:-16}
SIZE_MB=${SIZE_MB:-1024}
SMALL_COUNT=${SMALL_COUNT:-10000}
SMALL_SIZE=${SMALL_SIZE:-4096}
SCALE_COUNTS=${SCALE_COUNTS:-"200 1000 5000 10000 25000 50000"}
BASELINE_CSV=${BASELINE_CSV:-"$REPO_ROOT/docs/bench/10gbe-2026-07-05/tool_comparison.csv"}

[[ -x "$BLIT" ]] || { echo "blit binary not found at $BLIT (build with cargo build --release or set BLIT=)" >&2; exit 1; }

WORK=$(mktemp -d "${TMPDIR:-/tmp}/blit_tripwires.XXXXXX")
STAMP=$(date +%Y%m%dT%H%M%S)
LOG_DIR="$REPO_ROOT/logs/tripwires_$STAMP"
mkdir -p "$LOG_DIR"
MATRIX_CSV="$LOG_DIR/matrix.csv"
SCALE_CSV="$LOG_DIR/scale.csv"

HAVE_RSYNC=1; command -v rsync >/dev/null || HAVE_RSYNC=0
HAVE_RCLONE=1; command -v rclone >/dev/null || HAVE_RCLONE=0

log() { echo "$(date +%H:%M:%S) $*" | tee -a "$LOG_DIR/bench.log"; }

rssh() { ssh -o BatchMode=yes "$SSH_HOST" "$@"; }

# ── remote session lifecycle ─────────────────────────────────────────
SESSION=""            # per-invocation dir under REMOTE_ROOT
BLITD_STARTED=0
RSYNCD_STARTED=0

teardown() {
    rm -rf "$WORK"
    if [[ -n "$SESSION" ]]; then
        if (( BLITD_STARTED )); then
            rssh "kill \$(cat '$SESSION/blitd.pid') 2>/dev/null" || true
        fi
        if (( RSYNCD_STARTED )); then
            rssh "kill \$(cat '$SESSION/rsyncd.pid') 2>/dev/null" || true
        fi
        # Only ever the directory this invocation created.
        rssh "rm -rf '$SESSION'" || true
    fi
}
trap teardown EXIT

setup_remote() {
    [[ -n "$REMOTE_ROOT" ]] || { echo "DAEMON_HOST set but REMOTE_ROOT is empty" >&2; exit 1; }
    rssh "true" || { echo "cannot ssh to $SSH_HOST" >&2; exit 1; }
    SESSION="$REMOTE_ROOT/tripwires_$STAMP"
    rssh "mkdir -p '$SESSION/push' '$SESSION/seed'"

    HAVE_REMOTE_RSYNC=$(rssh "command -v rsync >/dev/null && echo 1 || echo 0")

    if (( SPIN_DAEMONS )); then
        [[ -n "$REMOTE_BLIT_DAEMON" ]] || { echo "SPIN_DAEMONS=1 needs REMOTE_BLIT_DAEMON" >&2; exit 1; }
        log "spinning blit-daemon on $SSH_HOST (--root $SESSION, port $BLIT_PORT)"
        rssh "nohup '$REMOTE_BLIT_DAEMON' --root '$SESSION' --port $BLIT_PORT --no-mdns \
                  > '$SESSION/blitd.log' 2>&1 & echo \$! > '$SESSION/blitd.pid'"
        BLITD_STARTED=1
        BLIT_MODULE=default
        BLITD_LOG="$SESSION/blitd.log"
        if [[ "$HAVE_REMOTE_RSYNC" == 1 ]]; then
            log "spinning rsyncd on $SSH_HOST (port $RSYNCD_PORT, module bench -> $SESSION)"
            rssh "printf 'port = %s\npid file = %s/rsyncd.pid\nuse chroot = false\n[bench]\n  path = %s\n  read only = false\n' \
                      '$RSYNCD_PORT' '$SESSION' '$SESSION' > '$SESSION/rsyncd.conf' && \
                  rsync --daemon --config='$SESSION/rsyncd.conf'"
            RSYNCD_STARTED=1
        else
            log "NOTE: rsync missing on $SSH_HOST — rsyncd + rsync_ssh cells skipped"
        fi
        sleep 1   # both daemons bind before the first cell
    fi
    BLIT_EP="$DAEMON_HOST:$BLIT_PORT:/$BLIT_MODULE/"    # trailing slash is load-bearing (endpoint.rs)
    RSYNCD_URL="rsync://$DAEMON_HOST:$RSYNCD_PORT/bench"
}

# ── timing ───────────────────────────────────────────────────────────
# One timed run: records transport,direction,workload,run,ms,status
# (identical schema to the committed baseline CSV). Never aborts the
# harness on tool failure — the status column carries it.
timed_row() {
    local transport="$1" direction="$2" workload="$3" run="$4"; shift 4
    local start end ms status=0
    start=$(date +%s%N)
    timeout "$TIMEOUT_S" "$@" >/dev/null 2>&1 || status=$?
    end=$(date +%s%N)
    ms=$(( (end - start) / 1000000 ))
    log "  $transport $direction $workload r$run: ${ms}ms (status $status)"
    echo "$transport,$direction,$workload,$run,$ms,$status" >> "$MATRIX_CSV"
}

fresh_local() { rm -rf "$1"; mkdir -p "$1"; }
fresh_remote() { rssh "rm -rf '$1' && mkdir -p '$1'"; }

# ── workload generation (same shapes as the baseline) ───────────────
gen_large() { mkdir -p "$1"; dd if=/dev/urandom of="$1/large_${SIZE_MB}M.bin" bs=1M count="$SIZE_MB" 2>/dev/null; }
gen_small_n() { # $1=dir $2=count $3=size
    local dir="$1" count="$2" size="$3" i sub
    mkdir -p "$dir"
    for i in $(seq 1 "$count"); do
        sub="$dir/d$(( i / 1000 ))"
        mkdir -p "$sub"
        dd if=/dev/urandom of="$sub/f${i}.dat" bs="$size" count=1 2>/dev/null
    done
}
gen_mixed() {
    mkdir -p "$1"
    dd if=/dev/urandom of="$1/big.bin" bs=1M count=512 2>/dev/null
    gen_small_n "$1/smalls" 5000 2048
}

# ── the matrix ───────────────────────────────────────────────────────
run_matrix() {
    echo "transport,direction,workload,run,ms,status" > "$MATRIX_CSV"

    log "=== generating workloads (large ${SIZE_MB}M / small ${SMALL_COUNT}x${SMALL_SIZE}B / mixed 512M+5000x2K) ==="
    gen_large "$WORK/src_large"
    gen_small_n "$WORK/src_small" "$SMALL_COUNT" "$SMALL_SIZE"
    gen_mixed "$WORK/src_mixed"

    local workload src run dst
    for workload in large small mixed; do
        src="$WORK/src_$workload"

        log "=== local cells: $workload ==="
        for run in $(seq 1 "$RUNS"); do
            dst="$WORK/dst_local"
            fresh_local "$dst"; timed_row blit local "$workload" "$run" "$BLIT" copy "$src/" "$dst/" --yes
            if (( HAVE_RSYNC )); then
                fresh_local "$dst"; timed_row rsync local "$workload" "$run" rsync -a --whole-file --inplace --no-compress "$src/" "$dst/"
            fi
            if (( HAVE_RCLONE )); then
                fresh_local "$dst"; timed_row rclone local "$workload" "$run" rclone copy "$src" "$dst" --ignore-checksum --transfers "$RCLONE_TRANSFERS"
            fi
            fresh_local "$dst"; timed_row cp local "$workload" "$run" cp -a "$src/." "$dst/"
        done

        [[ -n "$DAEMON_HOST" ]] || continue

        log "=== seeding pull source: $workload ==="
        fresh_remote "$SESSION/seed/$workload"
        "$BLIT" copy "$src/" "${BLIT_EP}seed/$workload/" --yes >/dev/null 2>&1 \
            || { echo "seeding $workload over blit failed — is the daemon reachable at $BLIT_EP ?" >&2; exit 1; }

        log "=== remote cells: $workload ==="
        for run in $(seq 1 "$RUNS"); do
            # push — fresh never-seen remote target every run
            fresh_remote "$SESSION/push/blit_${workload}_r${run}"
            timed_row blit push "$workload" "$run" \
                "$BLIT" copy "$src/" "${BLIT_EP}push/blit_${workload}_r${run}/" --yes
            if [[ "$HAVE_REMOTE_RSYNC" == 1 && $HAVE_RSYNC == 1 && $RSYNCD_STARTED == 1 ]]; then
                fresh_remote "$SESSION/push/rsyncd_${workload}_r${run}"
                timed_row rsyncd push "$workload" "$run" \
                    rsync -a --whole-file --inplace --no-compress "$src/" "$RSYNCD_URL/push/rsyncd_${workload}_r${run}/"
            fi
            if [[ "$HAVE_REMOTE_RSYNC" == 1 && $HAVE_RSYNC == 1 ]]; then
                fresh_remote "$SESSION/push/rsync_ssh_${workload}_r${run}"
                timed_row rsync_ssh push "$workload" "$run" \
                    rsync -a --whole-file --inplace --no-compress -e ssh "$src/" "$SSH_HOST:$SESSION/push/rsync_ssh_${workload}_r${run}/"
            fi
            if (( HAVE_RCLONE )); then
                fresh_remote "$SESSION/push/rclone_${workload}_r${run}"
                timed_row rclone_sftp push "$workload" "$run" \
                    rclone copy "$src" ":sftp,host=$SSH_HOST:$SESSION/push/rclone_${workload}_r${run}" \
                        --ignore-checksum --transfers "$RCLONE_TRANSFERS"
            fi

            # pull — same seeded source for every tool, fresh local target
            dst="$WORK/dst_pull"
            fresh_local "$dst"
            timed_row blit pull "$workload" "$run" "$BLIT" copy "${BLIT_EP}seed/$workload/" "$dst/" --yes
            if [[ $RSYNCD_STARTED == 1 && $HAVE_RSYNC == 1 ]]; then
                fresh_local "$dst"
                timed_row rsyncd pull "$workload" "$run" rsync -a --whole-file --inplace --no-compress "$RSYNCD_URL/seed/$workload/" "$dst/"
            fi
            if [[ "$HAVE_REMOTE_RSYNC" == 1 && $HAVE_RSYNC == 1 ]]; then
                fresh_local "$dst"
                timed_row rsync_ssh pull "$workload" "$run" rsync -a --whole-file --inplace --no-compress -e ssh "$SSH_HOST:$SESSION/seed/$workload/" "$dst/"
            fi
            if (( HAVE_RCLONE )); then
                fresh_local "$dst"
                timed_row rclone_sftp pull "$workload" "$run" \
                    rclone copy ":sftp,host=$SSH_HOST:$SESSION/seed/$workload" "$dst" \
                        --ignore-checksum --transfers "$RCLONE_TRANSFERS"
            fi
        done
    done
}

# ── stream-scaling probe ─────────────────────────────────────────────
# files/s vs the stream count the transfer ACTUALLY ran with, measured
# from the daemon's per-stream completion lines ("stream complete",
# data_plane.rs) — not from what the proposal table says it should be.
# The plan's acceptance curve: files/s rises with streams until a named
# hardware limiter binds; flattening at a policy-chosen count is the
# sf-2 finding.
run_scale() {
    [[ -n "$DAEMON_HOST" ]] || { log "scale mode needs DAEMON_HOST"; return; }
    echo "files,bytes,ms,files_per_sec,streams,status" > "$SCALE_CSV"
    local count src target before streams start end ms status
    for count in $SCALE_COUNTS; do
        src="$WORK/scale_src_$count"
        log "=== scale probe: $count x ${SMALL_SIZE}B ==="
        gen_small_n "$src" "$count" "$SMALL_SIZE"
        target="$SESSION/push/scale_$count"
        fresh_remote "$target"
        before=0
        [[ -n "$BLITD_LOG" ]] && before=$(rssh "wc -l < '$BLITD_LOG' 2>/dev/null || echo 0")
        status=0
        start=$(date +%s%N)
        timeout "$TIMEOUT_S" "$BLIT" copy "$src/" "${BLIT_EP}push/scale_$count/" --yes >/dev/null 2>&1 || status=$?
        end=$(date +%s%N)
        ms=$(( (end - start) / 1000000 ))
        streams=""
        [[ -n "$BLITD_LOG" ]] && streams=$(rssh "tail -n +$(( before + 1 )) '$BLITD_LOG' 2>/dev/null | grep -c 'stream complete'" || echo "")
        local fps
        fps=$(awk -v c="$count" -v ms="$ms" 'BEGIN { if (ms > 0) printf "%.1f", c * 1000 / ms; else printf "0" }')
        log "  $count files: ${ms}ms  ${fps} files/s  streams=${streams:-?} (status $status)"
        echo "$count,$(( count * SMALL_SIZE )),$ms,$fps,${streams},$status" >> "$SCALE_CSV"
        rm -rf "$src"
    done
    [[ -n "$BLITD_LOG" ]] || log "NOTE: no BLITD_LOG — streams column empty (set it, or use SPIN_DAEMONS=1)"
}

# ── summary: best-of per cell, tripwire verdict, baseline delta ─────
summarize() {
    log ""
    log "=== TRIPWIRE SUMMARY (best of $RUNS, successful runs only) ==="
    # exit 3 from awk marks "tripped"; anything else from awk is a bug.
    local tripped=0
    awk -F, '
        NR > 1 && $6 == 0 {
            cell = $2 "," $3
            key = $1 SUBSEP cell
            if (!(key in best) || $5 < best[key]) best[key] = $5
            cells[cell] = 1; tools[$1] = 1
        }
        END {
            printf "%-12s %-8s %10s %10s %-12s %s\n", "direction", "workload", "blit_ms", "rival_ms", "rival", "verdict"
            n = 0
            for (cell in cells) {
                if (!(("blit" SUBSEP cell) in best)) continue
                b = best["blit" SUBSEP cell]
                rbest = -1; rname = "-"
                for (t in tools) {
                    if (t == "blit") continue
                    k = t SUBSEP cell
                    if (k in best && (rbest < 0 || best[k] < rbest)) { rbest = best[k]; rname = t }
                }
                split(cell, dw, ",")
                verdict = "clean"
                if (rbest >= 0 && rbest < b) { verdict = "TRIPPED"; n++ }
                printf "%-12s %-8s %10d %10s %-12s %s\n", dw[1], dw[2], b, (rbest < 0 ? "-" : rbest), rname, verdict
            }
            exit (n > 0 ? 3 : 0)
        }' "$MATRIX_CSV" | sort | tee -a "$LOG_DIR/bench.log" || tripped=$?

    if [[ -f "$BASELINE_CSV" ]]; then
        log ""
        log "=== blit vs committed baseline ($BASELINE_CSV, best-of, +/-10% is run noise) ==="
        awk -F, '
            FNR == 1 { file++; next }
            $1 == "blit" && $6 == 0 {
                key = $2 "," $3
                if (file == 1) { if (!(key in base) || $5 < base[key]) base[key] = $5 }
                else           { if (!(key in now)  || $5 < now[key])  now[key] = $5 }
            }
            END {
                for (key in now) {
                    if (key in base)
                        printf "  blit %-14s %6dms -> %6dms  (%+.1f%%)\n", key, base[key], now[key], (now[key] - base[key]) * 100.0 / base[key]
                    else
                        printf "  blit %-14s (no baseline cell)\n", key
                }
            }' "$BASELINE_CSV" "$MATRIX_CSV" | sort | tee -a "$LOG_DIR/bench.log"
    fi
    return "$tripped"
}

# ── main ─────────────────────────────────────────────────────────────
log "tripwire harness: mode=$MODE host=${DAEMON_HOST:-<local-only>} runs=$RUNS out=$LOG_DIR"
(( HAVE_RSYNC ))  || log "NOTE: rsync not installed — rsync cells skipped"
(( HAVE_RCLONE )) || log "NOTE: rclone not installed — rclone cells skipped"

HAVE_REMOTE_RSYNC=0
[[ -n "$DAEMON_HOST" ]] && setup_remote

RC=0
if [[ "$MODE" == matrix || "$MODE" == all ]]; then
    run_matrix
    summarize || RC=$?
fi
if [[ "$MODE" == scale || "$MODE" == all ]]; then
    run_scale
fi

log ""
log "results: $LOG_DIR (matrix.csv / scale.csv / bench.log)"
exit "$RC"
