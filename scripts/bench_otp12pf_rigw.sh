#!/usr/bin/env bash
# bench_otp12pf_rigw.sh — focused pf-1 P1 phase diagnostic on q ↔ Windows.
#
# Execute this script ON q, from an isolated clean clone of the reviewed
# commit.  It measures semantic initiator roles, never legacy push/pull
# implementations: SOURCE always sends and DESTINATION always receives.
# The only varied property within a pair is which endpoint initiates the
# Transfer RPC and therefore which endpoint dials the peer.
#
# Registered diagnostic (128 timed transfers):
#   B1 trace OFF, forward cell order, pairs 1..4
#   B2 trace ON,  reverse cell order, pairs 1..4
#   B3 trace ON,  forward cell order, pairs 5..8
#   B4 trace OFF, reverse cell order, pairs 5..8
# Each round traverses cells base/reverse/reverse/base and runs the two roles
# adjacently.  Each trace state therefore has eight valid role pairs per cell,
# balanced four/four for which role goes first.
#
# This is the reduced P1 rig diagnostic.  It does NOT by itself close pf-1:
# the active plan separately requires the small-fixture/P2 work and 0f922de
# historical control before the pf-1 hard gate is complete.

set -Eeuo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

SELFTEST=${SELFTEST:-0}
PREFLIGHT_ONLY=${PREFLIGHT_ONLY:-0}
EXPECT_SHA=${EXPECT_SHA:-}

# The experiment identity is deliberately not configurable.  In particular,
# using a hostname here would hit q's stale netwatch-01 known_hosts entry;
# every q→Windows control and transfer uses the pinned numeric endpoint.
Q_EXPECT_HOST=q.lan
Q_NIC=en8
Q_IP=10.1.10.54
Q_MAC=00:01:d2:19:04:a3
WIN_SSH=michael@10.1.10.177
WIN_IP=10.1.10.177
WIN_NIC=Ethernet
WIN_MAC=34-5A-60-3E-78-8B
REGISTERED_MTU=9000
REGISTERED_MEDIA=10Gbase-T
Q_TO_WIN_MSS=8948
WIN_TO_Q_MSS=8960
PORT=9031
PAIRS_PER_BLOCK=4
LOAD1_MAX=3.0
SPOTLIGHT_CPU_MAX=10.0
WIN_CPU_MAX=20.0
SETTLE_NS=250000000
SETTLE_MIN_MS=250
SETTLE_MAX_MS=1000

Q_MODULE="$HOME/blit-bench-work"
Q_BLIT="$REPO_ROOT/target/release/blit"
Q_DAEMON="$REPO_ROOT/target/release/blit-daemon"
WIN_ROOT='D:/blit-test'
WIN_MODULE="$WIN_ROOT/rigw-module"
WIN_BINS="$WIN_ROOT/bins"
WIN_ACTIVE="$WIN_BINS/active/blit-daemon.exe"
WIN_PURGE="$WIN_ROOT/purge-standby.ps1"

SESSION_TAG=$(date -u +%Y%m%dT%H%M%SZ).$$
OUT_DIR=${OUT_DIR:-$REPO_ROOT/logs/otp12pf-rigw-$SESSION_TAG}
WIN_SESSION="$WIN_ROOT/rigw-pf1/$SESSION_TAG"

mkdir -p "$OUT_DIR/trace" "$OUT_DIR/client"
LOG="$OUT_DIR/bench.log"
RUNS_CSV="$OUT_DIR/runs.csv"
CLOCK_CSV="$OUT_DIR/clock-samples.csv"

LAST_ERROR=""
log() { printf '%s %s\n' "$(date -u +%H:%M:%SZ)" "$*" | tee -a "$LOG"; }
die() { LAST_ERROR="$*"; log "FATAL: $*"; exit 1; }
session_void() {
    local reason="$1"
    LAST_ERROR="$reason"
    printf '%s\n' "$reason" > "$OUT_DIR/SESSION-VOID"
    log "SESSION-VOID: $reason"
    exit 1
}

SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto \
    -o "ControlPath=$HOME/.ssh/cm-rigw-%r@%h-%p" -o ControlPersist=300)
wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }

q_daemon_pid=""
win_daemon_pid=""
win_cmd_pid=""
current_block=""
CLEANUP_MODE=0
CLEANUP_ERROR=""

teardown_die() {
    local reason="$1"
    if [[ "$CLEANUP_MODE" == 1 ]]; then
        CLEANUP_ERROR="${CLEANUP_ERROR:+$CLEANUP_ERROR; }$reason"
        log "CLEANUP-ERROR: $reason"
        return 1
    fi
    session_void "$reason"
}

reject_registered_overrides() {
    local name
    for name in RUNS CELLS MAC_HOST WIN_HOST WIN_SSH_OVERRIDE PORT_OVERRIDE \
        Q_NIC_OVERRIDE Q_IP_OVERRIDE TRACE_ORDER PAIRS_PER_BLOCK_OVERRIDE; do
        if [[ -n "${!name+x}" ]]; then
            die "$name is not configurable for the registered rig-W diagnostic"
        fi
    done
}

emit_schedule() {
    cat <<'EOF'
1,off,forward,1,4
2,on,reverse,1,4
3,on,forward,5,8
4,off,reverse,5,8
EOF
}

q_source_path() { printf '%s/src_%s' "$Q_MODULE" "$1"; }
win_source_path() { printf '%s/src_%s' "$WIN_MODULE" "$1"; }
q_destination_path() { printf '%s/rigw-sessions/%s/%s/container' "$Q_MODULE" "$SESSION_TAG" "$1"; }
win_destination_path() { printf '%s/rigw-sessions/%s/%s/container' "$WIN_MODULE" "$SESSION_TAG" "$1"; }
append_clock_row() {
    printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' "$@"
}
q_monotonic_ns() {
    python3 -c 'import time; print(time.monotonic_ns())'
}
settle_until_deadline() {
    python3 - "$1" <<'PY'
import sys, time

deadline_ns = int(sys.argv[1])
remaining_ns = deadline_ns - time.monotonic_ns()
if remaining_ns > 0:
    time.sleep(remaining_ns / 1_000_000_000)
print(time.monotonic_ns())
PY
}
successful_windows_log_phase_ok() {
    [[ "$1" == durability_verified ]]
}
fetch_successful_windows_client_log() {
    local arm_phase="$1" remote_err="$2" local_err="$3"
    successful_windows_log_phase_ok "$arm_phase" \
        || session_void "refusing successful Windows client-log fetch before destination durability"
    fetch_win_file "$remote_err" "$local_err"
}
embeds_clean_q() {
    local path="$1"
    LC_ALL=C grep -qa -- "+$HEAD_BUILD_ID" "$path" || return 1
    LC_ALL=C grep -qa -- "+$HEAD_BUILD_ID.dirty" "$path" && return 1
    return 0
}

selftest() {
    local got expected rows source_first destination_first clock_probe identity_file
    local selftest_client_done selftest_deadline selftest_settle_done run_arm_source
    reject_registered_overrides
    got=$(emit_schedule)
    expected=$'1,off,forward,1,4\n2,on,reverse,1,4\n3,on,forward,5,8\n4,off,reverse,5,8'
    [[ "$got" == "$expected" ]] || die "registered block schedule changed"

    rows=0; source_first=0; destination_first=0
    local block state pass first last round pair first_role
    while IFS=, read -r block state pass first last; do
        for ((round=1; round<=PAIRS_PER_BLOCK; round++)); do
            pair=$((first + round - 1))
            case "$round" in
                1|4) first_role=source_init; source_first=$((source_first + 4));;
                2|3) first_role=destination_init; destination_first=$((destination_first + 4));;
            esac
            [[ "$pair" -ge "$first" && "$pair" -le "$last" && -n "$first_role" ]]
            rows=$((rows + 8)) # four cells × two adjacent roles
        done
    done < <(emit_schedule)
    [[ "$rows" == 128 ]] || die "schedule emitted $rows arms, expected 128"
    [[ "$source_first" == 32 && "$destination_first" == 32 ]] \
        || die "schedule role-first balance changed"
    [[ "$(q_source_path mixed)" == "$Q_MODULE/src_mixed" ]]
    [[ "$(win_source_path mixed)" == "$WIN_MODULE/src_mixed" ]]
    [[ "$(q_destination_path probe)" == "$Q_MODULE/rigw-sessions/$SESSION_TAG/probe/container" ]]
    [[ "$(win_destination_path probe)" == "$WIN_MODULE/rigw-sessions/$SESSION_TAG/probe/container" ]]
    clock_probe=$(append_clock_row 1 run cell 1 source_init before 1 10 11 12 2 0)
    [[ "$(awk -F, '{print NF}' <<<"$clock_probe")" == 12 ]] \
        || die "clock sample row is not exactly 12 columns"
    [[ "$SETTLE_NS" == 250000000 && "$SETTLE_MIN_MS" == 250 && "$SETTLE_MAX_MS" == 1000 ]] \
        || die "registered post-client settle bounds changed"
    selftest_client_done=$(q_monotonic_ns)
    selftest_deadline=$((selftest_client_done + SETTLE_NS))
    selftest_settle_done=$(settle_until_deadline "$selftest_deadline")
    [[ "$selftest_settle_done" =~ ^[0-9]+$ && "$selftest_settle_done" -ge "$selftest_deadline" ]] \
        || die "absolute post-client deadline wait returned early"
    if successful_windows_log_phase_ok client_done; then
        die "successful Windows client log was fetchable before durability"
    fi
    successful_windows_log_phase_ok durability_verified \
        || die "successful Windows client log was blocked after durability"

    run_arm_source=$(declare -f run_arm)
    python3 - "$run_arm_source" <<'PY' || die "run_arm post-client ordering changed"
import sys

source = sys.argv[1]
markers = (
    'client_done_ns=$(q_monotonic_ns)',
    'read -r _ transfer_ms rc',
    'record_clock_samples "$block" "$run_id" "$cell" "$pair" "$role" after',
    'settle_done_ns=$(settle_until_deadline "$settle_deadline_ns")',
    'flush_out=$(flush_verify_q "$dest")',
    'flush_out=$(flush_verify_win "$dest")',
    'arm_phase=durability_verified',
    'fetch_successful_windows_client_log "$arm_phase" "$remote_err" "$werr"',
)
positions = []
for marker in markers:
    try:
        positions.append(source.index(marker))
    except ValueError as exc:
        raise SystemExit(f"missing run_arm ordering marker: {marker}") from exc
if positions != sorted(positions):
    raise SystemExit(f"run_arm ordering markers out of order: {positions}")
PY

    HEAD_BUILD_ID=0123456789ab
    identity_file=$(mktemp "${TMPDIR:-/tmp}/blit-rigw-identity.XXXXXX")
    printf 'blit+%s\0' "$HEAD_BUILD_ID" > "$identity_file"
    embeds_clean_q "$identity_file" || die "clean 12-character build identity was rejected"
    printf 'blit+%s.dirty.ffffffffffff\0' "$HEAD_BUILD_ID" > "$identity_file"
    if embeds_clean_q "$identity_file"; then
        rm -f "$identity_file"
        die "dirty build identity was accepted"
    fi
    rm -f "$identity_file"

    python3 "$SCRIPT_DIR/otp12pf_rigw_analyze_test.py" \
        >> "$LOG" 2>&1 || die "analyzer self-tests failed"
    log "SELFTEST OK: exact four-block/128-arm schedule and analyzer guards"
}

sha256_q() { shasum -a 256 "$1" | awk '{print $1}'; }
sha256_win() {
    wssh "(Get-FileHash -Algorithm SHA256 -LiteralPath '$1').Hash.ToLower()" \
        | tr -d '\r' | tail -1
}

float_le() { awk -v a="$1" -v b="$2" 'BEGIN { exit !(a <= b) }'; }

q_load1() {
    /usr/sbin/sysctl -n vm.loadavg | awk '{gsub(/[{}]/, ""); print $1}'
}

q_spotlight_cpu() {
    ps -axo %cpu=,comm= | awk '
        $2 ~ /(mds|mds_stores|mdworker|mdbulkimport)$/ { sum += $1 }
        END { printf "%.1f\n", sum + 0 }'
}

q_time_machine_gate() {
    local auto status
    auto=$(/usr/bin/defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null) \
        || die "q Time Machine AutoBackup setting is unreadable"
    [[ "$auto" == 0 ]] \
        || die "q Time Machine AutoBackup is enabled ($auto); do not mutate it from the harness"
    status=$(/usr/bin/tmutil status) || die "q Time Machine status is unreadable"
    grep -q 'Running = 0;' <<<"$status" \
        || die "q Time Machine is running"
}

q_quiet_gate() {
    local offenders load spot
    offenders=$(ps -axo pid=,comm= | awk -v owned="${q_daemon_pid:-}" '
        {
          n=$2; sub(/^.*\//, "", n)
          if ($1 != owned && (n == "cargo" || n == "rustc" || n == "blit-daemon" || n ~ /^codex($|-)/))
            print $1 ":" n
        }')
    [[ -z "$offenders" ]] || die "q has benchmark-conflicting processes: $offenders"
    q_time_machine_gate
    load=$(q_load1)
    [[ "$load" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "cannot parse q load1 '$load'"
    float_le "$load" "$LOAD1_MAX" || die "q load1 $load exceeds $LOAD1_MAX"
    spot=$(q_spotlight_cpu)
    [[ "$spot" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "cannot parse Spotlight CPU '$spot'"
    float_le "$spot" "$SPOTLIGHT_CPU_MAX" \
        || die "q Spotlight CPU $spot% exceeds $SPOTLIGHT_CPU_MAX%"
    log "quiet q: load1=$load Spotlight=${spot}% TimeMachine=disabled/stopped"
}

win_quiet_gate() {
    local out avg
    out=$(wssh '
$ErrorActionPreference = "Stop"
$bad = Get-Process cargo,rustc,blit-daemon -ErrorAction SilentlyContinue
if ($bad) { "BAD|" + (($bad | ForEach-Object { "$($_.Id):$($_.ProcessName)" }) -join ","); exit 7 }
$samples = 1..3 | ForEach-Object {
  $v = (Get-CimInstance Win32_Processor | Measure-Object LoadPercentage -Average).Average
  Start-Sleep -Seconds 1
  [double]$v
}
"CPU|$([math]::Round(($samples | Measure-Object -Average).Average,1))"
') || die "Windows quiet probe failed: $out"
    out=${out//$'\r'/}
    [[ "$out" != *BAD\|* ]] || die "Windows has benchmark-conflicting processes: $out"
    avg=$(sed -n 's/^CPU|//p' <<<"$out" | tail -1)
    [[ "$avg" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "cannot parse Windows CPU from '$out'"
    float_le "$avg" "$WIN_CPU_MAX" || die "Windows CPU ${avg}% exceeds ${WIN_CPU_MAX}%"
    log "quiet Windows: CPU=${avg}% and no cargo/rustc/blit-daemon"
}

q_topology_gate() {
    local raw route arp mtu media status iface route_mtu peer_mac
    [[ "$(hostname)" == "$Q_EXPECT_HOST" ]] \
        || die "this harness must execute on $Q_EXPECT_HOST, got $(hostname)"
    raw=$(/sbin/ifconfig "$Q_NIC") || die "cannot read q $Q_NIC"
    mtu=$(sed -n 's/.*[[:space:]]mtu[[:space:]]\([0-9][0-9]*\).*/\1/p' <<<"$raw" | head -1)
    media=$(sed -n 's/^[[:space:]]*media:[[:space:]]*\(.*\)$/\1/p' <<<"$raw" | head -1)
    status=$(sed -n 's/^[[:space:]]*status:[[:space:]]*\(.*\)$/\1/p' <<<"$raw" | head -1)
    grep -q "inet $Q_IP " <<<"$raw" || die "$Q_NIC does not own $Q_IP"
    grep -qi "ether $Q_MAC" <<<"$raw" || die "$Q_NIC MAC is not $Q_MAC"
    [[ "$mtu" == "$REGISTERED_MTU" ]] || die "$Q_NIC MTU is $mtu, expected $REGISTERED_MTU"
    [[ "$media" == *"$REGISTERED_MEDIA"* ]] || die "$Q_NIC media is '$media', expected $REGISTERED_MEDIA"
    [[ "$status" == active ]] || die "$Q_NIC status is '$status'"

    route=$(/sbin/route -n get "$WIN_IP") || die "q route probe failed"
    iface=$(awk '/interface:/ {print $2; exit}' <<<"$route")
    route_mtu=$(awk '/mtu/ {getline; print $(NF-1); exit}' <<<"$route")
    [[ "$iface" == "$Q_NIC" ]] || die "q routes $WIN_IP via $iface, expected $Q_NIC"
    [[ "$route_mtu" == "$REGISTERED_MTU" ]] \
        || die "q route to $WIN_IP reports MTU $route_mtu, expected $REGISTERED_MTU"
    /sbin/ping -c 1 -W 1000 "$WIN_IP" >/dev/null || die "q cannot ping $WIN_IP"
    arp=$(/usr/sbin/arp -n "$WIN_IP") || die "q ARP probe failed"
    peer_mac=$(sed -n 's/.* at \([^ ]*\) on .*/\1/p' <<<"$arp" | tr 'A-F' 'a-f')
    [[ "$peer_mac" == "$(tr 'A-F' 'a-f' <<<"${WIN_MAC//-/:}")" ]] \
        || die "q ARP for $WIN_IP is $peer_mac, expected peer ${WIN_MAC//-/:}"
    [[ "$peer_mac" != "$Q_MAC" ]] || die "q ARP points at q's own MAC (black-hole host route)"
    log "fabric q: $Q_NIC $Q_IP mtu=$mtu media=$media route=$iface peer=$peer_mac"
}

win_topology_gate() {
    local out
    out=$(wssh "
\$ErrorActionPreference = 'Stop'
\$a = Get-NetAdapter -Name '$WIN_NIC'
\$ip = Get-NetIPAddress -InterfaceAlias '$WIN_NIC' -AddressFamily IPv4 | Where-Object IPAddress -eq '$WIN_IP'
\$ni = Get-NetIPInterface -InterfaceAlias '$WIN_NIC' -AddressFamily IPv4
\$route = Find-NetRoute -RemoteIPAddress '$Q_IP' | Select-Object -First 1
if (-not \$ip) { throw 'registered IPv4 address absent' }
\"W|\$(\$a.Status)|\$(\$a.LinkSpeed)|\$(\$a.ReceiveLinkSpeed)|\$(\$a.TransmitLinkSpeed)|\$(\$a.MacAddress)|\$(\$ni.ConnectionState)|\$(\$ni.NlMtu)|\$(\$route.InterfaceAlias)|\$(\$route.IPAddress)\"
") || die "Windows topology probe failed: $out"
    out=${out//$'\r'/}
    [[ "$out" == "W|Up|10 Gbps|10000000000|10000000000|$WIN_MAC|Connected|$REGISTERED_MTU|$WIN_NIC|$WIN_IP" ]] \
        || die "Windows topology mismatch: '$out'"
    log "fabric Windows: $WIN_NIC $WIN_IP mtu=$REGISTERED_MTU link=10Gbps route/source pinned"
}

q_to_win_mss() {
    python3 - "$WIN_IP" <<'PY'
import socket, sys
s = socket.create_connection((sys.argv[1], 22), timeout=5)
print(f"{s.getsockopt(socket.IPPROTO_TCP, socket.TCP_MAXSEG)} {s.getsockname()[0]}")
s.close()
PY
}

win_to_q_mss() {
    wssh "
\$ErrorActionPreference = 'Stop'
\$s = [Net.Sockets.Socket]::new([Net.Sockets.AddressFamily]::InterNetwork,[Net.Sockets.SocketType]::Stream,[Net.Sockets.ProtocolType]::Tcp)
\$s.Connect('$Q_IP',22)
\$name = [Net.Sockets.SocketOptionName]4
\$b = \$s.GetSocketOption([Net.Sockets.SocketOptionLevel]::Tcp,\$name,4)
\$m = [BitConverter]::ToInt32(\$b,0)
\"M|\${m}|\$(\$s.LocalEndPoint.Address)\"
\$s.Dispose()
" | tr -d '\r' | tail -1
}

mss_gate() {
    local qout wout qm qip wm wip
    qout=$(q_to_win_mss) || die "q→Windows MSS probe failed"
    read -r qm qip <<<"$qout"
    [[ "$qm" == "$Q_TO_WIN_MSS" && "$qip" == "$Q_IP" ]] \
        || die "q→Windows MSS/source is '$qout', expected $Q_TO_WIN_MSS $Q_IP"
    wout=$(win_to_q_mss) || die "Windows→q MSS probe failed"
    IFS='|' read -r _ wm wip <<<"$wout"
    [[ "$wm" == "$WIN_TO_Q_MSS" && "$wip" == "$WIN_IP" ]] \
        || die "Windows→q MSS/source is '$wout', expected M|$WIN_TO_Q_MSS|$WIN_IP"
    log "path MSS: q→Windows=$qm via $qip; Windows→q=$wm via $wip"
}

firewall_gate() {
    local out
    out=$(wssh "
\$r = Get-NetFirewallRule -DisplayName 'blit-otp12-daemon' -ErrorAction SilentlyContinue
if (-not \$r) { exit 4 }
\$app = \$r | Get-NetFirewallApplicationFilter
\"F|\$(\$r.Enabled)|\$(\$r.Action)|\$(\$r.Direction)|\$(\$app.Program)\"
") || die "existing Windows firewall rule is absent/unreadable; harness will not create it"
    out=${out//$'\r'/}
    out=$(sed 's#\\#/#g' <<<"$out")
    out=$(tr 'A-Z' 'a-z' <<<"$out")
    local expected
    expected=$(tr 'A-Z' 'a-z' <<<"F|True|Allow|Inbound|$WIN_ACTIVE")
    [[ "$out" == "$expected" ]] \
        || die "Windows firewall rule mismatch: '$out'"
    log "firewall verified only: existing inbound allow is scoped to $WIN_ACTIVE"
}

ports_closed() {
    if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
        return 1
    fi
    wssh "if (Get-NetTCPConnection -State Listen -LocalPort $PORT -ErrorAction SilentlyContinue) { exit 9 }" \
        >/dev/null 2>&1
}

timer_gate() {
    local qms wout wms
    qms=$(python3 - <<'PY'
import time
t=time.monotonic_ns(); time.sleep(1); print(round((time.monotonic_ns()-t)/1_000_000))
PY
)
    [[ "$qms" -ge 950 && "$qms" -le 1050 ]] || die "q one-second timer calibrated to ${qms}ms"
    wout=$(wssh '$s=[Diagnostics.Stopwatch]::StartNew(); Start-Sleep -Seconds 1; $s.Stop(); "T|$([int]$s.Elapsed.TotalMilliseconds)"') \
        || die "Windows timer calibration failed"
    wout=${wout//$'\r'/}; wms=${wout##*|}
    [[ "$wms" -ge 950 && "$wms" -le 1050 ]] || die "Windows one-second timer calibrated to ${wms}ms"
    log "timer calibration: q=${qms}ms Windows=${wms}ms"
}

fixture_shape_q() {
    python3 - "$1" <<'PY'
import os, sys
n=b=0
for root, dirs, files in os.walk(sys.argv[1]):
    for name in files:
        p=os.path.join(root,name); n+=1; b+=os.path.getsize(p)
print(f"{n},{b}")
PY
}

fixture_shape_win() {
    wssh "
\$f = Get-ChildItem -LiteralPath '$1' -Recurse -File -ErrorAction Stop
\"S|\$(\$f.Count)|\$(if (\$f.Count) { (\$f | Measure-Object Length -Sum).Sum } else { 0 })\"
" | tr -d '\r' | sed -n 's/^S|//p' | tr '|' ',' | tail -1
}

verify_fixtures() {
    local shape want qgot wgot
    for shape in mixed large; do
        case "$shape" in
            mixed) want=5001,547110912;;
            large) want=1,1073741824;;
        esac
        qgot=$(fixture_shape_q "$(q_source_path "$shape")")
        wgot=$(fixture_shape_win "$(win_source_path "$shape")")
        [[ "$qgot" == "$want" ]] || die "q src_$shape is $qgot, expected $want"
        [[ "$wgot" == "$want" ]] || die "Windows canonical src_$shape is $wgot, expected $want"
    done
    log "canonical fixtures verified on both hosts (same paths for both initiator roles)"
}

write_manifest() {
    local qbh qdh wbh wdh
    qbh=$(sha256_q "$Q_BLIT"); qdh=$(sha256_q "$Q_DAEMON")
    wbh=$(sha256_win "$WIN_BINS/$HEAD_SHORT/blit.exe")
    wdh=$(sha256_win "$WIN_BINS/$HEAD_SHORT/blit-daemon.exe")
    cat > "$OUT_DIR/staging-manifest.csv" <<EOF
host,role,commit,sha256,path
q,client,$HEAD_FULL,$qbh,$Q_BLIT
q,daemon,$HEAD_FULL,$qdh,$Q_DAEMON
windows,client,$HEAD_FULL,$wbh,$WIN_BINS/$HEAD_SHORT/blit.exe
windows,daemon,$HEAD_FULL,$wdh,$WIN_BINS/$HEAD_SHORT/blit-daemon.exe
EOF
    WIN_DAEMON_HASH=$wdh
}

provenance_gate() {
    [[ -n "$EXPECT_SHA" ]] || die "EXPECT_SHA=<full reviewed commit> is required"
    HEAD_FULL=$(git -C "$REPO_ROOT" rev-parse HEAD)
    HEAD_SHORT=$(git -C "$REPO_ROOT" rev-parse --short=7 HEAD)
    HEAD_BUILD_ID=$(git -C "$REPO_ROOT" rev-parse --short=12 HEAD)
    [[ "$EXPECT_SHA" == "$HEAD_FULL" ]] \
        || die "EXPECT_SHA=$EXPECT_SHA but isolated clone is $HEAD_FULL"
    [[ -z $(git -C "$REPO_ROOT" status --porcelain --untracked-files=normal) ]] \
        || die "isolated q clone is dirty"
    [[ -x "$Q_BLIT" && -x "$Q_DAEMON" ]] || die "q release binaries are absent"
    embeds_clean_q "$Q_BLIT" \
        || die "q client does not embed a clean +$HEAD_BUILD_ID"
    embeds_clean_q "$Q_DAEMON" \
        || die "q daemon does not embed a clean +$HEAD_BUILD_ID"
    wssh "
if (-not (Test-Path -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit.exe')) { exit 2 }
if (-not (Test-Path -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe')) { exit 3 }
if (-not (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID')) { exit 4 }
if (-not (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID')) { exit 5 }
if (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID.dirty') { exit 6 }
if (Select-String -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe' -SimpleMatch -Quiet -Pattern '+$HEAD_BUILD_ID.dirty') { exit 7 }
" || die "Windows binaries are missing or do not embed a clean +$HEAD_BUILD_ID"
    write_manifest
    log "provenance exact: $HEAD_FULL on q and Windows"
}

preflight() {
    reject_registered_overrides
    command -v python3 >/dev/null || die "python3 required"
    command -v lsof >/dev/null || die "lsof required"
    command -v nc >/dev/null || die "nc required"
    sudo -n /usr/sbin/purge >/dev/null || die "q NOPASSWD purge grant is absent"
    provenance_gate
    ports_closed || die "port $PORT already has a listener on q or Windows"
    q_topology_gate
    win_topology_gate
    mss_gate
    firewall_gate
    q_quiet_gate
    win_quiet_gate
    timer_gate
    verify_fixtures
    log "PREFLIGHT OK: registered rig, exact binaries, canonical paths, quiet endpoints"
}

q_daemon_stop() {
    local pid="$q_daemon_pid" i
    [[ -z "$pid" ]] && return 0
    if kill -0 "$pid" 2>/dev/null; then
        local cmd
        cmd=$(ps -p "$pid" -o command= 2>/dev/null || true)
        [[ "$cmd" == *"$Q_DAEMON"* ]] \
            || { teardown_die "refusing to stop q PID $pid because it is not the launched daemon: $cmd"; return 1; }
        kill "$pid" || true
        for ((i=0; i<40; i++)); do
            kill -0 "$pid" 2>/dev/null || break
            sleep 0.25
        done
        kill -0 "$pid" 2>/dev/null \
            && { teardown_die "q daemon PID $pid survived exact teardown"; return 1; }
    fi
    q_daemon_pid=""
}

win_daemon_stop() {
    local pid="$win_daemon_pid" cmdpid="$win_cmd_pid" out pid_probe
    if [[ -z "$pid" && -z "$cmdpid" && -n "$current_block" ]]; then
        pid_probe=$(wssh "
\$d = Get-Content -LiteralPath '$WIN_SESSION/block_$current_block/daemon.pid' -ErrorAction SilentlyContinue
\$c = Get-Content -LiteralPath '$WIN_SESSION/block_$current_block/launcher.pid' -ErrorAction SilentlyContinue
\"P|\$c|\$d\"
" 2>/dev/null | tr -d '\r' | tail -1) || true
        IFS='|' read -r _ cmdpid pid <<<"$pid_probe"
    fi
    [[ -z "$pid" && -z "$cmdpid" ]] && return 0
    out=$(wssh "
\$ErrorActionPreference = 'Stop'
\$pid0 = if ('$pid' -match '^[0-9]+$') { [int]'$pid' } else { \$null }
\$cmd0 = if ('$cmdpid' -match '^[0-9]+$') { [int]'$cmdpid' } else { \$null }
if (\$pid0) {
  \$p = Get-CimInstance Win32_Process -Filter \"ProcessId=\$pid0\" -ErrorAction SilentlyContinue
  if (\$p) {
    \$actual = \$p.ExecutablePath.Replace([char]92,[char]47)
    if (\$p.Name -ne 'blit-daemon.exe' -or \$actual -ne '$WIN_ACTIVE') { throw \"PID identity mismatch: \$(\$p.Name) \$(\$p.ExecutablePath)\" }
    Stop-Process -Id \$pid0 -Force
  }
}
if (\$cmd0 -and (Get-Process -Id \$cmd0 -ErrorAction SilentlyContinue)) { Stop-Process -Id \$cmd0 -Force }
Start-Sleep -Milliseconds 250
if (\$pid0 -and (Get-Process -Id \$pid0 -ErrorAction SilentlyContinue)) { throw 'daemon survived teardown' }
if (\$cmd0 -and (Get-Process -Id \$cmd0 -ErrorAction SilentlyContinue)) { throw 'launcher survived teardown' }
'STOPPED'
") || { teardown_die "Windows exact daemon teardown failed: $out"; return 1; }
    win_daemon_pid=""; win_cmd_pid=""
}

fetch_win_file() {
    local remote="$1" local_path="$2" tmp="$local_path.base64" remote_hash local_hash
    wssh "
\$b = [IO.File]::ReadAllBytes('$remote')
[Convert]::ToBase64String(\$b)
" | tr -d '\r\n' > "$tmp" || session_void "failed to fetch Windows log $remote"
    python3 - "$tmp" "$local_path" <<'PY'
import base64, pathlib, sys
src, dst = map(pathlib.Path, sys.argv[1:])
dst.write_bytes(base64.b64decode(src.read_text(), validate=True))
src.unlink()
PY
    remote_hash=$(sha256_win "$remote")
    local_hash=$(sha256_q "$local_path")
    [[ "$remote_hash" == "$local_hash" ]] \
        || session_void "Windows log hash mismatch for $remote"
}

collect_block_logs() {
    local block="$1" dir="$OUT_DIR/trace/block_$block"
    mkdir -p "$dir"
    fetch_win_file "$WIN_SESSION/block_$block/daemon.err" "$dir/windows-daemon.err"
    wssh "Remove-Item -LiteralPath '$WIN_SESSION/block_$block' -Recurse -Force -ErrorAction Stop" \
        >/dev/null || session_void "failed to remove retrieved Windows block $block logs"
}

stop_daemons() {
    local block="$1"
    win_daemon_stop
    q_daemon_stop
    collect_block_logs "$block"
    ports_closed || session_void "port $PORT still listening after block $block teardown"
}

q_daemon_start() {
    local block="$1" state="$2" run_id="$3" dir="$OUT_DIR/trace/block_$block"
    mkdir -p "$dir"
    cat > "$dir/q-daemon.toml" <<EOF
[daemon]
bind = "0.0.0.0"
port = $PORT
no_mdns = true

[[module]]
name = "bench"
path = "$Q_MODULE"
EOF
    if [[ "$state" == on ]]; then
        BLIT_TRACE_SESSION_PHASES=1 BLIT_TRACE_RUN_ID="$run_id" \
            nohup "$Q_DAEMON" --config "$dir/q-daemon.toml" \
            > "$dir/q-daemon.out" 2> "$dir/q-daemon.err" &
    else
        env -u BLIT_TRACE_SESSION_PHASES -u BLIT_TRACE_RUN_ID \
            nohup "$Q_DAEMON" --config "$dir/q-daemon.toml" \
            > "$dir/q-daemon.out" 2> "$dir/q-daemon.err" &
    fi
    q_daemon_pid=$!
    sleep 1
    kill -0 "$q_daemon_pid" 2>/dev/null \
        || session_void "q daemon failed to start in block $block"
}

win_daemon_start() {
    local block="$1" state="$2" run_id="$3" out
    out=$(wssh "
\$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path '$WIN_SESSION/block_$block','$WIN_BINS/active' | Out-Null
Copy-Item -LiteralPath '$WIN_BINS/$HEAD_SHORT/blit-daemon.exe' -Destination '$WIN_ACTIVE' -Force
if ((Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_ACTIVE').Hash.ToLower() -ne '$WIN_DAEMON_HASH') { throw 'active daemon hash mismatch' }
Set-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.toml' -Value @(
  '[daemon]', 'bind = \"0.0.0.0\"', 'port = $PORT', 'no_mdns = true', '',
  '[[module]]', 'name = \"bench\"', 'path = \"$WIN_MODULE\"'
)
\$trace = if ('$state' -eq 'on') { @('set BLIT_TRACE_SESSION_PHASES=1','set BLIT_TRACE_RUN_ID=$run_id') } else { @('set BLIT_TRACE_SESSION_PHASES=','set BLIT_TRACE_RUN_ID=') }
Set-Content -LiteralPath '$WIN_SESSION/block_$block/start.cmd' -Value @(
  '@echo off', \$trace[0], \$trace[1],
  '\"$WIN_ACTIVE\" --config \"$WIN_SESSION/block_$block/daemon.toml\" > \"$WIN_SESSION/block_$block/daemon.out\" 2> \"$WIN_SESSION/block_$block/daemon.err\"'
)
\$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$block/start.cmd\"\"' }
if (\$r.ReturnValue -ne 0) { throw \"launcher return \$(\$r.ReturnValue)\" }
Set-Content -LiteralPath '$WIN_SESSION/block_$block/launcher.pid' -Value \$r.ProcessId
Start-Sleep -Seconds 1
\$d = Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object ParentProcessId -eq \$r.ProcessId | Select-Object -First 1
if (-not \$d) { Get-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.err' -ErrorAction SilentlyContinue; throw 'daemon child absent' }
Set-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.pid' -Value \$d.ProcessId
\"P|\$(\$r.ProcessId)|\$(\$d.ProcessId)\"
") || session_void "Windows daemon failed to start in block $block: $out"
    out=${out//$'\r'/}
    IFS='|' read -r _ win_cmd_pid win_daemon_pid <<<"$(grep '^P|' <<<"$out" | tail -1)"
    [[ "$win_cmd_pid" =~ ^[0-9]+$ && "$win_daemon_pid" =~ ^[0-9]+$ ]] \
        || session_void "cannot parse Windows daemon PIDs from '$out'"
}

start_daemons() {
    local block="$1" state="$2" run_id="$3"
    ports_closed || session_void "port $PORT occupied before block $block"
    q_daemon_start "$block" "$state" "$run_id"
    win_daemon_start "$block" "$state" "$run_id"
    sleep 1
    nc -z -w 3 "$WIN_IP" "$PORT" || session_void "q cannot reach Windows daemon in block $block"
    wssh "if (-not (Test-NetConnection -ComputerName '$Q_IP' -Port $PORT -InformationLevel Quiet)) { exit 8 }" \
        >/dev/null || session_void "Windows cannot reach q daemon in block $block"
    log "block $block daemons up, trace=$state, run_id=$run_id"
}

record_clock_samples() {
    local block="$1" run_id="$2" cell="$3" pair="$4" role="$5" phase="$6" sample before after remote rtt midpoint offset
    for sample in 1 2 3; do
        before=$(python3 -c 'import time; print(time.time_ns())')
        remote=$(wssh '([DateTime]::UtcNow.Ticks - 621355968000000000) * 100' | tr -cd '0-9')
        after=$(python3 -c 'import time; print(time.time_ns())')
        [[ "$remote" =~ ^[0-9]+$ ]] || session_void "clock probe returned '$remote'"
        rtt=$((after - before)); midpoint=$((before + rtt / 2)); offset=$((remote - midpoint))
        append_clock_row \
            "$block" "$run_id" "$cell" "$pair" "$role" "$phase" "$sample" \
            "$before" "$remote" "$after" "$rtt" "$offset" >> "$CLOCK_CSV"
    done
}

drain_both() {
    sync || return 1
    sudo -n /usr/sbin/purge >/dev/null || return 1
    wssh "
\$ErrorActionPreference = 'Stop'
Write-VolumeCache D
\$quiet = 0
for (\$i=0; \$i -lt 30; \$i++) {
  \$w = (Get-Counter '\\PhysicalDisk(_Total)\\Disk Write Bytes/sec' -SampleInterval 1 -MaxSamples 1).CounterSamples[0].CookedValue
  if (\$null -ne \$w -and [double]\$w -lt 1048576) { \$quiet++ } else { \$quiet=0 }
  if (\$quiet -ge 3) { break }
}
if (\$quiet -lt 3) { throw 'DRAIN-TIMEOUT' }
if (-not (Test-Path -LiteralPath '$WIN_PURGE')) { throw 'purge helper absent' }
& pwsh -NoProfile -File '$WIN_PURGE'
if (\$LASTEXITCODE -ne 0) { throw \"purge helper rc \$LASTEXITCODE\" }
'drained'
" >/dev/null || return 1
    printf drained
}

prepare_destination() {
    local direction="$1" dest="$2"
    if [[ "$direction" == wm ]]; then
        rm -rf "$dest"
        mkdir -p "$dest"
    else
        wssh "Remove-Item -LiteralPath '$dest' -Recurse -Force -ErrorAction SilentlyContinue; New-Item -ItemType Directory -Force -Path '$dest' | Out-Null"
    fi
}

flush_verify_q() {
    python3 - "$1" <<'PY'
import os, sys, time
t=time.monotonic_ns(); n=b=0
for root, dirs, files in os.walk(sys.argv[1]):
    for name in files:
        p=os.path.join(root,name)
        fd=os.open(p,os.O_RDONLY); os.fsync(fd); os.close(fd)
        n+=1; b+=os.path.getsize(p)
print(f"F|{round((time.monotonic_ns()-t)/1_000_000)}|{n}|{b}")
PY
}

flush_verify_win() {
    wssh "
\$ErrorActionPreference = 'Stop'
\$sw=[Diagnostics.Stopwatch]::StartNew(); Write-VolumeCache D; \$sw.Stop()
\$f=Get-ChildItem -LiteralPath '$1' -Recurse -File -ErrorAction Stop
\$bytes=if (\$f.Count) { (\$f | Measure-Object Length -Sum).Sum } else { 0 }
\"F|\$([int]\$sw.Elapsed.TotalMilliseconds)|\$(\$f.Count)|\$bytes\"
" | tr -d '\r' | tail -1
}

q_client_run() {
    local state="$1" run_id="$2" err="$3"; shift 3
    local trace_env=()
    if [[ "$state" == on ]]; then
        trace_env=(BLIT_TRACE_SESSION_PHASES=1 BLIT_TRACE_RUN_ID="$run_id")
    fi
    env -u BLIT_TRACE_SESSION_PHASES -u BLIT_TRACE_RUN_ID "${trace_env[@]}" \
        python3 - "$err" "$Q_BLIT" "$@" <<'PY'
import os, subprocess, sys, time
err, argv = sys.argv[1], sys.argv[2:]
with open(err, "wb") as e:
    t=time.monotonic_ns()
    p=subprocess.run(argv, stdout=subprocess.DEVNULL, stderr=e, env=os.environ.copy())
    ms=round((time.monotonic_ns()-t)/1_000_000)
print(f"R|{ms}|{p.returncode}")
PY
}

win_client_run() {
    local state="$1" run_id="$2" remote_err="$3"; shift 3
    local src="$1" dst="$2" flag="${3:-}" out
    out=$(wssh "
\$ErrorActionPreference = 'Stop'
if ('$state' -eq 'on') { \$env:BLIT_TRACE_SESSION_PHASES='1'; \$env:BLIT_TRACE_RUN_ID='$run_id' }
else { Remove-Item Env:BLIT_TRACE_SESSION_PHASES,Env:BLIT_TRACE_RUN_ID -ErrorAction SilentlyContinue }
\$sw=[Diagnostics.Stopwatch]::StartNew()
& '$WIN_BINS/$HEAD_SHORT/blit.exe' copy '$src' '$dst' --yes $flag > \$null 2> '$remote_err'
\$rc=\$LASTEXITCODE; \$sw.Stop()
\"R|\$([int]\$sw.Elapsed.TotalMilliseconds)|\${rc}\"
") || true
    out=${out//$'\r'/}
    grep '^R|' <<<"$out" | tail -1
}

session_id_from_log() {
    python3 - "$1" <<'PY'
import json, re, sys
ids=set()
with open(sys.argv[1], errors="replace") as f:
    for line in f:
        if line.startswith("[session-phase] "):
            ids.add(json.loads(line[len("[session-phase] "):])["session_id"])
if len(ids)>1: raise SystemExit(f"multiple session ids: {sorted(ids)}")
print(next(iter(ids), ""))
PY
}

run_arm() {
    local block="$1" state="$2" pass="$3" run_id="$4" cell="$5" pair="$6" role="$7" role_order="$8"
    local direction carrier shape flag="" dest rid qerr werr client_rel client_abs remote_err result transfer_ms rc flush_out flush_ms count bytes want drain session_id total
    local windows_client=0 arm_phase=client_done client_done_ns settle_deadline_ns settle_done_ns settled_ms
    direction=${cell%%_*}
    carrier=${cell#*_}; carrier=${carrier%%_*}
    shape=${cell##*_}
    [[ "$carrier" == grpc ]] && flag=--force-grpc
    rid="b${block}_${cell}_p${pair}_${role}"
    qerr="$OUT_DIR/client/$rid.err"
    remote_err="$WIN_SESSION/block_$block/$rid.client.err"
    werr="$OUT_DIR/client/$rid.windows.err"

    if [[ "$direction" == wm ]]; then
        dest=$(q_destination_path "$rid")
    else
        dest=$(win_destination_path "$rid")
    fi
    prepare_destination "$direction" "$dest" \
        || session_void "$rid could not precreate its destination container"

    drain=$(drain_both) || session_void "$rid cache/drain gate failed"
    record_clock_samples "$block" "$run_id" "$cell" "$pair" "$role" before

    if [[ "$direction/$role" == wm/source_init ]]; then
        windows_client=1; client_abs="$werr"; client_rel="client/$rid.windows.err"
        result=$(win_client_run "$state" "$run_id" "$remote_err" \
            "$(win_source_path "$shape")" "$Q_IP:$PORT:/bench/rigw-sessions/$SESSION_TAG/$rid/container/" "$flag")
    elif [[ "$direction/$role" == wm/destination_init ]]; then
        client_abs="$qerr"; client_rel="client/$rid.err"
        result=$(q_client_run "$state" "$run_id" "$qerr" \
            copy "$WIN_IP:$PORT:/bench/src_$shape" "$dest" --yes ${flag:+$flag})
    elif [[ "$direction/$role" == mw/source_init ]]; then
        client_abs="$qerr"; client_rel="client/$rid.err"
        result=$(q_client_run "$state" "$run_id" "$qerr" \
            copy "$(q_source_path "$shape")" "$WIN_IP:$PORT:/bench/rigw-sessions/$SESSION_TAG/$rid/container/" --yes ${flag:+$flag})
    elif [[ "$direction/$role" == mw/destination_init ]]; then
        windows_client=1; client_abs="$werr"; client_rel="client/$rid.windows.err"
        result=$(win_client_run "$state" "$run_id" "$remote_err" \
            "$Q_IP:$PORT:/bench/src_$shape" "$dest" "$flag")
    else
        session_void "unregistered arm $direction/$role"
    fi

    # Anchor all successful post-client work to the same q monotonic instant,
    # regardless of which endpoint ran the client.  Clock probes consume the
    # fixed 250 ms budget rather than adding role-dependent time ahead of it.
    client_done_ns=$(q_monotonic_ns)
    settle_deadline_ns=$((client_done_ns + SETTLE_NS))

    IFS='|' read -r _ transfer_ms rc <<<"$result"
    if [[ ! "$transfer_ms" =~ ^[0-9]+$ || ! "$rc" =~ ^[0-9]+$ ]]; then
        [[ "$windows_client" == 0 ]] || fetch_win_file "$remote_err" "$werr"
        session_void "$rid timer/client sentinel malformed: '$result'"
    fi
    if [[ "$rc" != 0 ]]; then
        # A failed remote client is already SESSION-VOID.  Preserve its log
        # now because teardown removes the remote session tree.
        [[ "$windows_client" == 0 ]] || fetch_win_file "$remote_err" "$werr"
        session_void "$rid client failed rc=$rc (see $client_rel)"
    fi

    record_clock_samples "$block" "$run_id" "$cell" "$pair" "$role" after
    settle_done_ns=$(settle_until_deadline "$settle_deadline_ns")
    [[ "$settle_done_ns" =~ ^[0-9]+$ && "$settle_done_ns" -ge "$settle_deadline_ns" ]] \
        || session_void "$rid absolute post-client settle returned early: '$settle_done_ns'"
    settled_ms=$(((settle_done_ns - client_done_ns) / 1000000))
    [[ "$settled_ms" -ge "$SETTLE_MIN_MS" && "$settled_ms" -lt "$SETTLE_MAX_MS" ]] \
        || session_void "$rid post-client settle was ${settled_ms}ms, expected [$SETTLE_MIN_MS,$SETTLE_MAX_MS)"

    # The destination OS—not the initiator role—selects the durability and
    # landed-tree probe.  This remains outside transfer_ms.
    if [[ "$direction" == wm ]]; then
        flush_out=$(flush_verify_q "$dest") || session_void "$rid q durability probe failed"
        rm -rf "$Q_MODULE/rigw-sessions/$SESSION_TAG/$rid"
    else
        flush_out=$(flush_verify_win "$dest") || session_void "$rid Windows durability probe failed"
        wssh "Remove-Item -LiteralPath '$WIN_MODULE/rigw-sessions/$SESSION_TAG/$rid' -Recurse -Force -ErrorAction SilentlyContinue"
    fi
    IFS='|' read -r _ flush_ms count bytes <<<"$flush_out"
    case "$shape" in mixed) want='5001|547110912';; large) want='1|1073741824';; esac
    [[ "$count|$bytes" == "$want" ]] \
        || session_void "$rid landed $count files/$bytes bytes, expected $want"
    [[ "$flush_ms" =~ ^[0-9]+$ ]] || session_void "$rid flush timer malformed: '$flush_out'"
    arm_phase=durability_verified

    if [[ "$windows_client" == 1 ]]; then
        fetch_successful_windows_client_log "$arm_phase" "$remote_err" "$werr"
    fi

    session_id=$(session_id_from_log "$client_abs") \
        || session_void "$rid client trace is malformed"
    if [[ "$state" == on && "$carrier" == tcp ]]; then
        [[ "$session_id" =~ ^[0-9a-f]{16}$ ]] \
            || session_void "$rid trace-on TCP client has session_id '$session_id'"
    else
        [[ -z "$session_id" ]] \
            || session_void "$rid emitted TCP phase trace in state=$state carrier=$carrier"
    fi

    total=$((transfer_ms + flush_ms))
    printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
        "$block" "$state" "$pass" "$cell" "$role" "$pair" "$role_order" \
        "$transfer_ms" "$settled_ms" "$flush_ms" "$total" "$rc" "$drain" yes \
        "$run_id" "$session_id" "$client_rel" >> "$RUNS_CSV"
    log "$rid: transfer=${transfer_ms}ms settled=${settled_ms}ms flush=${flush_ms}ms total=${total}ms session=${session_id:-none}"
}

cell_order() {
    local pass="$1" round="$2"
    local forward='wm_tcp_mixed mw_tcp_mixed wm_grpc_mixed wm_tcp_large'
    local reverse='wm_tcp_large wm_grpc_mixed mw_tcp_mixed wm_tcp_mixed'
    local base
    [[ "$pass" == forward ]] && base="$forward" || base="$reverse"
    case "$round" in 1|4) printf '%s\n' "$base";; 2|3) [[ "$base" == "$forward" ]] && printf '%s\n' "$reverse" || printf '%s\n' "$forward";; esac
}

run_block() {
    local block="$1" state="$2" pass="$3" first="$4" last="$5" run_id="${SESSION_TAG}-b${block}-${state}"
    local round pair cells cell first_role second_role
    q_quiet_gate; win_quiet_gate
    start_daemons "$block" "$state" "$run_id"
    for ((round=1; round<=PAIRS_PER_BLOCK; round++)); do
        pair=$((first + round - 1))
        [[ "$pair" -le "$last" ]] || session_void "block $block pair schedule overflow"
        q_quiet_gate
        case "$round" in
            1|4) first_role=source_init; second_role=destination_init;;
            2|3) first_role=destination_init; second_role=source_init;;
        esac
        cells=$(cell_order "$pass" "$round")
        local old_ifs="$IFS"; IFS=' '
        for cell in $cells; do
            IFS="$old_ifs"
            run_arm "$block" "$state" "$pass" "$run_id" "$cell" "$pair" "$first_role" 1
            run_arm "$block" "$state" "$pass" "$run_id" "$cell" "$pair" "$second_role" 2
            IFS=' '
        done
        IFS="$old_ifs"
    done
    stop_daemons "$block"
    q_quiet_gate; win_quiet_gate
}

end_gate() {
    q_topology_gate
    win_topology_gate
    mss_gate
    q_quiet_gate
    win_quiet_gate
    ports_closed || session_void "end gate found a listener on port $PORT"
}

cleanup_session_paths() {
    rm -rf "$Q_MODULE/rigw-sessions/$SESSION_TAG" 2>/dev/null || true
    wssh "Remove-Item -LiteralPath '$WIN_MODULE/rigw-sessions/$SESSION_TAG','$WIN_SESSION' -Recurse -Force -ErrorAction SilentlyContinue" \
        >/dev/null 2>&1 || true
}

on_exit() {
    local rc=$?
    trap - EXIT
    set +e
    CLEANUP_MODE=1
    if [[ -n "$win_daemon_pid" || -n "$win_cmd_pid" || -n "$current_block" ]]; then
        win_daemon_stop || rc=1
    fi
    if [[ -n "$q_daemon_pid" ]]; then q_daemon_stop || rc=1; fi
    cleanup_session_paths
    if [[ -n "$CLEANUP_ERROR" ]]; then
        printf '%s\n' "$CLEANUP_ERROR" > "$OUT_DIR/SESSION-VOID"
        rc=1
    fi
    if [[ $rc -ne 0 && ! -f "$OUT_DIR/SESSION-VOID" ]]; then
        printf '%s\n' "${LAST_ERROR:-unexpected harness failure rc=$rc}" > "$OUT_DIR/SESSION-VOID"
    fi
    exit "$rc"
}

main() {
    if [[ "$SELFTEST" == 1 ]]; then selftest; return; fi
    trap on_exit EXIT
    preflight
    if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
        log "PREFLIGHT_ONLY: no daemon started and no transfer timed"
        return
    fi

    printf '%s\n' 'block,trace_state,pass,cell,role,pair,role_order,transfer_ms,settled_ms,flush_ms,total_ms,exit,drain,valid,run_id,session_id,client_log' > "$RUNS_CSV"
    printf '%s\n' 'block,run_id,cell,pair,role,phase,sample,q_before_ns,windows_ns,q_after_ns,rtt_ns,offset_windows_minus_q_ns' > "$CLOCK_CSV"
    emit_schedule > "$OUT_DIR/schedule.csv"
    wssh "New-Item -ItemType Directory -Force -Path '$WIN_SESSION' | Out-Null"

    local block state pass first last
    while IFS=, read -r block state pass first last; do
        current_block="$block"
        run_block "$block" "$state" "$pass" "$first" "$last"
        current_block=""
    done < <(emit_schedule)

    end_gate
    python3 "$SCRIPT_DIR/otp12pf_rigw_analyze.py" "$OUT_DIR" \
        || session_void "phase/distribution analyzer rejected the session"
    printf '%s\n' "$HEAD_FULL" > "$OUT_DIR/SESSION-COMPLETE"
    log "SESSION COMPLETE: analyzer accepted exact inventory; results in $OUT_DIR"
}

main "$@"
