#!/usr/bin/env bash
# otp-2w: OLD-path baseline on the owner-designated CLOSER-SPEC pair
# (Mac client ↔ Windows daemon host, both 10GbE, both NVMe; the owner's
# words — not a claim of symmetry) — the rig
# for the otp-12 acceptance bar's cross-direction half, which
# D-2026-07-05-1 forbids evaluating on asymmetric endpoints (the
# Mac↔zoey rig; see scripts/bench_otp2_baseline.sh and
# docs/bench/otp2-baseline-2026-07-10/).
#
# Same methodology as the zoey harness, with the daemon-host half in
# PowerShell over ssh:
#   * cold caches: macOS `purge` (NOPASSWD sudoers rule) + Windows
#     standby-list purge (scripts/windows/purge-standby.ps1, staged to
#     the host at setup; admin token required);
#   * durable-at-destination windows: pushes end with
#     Write-VolumeCache <drive> on the host; pulls end with a per-file
#     fsync walk on the Mac (macOS sync(2) only schedules);
#   * disk drain before every run: three consecutive 2s samples of
#     PhysicalDisk(_Total) write bytes/sec under 1 MiB/s (NVMe box —
#     drains are near-instant; timeouts recorded, never silent);
#   * MEDIAN of RUNS (default 4) per cell; integer ms (even-count
#     median = floor of the mean of the middle two);
#   * fresh, per-invocation-unique destinations; no competitor rows.
#
# Usage (from the client Mac):
#   export WIN_SSH=michael@10.1.10.173
#   export WIN_HOST=10.1.10.173
#   export WIN_REPO='F:\dev\blit_v2'      # daemon binary lives in its target\release
#   export WIN_TEST='D:\blit-test'        # module root + config + logs (owner-designated)
#   ./scripts/bench_otp2w_baseline.sh
#
# First-run setup performed automatically (idempotent, admin):
#   * stages purge-standby.ps1 into $WIN_TEST;
#   * adds ONE program-scoped inbound firewall allow rule
#     ("blit-bench-daemon") for the daemon binary — remove with:
#     Remove-NetFirewallRule -DisplayName blit-bench-daemon

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
BLIT="$REPO_ROOT/target/release/blit"

WIN_SSH=${WIN_SSH:-michael@10.1.10.173}
WIN_HOST=${WIN_HOST:-10.1.10.173}
WIN_REPO=${WIN_REPO:-'F:\dev\blit_v2'}
WIN_TEST=${WIN_TEST:-'D:\blit-test'}
WIN_DRIVE=${WIN_DRIVE:-D}
PORT=${PORT:-9031}
RUNS=${RUNS:-4}
MAC_WORK=${MAC_WORK:-$HOME/blit-bench-work}

OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp2w_baseline_$(date +%Y%m%dT%H%M%S)}
mkdir -p "$OUT_DIR" "$MAC_WORK"

DAEMON_EXE="$WIN_REPO\\target\\release\\blit-daemon.exe"
REMOTE="$WIN_HOST:$PORT:/bench/"

log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log"; }
# ControlMaster multiplexing: ssh to this host costs ~0.5s per
# connection (pwsh default-shell spawn); reuse one connection so the
# many drain/purge round trips don't dominate wall time.
SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto -o "ControlPath=$HOME/.ssh/cm-%r@%h-%p" -o ControlPersist=300)
wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }
now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }

# --- Self-timed durability steps --------------------------------------
# The timed window must contain the TRANSFER plus the DESTINATION
# flush — and nothing else. Wrapping `ssh host flush` in the window
# adds connection setup + shell spawn (~0.5s to this host, ~1.2s to
# zoey — measured), which lands only on push cells and inflated the
# first recorded session's push/pull ratios (codex otp-2w F3, upheld
# and quantified). Each durability step therefore times ITSELF on the
# destination machine and reports only its own duration; the harness
# adds that to the locally-timed transfer segment.
flush_dest_ms() {   # Windows volume flush; prints its own elapsed ms
    # tr strips the CRLF PowerShell emits — bash arithmetic on a value
    # with a trailing CR is a syntax error (found live on the first
    # self-timed run).
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

# --- Preflight -------------------------------------------------------
[[ -x "$BLIT" ]] || { echo "missing $BLIT (cargo build --release first)"; exit 1; }
command -v python3 >/dev/null || { echo "python3 required"; exit 1; }
sudo -n /usr/sbin/purge || { echo "need the NOPASSWD purge sudoers rule"; exit 1; }
wssh "if (-not (Test-Path '$DAEMON_EXE')) { exit 1 }" || {
    echo "daemon binary missing at $DAEMON_EXE (build on the host first)"; exit 1; }
BUILD_SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD)
SESSION_TAG=$(date +%H%M%S).$$
log "build sha: $BUILD_SHA  client: macOS  daemon host: $WIN_HOST  session: $SESSION_TAG"

# --- One-time host setup (idempotent) ---------------------------------
setup_host() {
    scp -q -o BatchMode=yes "$SCRIPT_DIR/windows/purge-standby.ps1" \
        "$WIN_SSH:$WIN_TEST/purge-standby.ps1" 2>/dev/null || {
        wssh "New-Item -ItemType Directory -Force -Path '$WIN_TEST' | Out-Null"
        scp -q -o BatchMode=yes "$SCRIPT_DIR/windows/purge-standby.ps1" \
            "$WIN_SSH:$WIN_TEST/purge-standby.ps1"
    }
    wssh "New-Item -ItemType Directory -Force -Path '$WIN_TEST\\bench-module' | Out-Null
if (-not (Get-NetFirewallRule -DisplayName blit-bench-daemon -ErrorAction SilentlyContinue)) {
  New-NetFirewallRule -DisplayName blit-bench-daemon -Direction Inbound -Program '$DAEMON_EXE' -Action Allow | Out-Null
  'firewall rule added'
} else { 'firewall rule present' }"
}

# --- Daemon lifecycle --------------------------------------------------
# The module path is written as a TOML LITERAL string (single quotes):
# double-quoted TOML treats backslash sequences like \b as escapes,
# which silently corrupts Windows paths.
#
# The daemon is launched via WMI (Win32_Process.Create), NOT
# Start-Process: Windows OpenSSH puts the session in a job object and
# kills its children on disconnect, so a Start-Process daemon dies the
# moment the launching ssh command returns. A WMI-created process is
# parented outside the session and survives; `cmd /c` supplies the log
# redirection Win32_Process.Create lacks.
# Stale-daemon refusal + PID tracking (codex otp-2w F2): a leftover
# daemon would mask a new bind failure and get benchmarked in place of
# this build, so the launch REFUSES if any blit-daemon already runs;
# the fresh daemon's PID is recorded and teardown kills exactly that
# PID — never by name.
start_daemon() {
    wssh "if (Get-Process blit-daemon -ErrorAction SilentlyContinue) { 'STALE blit-daemon already running - stop it first (Stop-Process -Name blit-daemon)'; exit 1 }" || {
        echo "refusing to start over a stale daemon"; exit 1; }
    wssh "Set-Content -Path '$WIN_TEST\\bench-config.toml' -Value @'
[daemon]
bind = \"0.0.0.0\"
port = $PORT
no_mdns = true

[[module]]
name = \"bench\"
path = '$WIN_TEST\\bench-module'
'@
\$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = 'cmd /c \"\"$DAEMON_EXE\" --config \"$WIN_TEST\\bench-config.toml\" > \"$WIN_TEST\\daemon-out.log\" 2> \"$WIN_TEST\\daemon-err.log\"\"' }
if (\$r.ReturnValue -ne 0) { \"wmi create failed: \$(\$r.ReturnValue)\"; exit 1 }"
    sleep 2
    wssh "\$d = Get-Process blit-daemon -ErrorAction SilentlyContinue
if (-not \$d) { Get-Content '$WIN_TEST\\daemon-err.log' -ErrorAction SilentlyContinue | Select-Object -First 10; exit 1 }
Set-Content -Path '$WIN_TEST\\daemon.pid' -Value \$d.Id" || {
        echo "daemon failed to start"; exit 1; }
    log "daemon up on $WIN_HOST:$PORT (module bench -> $WIN_TEST\\bench-module)"
}

stop_daemon() {
    wssh "\$p = Get-Content '$WIN_TEST\\daemon.pid' -ErrorAction SilentlyContinue
if (\$p) { Stop-Process -Id \$p -Force -ErrorAction SilentlyContinue; Remove-Item '$WIN_TEST\\daemon.pid' -ErrorAction SilentlyContinue }" || true
}
sweep_push_dirs() {
    wssh "Remove-Item -Recurse -Force '$WIN_TEST\\bench-module\\push_${SESSION_TAG}_*' -ErrorAction SilentlyContinue" || true
}
trap 'stop_daemon; sweep_push_dirs' EXIT

# --- Drain + cold caches ----------------------------------------------
# A failed counter read must never count as quiet (codex otp-2w F1:
# non-terminating errors leave $w = $null, and $null -lt N is true in
# PowerShell) — errors terminate, and only a real numeric sample below
# the threshold increments the quiet count.
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

drop_caches() {   # $1 = run label
    local outcome
    outcome=$(drain_host || true)
    echo "$1: ${outcome:-DRAIN-ERROR}" >> "$OUT_DIR/drain.log"
    # Anything but a positive "drained" report is a warned anomaly —
    # a timeout AND a failed/empty probe alike (fail loud, not open).
    [[ "$outcome" == drained* ]] || log "  WARNING: $1 ran UNDRAINED (${outcome:-probe failed})"
    sync
    sudo -n /usr/sbin/purge
    wssh "pwsh -NoProfile -File '$WIN_TEST\\purge-standby.ps1'" >/dev/null
}

# --- Fixtures (reused from the zoey run when present) -------------------
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
}

# --- Timing core --------------------------------------------------------
CSV="$OUT_DIR/results.csv"
echo "cell,run,ms" > "$CSV"
SUMMARY="$OUT_DIR/summary.csv"
echo "cell,median_ms,avg_ms,best_ms" > "$SUMMARY"

finish_cell() {
    local label="$1" total="$2" best="$3"
    local median
    median=$(grep "^$label," "$CSV" | cut -d, -f3 | sort -n | awk '
        { v[NR] = $1 }
        END { if (NR % 2) print v[(NR+1)/2];
              else print int((v[NR/2] + v[NR/2+1]) / 2) }')
    echo "$label,$median,$(( total / RUNS )),$best" >> "$SUMMARY"
    log "  $label median: ${median}ms avg: $(( total / RUNS ))ms best: ${best}ms"
}

push_cell() {    # label src flag(optional)
    local label="$1" src="$2" flag="${3:-}"
    local total=0 best=999999999 run start end ms flush_ms
    for run in $(seq 1 "$RUNS"); do
        drop_caches "$label-r$run"
        start=$(now_ms)
        # shellcheck disable=SC2086
        "$BLIT" copy "$src" "${REMOTE}push_${SESSION_TAG}_${label}_r${run}/" --yes $flag >/dev/null 2>&1
        end=$(now_ms)
        flush_ms=$(flush_dest_ms)         # durable at dest, self-timed
        ms=$(( end - start + flush_ms ))
        total=$(( total + ms )); (( ms < best )) && best=$ms
        log "  $label run $run: ${ms}ms (flush ${flush_ms}ms)"
        echo "$label,$run,$ms" >> "$CSV"
    done
    finish_cell "$label" "$total" "$best"
}

pull_cell() {    # label remote_src flag(optional)
    local label="$1" remote_src="$2" flag="${3:-}"
    local total=0 best=999999999 run start end ms fsync_ms
    for run in $(seq 1 "$RUNS"); do
        rm -rf "$MAC_WORK/dst_pull"
        mkdir -p "$MAC_WORK/dst_pull"
        drop_caches "$label-r$run"
        start=$(now_ms)
        # shellcheck disable=SC2086
        "$BLIT" copy "$remote_src" "$MAC_WORK/dst_pull" --yes $flag >/dev/null 2>&1
        end=$(now_ms)
        fsync_ms=$(fsync_tree_ms "$MAC_WORK/dst_pull")   # durable, self-timed
        ms=$(( end - start + fsync_ms ))
        total=$(( total + ms )); (( ms < best )) && best=$ms
        log "  $label run $run: ${ms}ms (fsync ${fsync_ms}ms)"
        echo "$label,$run,$ms" >> "$CSV"
    done
    finish_cell "$label" "$total" "$best"
}

# --- Matrix --------------------------------------------------------------
main() {
    gen_fixtures
    setup_host
    start_daemon

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
    log "=== SUMMARY (cold-cache, drained, durable, $RUNS runs/cell) ==="
    column -t -s, "$SUMMARY" | tee -a "$OUT_DIR/bench.log"
    log "results: $CSV"
}

main "$@"
