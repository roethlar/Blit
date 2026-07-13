#!/usr/bin/env bash
# =============================================================================
# bench_win_local_ab.sh — blit vs robocopy, LOCAL D: -> E: on netwatch-01.
#
# WHY (owner, 2026-07-13): a local-only A/B on the Windows box. It strips the
# network out entirely — no MTU, no MSS, no initiator layout, no daemon, no
# carrier. If blit trails robocopy HERE, the problem was never the wire.
#
# It is also the cleanest hardware we own for the question: D: and E: are two
# SEPARATE, IDENTICAL Crucial T705 4TB NVMe drives (disk#0 and disk#3), so a
# D:->E: copy has no read/write contention on one device and neither side of
# the copy is a bottleneck the other lacks.
#
# WHAT IT CAN AND CANNOT ANSWER (do not conflate — the bench_baseline_tools.sh
# distinction applies here too):
#   * It CANNOT test the initiator axis. Both tools run on Windows against
#     local paths; there is no initiator to vary. It says NOTHING about P1.
#   * It CAN test the SHIPPING BAR locally ("does blit beat robocopy on the
#     same box, same files, same disks") and it CAN expose a blit-side local
#     regression that the network harnesses would confound with the wire.
#   * The comparison is cross-tool, so it is NOT a controlled protocol
#     comparison — robocopy is a plain Win32 copy loop, blit rides the unified
#     transfer_session (local included, since otp-11). Report it as such.
#
# METHODOLOGY: identical to the blit rig harnesses (anything less is not
# comparable) — cold caches every run, writeback drained BEFORE the window,
# fresh never-seen destination per run, destination container precreated
# outside the window on both arms, durability keyed by the DESTINATION volume,
# ABBA interleave, pair-void with a 2xRUNS cap, nonzero exit voids the run.
# The per-run body runs entirely on Windows (scripts/windows/local-ab-run.ps1)
# so the ssh round trip is outside the timed window.
#
# Build note: bins\f35702a\blit.exe is the SHIPPING transfer code — the only
# delta f35702a..HEAD is bb28ddd (cargo fmt on blit-app/endpoints.rs + a test),
# and otp-11 (local rides the unified session) is already IN f35702a.
#
# Usage:
#   bash scripts/bench_win_local_ab.sh
#   RUNS=8 FIXTURES=mixed bash scripts/bench_win_local_ab.sh
#   PREFLIGHT_ONLY=1 bash scripts/bench_win_local_ab.sh
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

WIN_SSH="${WIN_SSH:-michael@netwatch-01}"
RUNS="${RUNS:-4}"
FIXTURES="${FIXTURES:-large,small,mixed}"
BLIT_EXE="${BLIT_EXE:-D:\\blit-test\\bins\\f35702a\\blit.exe}"
BLIT_SHA="${BLIT_SHA:-f35702a}"
WIN_TEST="${WIN_TEST:-D:\\blit-test}"
SRC_ROOT="${SRC_ROOT:-$WIN_TEST\\bench-module}"
DEST_BASE="${DEST_BASE:-E:\\blit-local-bench}"
DEST_DRIVE="${DEST_DRIVE:-E}"
PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"

SESSION="$(date +%Y%m%dT%H%M%S)"
OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/win_local_ab_$SESSION}"
mkdir -p "$OUT_DIR"
CSV="$OUT_DIR/runs.csv"
echo "fixture,tool,slot,attempt,ms,flush_ms,exit,files,drain,valid" > "$CSV"

SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ControlMaster=auto
         -o "ControlPath=$HOME/.ssh/cm-%r@%h-%p" -o ControlPersist=300)
wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }
log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log"; }
die() { log "FATAL: $*"; exit 1; }

# Fixture shapes — verified, never assumed (the otp-12a F2 rule).
FIX_FILES_large=1
FIX_FILES_small=10000
FIX_FILES_mixed=5001

preflight() {
    [[ "$RUNS" =~ ^[0-9]+$ && "$RUNS" -ge 2 ]] || die "RUNS must be >= 2 (got '$RUNS')"
    wssh "exit 0" || die "cannot ssh $WIN_SSH"
    wssh "if (-not (Test-Path '$BLIT_EXE')) { exit 1 }" \
        || die "blit not staged at $BLIT_EXE"
    wssh "if (Select-String -Path '$BLIT_EXE' -SimpleMatch -Quiet -Pattern '+$BLIT_SHA') { exit 0 } else { exit 1 }" \
        || die "$BLIT_EXE does not embed +$BLIT_SHA — restage the build"
    wssh "if (-not (Test-Path '$WIN_TEST\\purge-standby.ps1')) { exit 1 }" \
        || die "purge-standby.ps1 not staged at $WIN_TEST"
    wssh "if ((Get-Volume -DriveLetter $DEST_DRIVE).SizeRemaining -lt 20GB) { exit 1 }" \
        || die "less than 20 GB free on ${DEST_DRIVE}: — refusing to run"
    local w want got
    for w in ${FIXTURES//,/ }; do
        want="$(eval echo "\$FIX_FILES_$w")"
        [[ -n "$want" ]] || die "unknown fixture '$w' (want: large|small|mixed)"
        got="$(wssh "(Get-ChildItem '$SRC_ROOT\\pull_src_$w\\src_$w' -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count" | tr -cd '0-9')"
        [[ "$got" == "$want" ]] \
            || die "fixture src_$w has $got files, want $want — restage it before measuring"
        log "  fixture src_$w verified ($got files)"
    done
    # Stale destination from an interrupted run would be re-used as a warm cache.
    wssh "Remove-Item -Recurse -Force '$DEST_BASE' -ErrorAction SilentlyContinue" || true
    log "preflight OK  blit=$BLIT_SHA  runs/arm=$RUNS  fixtures=$FIXTURES  D: -> ${DEST_DRIVE}:"
}

stage_runner() {
    wssh "New-Item -ItemType Directory -Force -Path '$WIN_TEST' | Out-Null"
    scp -q -o BatchMode=yes "$SCRIPT_DIR/windows/local-ab-run.ps1" \
        "$WIN_SSH:$WIN_TEST/local-ab-run.ps1" || die "failed to stage local-ab-run.ps1"
    log "runner staged at $WIN_TEST\\local-ab-run.ps1"
}

# One timed run. Sets RUN_MS/RUN_FLUSH/RUN_EXIT/RUN_FILES/RUN_DRAIN/RUN_VALID.
RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=0; RUN_FILES=0; RUN_DRAIN=""; RUN_VALID=yes
one_run() {   # fixture tool tag
    local w="$1" tool="$2" tag="$3" out
    out="$(wssh "pwsh -NoProfile -File '$WIN_TEST\\local-ab-run.ps1' -Tool $tool -Src '$SRC_ROOT\\pull_src_$w\\src_$w' -DestRoot '$DEST_BASE\\$tag' -BlitExe '$BLIT_EXE' -DestDrive $DEST_DRIVE" 2>>"$OUT_DIR/err.log" \
        | tr -d '\r' | sed -n 's/.*R:\([0-9-]*,[0-9-]*,[0-9-]*,[0-9]*,[A-Za-z0-9_-]*\):R.*/\1/p' | head -1)"
    if [[ -z "$out" ]]; then
        RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=99; RUN_FILES=0; RUN_DRAIN="PARSE-FAIL"; RUN_VALID=no
        return 0
    fi
    IFS=, read -r RUN_MS RUN_FLUSH RUN_EXIT RUN_FILES RUN_DRAIN <<< "$out"
    RUN_VALID=yes
    # robocopy: exit 0-7 is SUCCESS (8+ is failure). Every other tool: 0 only.
    if [[ "$2" == robocopy ]]; then
        [[ "$RUN_EXIT" -lt 8 ]] || RUN_VALID=no
    else
        [[ "$RUN_EXIT" == 0 ]] || RUN_VALID=no
    fi
    [[ "$RUN_FLUSH" -ge 0 ]] || { RUN_VALID=no; RUN_FLUSH=0; }   # failed flush voids
    [[ "$RUN_DRAIN" == drained* ]] || RUN_VALID=no                # undrained voids
    local want; want="$(eval echo "\$FIX_FILES_$1")"
    [[ "$RUN_FILES" == "$want" ]] || RUN_VALID=no                 # wrong tree voids
    RUN_MS=$(( RUN_MS + RUN_FLUSH ))
}

run_cell() {   # fixture — ABBA over blit/robocopy, pair-void, 2xRUNS cap
    local w="$1" slot=1 attempts=0 valid=0 max=$(( 2 * RUNS ))
    log "=== $w (blit vs robocopy, ABBA, $RUNS pairs) ==="
    # WARM-UP, DISCARDED (found live 2026-07-13): the cell's FIRST timed run
    # was systematically slow (small: 2256ms vs 1396-1402ms; mixed: 1339ms vs
    # 935-946ms) while `large` — whose predecessor cell leaves almost no
    # cleanup behind — showed none. It is the PREVIOUS cell's teardown (a
    # 10k-file Remove-Item) still settling past the drain, not a property of
    # the tool. ABBA fixes `blit` as slot 1's first arm, so that bill landed on
    # blit in EVERY cell — a systematic bias against whichever tool goes first.
    # One untimed run per cell absorbs it; its result is thrown away.
    one_run "$w" blit "${SESSION}_${w}_warmup"
    log "  $w/warmup (untimed, discarded): ${RUN_MS}ms ($RUN_DRAIN)"
    while (( valid < RUNS && attempts < max )); do
        attempts=$(( attempts + 1 ))
        local order pair_valid=yes tool rowA="" rowB=""
        if (( slot % 2 )); then order="blit robocopy"; else order="robocopy blit"; fi
        for tool in $order; do
            one_run "$w" "$tool" "${SESSION}_${w}_${tool}_s${slot}a${attempts}"
            [[ "$RUN_VALID" == yes ]] || pair_valid=no
            local row="$w,$tool,$slot,$attempts,$RUN_MS,$RUN_FLUSH,$RUN_EXIT,$RUN_FILES,$RUN_DRAIN"
            if [[ "$tool" == blit ]]; then rowA="$row"; else rowB="$row"; fi
            log "  $w/$tool slot $slot (attempt $attempts): ${RUN_MS}ms (flush ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_FILES files, $RUN_DRAIN)"
        done
        echo "$rowA,$pair_valid" >> "$CSV"
        echo "$rowB,$pair_valid" >> "$CSV"
        if [[ "$pair_valid" == yes ]]; then
            valid=$(( valid + 1 )); slot=$(( slot + 1 ))
        else
            log "  $w: pair at slot $slot VOIDED — re-running the slot"
        fi
    done
    (( valid >= RUNS )) || log "  $w INCOMPLETE: $valid/$RUNS valid pairs after $attempts attempts"
}

summarize() {
    python3 - "$CSV" "$OUT_DIR/summary.csv" <<'PYEOF'
import csv, sys, statistics as st
runs_p, summary_p = sys.argv[1:3]
rows = [r for r in csv.DictReader(open(runs_p)) if r["valid"] == "yes"]
by = {}
for r in rows:
    by.setdefault((r["fixture"], r["tool"]), []).append(int(r["ms"]))

with open(summary_p, "w") as f:
    f.write("fixture,tool,median_ms,best_ms,spread_pct,n\n")
    for k in sorted(by):
        v = sorted(by[k])
        spread = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
        f.write(f"{k[0]},{k[1]},{int(st.median(v))},{min(v)},{spread},{len(v)}\n")

print(f"\n{'fixture':8} {'blit':>9} {'robocopy':>9} {'ratio':>7}  verdict")
print("-" * 48)
for w in ("large", "small", "mixed"):
    b, r = by.get((w, "blit")), by.get((w, "robocopy"))
    if not b or not r:
        continue
    bm, rm = st.median(b), st.median(r)
    ratio = bm / rm
    verdict = "blit FASTER" if ratio < 1.0 else "blit SLOWER"
    print(f"{w:8} {bm:8.0f}ms {rm:8.0f}ms {ratio:7.3f}  {verdict}")
print("\nratio = blit / robocopy  (<1.00 means blit wins)")
print("Cross-tool wall clock, NOT a controlled protocol comparison.")
print("This says NOTHING about P1 — there is no initiator axis in a local copy.")
PYEOF
}

main() {
    preflight
    [[ "$PREFLIGHT_ONLY" == 1 ]] && { log "PREFLIGHT_ONLY: nothing timed"; exit 0; }
    stage_runner
    local w
    for w in ${FIXTURES//,/ }; do run_cell "$w"; done
    wssh "Remove-Item -Recurse -Force '$DEST_BASE' -ErrorAction SilentlyContinue" || true
    summarize | tee -a "$OUT_DIR/bench.log"
    log "runs: $CSV"
}

main "$@"
