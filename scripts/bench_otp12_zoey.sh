#!/usr/bin/env bash
# otp-12a: interleaved OLD-vs-NEW converge-up matrix on the Mac<->zoey rig
# (ONE_TRANSFER_PATH slice otp-12, sub-slice 12a; design:
# docs/plan/OTP12_ACCEPTANCE_RUN.md D1/D2/D5/D6).
#
# What this measures: the otp-2 verdict matrix ({large,small,mixed} x
# {push,pull} x {tcp,grpc} = 12 comparisons) rerun as matched-pair A/B —
# arm "old" = the pinned pre-cutover pair (default e757dcc: Mac client
# rebuilt at that sha in a detached worktree, zoey daemon already staged
# in blit-temp since 2026-07-10), arm "new" = the run commit's pair.
# This rig anchors PER-DIRECTION converge-up ONLY (hardware-asymmetric
# endpoints, D-2026-07-05-1): a clean PASS needs new <= x1.10 of BOTH
# references — the same-session old arm AND the committed 2026-07-10
# baseline median (docs/bench/otp2-baseline-2026-07-10/summary.csv).
# Cross-direction and invariance claims live on rig W (otp-12b), never
# here.
#
# Methodology inherited verbatim from scripts/bench_otp2_baseline.sh
# (cold caches both ends, drain-then-purge order, durable self-timed
# destination flush, fresh never-seen destinations, wall-clock windows,
# median = floor of the mean of the middle two). New in otp-12a:
#   * ABBA counterbalanced interleave (codex design F5): pair slots run
#     old,new / new,old / old,new / new,old — each arm leads half the
#     pairs, so arm never confounds with within-pair order on the
#     stateful pool.
#   * Valid-run rule (codex design F7): a run with a nonzero blit exit
#     OR an undrained pre-run window voids its whole PAIR; the pair is
#     re-run at the same slot until RUNS valid pairs exist, capped at
#     2*RUNS pair attempts per comparison; at the cap the comparison is
#     recorded INCOMPLETE — never a silent pass, never a short median.
#   * Exit codes checked (the old harness swallowed them inside the
#     timed window); per-run blit output kept under $OUT_DIR/blit-logs/.
#   * verdicts.csv computed at the end against both references
#     (PASS / FAIL-SAME-SESSION / FAIL-REFERENCE-DRIFT / FAIL-BOTH /
#     INCOMPLETE, per design D2).
#   * Escalation (manual, design D2): a comparison that straddles its
#     bar with either arm's spread > 25% is re-run in a fresh session
#     at RUNS=8; both sessions get committed.
#
# Usage (from the client Mac):
#   export ZOEY_SSH=root@zoey
#   export ZOEY_TEMP=/volume/<pool>/.srv/.unifi-drive/michael/.data/blit-temp
#   export ZOEY_HOST=10.1.10.206        # pin the 10GbE path by IP
#   RUNS=4 ./scripts/bench_otp12_zoey.sh
#   PREFLIGHT_ONLY=1 ./scripts/bench_otp12_zoey.sh   # checks only
#
# Prerequisites:
#   * NEW pair: `cargo build --release` at the run commit with a CLEAN
#     tree (a dirty build mints a distinct build id and the
#     D-2026-07-05-2 handshake refuses the pair); zoey daemon zigbuilt
#     (aarch64-musl, static) at the SAME commit and staged at
#     $ZOEY_TEMP/blit-daemon-<sha>.
#   * OLD pair: Mac client rebuilt at $OLD_SHA in a detached worktree
#     and staged at $MAC_WORK/bins/blit-$OLD_SHA; zoey's pinned old
#     daemon at $ZOEY_TEMP/blit-daemon (.agents/machines.md staging,
#     kept for otp-12).
#   * The OLD pair predates the handshake: its provenance is the
#     staging record — this script records sha256 of every binary into
#     staging-manifest.txt. The NEW pair's smoke transfer doubles as
#     its identity check (a mismatched pair refuses with
#     BUILD_MISMATCH at the first frame).
#   * python3 + a NOPASSWD sudoers rule for /usr/sbin/purge on the Mac.
#   * A RIG RUN needs the owner's fresh go for daemon runs on zoey
#     (standing STATE rule). PREFLIGHT_ONLY=1 starts no daemon and
#     times nothing (read-only ssh checks + local purge probe).
#
# Everything on the daemon host stays inside $ZOEY_TEMP (owner rule).

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

ZOEY_SSH=${ZOEY_SSH:-root@zoey}
ZOEY_TEMP=${ZOEY_TEMP:?set ZOEY_TEMP to the blit-temp folder on the daemon host}
ZOEY_HOST=${ZOEY_HOST:-10.1.10.206}
PORT=${PORT:-9031}
RUNS=${RUNS:-4}
PREFLIGHT_ONLY=${PREFLIGHT_ONLY:-0}
# Real-disk client workdir. NOT /tmp: keep the client end on APFS SSD.
MAC_WORK=${MAC_WORK:-$HOME/blit-bench-work}

OLD_SHA=${OLD_SHA_ZOEY:-e757dcc}
NEW_SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD)
NEW_BLIT=${NEW_BLIT:-$REPO_ROOT/target/release/blit}
OLD_BLIT=${OLD_BLIT:-$MAC_WORK/bins/blit-$OLD_SHA}
# The 2026-07-10 staging at $ZOEY_TEMP/blit-daemon FAILED provenance
# (embeds 731023bfc8a1.dirty.…, not e757dcc — correction note in the
# otp-2 README); both arms therefore run sha-named CLEAN rebuilds
# staged beside it. The original is left untouched as the otp-2
# artifact.
OLD_DAEMON=${OLD_DAEMON:-$ZOEY_TEMP/blit-daemon-$OLD_SHA}
NEW_DAEMON=${NEW_DAEMON:-$ZOEY_TEMP/blit-daemon-$NEW_SHA}
# The committed reference is FIXED (pre-registered, design D2) — no env
# override (codex otp-12a F5); its sha256 is recorded in the manifest.
BASELINE_SUMMARY="$REPO_ROOT/docs/bench/otp2-baseline-2026-07-10/summary.csv"

OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp12_zoey_$(date +%Y%m%dT%H%M%S)}
mkdir -p "$OUT_DIR" "$OUT_DIR/blit-logs" "$MAC_WORK"

MODULE_ROOT="$ZOEY_TEMP/bench-module"
REMOTE="$ZOEY_HOST:$PORT:/bench/"

log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log"; }
die() { log "FATAL: $*"; exit 1; }
# ControlMaster multiplexing: an ssh connection to this host costs
# ~1.2s (slow-core key exchange) — reuse one connection.
SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto -o "ControlPath=$HOME/.ssh/cm-%r@%h-%p" -o ControlPersist=300)
zssh() { ssh "${SSH_MUX[@]}" "$ZOEY_SSH" "$@"; }
# Wall-clock ms across two separate python3 processes (deliberate; see
# bench_otp2_baseline.sh for why monotonic is wrong here).
now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }
# Self-timed durability steps (codex otp-2w F3): the timed window is
# transfer + destination flush and NOTHING else; each flush times
# ITSELF on the destination and reports only its own duration.
sync_dest_ms() {   # Linux sync on the daemon host; prints its elapsed ms
    zssh 'a=$(awk "{print int(\$1*1000)}" /proc/uptime); sync; b=$(awk "{print int(\$1*1000)}" /proc/uptime); echo $((b-a))'
}
# Durable pull window: macOS sync(2) only SCHEDULES writes; fsync every
# landed file instead (media-level F_FULLFSYNC deliberately not used —
# the Linux side does not pay media flush either).
fsync_tree_ms() {
    python3 - "$1" <<'PYEOF'
import os, sys, time
t = time.monotonic()
for root, dirs, files in os.walk(sys.argv[1]):
    for name in files:
        fd = os.open(os.path.join(root, name), os.O_RDONLY)
        os.fsync(fd)
        os.close(fd)
print(int((time.monotonic() - t) * 1000))
PYEOF
}

arm_blit()   { case "$1" in old) echo "$OLD_BLIT";;   new) echo "$NEW_BLIT";;   esac; }
arm_daemon() { case "$1" in old) echo "$OLD_DAEMON";; new) echo "$NEW_DAEMON";; esac; }
arm_sha()    { case "$1" in old) echo "$OLD_SHA";;    new) echo "$NEW_SHA";;    esac; }

# --- Preflight ---------------------------------------------------------
preflight() {
    [[ "$RUNS" == 4 || "$RUNS" == 8 ]] \
        || die "RUNS must be 4 (standard) or 8 (the D2 escalation) — got '$RUNS' (codex otp-12a F8: odd values break ABBA balance)"
    [[ -x "$NEW_BLIT" ]] || die "missing $NEW_BLIT (cargo build --release first)"
    [[ -x "$OLD_BLIT" ]] || die "old client not staged at $OLD_BLIT (rebuild at $OLD_SHA in a detached worktree: git worktree add --detach /tmp/blit-old $OLD_SHA && cargo build --release in it, then copy target/release/blit here)"
    command -v python3 >/dev/null || die "python3 required (timing + fsync_tree + verdicts)"
    [[ -f "$BASELINE_SUMMARY" ]] || die "committed baseline not found at $BASELINE_SUMMARY"
    sudo -n /usr/sbin/purge || die "cold-cache purge needs a NOPASSWD sudoers rule for /usr/sbin/purge"
    zssh "test -x '$OLD_DAEMON'" || die "old daemon not staged at $OLD_DAEMON"
    zssh "test -x '$NEW_DAEMON'" || die "new daemon not staged at $NEW_DAEMON (zigbuild aarch64-musl at $NEW_SHA, stage BESIDE the old one)"
    # Provenance enforcement (codex otp-12a F3): a stale-but-matching
    # pair passes the handshake yet is not the labeled build. Every
    # binary must embed its arm's sha (session_build_id/BLIT_GIT_SHA is
    # a compile-time literal in the binary; the old commits embed it
    # too — they postdate otp-3).
    # -a + LC_ALL=C are load-bearing: BSD grep on macOS silently
    # misses matches inside binaries without them (UTF-8 line
    # handling) — observed live against a binary that provably embeds
    # the id (2026-07-12 staging session).
    LC_ALL=C grep -qa "$NEW_SHA" "$NEW_BLIT" \
        || die "$NEW_BLIT does not embed $NEW_SHA — rebuild at the run commit (stale target/release?)"
    LC_ALL=C grep -qa "$OLD_SHA" "$OLD_BLIT" \
        || die "$OLD_BLIT does not embed $OLD_SHA — restage the old client"
    zssh "grep -qa '$NEW_SHA' '$NEW_DAEMON'" \
        || die "$NEW_DAEMON does not embed $NEW_SHA — restage the new daemon"
    zssh "grep -qa '$OLD_SHA' '$OLD_DAEMON'" \
        || die "$OLD_DAEMON does not embed $OLD_SHA — the staged old daemon is not the pinned pair"
    # Stale-daemon refusal (the otp-2w F2 posture, new on this rig): a
    # leftover daemon would mask a bind failure and get benchmarked in
    # place of the arm's build.
    if zssh "pgrep blit-daemon >/dev/null 2>&1"; then
        die "a blit-daemon is already running on zoey — stop it first"
    fi
    # Clean tree is MANDATORY for the new arm (design D1: dirty builds
    # mint <sha>.dirty.* ids; the run must be the recorded commit) —
    # die, don't warn (codex otp-12a F3).
    [[ -z $(git -C "$REPO_ROOT" status --porcelain) ]] \
        || die "working tree DIRTY — the recorded run must be a clean checkout of $NEW_SHA (D-2026-07-05-2)"
    log "preflight OK  old pair: $OLD_SHA  new pair: $NEW_SHA  runs/arm: $RUNS"
}

sha256_local() {   # $1 = path; dies on failure (codex otp-12a F3: no blanks)
    local h
    h=$(shasum -a 256 "$1" | cut -d' ' -f1) || die "sha256 failed for $1"
    [[ ${#h} -eq 64 ]] || die "sha256 produced '$h' for $1"
    echo "$h"
}
sha256_remote() {   # $1 = remote path; dies on failure
    local h
    h=$(zssh "sha256sum '$1'" | cut -d' ' -f1) || die "remote sha256 failed for $1"
    [[ ${#h} -eq 64 ]] || die "remote sha256 produced '$h' for $1"
    echo "$h"
}
write_manifest() {   # binary provenance for the evidence README (design D6)
    local h_oc h_nc h_od h_nd h_ref
    h_oc=$(sha256_local "$OLD_BLIT")
    h_nc=$(sha256_local "$NEW_BLIT")
    h_od=$(sha256_remote "$OLD_DAEMON")
    h_nd=$(sha256_remote "$NEW_DAEMON")
    h_ref=$(sha256_local "$BASELINE_SUMMARY")
    {
        echo "arm,role,sha,sha256,path"
        echo "old,client,$OLD_SHA,$h_oc,$OLD_BLIT"
        echo "new,client,$NEW_SHA,$h_nc,$NEW_BLIT"
        echo "old,daemon,$OLD_SHA,$h_od,$OLD_DAEMON"
        echo "new,daemon,$NEW_SHA,$h_nd,$NEW_DAEMON"
        echo "-,reference,-,$h_ref,$BASELINE_SUMMARY"
    } > "$OUT_DIR/staging-manifest.txt"
    log "staging manifest recorded (5 hashes)"
}

# --- Daemon lifecycle (everything inside ZOEY_TEMP; one arm at a time) --
# The EXIT trap acts only after THIS session started a daemon, and the
# kill is comm-verified — a stale pidfile's recycled PID is never killed
# (codex otp-12a F4).
CURRENT_ARM=""
DAEMON_EVER_STARTED=0
start_daemon() {   # $1 = arm
    local arm="$1" bin
    bin=$(arm_daemon "$arm")
    DAEMON_EVER_STARTED=1
    zssh "mkdir -p '$MODULE_ROOT' && cat > '$ZOEY_TEMP/bench-config.toml' <<EOF
[daemon]
bind = \"0.0.0.0\"
port = $PORT
no_mdns = true

[[module]]
name = \"bench\"
path = \"$MODULE_ROOT\"
EOF
nohup '$bin' --config '$ZOEY_TEMP/bench-config.toml' \
  > '$ZOEY_TEMP/bench-daemon.log' 2>&1 &
echo \$! > '$ZOEY_TEMP/bench-daemon.pid'"
    sleep 1
    zssh "kill -0 \$(cat '$ZOEY_TEMP/bench-daemon.pid')" \
        || { zssh "cat '$ZOEY_TEMP/bench-daemon.log'"; die "$arm daemon failed to start"; }
    CURRENT_ARM="$arm"
    log "daemon up ($arm pair, $(arm_sha "$arm")) on $ZOEY_HOST:$PORT"
}
stop_daemon() {
    zssh "p=\$(cat '$ZOEY_TEMP/bench-daemon.pid' 2>/dev/null); \
          if [ -n \"\$p\" ] && grep -q blit-daemon \"/proc/\$p/comm\" 2>/dev/null; then kill \"\$p\"; fi; \
          rm -f '$ZOEY_TEMP/bench-daemon.pid'" || true
    CURRENT_ARM=""
}
on_exit() {
    if [[ "$DAEMON_EVER_STARTED" == 1 ]]; then
        stop_daemon
        sweep_push_dirs
    fi
    rm -rf "$MAC_WORK/dst_pull_${SESSION_TAG}_"* 2>/dev/null || true
}
ensure_daemon() {   # $1 = arm; swap only when the arm changes (untimed)
    [[ "$CURRENT_ARM" == "$1" ]] && return 0
    [[ -n "$CURRENT_ARM" ]] && stop_daemon
    start_daemon "$1"
}
# Sweep this invocation's push destinations even on an interrupted run —
# never leave content a rerun could no-op onto. Staged pull sources are
# kept for re-runs by design (shared across arms, design D5).
sweep_push_dirs() {
    zssh "cd '$MODULE_ROOT' 2>/dev/null && rm -rf push_${SESSION_TAG}_*" || true
}

# --- Pool drain + cold caches, both ends -------------------------------
# Order matters: FIRST flush dirty pages (sync — Linux sync waits), THEN
# wait for the tier to destage until quiet (three consecutive 2s windows
# under 2 MiB written; timeout 240s), then cold the caches. An undrained
# window VOIDS the pair (design F7) — recorded, never silent.
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

RUN_DRAIN=""
drop_caches() {   # $1 = run label; sets RUN_DRAIN
    local outcome
    outcome=$(drain_pool || true)
    RUN_DRAIN=${outcome:-DRAIN-ERROR}
    RUN_DRAIN=${RUN_DRAIN// /_}
    echo "$1: $RUN_DRAIN" >> "$OUT_DIR/drain.log"
    [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: $1 window UNDRAINED ($RUN_DRAIN) — pair voided, will re-run"
    sync
    sudo -n /usr/sbin/purge
    zssh "echo 3 > /proc/sys/vm/drop_caches"
}

# --- Fixtures (client disk; generated once; shapes = otp-2/sf-1) -------
# Existence alone is NOT trusted (codex otp-12a F2): an interrupted
# generation/staging leaves a partial workload that later runs would
# silently benchmark. Every fixture is verified by file count + byte
# sum; a present-but-wrong dir is a hard stop with an explicit removal
# instruction (never auto-deleted).
FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
FIX_COUNT_small=10000; FIX_BYTES_small=40960000
FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912

fixture_shape() {   # $1 = dir; prints "count,bytes" (macOS stat)
    find "$1" -type f -exec stat -f%z {} + 2>/dev/null \
        | awk '{ s += $1 } END { printf "%d,%d\n", NR, s }'
}
verify_fixture() {   # $1 = workload name; dies on shape mismatch
    local w="$1" want_count want_bytes got
    want_count=$(eval echo "\$FIX_COUNT_$w")
    want_bytes=$(eval echo "\$FIX_BYTES_$w")
    got=$(fixture_shape "$MAC_WORK/src_$w")
    [[ "$got" == "$want_count,$want_bytes" ]] \
        || die "fixture src_$w has shape $got, want $want_count,$want_bytes — partial generation? remove $MAC_WORK/src_$w and re-run"
}
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
    local w
    for w in large small mixed; do verify_fixture "$w"; done
    log "fixtures verified (count + byte sum)"
}

# --- Smoke + staging ----------------------------------------------------
smoke_pair() {   # $1 = arm; 1-file untimed transfer proves the pair works.
    # For the NEW pair this is also the build-identity check: a
    # mismatched pair refuses with BUILD_MISMATCH at the first frame
    # (D-2026-07-05-2). The OLD pair has no handshake — its identity is
    # the staging manifest.
    local arm="$1" blit
    blit=$(arm_blit "$arm")
    ensure_daemon "$arm"
    mkdir -p "$MAC_WORK/smoke_src"
    echo "otp12-smoke" > "$MAC_WORK/smoke_src/probe.txt"
    "$blit" copy "$MAC_WORK/smoke_src" "${REMOTE}push_${SESSION_TAG}_smoke_${arm}/" --yes \
        > "$OUT_DIR/blit-logs/smoke_$arm.log" 2>&1 \
        || die "smoke transfer FAILED for the $arm pair (blit-logs/smoke_$arm.log; on the new pair a BUILD_MISMATCH means the staged daemon is not $NEW_SHA)"
    log "smoke ok: $arm pair"
}

stage_pull_sources() {
    # Untimed; sources are SHARED across arms by design (bytes are
    # bytes — design D5); kept across sessions. A kept dir is verified
    # by remote file count (codex otp-12a F2) — a partial staging is
    # re-staged (blit converges: identical files skip, missing land),
    # then re-verified.
    log "staging pull sources (untimed, new pair)"
    ensure_daemon new
    local w want got
    for w in large small mixed; do
        want=$(eval echo "\$FIX_COUNT_$w")
        got=$(zssh "find '$MODULE_ROOT/pull_src_$w/src_$w' -type f 2>/dev/null | wc -l" | tr -d '[:space:]')
        if [[ "$got" == "$want" ]]; then
            log "  pull_src_$w verified ($got files, kept from a prior session)"
            continue
        fi
        log "  pull_src_$w has $got/$want files — (re)staging"
        "$NEW_BLIT" copy "$MAC_WORK/src_$w" "${REMOTE}pull_src_$w/" --yes \
            > "$OUT_DIR/blit-logs/stage_$w.log" 2>&1 \
            || die "staging pull_src_$w failed (blit-logs/stage_$w.log)"
        got=$(zssh "find '$MODULE_ROOT/pull_src_$w/src_$w' -type f 2>/dev/null | wc -l" | tr -d '[:space:]')
        [[ "$got" == "$want" ]] \
            || die "pull_src_$w still wrong after staging ($got/$want files)"
        log "  staged pull_src_$w ($got files)"
    done
}

# --- Timed runs ---------------------------------------------------------
CSV="$OUT_DIR/runs.csv"
echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid" > "$CSV"
META="$OUT_DIR/meta.csv"
echo "cell,pairs_attempted,complete" > "$META"

RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=0; RUN_VALID=yes

timed_push_run() {   # arm cell rid src [flags...]; fresh dest per run
    local arm="$1" cell="$2" rid="$3" src="$4"; shift 4
    local blit start end rc=0
    blit=$(arm_blit "$arm")
    ensure_daemon "$arm"
    drop_caches "${cell}_${arm}-$rid"
    start=$(now_ms)
    # stdout (arm-dependent progress volume) goes to /dev/null exactly
    # like the frozen harness; only stderr — silent unless failing — is
    # kept for diagnostics (codex otp-12a F6).
    "$blit" copy "$src" "${REMOTE}push_${SESSION_TAG}_${cell}_${arm}_${rid}/" --yes "$@" \
        > /dev/null 2> "$OUT_DIR/blit-logs/${cell}_${arm}_${rid}.err" || rc=$?
    end=$(now_ms)
    RUN_FLUSH=$(sync_dest_ms)   # durable at dest, self-timed
    # Sweep THIS run's destination now that its flush is measured
    # (2026-07-12 session lesson: accumulated destinations drove the
    # daemon host into an I/O-backlog storm — load 444, 10x run times,
    # both arms equally; per-run deletion kept back-to-back probes at
    # baseline. Outside the timed window; the next run's drain loop
    # absorbs the deletion I/O. The EXIT sweep stays as backstop.)
    zssh "rm -rf '$MODULE_ROOT/push_${SESSION_TAG}_${cell}_${arm}_${rid}'"
    RUN_MS=$(( end - start + RUN_FLUSH ))
    RUN_EXIT=$rc
    RUN_VALID=yes
    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
}

timed_pull_run() {   # arm cell rid remote_src [flags...]; fresh dest per run
    local arm="$1" cell="$2" rid="$3" rsrc="$4"; shift 4
    local blit start end rc=0
    blit=$(arm_blit "$arm")
    ensure_daemon "$arm"
    # Never-seen destination path per run (design D5; codex otp-12a
    # F7), removed after its flush is measured so pulls don't
    # accumulate GiBs on the client disk.
    local dst="$MAC_WORK/dst_pull_${SESSION_TAG}_${cell}_${arm}_${rid}"
    mkdir -p "$dst"
    drop_caches "${cell}_${arm}-$rid"
    start=$(now_ms)
    "$blit" copy "$rsrc" "$dst" --yes "$@" \
        > /dev/null 2> "$OUT_DIR/blit-logs/${cell}_${arm}_${rid}.err" || rc=$?
    end=$(now_ms)
    RUN_FLUSH=$(fsync_tree_ms "$dst")   # durable, self-timed
    rm -rf "$dst"
    RUN_MS=$(( end - start + RUN_FLUSH ))
    RUN_EXIT=$rc
    RUN_VALID=yes
    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
}

run_comparison() {   # cell kind src_or_remote [flags...]
    local cell="$1" kind="$2" src="$3"; shift 3
    local slot=1 attempts=0 valid=0 max_attempts=$(( 2 * RUNS ))
    log "=== $cell (interleaved old/new, ABBA, $RUNS pairs) ==="
    while (( valid < RUNS && attempts < max_attempts )); do
        attempts=$(( attempts + 1 ))
        # ABBA: odd slots run old first, even slots run new first.
        local order pair_valid=yes arm rid
        if (( slot % 2 )); then order="old new"; else order="new old"; fi
        local row_old="" row_new=""
        for arm in $order; do
            rid="s${slot}a${attempts}"
            if [[ "$kind" == push ]]; then
                timed_push_run "$arm" "$cell" "$rid" "$src" "$@"
            else
                timed_pull_run "$arm" "$cell" "$rid" "$src" "$@"
            fi
            [[ "$RUN_VALID" == yes ]] || pair_valid=no
            local row="$cell,$arm,$(arm_sha "$arm"),mac,$slot,$RUN_MS,$RUN_FLUSH,$RUN_EXIT,$RUN_DRAIN"
            if [[ "$arm" == old ]]; then row_old="$row"; else row_new="$row"; fi
            log "  $cell/$arm slot $slot (attempt $attempts): ${RUN_MS}ms (flush ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_DRAIN)"
        done
        # The valid column reflects the PAIR's fate (design F7): an
        # individually-clean run whose partner voided does not count.
        echo "$row_old,$pair_valid" >> "$CSV"
        echo "$row_new,$pair_valid" >> "$CSV"
        if [[ "$pair_valid" == yes ]]; then
            valid=$(( valid + 1 )); slot=$(( slot + 1 ))
        else
            log "  $cell: pair at slot $slot VOIDED — re-running the slot"
        fi
    done
    if (( valid < RUNS )); then
        echo "$cell,$attempts,no" >> "$META"
        log "  $cell INCOMPLETE: $valid/$RUNS valid pairs after $attempts attempts"
    else
        echo "$cell,$attempts,yes" >> "$META"
    fi
}

# --- Verdicts (design D2: both references must pass) --------------------
compute_verdicts() {
    python3 - "$CSV" "$META" "$BASELINE_SUMMARY" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" <<'PYEOF'
import csv, sys
runs_p, meta_p, base_p, summary_p, verdicts_p = sys.argv[1:6]
rows = list(csv.DictReader(open(runs_p)))
meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
base = {r["cell"]: int(r["median_ms"]) for r in csv.DictReader(open(base_p))}

by_arm = {}
voided = {}
for r in rows:
    key = (r["cell"], r["arm"])
    if r["valid"] == "yes":
        by_arm.setdefault(key, []).append(int(r["ms"]))
    else:
        voided[key] = voided.get(key, 0) + 1

def median(v):
    v = sorted(v)
    n = len(v)
    return v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) // 2

# A cell is usable only when its comparison completed (RUNS valid
# pairs, codex otp-12a F1): summary medians are written for complete
# cells ONLY — never a median over fewer than RUNS valid runs. The
# verdict loop iterates EVERY attempted comparison (meta), so a
# zero-valid cell still surfaces as INCOMPLETE.
def complete(cell):
    return (meta[cell]["complete"] == "yes"
            and (cell, "new") in by_arm and (cell, "old") in by_arm)

with open(summary_p, "w") as f:
    f.write("cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted\n")
    for (cell, arm) in sorted(by_arm):
        if not complete(cell):
            continue
        v = by_arm[(cell, arm)]
        spread = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
        f.write(f"{cell},{arm},{median(v)},{sum(v)//len(v)},{min(v)},{spread},"
                f"{voided.get((cell, arm), 0)},{meta[cell]['pairs_attempted']}\n")

def bar_pass(new, ref):   # new <= ref * 1.10, integer-exact
    return 10 * new <= 11 * ref

with open(verdicts_p, "w") as f:
    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome\n")
    for cell in sorted(meta):
        if not complete(cell):
            f.write(f"{cell},converge,new,combined,,,,1.10,INCOMPLETE\n")
            continue
        new_m = median(by_arm[(cell, "new")])
        old_m = median(by_arm[(cell, "old")])
        if cell not in base:
            # Fail CLOSED (codex otp-12a F5): every matrix cell has a
            # committed reference row; a miss is a harness/reference
            # bug, not a benchmark outcome.
            sys.exit(f"FATAL: no committed reference row for {cell} in {base_p}")
        ref_m = base[cell]
        p1 = bar_pass(new_m, old_m)
        p2 = bar_pass(new_m, ref_m)
        f.write(f"{cell},converge,new,old_session,{new_m},{old_m},"
                f"{new_m/old_m:.3f},1.10,{'PASS' if p1 else 'FAIL'}\n")
        f.write(f"{cell},converge,new,old_committed,{new_m},{ref_m},"
                f"{new_m/ref_m:.3f},1.10,{'PASS' if p2 else 'FAIL'}\n")
        combined = ("PASS" if p1 and p2
                    else "FAIL-REFERENCE-DRIFT" if p1
                    else "FAIL-SAME-SESSION" if p2
                    else "FAIL-BOTH")
        f.write(f"{cell},converge,new,combined,{new_m},,,1.10,{combined}\n")
PYEOF
}

# --- Matrix -------------------------------------------------------------
main() {
    preflight
    write_manifest
    if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
        log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
        exit 0
    fi
    BUILD_LINE="old=$OLD_SHA new=$NEW_SHA"
    log "session $SESSION_TAG  $BUILD_LINE  client: $(uname -m) macOS  daemon host: $ZOEY_HOST"

    gen_fixtures
    smoke_pair old
    smoke_pair new
    stage_pull_sources

    local w
    for w in large small mixed; do
        run_comparison "push_tcp_${w}"  push "$MAC_WORK/src_$w"
        run_comparison "push_grpc_${w}" push "$MAC_WORK/src_$w" --force-grpc
        run_comparison "pull_tcp_${w}"  pull "${REMOTE}pull_src_$w/src_$w/"
        run_comparison "pull_grpc_${w}" pull "${REMOTE}pull_src_$w/src_$w/" --force-grpc
    done

    stop_daemon
    compute_verdicts

    log ""
    log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/comparison; ABBA) ==="
    column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
    log ""
    log "=== VERDICTS (design D2: PASS needs BOTH references) ==="
    column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"
    log "runs: $CSV"
}

SESSION_TAG=$(date +%H%M%S).$$
trap on_exit EXIT
main "$@"
