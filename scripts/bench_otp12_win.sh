#!/usr/bin/env bash
# otp-12b: the Mac<->Windows acceptance session (ONE_TRANSFER_PATH slice
# otp-12, sub-slice 12b; design: docs/plan/OTP12_ACCEPTANCE_RUN.md
# D1-D3/D5/D6). Two blocks on the owner-designated closest-spec pair:
#
#   BLOCK 1 — converge-up (Mac-initiated, matches the otp-2w recorded
#   conditions): {large,small,mixed} x {push,pull} x {tcp,grpc} = 12
#   comparisons, matched-pair interleaved A/B — arm "old" = the pinned
#   pre-cutover pair (default 0f922de: Mac client rebuilt in a detached
#   worktree; Windows daemon built natively at that commit), arm "new"
#   = the run commit's pair. Verdicts against BOTH references (the
#   same-session old arm AND docs/bench/otp2w-baseline-2026-07-10/
#   summary.csv), per design D2 as amended.
#
#   BLOCK 2 — initiator/verb invariance (NEW pair only; the owner's
#   sentence, measured): per data direction x fixture x carrier, arm
#   "mac_init" vs arm "win_init" interleaved ABBA. Data Mac->Win (mw_*):
#   Mac client pushes vs Windows client pulls the SAME physical source
#   (the Mac module root IS $MAC_WORK — design F6). Data Win->Mac
#   (wm_*): Mac client pulls vs Windows client pushes the same staged
#   tree on D:. Cell grammar: <mw|wm>_<carrier>_<fixture>. Every arm
#   also gets converge rows against its data direction's old references
#   (design F3: no tolerance compounding), plus the F4 cross-direction
#   rows and the D-2026-07-12-1 discriminator gap rows (recorded, never
#   self-adjudicated).
#
# Methodology inherited verbatim from scripts/bench_otp2w_baseline.sh
# (self-timed durability: Write-VolumeCache on Windows / per-file fsync
# walk on macOS, keyed by DESTINATION OS never verb; Get-Counter drain;
# standby-list purge + macOS purge; WMI daemon launch — Windows OpenSSH
# kills session children; TOML literal-string module paths; stale-daemon
# refusal + PID-scoped teardown) and from bench_otp12_zoey.sh (ABBA
# counterbalance, pair-void valid-run rule with 2xRUNS cap + INCOMPLETE,
# exit codes checked, +sha provenance, sha256 staging manifest,
# PREFLIGHT_ONLY, CELLS allowlist for D2 escalations, per-run
# destination sweep after the measured flush — the zoey I/O-storm
# lesson, kept uniform here).
#
# Windows-side timed windows (win_init arms) are measured ON Windows —
# a Stopwatch brackets the blit.exe invocation inside one ssh call and
# prints "<ms>,<exit>"; the ssh round trip stays outside the window by
# construction (the otp-2w F3 rule applied to a whole client run).
#
# Usage (from the client Mac):
#   export WIN_SSH=michael@10.1.10.173
#   export WIN_HOST=10.1.10.173
#   export WIN_TEST='D:\blit-test'
#   export MAC_HOST=<the Mac's 10GbE IP>      # required, no default
#   RUNS=4 ./scripts/bench_otp12_win.sh
#   PREFLIGHT_ONLY=1 ./scripts/bench_otp12_win.sh
#   CELLS=<comma-list> RUNS=8 ./scripts/bench_otp12_win.sh   # escalation
#
# Staging prerequisites (the rig session does these before preflight):
#   * Mac: clean tree at the run commit; `cargo build --release` (client
#     AND daemon — the Mac daemon serves block 2); old client rebuilt at
#     $OLD_SHA in a detached worktree -> $MAC_WORK/bins/blit-$OLD_SHA.
#   * Windows: BEFORE moving the checkout, copy the detached-build exes
#     aside to $WIN_TEST\bins\$OLD_SHA\; then fresh git bundle ->
#     checkout the run commit -> native `cargo build --release` ->
#     copy blit-daemon.exe AND blit.exe to $WIN_TEST\bins\<run sha>\.
#     Daemons always LAUNCH from the fixed path
#     $WIN_TEST\bins\active\blit-daemon.exe (arm swap = Copy-Item over
#     it) so ONE program-scoped firewall rule covers both arms
#     ("blit-otp12-daemon"; the otp-2w rule points at the repo path and
#     is left alone).
#   * Pre-cutover CLIENT binaries embed no build id (otp-12a-run F1):
#     old-client provenance = the clean-worktree rebuild + the manifest,
#     acknowledged via OLD_CLIENT_PROVENANCE_BY_BUILD=1.

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

WIN_SSH=${WIN_SSH:-michael@10.1.10.173}
WIN_HOST=${WIN_HOST:-10.1.10.173}
WIN_TEST=${WIN_TEST:-'D:\blit-test'}
WIN_DRIVE=${WIN_DRIVE:-D}
MAC_HOST=${MAC_HOST:?set MAC_HOST to the Mac's 10GbE IP (the Windows-initiated arms dial it)}
PORT=${PORT:-9031}
RUNS=${RUNS:-4}
PREFLIGHT_ONLY=${PREFLIGHT_ONLY:-0}
CELLS=${CELLS:-}
MAC_WORK=${MAC_WORK:-$HOME/blit-bench-work}
# The Mac module root IS the fixture workdir (design F6): both
# initiators of a Mac->Win cell read the same physical inodes.
MAC_MODULE_ROOT=${MAC_MODULE_ROOT:-$MAC_WORK}

OLD_SHA=${OLD_SHA_WIN:-0f922de}
NEW_SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD)
NEW_BLIT=${NEW_BLIT:-$REPO_ROOT/target/release/blit}
MAC_DAEMON=${MAC_DAEMON:-$REPO_ROOT/target/release/blit-daemon}
OLD_BLIT=${OLD_BLIT:-$MAC_WORK/bins/blit-$OLD_SHA}
WIN_BINS="$WIN_TEST\\bins"
OLD_WIN_DAEMON="$WIN_BINS\\$OLD_SHA\\blit-daemon.exe"
NEW_WIN_DAEMON="$WIN_BINS\\$NEW_SHA\\blit-daemon.exe"
ACTIVE_WIN_DAEMON="$WIN_BINS\\active\\blit-daemon.exe"
WIN_BLIT="$WIN_BINS\\$NEW_SHA\\blit.exe"
# Fixed committed reference (pre-registered, D2) — no override.
BASELINE_SUMMARY="$REPO_ROOT/docs/bench/otp2w-baseline-2026-07-10/summary.csv"

OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp12_win_$(date +%Y%m%dT%H%M%S)}
mkdir -p "$OUT_DIR" "$OUT_DIR/blit-logs" "$MAC_WORK"

WIN_MODULE="$WIN_TEST\\bench-module"
WIN_REMOTE="$WIN_HOST:$PORT:/bench/"
MAC_REMOTE="$MAC_HOST:$PORT:/bench/"

log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log"; }
die() { log "FATAL: $*"; exit 1; }
SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto -o "ControlPath=$HOME/.ssh/cm-%r@%h-%p" -o ControlPersist=300)
wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }
now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }

# --- Self-timed durability (destination-OS-keyed, never verb-keyed) ----
flush_win_ms() {   # Windows volume flush; prints its own elapsed ms
    local v
    v=$(wssh "\$sw = [Diagnostics.Stopwatch]::StartNew(); Write-VolumeCache $WIN_DRIVE; \$sw.Stop(); [int]\$sw.Elapsed.TotalMilliseconds" | tr -cd '0-9')
    echo "${v:-0}"
}
fsync_tree_ms() {   # macOS per-file fsync walk; prints its own elapsed ms
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

want_cell() { [[ -z "$CELLS" ]] || [[ ",$CELLS," == *",$1,"* ]]; }

# --- Provenance + manifest (otp-12a lessons: +sha form, fail closed) ---
sha256_local() {
    local h
    h=$(shasum -a 256 "$1" | cut -d' ' -f1) || die "sha256 failed for $1"
    [[ ${#h} -eq 64 ]] || die "sha256 produced '$h' for $1"
    echo "$h"
}
sha256_win() {
    local h
    h=$(wssh "(Get-FileHash -Algorithm SHA256 '$1').Hash" | tr -cd '0-9A-Fa-f' | tr 'A-F' 'a-f') \
        || die "remote sha256 failed for $1"
    [[ ${#h} -eq 64 ]] || die "remote sha256 produced '$h' for $1"
    echo "$h"
}
win_embeds() {   # $1 = exe path, $2 = sha; exit 0 iff '+sha' present
    wssh "if (Select-String -Path '$1' -SimpleMatch -Quiet -Pattern '+$2') { 'yes' } else { exit 1 }" >/dev/null
}

preflight() {
    [[ "$RUNS" == 4 || "$RUNS" == 8 ]] \
        || die "RUNS must be 4 (standard) or 8 (the D2 escalation) — got '$RUNS'"
    [[ -x "$NEW_BLIT" ]] || die "missing $NEW_BLIT (cargo build --release first)"
    [[ -x "$MAC_DAEMON" ]] || die "missing $MAC_DAEMON (the Mac daemon serves block 2)"
    [[ -x "$OLD_BLIT" ]] || die "old client not staged at $OLD_BLIT (detached worktree rebuild at $OLD_SHA)"
    command -v python3 >/dev/null || die "python3 required"
    [[ -f "$BASELINE_SUMMARY" ]] || die "committed baseline not found at $BASELINE_SUMMARY"
    sudo -n /usr/sbin/purge || die "need the NOPASSWD purge sudoers rule"
    wssh "if (-not (Test-Path '$OLD_WIN_DAEMON')) { exit 1 }" \
        || die "old daemon not staged at $OLD_WIN_DAEMON (copy the detached-build exe aside BEFORE moving the checkout)"
    wssh "if (-not (Test-Path '$NEW_WIN_DAEMON')) { exit 1 }" \
        || die "new daemon not staged at $NEW_WIN_DAEMON (native build at $NEW_SHA)"
    wssh "if (-not (Test-Path '$WIN_BLIT')) { exit 1 }" \
        || die "new Windows client not staged at $WIN_BLIT"
    # Provenance: +sha form (bare shas match cargo build-dir paths).
    LC_ALL=C grep -qa "+$NEW_SHA" "$NEW_BLIT" \
        || die "$NEW_BLIT does not embed +$NEW_SHA — rebuild at the run commit"
    LC_ALL=C grep -qa "+$NEW_SHA" "$MAC_DAEMON" \
        || die "$MAC_DAEMON does not embed +$NEW_SHA — rebuild at the run commit"
    win_embeds "$NEW_WIN_DAEMON" "$NEW_SHA" \
        || die "$NEW_WIN_DAEMON does not embed +$NEW_SHA — restage the native build"
    win_embeds "$WIN_BLIT" "$NEW_SHA" \
        || die "$WIN_BLIT does not embed +$NEW_SHA — restage the native build"
    win_embeds "$OLD_WIN_DAEMON" "$OLD_SHA" \
        || die "$OLD_WIN_DAEMON does not embed +$OLD_SHA — the staged old daemon is not the pinned pair"
    if LC_ALL=C grep -qa "+$OLD_SHA" "$OLD_BLIT"; then
        :
    elif [[ "${OLD_CLIENT_PROVENANCE_BY_BUILD:-0}" == 1 ]]; then
        log "old client: no embedded +$OLD_SHA id (pre-cutover binary); provenance = clean-worktree build + manifest (acknowledged)"
    else
        die "$OLD_BLIT does not embed +$OLD_SHA; if it is the pre-cutover client rebuilt clean per D6, re-run with OLD_CLIENT_PROVENANCE_BY_BUILD=1"
    fi
    # Stale refusal, both hosts.
    if wssh "if (Get-Process blit-daemon -ErrorAction SilentlyContinue) { exit 0 } else { exit 1 }" 2>/dev/null; then
        die "a blit-daemon is already running on the Windows host — stop it first"
    fi
    if pgrep -x blit-daemon >/dev/null 2>&1; then
        die "a blit-daemon is already running on the Mac — stop it first"
    fi
    [[ -z $(git -C "$REPO_ROOT" status --porcelain) ]] \
        || die "working tree DIRTY — the recorded run must be a clean checkout of $NEW_SHA"
    log "preflight OK  old pair: $OLD_SHA  new pair: $NEW_SHA  runs/arm: $RUNS  mac endpoint: $MAC_HOST:$PORT"
}

write_manifest() {
    local f="$OUT_DIR/staging-manifest.txt"
    {
        echo "arm,role,sha,sha256,path"
        echo "old,client,$OLD_SHA,$(sha256_local "$OLD_BLIT"),$OLD_BLIT"
        echo "new,client,$NEW_SHA,$(sha256_local "$NEW_BLIT"),$NEW_BLIT"
        echo "new,mac-daemon,$NEW_SHA,$(sha256_local "$MAC_DAEMON"),$MAC_DAEMON"
        echo "old,win-daemon,$OLD_SHA,$(sha256_win "$OLD_WIN_DAEMON"),$OLD_WIN_DAEMON"
        echo "new,win-daemon,$NEW_SHA,$(sha256_win "$NEW_WIN_DAEMON"),$NEW_WIN_DAEMON"
        echo "new,win-client,$NEW_SHA,$(sha256_win "$WIN_BLIT"),$WIN_BLIT"
        echo "-,reference,-,$(sha256_local "$BASELINE_SUMMARY"),$BASELINE_SUMMARY"
    } > "$f"
    log "staging manifest recorded (7 hashes)"
}

# --- One-time host setup (idempotent) ----------------------------------
setup_host() {
    scp -q -o BatchMode=yes "$SCRIPT_DIR/windows/purge-standby.ps1" \
        "$WIN_SSH:$WIN_TEST/purge-standby.ps1" 2>/dev/null || {
        wssh "New-Item -ItemType Directory -Force -Path '$WIN_TEST' | Out-Null"
        scp -q -o BatchMode=yes "$SCRIPT_DIR/windows/purge-standby.ps1" \
            "$WIN_SSH:$WIN_TEST/purge-standby.ps1"
    }
    wssh "New-Item -ItemType Directory -Force -Path '$WIN_MODULE','$WIN_BINS\\active' | Out-Null
if (-not (Get-NetFirewallRule -DisplayName blit-otp12-daemon -ErrorAction SilentlyContinue)) {
  New-NetFirewallRule -DisplayName blit-otp12-daemon -Direction Inbound -Program '$ACTIVE_WIN_DAEMON' -Action Allow | Out-Null
  'firewall rule added (blit-otp12-daemon -> active path)'
} else { 'firewall rule present' }"
}

# --- Windows daemon lifecycle (arm-swapped via the fixed active path) ---
WIN_ARM=""
WIN_DAEMON_STARTED=0
win_daemon_start() {   # $1 = old|new
    local arm="$1" src
    case "$arm" in old) src="$OLD_WIN_DAEMON";; new) src="$NEW_WIN_DAEMON";; esac
    wssh "if (Get-Process blit-daemon -ErrorAction SilentlyContinue) { 'STALE blit-daemon running'; exit 1 }" \
        || die "refusing to start over a stale Windows daemon"
    WIN_DAEMON_STARTED=1
    wssh "Copy-Item '$src' '$ACTIVE_WIN_DAEMON' -Force
Set-Content -Path '$WIN_TEST\\bench-config.toml' -Value @'
[daemon]
bind = \"0.0.0.0\"
port = $PORT
no_mdns = true

[[module]]
name = \"bench\"
path = '$WIN_MODULE'
'@
\$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = 'cmd /c \"\"$ACTIVE_WIN_DAEMON\" --config \"$WIN_TEST\\bench-config.toml\" > \"$WIN_TEST\\daemon-out.log\" 2> \"$WIN_TEST\\daemon-err.log\"\"' }
if (\$r.ReturnValue -ne 0) { \"wmi create failed: \$(\$r.ReturnValue)\"; exit 1 }"
    sleep 2
    wssh "\$d = Get-Process blit-daemon -ErrorAction SilentlyContinue
if (-not \$d) { Get-Content '$WIN_TEST\\daemon-err.log' -ErrorAction SilentlyContinue | Select-Object -First 10; exit 1 }
Set-Content -Path '$WIN_TEST\\daemon.pid' -Value \$d.Id" \
        || die "$arm Windows daemon failed to start"
    WIN_ARM="$arm"
    log "windows daemon up ($arm pair) on $WIN_HOST:$PORT"
}
win_daemon_stop() {
    wssh "\$p = Get-Content '$WIN_TEST\\daemon.pid' -ErrorAction SilentlyContinue
if (\$p) {
  \$proc = Get-Process -Id \$p -ErrorAction SilentlyContinue
  if (\$proc -and \$proc.ProcessName -eq 'blit-daemon') { Stop-Process -Id \$p -Force }
  Remove-Item '$WIN_TEST\\daemon.pid' -ErrorAction SilentlyContinue
}" || true
    WIN_ARM=""
}
win_ensure() {   # $1 = arm; swap only on change (untimed)
    [[ "$WIN_ARM" == "$1" ]] && return 0
    [[ -n "$WIN_ARM" ]] && win_daemon_stop
    win_daemon_start "$1"
}

# --- Mac daemon lifecycle (new build only; serves block 2) --------------
MAC_DAEMON_STARTED=0
mac_daemon_start() {
    pgrep -x blit-daemon >/dev/null 2>&1 && die "refusing to start over a stale Mac daemon"
    cat > "$MAC_WORK/bench-daemon-config.toml" <<EOF
[daemon]
bind = "0.0.0.0"
port = $PORT
no_mdns = true

[[module]]
name = "bench"
path = "$MAC_MODULE_ROOT"
EOF
    MAC_DAEMON_STARTED=1
    nohup "$MAC_DAEMON" --config "$MAC_WORK/bench-daemon-config.toml" \
        > "$MAC_WORK/bench-daemon.log" 2>&1 &
    echo $! > "$MAC_WORK/bench-daemon.pid"
    sleep 1
    kill -0 "$(cat "$MAC_WORK/bench-daemon.pid")" 2>/dev/null \
        || { tail -5 "$MAC_WORK/bench-daemon.log"; die "Mac daemon failed to start"; }
    log "mac daemon up on $MAC_HOST:$PORT (module bench -> $MAC_MODULE_ROOT)"
}
mac_daemon_stop() {
    local p
    p=$(cat "$MAC_WORK/bench-daemon.pid" 2>/dev/null) || true
    if [[ -n "${p:-}" ]] && ps -p "$p" -o comm= 2>/dev/null | grep -q blit-daemon; then
        kill "$p"
    fi
    rm -f "$MAC_WORK/bench-daemon.pid"
}

sweep_win_push_dirs() {
    wssh "Remove-Item -Recurse -Force '$WIN_MODULE\\push_${SESSION_TAG}_*' -ErrorAction SilentlyContinue" || true
}
on_exit() {
    if [[ "$WIN_DAEMON_STARTED" == 1 ]]; then win_daemon_stop; sweep_win_push_dirs; fi
    [[ "$MAC_DAEMON_STARTED" == 1 ]] && mac_daemon_stop
    rm -rf "$MAC_WORK/dst_pull_${SESSION_TAG}_"* "$MAC_MODULE_ROOT/push_${SESSION_TAG}_"* 2>/dev/null || true
}

# --- Drain + cold caches -------------------------------------------------
drain_host() {
    wssh '$ErrorActionPreference = "Stop"
Write-VolumeCache '"$WIN_DRIVE"'
$quiet = 0
for ($i = 0; $i -lt 60; $i++) {
  $w = (Get-Counter "\PhysicalDisk(_Total)\Disk Write Bytes/sec" -SampleInterval 2 -MaxSamples 1).CounterSamples[0].CookedValue
  if ($null -ne $w -and [double]$w -lt 1048576) { $quiet++ } else { $quiet = 0 }
  if ($quiet -ge 3) { "drained $(($i+1)*2)s"; exit 0 }
}
"DRAIN-TIMEOUT"'
}
RUN_DRAIN=""
drop_caches() {   # $1 = run label; sets RUN_DRAIN (pair-voiding, D2)
    local outcome
    outcome=$(drain_host || true)
    RUN_DRAIN=${outcome:-DRAIN-ERROR}
    RUN_DRAIN=${RUN_DRAIN// /_}
    echo "$1: $RUN_DRAIN" >> "$OUT_DIR/drain.log"
    [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: $1 window UNDRAINED ($RUN_DRAIN) — pair voided, will re-run"
    sync
    sudo -n /usr/sbin/purge
    wssh "pwsh -NoProfile -File '$WIN_TEST\\purge-standby.ps1'" >/dev/null
}

# --- Fixtures (shape-verified; the otp-12a F2 rule) ----------------------
FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
FIX_COUNT_small=10000; FIX_BYTES_small=40960000
FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
fixture_shape() {
    find "$1" -type f -exec stat -f%z {} + 2>/dev/null \
        | awk '{ s += $1 } END { printf "%d,%d\n", NR, s }'
}
verify_fixture() {
    local w="$1" want_count want_bytes got
    want_count=$(eval echo "\$FIX_COUNT_$w")
    want_bytes=$(eval echo "\$FIX_BYTES_$w")
    got=$(fixture_shape "$MAC_WORK/src_$w")
    [[ "$got" == "$want_count,$want_bytes" ]] \
        || die "fixture src_$w has shape $got, want $want_count,$want_bytes — remove $MAC_WORK/src_$w and re-run"
}
gen_fixtures() {
    if [[ ! -d "$MAC_WORK/src_large" ]]; then
        mkdir -p "$MAC_WORK/src_large"
        dd if=/dev/urandom of="$MAC_WORK/src_large/large_1024M.bin" bs=1m count=1024 2>/dev/null
    fi
    if [[ ! -d "$MAC_WORK/src_small" ]]; then
        mkdir -p "$MAC_WORK/src_small"
        for i in $(seq 1 10000); do
            d="$MAC_WORK/src_small/d$(( i / 1000 ))"; mkdir -p "$d"
            dd if=/dev/urandom of="$d/f${i}.dat" bs=4096 count=1 2>/dev/null
        done
    fi
    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
        mkdir -p "$MAC_WORK/src_mixed"
        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
        for i in $(seq 1 5000); do
            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
            dd if=/dev/urandom of="$d/f${i}.dat" bs=2048 count=1 2>/dev/null
        done
    fi
    local w
    for w in large small mixed; do verify_fixture "$w"; done
    log "fixtures verified (count + byte sum)"
}

win_module_count() {   # $1 = subpath under the module; prints file count
    wssh "(Get-ChildItem -Path '$WIN_MODULE\\$1' -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count" | tr -cd '0-9'
}
stage_pull_sources() {
    # Shared across arms by design (D5); verified by remote file count;
    # staged with the NEW pair; the same trees serve block 1 pulls and
    # block 2 win_init pushes (one physical source per direction, F6).
    log "staging pull sources on the Windows module (untimed, new pair)"
    win_ensure new
    local w want got
    for w in large small mixed; do
        want=$(eval echo "\$FIX_COUNT_$w")
        got=$(win_module_count "pull_src_$w\\src_$w"); got=${got:-0}
        if [[ "$got" == "$want" ]]; then
            log "  pull_src_$w verified ($got files, kept)"
            continue
        fi
        log "  pull_src_$w has $got/$want files — (re)staging"
        "$NEW_BLIT" copy "$MAC_WORK/src_$w" "${WIN_REMOTE}pull_src_$w/" --yes \
            > /dev/null 2> "$OUT_DIR/blit-logs/stage_$w.err" \
            || die "staging pull_src_$w failed"
        got=$(win_module_count "pull_src_$w\\src_$w"); got=${got:-0}
        [[ "$got" == "$want" ]] || die "pull_src_$w still wrong after staging ($got/$want)"
        log "  staged pull_src_$w ($got files)"
    done
}

# --- Timed runs -----------------------------------------------------------
CSV="$OUT_DIR/runs.csv"
echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid" > "$CSV"
META="$OUT_DIR/meta.csv"
echo "cell,pairs_attempted,complete" > "$META"

RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=0; RUN_VALID=yes

# Mac-initiated runs (block 1 both arms; block 2 mac_init arms).
mac_push_run() {   # blit_bin cell rid dest_remote src [flags...]
    local blit="$1" cell="$2" rid="$3" dest="$4" src="$5"; shift 5
    local start end rc=0
    drop_caches "${cell}-$rid"
    start=$(now_ms)
    "$blit" copy "$src" "${dest}push_${SESSION_TAG}_${cell}_${rid}/" --yes "$@" \
        > /dev/null 2> "$OUT_DIR/blit-logs/${cell}_${rid}.err" || rc=$?
    end=$(now_ms)
    if [[ "$dest" == "$WIN_REMOTE" ]]; then
        RUN_FLUSH=$(flush_win_ms)
        wssh "Remove-Item -Recurse -Force '$WIN_MODULE\\push_${SESSION_TAG}_${cell}_${rid}' -ErrorAction SilentlyContinue" || true
    else
        RUN_FLUSH=$(fsync_tree_ms "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}")
        rm -rf "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}"
    fi
    RUN_MS=$(( end - start + RUN_FLUSH )); RUN_EXIT=$rc; RUN_VALID=yes
    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
}
mac_pull_run() {   # blit_bin cell rid remote_src [flags...]
    local blit="$1" cell="$2" rid="$3" rsrc="$4"; shift 4
    local start end rc=0
    local dst="$MAC_WORK/dst_pull_${SESSION_TAG}_${cell}_${rid}"
    mkdir -p "$dst"
    drop_caches "${cell}-$rid"
    start=$(now_ms)
    "$blit" copy "$rsrc" "$dst" --yes "$@" \
        > /dev/null 2> "$OUT_DIR/blit-logs/${cell}_${rid}.err" || rc=$?
    end=$(now_ms)
    RUN_FLUSH=$(fsync_tree_ms "$dst")
    rm -rf "$dst"
    RUN_MS=$(( end - start + RUN_FLUSH )); RUN_EXIT=$rc; RUN_VALID=yes
    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
}
# Windows-initiated runs (block 2 win_init arms): the transfer window is
# a Stopwatch ON Windows printing "<ms>,<exit>"; CRLF-stripped.
win_client_run() {   # cell rid src dst flags_string; sets T_MS/T_RC
    local cell="$1" rid="$2" src="$3" dst="$4" flags="${5:-}"
    local out
    out=$(wssh "\$sw = [Diagnostics.Stopwatch]::StartNew(); & '$WIN_BLIT' copy '$src' '$dst' --yes $flags > \$null 2> '$WIN_TEST\\client-err.log'; \$rc = \$LASTEXITCODE; \$sw.Stop(); \"\$([int]\$sw.Elapsed.TotalMilliseconds),\$rc\"" | tr -cd '0-9,')
    T_MS=${out%%,*}; T_RC=${out##*,}
    [[ -n "$T_MS" && -n "$T_RC" && "$out" == *,* ]] || { T_MS=0; T_RC=99; }
    if [[ "$T_RC" != 0 ]]; then
        wssh "Get-Content '$WIN_TEST\\client-err.log' -ErrorAction SilentlyContinue | Select-Object -First 20" \
            > "$OUT_DIR/blit-logs/${cell}_${rid}.err" 2>&1 || true
    fi
}
win_pull_run() {   # cell rid remote_src(from mac) [flag]; dest = win module
    local cell="$1" rid="$2" rsrc="$3" flag="${4:-}"
    drop_caches "${cell}-$rid"
    win_client_run "$cell" "$rid" "$rsrc" "$WIN_MODULE\\pull_${SESSION_TAG}_${cell}_${rid}" "$flag"
    RUN_FLUSH=$(flush_win_ms)
    wssh "Remove-Item -Recurse -Force '$WIN_MODULE\\pull_${SESSION_TAG}_${cell}_${rid}' -ErrorAction SilentlyContinue" || true
    RUN_MS=$(( T_MS + RUN_FLUSH )); RUN_EXIT=$T_RC; RUN_VALID=yes
    [[ "$T_RC" == 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
}
win_push_run() {   # cell rid src(win local path) [flag]; dest = mac module
    local cell="$1" rid="$2" src="$3" flag="${4:-}"
    drop_caches "${cell}-$rid"
    win_client_run "$cell" "$rid" "$src" "${MAC_REMOTE}push_${SESSION_TAG}_${cell}_${rid}/" "$flag"
    RUN_FLUSH=$(fsync_tree_ms "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}")
    rm -rf "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}"
    RUN_MS=$(( T_MS + RUN_FLUSH )); RUN_EXIT=$T_RC; RUN_VALID=yes
    [[ "$T_RC" == 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
}

# One interleaved comparison; ABBA; pair-void; INCOMPLETE at the cap.
# run_one <cell> <armA> <armB> <fnA...>|<fnB...> dispatch happens via
# small wrappers below to keep bash 3.2-simple.
run_pair_loop() {   # cell armA armB runA_fn runB_fn (fns take: cell rid)
    local cell="$1" armA="$2" armB="$3" fnA="$4" fnB="$5"
    local slot=1 attempts=0 valid=0 max_attempts=$(( 2 * RUNS ))
    log "=== $cell ($armA vs $armB, ABBA, $RUNS pairs) ==="
    while (( valid < RUNS && attempts < max_attempts )); do
        attempts=$(( attempts + 1 ))
        local order pair_valid=yes arm fn rid rowA="" rowB=""
        if (( slot % 2 )); then order="A B"; else order="B A"; fi
        for arm in $order; do
            rid="s${slot}a${attempts}"
            if [[ "$arm" == A ]]; then fn="$fnA"; else fn="$fnB"; fi
            "$fn" "$cell" "$rid"
            [[ "$RUN_VALID" == yes ]] || pair_valid=no
            local aname bld init
            if [[ "$arm" == A ]]; then aname="$armA"; else aname="$armB"; fi
            case "$aname" in
                old) bld="$OLD_SHA"; init=mac;;
                new|mac_init) bld="$NEW_SHA"; init=mac;;
                win_init) bld="$NEW_SHA"; init=win;;
            esac
            local row="$cell,$aname,$bld,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_EXIT,$RUN_DRAIN"
            if [[ "$arm" == A ]]; then rowA="$row"; else rowB="$row"; fi
            log "  $cell/$aname slot $slot (attempt $attempts): ${RUN_MS}ms (flush ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_DRAIN)"
        done
        echo "$rowA,$pair_valid" >> "$CSV"
        echo "$rowB,$pair_valid" >> "$CSV"
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

# Block-1 arm wrappers (Mac-initiated; daemon arm follows the run arm).
CUR_W=""; CUR_FLAG=""
b1_push_old() { win_ensure old; mac_push_run "$OLD_BLIT" "$1" "$2" "$WIN_REMOTE" "$MAC_WORK/src_$CUR_W" $CUR_FLAG; }
b1_push_new() { win_ensure new; mac_push_run "$NEW_BLIT" "$1" "$2" "$WIN_REMOTE" "$MAC_WORK/src_$CUR_W" $CUR_FLAG; }
b1_pull_old() { win_ensure old; mac_pull_run "$OLD_BLIT" "$1" "$2" "${WIN_REMOTE}pull_src_$CUR_W/src_$CUR_W/" $CUR_FLAG; }
b1_pull_new() { win_ensure new; mac_pull_run "$NEW_BLIT" "$1" "$2" "${WIN_REMOTE}pull_src_$CUR_W/src_$CUR_W/" $CUR_FLAG; }
# Block-2 arm wrappers (new pair; both daemons stay up). Both arms of a
# pair do IDENTICAL work: no-trailing-slash sources everywhere, so both
# initiators land the same one-level-nested tree at the destination
# (avoids betting on Windows trailing-separator semantics; block 1
# keeps the otp-2w shapes verbatim for baseline comparability).
b2_mw_mac() { mac_push_run "$NEW_BLIT" "$1" "$2" "$WIN_REMOTE" "$MAC_WORK/src_$CUR_W" $CUR_FLAG; }
b2_mw_win() { win_pull_run "$1" "$2" "${MAC_REMOTE}src_$CUR_W" "$CUR_FLAG"; }
b2_wm_mac() { mac_pull_run "$NEW_BLIT" "$1" "$2" "${WIN_REMOTE}pull_src_$CUR_W/src_$CUR_W" $CUR_FLAG; }
b2_wm_win() { win_push_run "$1" "$2" "$WIN_MODULE\\pull_src_$CUR_W\\src_$CUR_W" "$CUR_FLAG"; }

smoke() {   # arm smoke transfers (untimed): old pair, new pair, win->mac
    mkdir -p "$MAC_WORK/smoke_src"
    echo "otp12b-smoke" > "$MAC_WORK/smoke_src/probe.txt"
    win_ensure old
    "$OLD_BLIT" copy "$MAC_WORK/smoke_src" "${WIN_REMOTE}push_${SESSION_TAG}_smoke_old/" --yes \
        > /dev/null 2> "$OUT_DIR/blit-logs/smoke_old.err" || die "old-pair smoke FAILED"
    log "smoke ok: old pair"
    win_ensure new
    "$NEW_BLIT" copy "$MAC_WORK/smoke_src" "${WIN_REMOTE}push_${SESSION_TAG}_smoke_new/" --yes \
        > /dev/null 2> "$OUT_DIR/blit-logs/smoke_new.err" || die "new-pair smoke FAILED (BUILD_MISMATCH here = staged daemon is not $NEW_SHA)"
    log "smoke ok: new pair"
    win_client_run "smoke_winmac" "s0" "${MAC_REMOTE}smoke_src/" "$WIN_MODULE\\pull_${SESSION_TAG}_smoke\\" ""
    [[ "$T_RC" == 0 ]] || die "win->mac smoke FAILED (rc=$T_RC — macOS application firewall blocking the Mac daemon? see blit-logs/smoke_winmac_s0.err)"
    wssh "Remove-Item -Recurse -Force '$WIN_MODULE\\pull_${SESSION_TAG}_smoke' -ErrorAction SilentlyContinue" || true
    log "smoke ok: win->mac (mac daemon reachable; firewall clear)"
}

# --- Verdicts (design D2 as amended; F3; F4 + discriminator recorded) ----
compute_verdicts() {
    python3 - "$CSV" "$META" "$BASELINE_SUMMARY" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" <<'PYEOF'
import csv, sys
runs_p, meta_p, base_p, summary_p, verdicts_p = sys.argv[1:6]
rows = list(csv.DictReader(open(runs_p)))
meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
base = {r["cell"]: int(r["median_ms"]) for r in csv.DictReader(open(base_p))}

by_arm, voided = {}, {}
for r in rows:
    key = (r["cell"], r["arm"])
    if r["valid"] == "yes":
        by_arm.setdefault(key, []).append(int(r["ms"]))
    else:
        voided[key] = voided.get(key, 0) + 1

def median(v):
    v = sorted(v); n = len(v)
    return v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) // 2

def complete(cell):
    if cell not in meta or meta[cell]["complete"] != "yes":
        return False
    arms = [a for (c, a) in by_arm if c == cell]
    return len(arms) == 2

def bar(new, ref):   # new <= ref * 1.10, integer-exact
    return 10 * new <= 11 * ref

out = open(verdicts_p, "w")
out.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome\n")

with open(summary_p, "w") as f:
    f.write("cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted\n")
    for (cell, arm) in sorted(by_arm):
        if not complete(cell):
            continue
        v = by_arm[(cell, arm)]
        spread = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
        f.write(f"{cell},{arm},{median(v)},{sum(v)//len(v)},{min(v)},{spread},"
                f"{voided.get((cell, arm), 0)},{meta[cell]['pairs_attempted']}\n")

def m(cell, arm):
    return median(by_arm[(cell, arm)]) if (cell, arm) in by_arm else None

# Block 1: converge-up, both references (12a logic verbatim).
b1_cells = sorted(c for c in meta if c.split("_")[0] in ("push", "pull"))
for cell in b1_cells:
    if not complete(cell):
        out.write(f"{cell},converge,new,combined,,,,1.10,INCOMPLETE\n")
        continue
    new_m, old_m = m(cell, "new"), m(cell, "old")
    if cell not in base:
        sys.exit(f"FATAL: no committed reference row for {cell}")
    ref_m = base[cell]
    p1, p2 = bar(new_m, old_m), bar(new_m, ref_m)
    out.write(f"{cell},converge,new,old_session,{new_m},{old_m},{new_m/old_m:.3f},1.10,{'PASS' if p1 else 'FAIL'}\n")
    out.write(f"{cell},converge,new,old_committed,{new_m},{ref_m},{new_m/ref_m:.3f},1.10,{'PASS' if p2 else 'FAIL'}\n")
    combined = ("PASS" if p1 and p2 else "FAIL-REFERENCE-DRIFT" if p1
                else "FAIL-SAME-SESSION" if p2 else "FAIL-BOTH")
    out.write(f"{cell},converge,new,combined,{new_m},,,1.10,{combined}\n")

# Block 2: invariance + per-arm converge (F3) + cross rows (F4) +
# discriminator gap rows (D-2026-07-12-1; recorded, not adjudicated).
b2_cells = sorted(c for c in meta if c.split("_")[0] in ("mw", "wm"))
for cell in b2_cells:
    if not complete(cell):
        out.write(f"{cell},invariance,mac_init,win_init,,,,1.10,INCOMPLETE\n")
        continue
    a, b = m(cell, "mac_init"), m(cell, "win_init")
    hi, lo = max(a, b), min(a, b)
    inv = bar(hi, lo)   # max/min <= 1.10
    out.write(f"{cell},invariance,mac_init,win_init,{a},{b},{hi/lo:.3f},1.10,{'PASS' if inv else 'FAIL'}\n")
    # F3: each arm independently meets the direction's converge bars.
    d, carrier, fixture = cell.split("_")
    verb = "push" if d == "mw" else "pull"
    b1 = f"{verb}_{carrier}_{fixture}"
    ref_m = base.get(b1)
    old_sess = m(b1, "old")
    for armname, val in (("mac_init", a), ("win_init", b)):
        if old_sess is not None:
            out.write(f"{cell},converge,{armname},old_session,{val},{old_sess},{val/old_sess:.3f},1.10,{'PASS' if bar(val, old_sess) else 'FAIL'}\n")
        if ref_m is not None:
            out.write(f"{cell},converge,{armname},old_committed,{val},{ref_m},{val/ref_m:.3f},1.10,{'PASS' if bar(val, ref_m) else 'FAIL'}\n")
    # F4 cross: each direction vs min of the two committed old
    # directions for this fixture x carrier.
    p_ref, l_ref = base.get(f"push_{carrier}_{fixture}"), base.get(f"pull_{carrier}_{fixture}")
    if p_ref is not None and l_ref is not None:
        cross_ref = min(p_ref, l_ref)
        worst = max(a, b)
        out.write(f"{cell},cross,worst_arm,min_old_committed,{worst},{cross_ref},{worst/cross_ref:.3f},1.10,{'PASS' if bar(worst, cross_ref) else 'FAIL'}\n")

# Discriminator gap rows: same-session old direction gap vs unified gap
# (per fixture x carrier; needs both directions complete).
for carrier in ("tcp", "grpc"):
    for fixture in ("large", "small", "mixed"):
        po, lo_ = m(f"push_{carrier}_{fixture}", "old"), m(f"pull_{carrier}_{fixture}", "old")
        mw = [m(f"mw_{carrier}_{fixture}", x) for x in ("mac_init", "win_init")]
        wm = [m(f"wm_{carrier}_{fixture}", x) for x in ("mac_init", "win_init")]
        if None in (po, lo_) or None in mw or None in wm:
            continue
        old_gap = po / lo_
        new_gap = max(mw) / max(wm) if max(wm) else 0
        out.write(f"gap_{carrier}_{fixture},cross-gap,old_push/old_pull,new_mw/new_wm,"
                  f"{po},{lo_},{old_gap:.3f},,RECORDED\n")
        out.write(f"gap_{carrier}_{fixture},cross-gap,new_mw,new_wm,"
                  f"{max(mw)},{max(wm)},{new_gap:.3f},,RECORDED\n")
out.close()
PYEOF
}

# --- Matrix ----------------------------------------------------------------
main() {
    preflight
    write_manifest
    if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
        log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
        exit 0
    fi
    log "session $SESSION_TAG  old=$OLD_SHA new=$NEW_SHA  win: $WIN_HOST  mac: $MAC_HOST"

    gen_fixtures
    setup_host
    mac_daemon_start
    smoke
    stage_pull_sources

    local w flag carrier
    # BLOCK 1 — converge-up (old vs new, Mac-initiated).
    for w in large small mixed; do
        for carrier in tcp grpc; do
            [[ "$carrier" == grpc ]] && flag="--force-grpc" || flag=""
            CUR_W="$w"; CUR_FLAG="$flag"
            if want_cell "push_${carrier}_${w}"; then
                run_pair_loop "push_${carrier}_${w}" old new b1_push_old b1_push_new
            fi
            if want_cell "pull_${carrier}_${w}"; then
                run_pair_loop "pull_${carrier}_${w}" old new b1_pull_old b1_pull_new
            fi
        done
    done

    # BLOCK 2 — invariance (mac_init vs win_init, new pair only).
    win_ensure new
    for w in large small mixed; do
        for carrier in tcp grpc; do
            [[ "$carrier" == grpc ]] && flag="--force-grpc" || flag=""
            CUR_W="$w"; CUR_FLAG="$flag"
            if want_cell "mw_${carrier}_${w}"; then
                run_pair_loop "mw_${carrier}_${w}" mac_init win_init b2_mw_mac b2_mw_win
            fi
            if want_cell "wm_${carrier}_${w}"; then
                run_pair_loop "wm_${carrier}_${w}" mac_init win_init b2_wm_mac b2_wm_win
            fi
        done
    done

    if [[ -n "$CELLS" ]]; then
        local c
        for c in ${CELLS//,/ }; do
            grep -q "^$c," "$META" \
                || die "CELLS entry '$c' matched no comparison — nothing was measured for it"
        done
    fi

    win_daemon_stop
    mac_daemon_stop
    compute_verdicts

    log ""
    log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/comparison; ABBA) ==="
    column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
    log ""
    log "=== VERDICTS (D2 both-references; invariance; F4 cross + gap rows) ==="
    column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"
    log "runs: $CSV"
}

SESSION_TAG=$(date +%H%M%S).$$
trap on_exit EXIT
T_MS=0; T_RC=0
main "$@"
