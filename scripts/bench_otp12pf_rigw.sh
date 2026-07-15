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

LOG="$OUT_DIR/bench.log"
RUNS_CSV="$OUT_DIR/runs.csv"
CLOCK_CSV="$OUT_DIR/clock-samples.csv"

LAST_ERROR=""
OUTPUT_CLAIMED=0
OUTPUT_CLAIM_ERROR=""
log() {
    local line
    line="$(date -u +%H:%M:%SZ) $*"
    if [[ "$OUTPUT_CLAIMED" == 1 ]]; then
        printf '%s\n' "$line" | tee -a "$LOG"
    else
        printf '%s\n' "$line" >&2
    fi
}
die() { LAST_ERROR="$*"; log "FATAL: $*"; exit 1; }
append_void_line() {
    printf '%s\n' "$1" >> "$OUT_DIR/SESSION-VOID"
}
session_void() {
    local reason="$1"
    LAST_ERROR="$reason"
    append_void_line "$reason"
    log "SESSION-VOID: $reason"
    exit 1
}

reserve_evidence_dir() {
    local target="$1" parent
    OUTPUT_CLAIM_ERROR=""
    if [[ -e "$target" || -L "$target" ]]; then
        if [[ -f "$target/SESSION-COMPLETE" ]]; then
            OUTPUT_CLAIM_ERROR="refusing output directory with stale SESSION-COMPLETE: $target"
        elif [[ -f "$target/SESSION-VOID" ]]; then
            OUTPUT_CLAIM_ERROR="refusing output directory with stale SESSION-VOID: $target"
        else
            OUTPUT_CLAIM_ERROR="refusing existing output path (must be fresh): $target"
        fi
        return 1
    fi
    parent=$(dirname "$target")
    mkdir -p "$parent" || {
        OUTPUT_CLAIM_ERROR="cannot create output parent: $parent"
        return 1
    }
    mkdir "$target" || {
        OUTPUT_CLAIM_ERROR="cannot atomically claim output directory: $target"
        return 1
    }
    mkdir "$target/trace" "$target/client" "$target/fixtures" "$target/landed" || {
        OUTPUT_CLAIM_ERROR="cannot initialize output directory: $target"
        rm -rf "$target"
        return 1
    }
}

claim_output_dir() {
    reserve_evidence_dir "$OUT_DIR" || return 1
    OUTPUT_CLAIMED=1
}

SSH_MUX=(-o BatchMode=yes -o ControlMaster=auto \
    -o ConnectTimeout=5 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 \
    -o "ControlPath=$HOME/.ssh/cm-rigw-%r@%h-%p" -o ControlPersist=300)
wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }

q_daemon_pid=""
win_daemon_pid=""
win_cmd_pid=""
current_block=""
CLEANUP_MODE=0
CLEANUP_ERROR=""
REGISTERED_RUN_STARTED=0
SESSION_FINALIZED=0
STRICT_CLEANUP_VERIFIED=0
Q_SESSION_MAY_EXIST=0
WIN_SESSION_MAY_EXIST=0
LOCAL_EVIDENCE_COMPLETE=0

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
    local manifest_tmp canonical_manifest landed_manifest tree_digest
    local freshness_tmp freshness_case marker before analyzer_log
    local win_stop_source win_start_source finalize_tmp failure_tmp trap_calls trap_rc
    local signal signal_dir signal_rc contract_tmp on_exit_source append_tmp
    local cleanup_tmp remembered port_checks strict_cleanup_source
    local destination_tmp prepare_destination_source
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

    win_stop_source=$(declare -f win_daemon_stop)
    win_start_source=$(declare -f win_daemon_start)
    python3 - "$win_stop_source" "$win_start_source" <<'PY' \
        || die "Windows launcher/daemon identity contract changed"
import sys

stop, start = sys.argv[1:]
stop_markers = (
    r"\$c.Name -ine 'cmd.exe'",
    r"\$actualLauncher -ine \$expectedLauncher",
    r"\$d.ParentProcessId -ne \$cmd0",
    r"Stop-Process -Id \$pid0",
    r"Stop-Process -Id \$cmd0",
)
try:
    positions = [stop.index(marker) for marker in stop_markers]
except ValueError as exc:
    raise SystemExit(f"missing exact stop identity marker: {exc}") from exc
if max(positions[:3]) >= min(positions[3:]):
    raise SystemExit("a Windows process can be stopped before all identities validate")
for marker in (
    r"\$actualLauncher -ine \$launcherCommand",
    r"\$actualDaemon -ine '$WIN_ACTIVE'",
    r"\$d.ParentProcessId -ne \$r.ProcessId",
):
    if marker not in start:
        raise SystemExit(f"missing start identity marker: {marker}")
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

    manifest_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-manifest.XXXXXX")
    mkdir -p "$manifest_tmp/source/sub" "$manifest_tmp/container/src_mixed/sub"
    printf 'a' > "$manifest_tmp/source/a"
    printf 'bc' > "$manifest_tmp/source/sub/b"
    printf 'a' > "$manifest_tmp/container/src_mixed/a"
    printf 'bc' > "$manifest_tmp/container/src_mixed/sub/b"
    canonical_manifest="$manifest_tmp/canonical.manifest"
    landed_manifest="$manifest_tmp/landed.manifest"
    write_q_tree_manifest "$manifest_tmp/source" "$canonical_manifest"
    write_q_tree_manifest \
        "$manifest_tmp/container" "$landed_manifest" src_mixed
    tree_digest=$(matching_manifest_digest "$canonical_manifest" "$landed_manifest") \
        || die "identical relative-path/size manifests did not match"
    [[ "$tree_digest" =~ ^[0-9a-f]{64}$ ]] \
        || die "tree manifest digest is malformed"
    printf 'aa' > "$manifest_tmp/container/src_mixed/a"
    printf 'b' > "$manifest_tmp/container/src_mixed/sub/b"
    write_q_tree_manifest \
        "$manifest_tmp/container" "$landed_manifest" src_mixed
    if matching_manifest_digest "$canonical_manifest" "$landed_manifest" >/dev/null; then
        rm -rf "$manifest_tmp"
        die "same-count/same-byte tree with swapped file sizes was accepted"
    fi
    rm -rf "$manifest_tmp/container/src_mixed"
    mkdir -p "$manifest_tmp/container/wrapper/src_mixed"
    if write_q_tree_manifest \
        "$manifest_tmp/container" "$landed_manifest" src_mixed 2>/dev/null; then
        rm -rf "$manifest_tmp"
        die "wrong landed root wrapper was accepted"
    fi
    rm -rf "$manifest_tmp"

    freshness_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-freshness.XXXXXX")
    reserve_evidence_dir "$freshness_tmp/new-evidence" \
        || die "fresh evidence directory was rejected: $OUTPUT_CLAIM_ERROR"
    for marker in SESSION-COMPLETE SESSION-VOID unrelated.txt; do
        freshness_case="$freshness_tmp/$marker"
        mkdir "$freshness_case"
        printf 'preserve-me\n' > "$freshness_case/$marker"
        before=$(sha256_q "$freshness_case/$marker")
        if reserve_evidence_dir "$freshness_case"; then
            rm -rf "$freshness_tmp"
            die "stale output directory containing $marker was accepted"
        fi
        [[ "$(sha256_q "$freshness_case/$marker")" == "$before" ]] \
            || die "stale output rejection modified $marker"
    done
    rm -rf "$freshness_tmp"

    destination_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-destination.XXXXXX")
    mkdir -p "$destination_tmp/container/src_mixed"
    printf 'stale\n' > "$destination_tmp/container/src_mixed/stale"
    (
        rm() { return 73; }
        if prepare_destination wm "$destination_tmp/container"; then
            die "q destination reset masked a failed removal"
        fi
    )
    [[ "$(< "$destination_tmp/container/src_mixed/stale")" == stale ]] \
        || die "failed q destination reset modified retained evidence"
    prepare_destination wm "$destination_tmp/container" \
        || die "q destination reset rejected a removable tree"
    [[ -d "$destination_tmp/container" && ! -L "$destination_tmp/container" ]]
    [[ -z "$(find "$destination_tmp/container" -mindepth 1 -maxdepth 1 -print -quit)" ]] \
        || die "q destination reset left stale content"
    rm -rf "$destination_tmp"

    prepare_destination_source=$(declare -f prepare_destination)
    python3 - "$prepare_destination_source" <<'PY' \
        || die "Windows destination reset source contract changed"
import sys

source = sys.argv[1]
for marker in (
    r"\$ErrorActionPreference = 'Stop'",
    r"Remove-Item -LiteralPath '$dest' -Recurse -Force -ErrorAction Stop",
    r"Test-Path -LiteralPath '$dest' -PathType Container",
    r"Get-ChildItem -LiteralPath '$dest' -Force -ErrorAction Stop",
    'ReparsePoint',
):
    if marker not in source:
        raise SystemExit(f"missing Windows destination reset marker: {marker}")
windows = source.split('else', 1)[1]
if 'SilentlyContinue' in windows:
    raise SystemExit("Windows destination reset suppresses removal errors")
PY

    finalize_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-finalize.XXXXXX")
    (
        OUT_DIR="$finalize_tmp/fails"
        mkdir "$OUT_DIR"
        HEAD_FULL=0123456789abcdef
        LOCAL_EVIDENCE_COMPLETE=1
        strict_success_cleanup() { return 1; }
        if finalize_registered_session; then
            die "registered finalization accepted failed strict cleanup"
        fi
        [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]] \
            || die "failed strict cleanup left SESSION-COMPLETE"
    )
    (
        OUT_DIR="$finalize_tmp/incomplete-local"
        mkdir "$OUT_DIR"
        HEAD_FULL=0123456789abcdef
        LOCAL_EVIDENCE_COMPLETE=0
        strict_success_cleanup() {
            die "finalization cleaned paths before local evidence was complete"
        }
        if finalize_registered_session; then
            die "registered finalization accepted incomplete local evidence"
        fi
        [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]]
    )
    (
        OUT_DIR="$finalize_tmp/succeeds"
        mkdir "$OUT_DIR"
        HEAD_FULL=0123456789abcdef
        LOCAL_EVIDENCE_COMPLETE=1
        strict_success_cleanup() {
            [[ ! -e "$OUT_DIR/SESSION-COMPLETE" ]] || return 1
            STRICT_CLEANUP_VERIFIED=1
        }
        finalize_registered_session \
            || die "registered finalization rejected verified strict cleanup"
        [[ "$SESSION_FINALIZED" == 1 ]]
        [[ "$(< "$OUT_DIR/SESSION-COMPLETE")" == "$HEAD_FULL" ]]
    )

    cleanup_tmp="$finalize_tmp/strict"
    mkdir -p "$cleanup_tmp/q/rigw-sessions/fail-remote"
    printf 'retain me\n' > "$cleanup_tmp/q/rigw-sessions/fail-remote/sentinel"
    (
        Q_MODULE="$cleanup_tmp/q"
        SESSION_TAG=fail-remote
        Q_SESSION_MAY_EXIST=1
        WIN_SESSION_MAY_EXIST=1
        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
        ports_closed() { return 0; }
        wssh() { return 1; }
        if strict_success_cleanup; then
            die "strict cleanup accepted a Windows deletion failure"
        fi
        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]]
        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]] \
            || die "Windows cleanup failure deleted q evidence first"
        [[ "$(< "$Q_MODULE/rigw-sessions/$SESSION_TAG/sentinel")" == 'retain me' ]] \
            || die "Windows cleanup failure modified q evidence"
    )
    mkdir -p "$cleanup_tmp/q/rigw-sessions/open-port"
    (
        Q_MODULE="$cleanup_tmp/q"
        SESSION_TAG=open-port
        Q_SESSION_MAY_EXIST=1
        WIN_SESSION_MAY_EXIST=1
        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
        ports_closed() { return 1; }
        wssh() { die "strict cleanup reached deletion with an open port"; }
        if strict_success_cleanup; then
            die "strict cleanup accepted an open port"
        fi
        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]]
        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
    )
    mkdir -p "$cleanup_tmp/q/rigw-sessions/surviving-q"
    (
        Q_MODULE="$cleanup_tmp/q"
        SESSION_TAG=surviving-q
        Q_SESSION_MAY_EXIST=1
        WIN_SESSION_MAY_EXIST=1
        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
        ports_closed() { return 0; }
        wssh() { return 0; }
        rm() { return 0; }
        if strict_success_cleanup; then
            die "strict cleanup accepted a surviving q session tree"
        fi
        [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]]
        [[ -d "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
    )
    mkdir -p "$cleanup_tmp/q/rigw-sessions/succeeds"
    (
        Q_MODULE="$cleanup_tmp/q"
        SESSION_TAG=succeeds
        Q_SESSION_MAY_EXIST=1
        WIN_SESSION_MAY_EXIST=1
        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
        port_checks=0
        ports_closed() { port_checks=$((port_checks + 1)); return 0; }
        wssh() { return 0; }
        strict_success_cleanup || die "strict cleanup rejected a clean session"
        [[ "$STRICT_CLEANUP_VERIFIED" == 1 ]]
        [[ "$port_checks" == 2 ]] || die "strict cleanup ran $port_checks port checks"
        [[ "$Q_SESSION_MAY_EXIST" == 0 && "$WIN_SESSION_MAY_EXIST" == 0 ]]
        [[ ! -e "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]]
    )
    mkdir -p "$cleanup_tmp/q/rigw-sessions/late-port"
    (
        Q_MODULE="$cleanup_tmp/q"
        SESSION_TAG=late-port
        Q_SESSION_MAY_EXIST=1
        WIN_SESSION_MAY_EXIST=1
        q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
        port_checks=0
        ports_closed() {
            port_checks=$((port_checks + 1))
            [[ "$port_checks" == 1 ]]
        }
        wssh() { return 0; }
        if strict_success_cleanup; then
            die "strict cleanup accepted a listener appearing during deletion"
        fi
        [[ "$STRICT_CLEANUP_VERIFIED" == 0 && "$port_checks" == 2 ]]
    )
    for remembered in q daemon launcher block; do
        (
            Q_MODULE="$cleanup_tmp/q"
            SESSION_TAG="remembered-$remembered"
            q_daemon_pid=""; win_daemon_pid=""; win_cmd_pid=""; current_block=""
            case "$remembered" in
                q) q_daemon_pid=11;;
                daemon) win_daemon_pid=22;;
                launcher) win_cmd_pid=33;;
                block) current_block=4;;
            esac
            ports_closed() { die "strict cleanup ignored remembered $remembered state"; }
            if strict_success_cleanup; then
                die "strict cleanup accepted remembered $remembered state"
            fi
            [[ "$STRICT_CLEANUP_VERIFIED" == 0 ]]
        )
    done
    strict_cleanup_source=$(declare -f strict_success_cleanup)
    python3 - "$strict_cleanup_source" <<'PY' \
        || die "strict cleanup source contract changed"
import sys

source = sys.argv[1]
for marker in (
    "'$WIN_MODULE/rigw-sessions/$SESSION_TAG'",
    "'$WIN_SESSION'",
    r"Remove-Item -LiteralPath \$path -Recurse -Force -ErrorAction Stop",
    r'if (Test-Path -LiteralPath \$path) { throw',
):
    if marker not in source:
        raise SystemExit(f"missing strict Windows cleanup marker: {marker}")
if source.count('ports_closed') != 2:
    raise SystemExit("strict cleanup must check closed ports before and after deletion")
if source.index('ports_closed') > source.index('Remove-Item -LiteralPath'):
    raise SystemExit("strict cleanup deletes evidence before its first port check")
if source.rindex('ports_closed') < source.index('rm -rf --'):
    raise SystemExit("strict cleanup lacks a post-deletion port check")
PY
    rm -rf "$finalize_tmp"

    failure_tmp=$(mktemp -d "${TMPDIR:-/tmp}/blit-rigw-failure.XXXXXX")
    trap_calls="$failure_tmp/remote-calls"
    mkdir -p "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG"
    printf 'retain me\n' > "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG/sentinel"
    set +e
    (
        set +e
        OUT_DIR="$failure_tmp/evidence"
        mkdir "$OUT_DIR"
        LOG="$OUT_DIR/bench.log"
        OUTPUT_CLAIMED=1
        printf 'primary failure\n' > "$OUT_DIR/SESSION-VOID"
        printf 'must disappear\n' > "$OUT_DIR/SESSION-COMPLETE"
        printf 'must disappear\n' > "$OUT_DIR/SESSION-COMPLETE.tmp"
        REGISTERED_RUN_STARTED=1
        SESSION_FINALIZED=0
        STRICT_CLEANUP_VERIFIED=0
        Q_SESSION_MAY_EXIST=1
        WIN_SESSION_MAY_EXIST=1
        Q_MODULE="$failure_tmp/q-module"
        current_block=1
        q_daemon_pid=""
        win_daemon_pid=""
        win_cmd_pid=""
        wssh() {
            printf '%s\n' "$*" >> "$trap_calls"
            return 1
        }
        false
        on_exit
    )
    trap_rc=$?
    set -e
    [[ "$trap_rc" == 1 ]] || die "failure trap returned $trap_rc, expected 1"
    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE" ]]
    [[ ! -e "$failure_tmp/evidence/SESSION-COMPLETE.tmp" ]]
    grep -Fxq 'primary failure' "$failure_tmp/evidence/SESSION-VOID" \
        || die "failure trap discarded the primary reason"
    grep -Fq 'cleanup errors: Windows PID recovery failed' "$failure_tmp/evidence/SESSION-VOID" \
        || die "failure trap omitted its cleanup error"
    grep -Fq "q session evidence may remain; inspect $failure_tmp/q-module/rigw-sessions/$SESSION_TAG" \
        "$failure_tmp/evidence/SESSION-VOID" \
        || die "failure trap omitted the q evidence path"
    grep -Fq "Windows evidence may remain; inspect $WIN_SESSION and $WIN_MODULE/rigw-sessions/$SESSION_TAG" \
        "$failure_tmp/evidence/SESSION-VOID" \
        || die "failure trap omitted the Windows evidence path"
    if grep -Fq 'Remove-Item' "$trap_calls"; then
        die "failure trap issued destructive remote cleanup"
    fi
    [[ "$(< "$failure_tmp/q-module/rigw-sessions/$SESSION_TAG/sentinel")" == 'retain me' ]] \
        || die "failure trap modified q session evidence"
    on_exit_source=$(declare -f on_exit)
    if [[ "$on_exit_source" == *'rm -rf'* \
        || "$on_exit_source" == *'Remove-Item'* \
        || "$on_exit_source" == *'strict_success_cleanup'* ]]; then
        die "failure trap contains a destructive session-cleanup path"
    fi

    append_tmp="$failure_tmp/append-contract"
    mkdir "$append_tmp"
    printf 'original reason\n' > "$append_tmp/SESSION-VOID"
    set +e
    (
        OUT_DIR="$append_tmp"
        LOG="$OUT_DIR/bench.log"
        OUTPUT_CLAIMED=1
        session_void 'later context'
    ) >/dev/null 2>&1
    trap_rc=$?
    set -e
    [[ "$trap_rc" == 1 ]] || die "session_void append probe returned $trap_rc"
    [[ "$(< "$append_tmp/SESSION-VOID")" == $'original reason\nlater context' ]] \
        || die "session_void overwrote an earlier failure reason"

    contract_tmp="$failure_tmp/exit-contract"
    mkdir "$contract_tmp"
    set +e
    (
        set +e
        OUT_DIR="$contract_tmp"
        LOG="$OUT_DIR/bench.log"
        OUTPUT_CLAIMED=1
        REGISTERED_RUN_STARTED=1
        SESSION_FINALIZED=0
        STRICT_CLEANUP_VERIFIED=0
        WIN_SESSION_MAY_EXIST=0
        true
        on_exit
    )
    trap_rc=$?
    set -e
    [[ "$trap_rc" == 1 ]] \
        || die "unfinalized registered zero-exit returned $trap_rc"
    grep -Fq 'registered run returned without finalizing the session' \
        "$contract_tmp/SESSION-VOID" \
        || die "unfinalized registered zero-exit omitted its reason"

    contract_tmp="$failure_tmp/marker-contract"
    mkdir "$contract_tmp"
    set +e
    (
        set +e
        OUT_DIR="$contract_tmp"
        LOG="$OUT_DIR/bench.log"
        OUTPUT_CLAIMED=1
        REGISTERED_RUN_STARTED=1
        SESSION_FINALIZED=1
        STRICT_CLEANUP_VERIFIED=1
        LOCAL_EVIDENCE_COMPLETE=1
        HEAD_FULL=0123456789abcdef
        true
        on_exit
    )
    trap_rc=$?
    set -e
    [[ "$trap_rc" == 1 ]] \
        || die "finalized flags without a completion marker returned $trap_rc"
    grep -Fq 'registered completion marker is absent or invalid' \
        "$contract_tmp/SESSION-VOID" \
        || die "missing registered completion marker omitted its reason"

    contract_tmp="$failure_tmp/wrong-marker-contract"
    mkdir "$contract_tmp"
    printf 'wrong-build\n' > "$contract_tmp/SESSION-COMPLETE"
    set +e
    (
        set +e
        OUT_DIR="$contract_tmp"
        LOG="$OUT_DIR/bench.log"
        OUTPUT_CLAIMED=1
        REGISTERED_RUN_STARTED=1
        SESSION_FINALIZED=1
        STRICT_CLEANUP_VERIFIED=1
        LOCAL_EVIDENCE_COMPLETE=1
        HEAD_FULL=0123456789abcdef
        true
        on_exit
    )
    trap_rc=$?
    set -e
    [[ "$trap_rc" == 1 ]] \
        || die "wrong completion marker returned $trap_rc"
    [[ ! -e "$contract_tmp/SESSION-COMPLETE" ]] \
        || die "wrong completion marker survived failure handling"

    contract_tmp="$failure_tmp/preflight-contract"
    mkdir "$contract_tmp"
    set +e
    (
        set +e
        OUT_DIR="$contract_tmp"
        LOG="$OUT_DIR/bench.log"
        OUTPUT_CLAIMED=1
        REGISTERED_RUN_STARTED=0
        SESSION_FINALIZED=0
        STRICT_CLEANUP_VERIFIED=0
        true
        on_exit
    )
    trap_rc=$?
    set -e
    [[ "$trap_rc" == 1 ]] \
        || die "unclean preflight zero-exit returned $trap_rc"
    grep -Fq 'successful exit lacked verified strict cleanup' \
        "$contract_tmp/SESSION-VOID" \
        || die "unclean preflight zero-exit omitted its reason"

    contract_tmp="$failure_tmp/preflight-marker-contract"
    mkdir "$contract_tmp"
    printf 'not allowed\n' > "$contract_tmp/SESSION-COMPLETE"
    set +e
    (
        set +e
        OUT_DIR="$contract_tmp"
        LOG="$OUT_DIR/bench.log"
        OUTPUT_CLAIMED=1
        REGISTERED_RUN_STARTED=0
        SESSION_FINALIZED=0
        STRICT_CLEANUP_VERIFIED=1
        true
        on_exit
    )
    trap_rc=$?
    set -e
    [[ "$trap_rc" == 1 ]] \
        || die "preflight completion marker returned $trap_rc"
    [[ ! -e "$contract_tmp/SESSION-COMPLETE" ]] \
        || die "preflight completion marker survived failure handling"

    for marker in SESSION-VOID SESSION-COMPLETE.tmp; do
        contract_tmp="$failure_tmp/preflight-$marker-contract"
        mkdir "$contract_tmp"
        printf 'not allowed\n' > "$contract_tmp/$marker"
        set +e
        (
            set +e
            OUT_DIR="$contract_tmp"
            LOG="$OUT_DIR/bench.log"
            OUTPUT_CLAIMED=1
            REGISTERED_RUN_STARTED=0
            SESSION_FINALIZED=0
            STRICT_CLEANUP_VERIFIED=1
            true
            on_exit
        )
        trap_rc=$?
        set -e
        [[ "$trap_rc" == 1 ]] \
            || die "preflight $marker returned $trap_rc"
        if [[ "$marker" == SESSION-VOID ]]; then
            [[ "$(sed -n '1p' "$contract_tmp/SESSION-VOID")" == 'not allowed' ]] \
                || die "preflight VOID rejection replaced its primary reason"
        else
            grep -Fq 'successful exit retained a failure or temporary marker' \
                "$contract_tmp/SESSION-VOID" \
                || die "preflight $marker omitted its rejection reason"
        fi
    done

    for signal in HUP INT TERM; do
        signal_dir="$failure_tmp/signal-$signal"
        mkdir "$signal_dir"
        set +e
        bash -c '
set -Eeuo pipefail
source "$1"
OUT_DIR="$2"
LOG="$OUT_DIR/bench.log"
OUTPUT_CLAIMED=1
REGISTERED_RUN_STARTED=1
SESSION_FINALIZED=0
STRICT_CLEANUP_VERIFIED=0
Q_SESSION_MAY_EXIST=1
WIN_SESSION_MAY_EXIST=1
current_block=1
q_daemon_pid=111
win_daemon_pid=222
win_cmd_pid=333
win_daemon_stop() {
    printf "windows\n" >> "$OUT_DIR/stops"
    win_daemon_pid=""; win_cmd_pid=""; current_block=""
}
q_daemon_stop() {
    printf "q\n" >> "$OUT_DIR/stops"
    q_daemon_pid=""
}
trap on_exit EXIT
install_signal_traps
kill -s "$3" "$$"
sleep 2
exit 99
' _ "$SCRIPT_DIR/bench_otp12pf_rigw.sh" "$signal_dir" "$signal"
        signal_rc=$?
        set -e
        [[ "$signal_rc" == 1 ]] \
            || die "$signal cleanup returned $signal_rc, expected 1"
        grep -Fxq "received $signal" "$signal_dir/SESSION-VOID" \
            || die "$signal cleanup omitted its signal reason"
        [[ "$(LC_ALL=C sort "$signal_dir/stops")" == $'q\nwindows' ]] \
            || die "$signal cleanup did not invoke both exact-owned teardown paths"
        [[ ! -e "$signal_dir/SESSION-COMPLETE" ]]
    done
    rm -rf "$failure_tmp"

    analyzer_log=$(mktemp "${TMPDIR:-/tmp}/blit-rigw-analyzer.XXXXXX")
    if ! python3 "$SCRIPT_DIR/otp12pf_rigw_analyze_test.py" \
        > "$analyzer_log" 2>&1; then
        cat "$analyzer_log" >&2
        rm -f "$analyzer_log"
        die "analyzer self-tests failed"
    fi
    rm -f "$analyzer_log"
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

write_q_tree_manifest() {
    python3 - "$1" "$2" "${3:-}" <<'PY'
import base64, os, pathlib, stat, sys

root = pathlib.Path(sys.argv[1])
output = pathlib.Path(sys.argv[2])
expected_root = sys.argv[3]
if not root.is_dir() or root.is_symlink():
    raise SystemExit(f"manifest root is not a plain directory: {root}")
if expected_root:
    entries = list(root.iterdir())
    if (
        len(entries) != 1
        or entries[0].name != expected_root
        or not entries[0].is_dir()
        or entries[0].is_symlink()
    ):
        raise SystemExit(
            f"landed container must contain exactly plain directory {expected_root}"
        )
    root = entries[0]

lines = []
def walk_error(error):
    raise error

for current, dirs, files in os.walk(root, followlinks=False, onerror=walk_error):
    for name in dirs:
        path = pathlib.Path(current, name)
        mode = path.lstat().st_mode
        if not stat.S_ISDIR(mode) or stat.S_ISLNK(mode):
            raise SystemExit(f"non-directory/reparse entry in manifest: {path}")
    for name in files:
        path = pathlib.Path(current, name)
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode):
            raise SystemExit(f"non-regular entry in manifest: {path}")
        relative = path.relative_to(root).as_posix()
        encoded = base64.b64encode(relative.encode("utf-8")).decode("ascii")
        lines.append(f"{encoded},{info.st_size}")
lines.sort()
output.write_text("".join(f"{line}\n" for line in lines), encoding="ascii")
PY
}

write_win_tree_manifest() {
    local root="$1" remote_out="$2" local_out="$3" expected_root="${4:-}"
    wssh "
\$ErrorActionPreference = 'Stop'
\$root = (Resolve-Path -LiteralPath '$root').Path.TrimEnd([char]92,[char]47)
if ('$expected_root') {
  \$entries = @(Get-ChildItem -LiteralPath \$root -Force -ErrorAction Stop)
  if (\$entries.Count -ne 1 -or -not \$entries[0].PSIsContainer -or \$entries[0].Name -cne '$expected_root' -or ((\$entries[0].Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) { throw 'landed root layout mismatch' }
  \$root = \$entries[0].FullName.TrimEnd([char]92,[char]47)
}
\$lines = [Collections.Generic.List[string]]::new()
foreach (\$item in @(Get-ChildItem -LiteralPath \$root -Recurse -Force -ErrorAction Stop)) {
  if ((\$item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw \"reparse entry in manifest: \$(\$item.FullName)\" }
  if (\$item.PSIsContainer) { continue }
  if (-not (\$item -is [IO.FileInfo])) { throw \"non-regular entry in manifest: \$(\$item.FullName)\" }
  \$rel = \$item.FullName.Substring(\$root.Length).TrimStart([char]92,[char]47).Replace([char]92,[char]47)
  \$b64 = [Convert]::ToBase64String([Text.UTF8Encoding]::new(\$false,\$true).GetBytes(\$rel))
  \$lines.Add(\"\$b64,\$([uint64]\$item.Length)\")
}
\$ordered = [string[]]\$lines.ToArray()
[Array]::Sort(\$ordered, [StringComparer]::Ordinal)
\$text = if (\$ordered.Count) { (\$ordered -join \"`n\") + \"`n\" } else { '' }
[IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName('$remote_out')) | Out-Null
[IO.File]::WriteAllText('$remote_out', \$text, [Text.UTF8Encoding]::new(\$false))
" || return 1
    fetch_win_file "$remote_out" "$local_out" || return 1
    LC_ALL=C sort -o "$local_out" "$local_out"
}

matching_manifest_digest() {
    local canonical="$1" landed="$2"
    cmp -s "$canonical" "$landed" || return 1
    sha256_q "$landed"
}

verify_fixtures() {
    local shape want qgot wgot qmanifest wmanifest qhash
    printf '%s\n' 'shape,sha256,q_manifest,windows_manifest' \
        > "$OUT_DIR/fixture-manifests.csv"
    WIN_SESSION_MAY_EXIST=1
    wssh "New-Item -ItemType Directory -Force -Path '$WIN_SESSION/fixtures' | Out-Null" \
        || die "cannot create Windows fixture evidence directory"
    for shape in mixed large; do
        case "$shape" in
            mixed) want=5001,547110912;;
            large) want=1,1073741824;;
        esac
        qgot=$(fixture_shape_q "$(q_source_path "$shape")")
        wgot=$(fixture_shape_win "$(win_source_path "$shape")")
        [[ "$qgot" == "$want" ]] || die "q src_$shape is $qgot, expected $want"
        [[ "$wgot" == "$want" ]] || die "Windows canonical src_$shape is $wgot, expected $want"
        qmanifest="$OUT_DIR/fixtures/src_$shape.manifest"
        wmanifest="$OUT_DIR/fixtures/windows-src_$shape.manifest"
        write_q_tree_manifest "$(q_source_path "$shape")" "$qmanifest" \
            || die "q src_$shape manifest failed"
        write_win_tree_manifest \
            "$(win_source_path "$shape")" \
            "$WIN_SESSION/fixtures/src_$shape.manifest" "$wmanifest" \
            || die "Windows src_$shape manifest failed"
        qhash=$(matching_manifest_digest "$qmanifest" "$wmanifest") \
            || die "q and Windows src_$shape relative-path/size manifests differ"
        printf '%s,%s,%s,%s\n' \
            "$shape" "$qhash" "fixtures/src_$shape.manifest" \
            "fixtures/windows-src_$shape.manifest" \
            >> "$OUT_DIR/fixture-manifests.csv"
    done
    log "canonical fixtures verified byte-for-byte by relative path and size on both hosts"
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
        if ! pid_probe=$(wssh "
\$d = Get-Content -LiteralPath '$WIN_SESSION/block_$current_block/daemon.pid' -ErrorAction SilentlyContinue
\$c = Get-Content -LiteralPath '$WIN_SESSION/block_$current_block/launcher.pid' -ErrorAction SilentlyContinue
\"P|\$c|\$d\"
" 2>/dev/null | tr -d '\r' | tail -1); then
            teardown_die "Windows PID recovery failed for block $current_block"
            return 1
        fi
        IFS='|' read -r _ cmdpid pid <<<"$pid_probe"
    fi
    if [[ -z "$pid" && -z "$cmdpid" ]]; then
        if [[ -n "$current_block" ]] && ! wssh \
            "if (Get-NetTCPConnection -State Listen -LocalPort $PORT -ErrorAction SilentlyContinue) { exit 9 }" \
            >/dev/null 2>&1; then
            teardown_die "Windows PID files are empty but port $PORT may still be open"
            return 1
        fi
        return 0
    fi
    [[ -z "$pid" || "$pid" =~ ^[0-9]+$ ]] \
        || { teardown_die "invalid remembered Windows daemon PID '$pid'"; return 1; }
    [[ -z "$cmdpid" || "$cmdpid" =~ ^[0-9]+$ ]] \
        || { teardown_die "invalid remembered Windows launcher PID '$cmdpid'"; return 1; }
    [[ -n "$current_block" ]] \
        || { teardown_die "cannot verify Windows launcher without a current block"; return 1; }
    out=$(wssh "
\$ErrorActionPreference = 'Stop'
\$pid0 = if ('$pid' -match '^[0-9]+$') { [int]'$pid' } else { \$null }
\$cmd0 = if ('$cmdpid' -match '^[0-9]+$') { [int]'$cmdpid' } else { \$null }
\$expectedLauncher = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$current_block/start.cmd\"\"'
\$d = if (\$pid0) { Get-CimInstance Win32_Process -Filter \"ProcessId=\$pid0\" -ErrorAction SilentlyContinue } else { \$null }
\$c = if (\$cmd0) { Get-CimInstance Win32_Process -Filter \"ProcessId=\$cmd0\" -ErrorAction SilentlyContinue } else { \$null }
if (\$d) {
  \$actual = if (\$d.ExecutablePath) { \$d.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  if (\$d.Name -ine 'blit-daemon.exe' -or \$actual -ine '$WIN_ACTIVE') { throw \"daemon PID identity mismatch: \$(\$d.Name) \$(\$d.ExecutablePath)\" }
  if (\$c -and \$d.ParentProcessId -ne \$cmd0) { throw \"daemon parent mismatch: \$(\$d.ParentProcessId) != \$cmd0\" }
}
if (\$c) {
  \$actualLauncher = if (\$c.CommandLine) { \$c.CommandLine.Replace([char]92,[char]47).Trim() } else { '' }
  if (\$c.Name -ine 'cmd.exe' -or \$actualLauncher -ine \$expectedLauncher) { throw \"launcher command mismatch: \$(\$c.Name) \$actualLauncher\" }
}
# Every identity is validated before either remembered PID is stopped.
if (\$pid0) {
  if (\$d) {
    Stop-Process -Id \$pid0 -Force
  }
}
if (\$c) { Stop-Process -Id \$cmd0 -Force }
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
\$launcherCommand = 'cmd.exe /d /c \"\"$WIN_SESSION/block_$block/start.cmd\"\"'
\$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = \$launcherCommand }
if (\$r.ReturnValue -ne 0) { throw \"launcher return \$(\$r.ReturnValue)\" }
Set-Content -LiteralPath '$WIN_SESSION/block_$block/launcher.pid' -Value \$r.ProcessId
Start-Sleep -Seconds 1
\$c = Get-CimInstance Win32_Process -Filter \"ProcessId=\$(\$r.ProcessId)\" -ErrorAction SilentlyContinue
\$actualLauncher = if (\$c -and \$c.CommandLine) { \$c.CommandLine.Replace([char]92,[char]47).Trim() } else { '' }
if (-not \$c -or \$c.Name -ine 'cmd.exe' -or \$actualLauncher -ine \$launcherCommand) { throw \"launcher identity mismatch: \$(\$c.Name) \$actualLauncher\" }
\$d = Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object ParentProcessId -eq \$r.ProcessId | Select-Object -First 1
if (-not \$d) { Get-Content -LiteralPath '$WIN_SESSION/block_$block/daemon.err' -ErrorAction SilentlyContinue; throw 'daemon child absent' }
\$actualDaemon = if (\$d.ExecutablePath) { \$d.ExecutablePath.Replace([char]92,[char]47) } else { '' }
if (\$actualDaemon -ine '$WIN_ACTIVE' -or \$d.ParentProcessId -ne \$r.ProcessId) { throw \"daemon identity mismatch: \$(\$d.ExecutablePath) parent=\$(\$d.ParentProcessId)\" }
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
    local direction="$1" dest="$2" first
    if [[ "$direction" == wm ]]; then
        rm -rf -- "$dest" || return 1
        [[ ! -e "$dest" && ! -L "$dest" ]] || return 1
        mkdir -p -- "$dest" || return 1
        [[ -d "$dest" && ! -L "$dest" ]] || return 1
        first=$(find "$dest" -mindepth 1 -maxdepth 1 -print -quit) || return 1
        [[ -z "$first" ]] || return 1
    else
        wssh "
\$ErrorActionPreference = 'Stop'
if (Test-Path -LiteralPath '$dest') {
  Remove-Item -LiteralPath '$dest' -Recurse -Force -ErrorAction Stop
}
if (Test-Path -LiteralPath '$dest') { throw 'destination removal did not land' }
New-Item -ItemType Directory -Force -Path '$dest' -ErrorAction Stop | Out-Null
if (-not (Test-Path -LiteralPath '$dest' -PathType Container)) { throw 'destination is not a directory' }
\$item = Get-Item -LiteralPath '$dest' -Force -ErrorAction Stop
if ((\$item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'destination is a reparse point' }
if (@(Get-ChildItem -LiteralPath '$dest' -Force -ErrorAction Stop).Count -ne 0) { throw 'destination is not empty' }
" || return 1
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
    local landed_root landed_manifest canonical_manifest remote_manifest tree_manifest_sha256
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
        # Fetch this client log opportunistically; the failure trap also keeps
        # the remote session tree intact for postmortem evidence.
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
    landed_root="src_$shape"
    landed_manifest="$OUT_DIR/landed/$rid.manifest"
    canonical_manifest="$OUT_DIR/fixtures/src_$shape.manifest"
    if [[ "$direction" == wm ]]; then
        flush_out=$(flush_verify_q "$dest") || session_void "$rid q durability probe failed"
        write_q_tree_manifest "$dest" "$landed_manifest" "$landed_root" \
            || session_void "$rid q landed root/manifest verification failed"
    else
        flush_out=$(flush_verify_win "$dest") || session_void "$rid Windows durability probe failed"
        remote_manifest="$WIN_SESSION/block_$block/$rid.tree.manifest"
        write_win_tree_manifest \
            "$dest" "$remote_manifest" "$landed_manifest" "$landed_root" \
            || session_void "$rid Windows landed root/manifest verification failed"
    fi
    IFS='|' read -r _ flush_ms count bytes <<<"$flush_out"
    case "$shape" in mixed) want='5001|547110912';; large) want='1|1073741824';; esac
    [[ "$count|$bytes" == "$want" ]] \
        || session_void "$rid landed $count files/$bytes bytes, expected $want"
    [[ "$flush_ms" =~ ^[0-9]+$ ]] || session_void "$rid flush timer malformed: '$flush_out'"
    tree_manifest_sha256=$(matching_manifest_digest \
        "$canonical_manifest" "$landed_manifest") \
        || session_void "$rid landed relative-path/size manifest differs from canonical"
    [[ "$tree_manifest_sha256" =~ ^[0-9a-f]{64}$ ]] \
        || session_void "$rid tree manifest digest is malformed"
    if [[ "$direction" == wm ]]; then
        rm -rf "$Q_MODULE/rigw-sessions/$SESSION_TAG/$rid"
    else
        wssh "Remove-Item -LiteralPath '$WIN_MODULE/rigw-sessions/$SESSION_TAG/$rid' -Recurse -Force -ErrorAction Stop" \
            || session_void "$rid failed to remove verified Windows destination"
    fi
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
    printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
        "$block" "$state" "$pass" "$cell" "$role" "$pair" "$role_order" \
        "$transfer_ms" "$settled_ms" "$flush_ms" "$total" "$landed_root" \
        "$tree_manifest_sha256" "$rc" "$drain" yes "$run_id" "$session_id" \
        "$client_rel" >> "$RUNS_CSV"
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

strict_success_cleanup() {
    STRICT_CLEANUP_VERIFIED=0
    [[ -z "$q_daemon_pid" ]] \
        || { LAST_ERROR="strict cleanup found remembered q daemon PID $q_daemon_pid"; return 1; }
    [[ -z "$win_daemon_pid" ]] \
        || { LAST_ERROR="strict cleanup found remembered Windows daemon PID $win_daemon_pid"; return 1; }
    [[ -z "$win_cmd_pid" ]] \
        || { LAST_ERROR="strict cleanup found remembered Windows launcher PID $win_cmd_pid"; return 1; }
    [[ -z "$current_block" ]] \
        || { LAST_ERROR="strict cleanup found current block $current_block"; return 1; }

    ports_closed \
        || { LAST_ERROR="strict cleanup found port $PORT still listening"; return 1; }
    wssh "
\$ErrorActionPreference = 'Stop'
\$paths = @('$WIN_MODULE/rigw-sessions/$SESSION_TAG', '$WIN_SESSION')
foreach (\$path in \$paths) {
  if (Test-Path -LiteralPath \$path) {
    Remove-Item -LiteralPath \$path -Recurse -Force -ErrorAction Stop
  }
  if (Test-Path -LiteralPath \$path) { throw \"strict cleanup left \$path\" }
}
    " >/dev/null \
        || { LAST_ERROR="strict cleanup could not remove and verify Windows session trees"; return 1; }
    WIN_SESSION_MAY_EXIST=0
    if [[ "$Q_SESSION_MAY_EXIST" == 1 ]]; then
        rm -rf -- "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
            || { LAST_ERROR="strict cleanup could not remove q session tree"; return 1; }
    fi
    [[ ! -e "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
        && ! -L "$Q_MODULE/rigw-sessions/$SESSION_TAG" ]] \
        || { LAST_ERROR="strict cleanup found a surviving or unexpected q session tree"; return 1; }
    Q_SESSION_MAY_EXIST=0
    ports_closed \
        || { LAST_ERROR="strict cleanup found port $PORT reopened during deletion"; return 1; }
    STRICT_CLEANUP_VERIFIED=1
}

finalize_registered_session() {
    local complete_tmp="$OUT_DIR/SESSION-COMPLETE.tmp"
    SESSION_FINALIZED=0
    [[ "$LOCAL_EVIDENCE_COMPLETE" == 1 ]] \
        || { LAST_ERROR="refusing cleanup before local evidence is complete"; return 1; }
    strict_success_cleanup || return 1
    [[ "$STRICT_CLEANUP_VERIFIED" == 1 ]] \
        || { LAST_ERROR="strict cleanup returned without verification"; return 1; }
    [[ ! -e "$OUT_DIR/SESSION-VOID" && ! -L "$OUT_DIR/SESSION-VOID" ]] \
        || { LAST_ERROR="refusing to complete a void session"; return 1; }
    [[ ! -e "$OUT_DIR/SESSION-COMPLETE" && ! -L "$OUT_DIR/SESSION-COMPLETE" ]] \
        || { LAST_ERROR="refusing to replace an existing completion marker"; return 1; }
    [[ ! -e "$complete_tmp" && ! -L "$complete_tmp" ]] \
        || { LAST_ERROR="refusing to replace an existing completion temporary"; return 1; }
    printf '%s\n' "$HEAD_FULL" > "$complete_tmp" || return 1
    mv "$complete_tmp" "$OUT_DIR/SESSION-COMPLETE" || return 1
    SESSION_FINALIZED=1
}

record_failure_evidence() {
    append_void_line "local evidence preserved at $OUT_DIR"
    if [[ "$Q_SESSION_MAY_EXIST" == 1 ]]; then
        append_void_line "q session evidence may remain; inspect $Q_MODULE/rigw-sessions/$SESSION_TAG"
    fi
    if [[ "$WIN_SESSION_MAY_EXIST" == 1 ]]; then
        append_void_line "Windows evidence may remain; inspect $WIN_SESSION and $WIN_MODULE/rigw-sessions/$SESSION_TAG"
    fi
}

on_signal() {
    local signal="$1" code="$2"
    LAST_ERROR="received $signal"
    trap '' HUP INT TERM
    exit "$code"
}

install_signal_traps() {
    trap 'on_signal HUP 129' HUP
    trap 'on_signal INT 130' INT
    trap 'on_signal TERM 143' TERM
}

registered_completion_marker_valid() {
    local marker="$OUT_DIR/SESSION-COMPLETE" lines
    [[ "$LOCAL_EVIDENCE_COMPLETE" == 1 \
        && -n "${HEAD_FULL:-}" && -f "$marker" && ! -L "$marker" ]] || return 1
    lines=$(LC_ALL=C wc -l < "$marker") || return 1
    lines=${lines//[[:space:]]/}
    [[ "$lines" == 1 && "$(< "$marker")" == "$HEAD_FULL" ]] || return 1
    [[ ! -e "$OUT_DIR/SESSION-COMPLETE.tmp" \
        && ! -L "$OUT_DIR/SESSION-COMPLETE.tmp" ]] || return 1
    [[ ! -e "$OUT_DIR/SESSION-VOID" && ! -L "$OUT_DIR/SESSION-VOID" ]]
}

on_exit() {
    local rc=$?
    trap - EXIT
    trap '' HUP INT TERM
    set +e
    if [[ $rc -eq 0 && "$OUTPUT_CLAIMED" == 1 \
        && ( -e "$OUT_DIR/SESSION-VOID" || -L "$OUT_DIR/SESSION-VOID" \
            || -e "$OUT_DIR/SESSION-COMPLETE.tmp" \
            || -L "$OUT_DIR/SESSION-COMPLETE.tmp" ) ]]; then
        LAST_ERROR="successful exit retained a failure or temporary marker"
        rc=1
    fi
    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 1 \
        && "$SESSION_FINALIZED" != 1 ]]; then
        LAST_ERROR="registered run returned without finalizing the session"
        rc=1
    fi
    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 1 ]] \
        && ! registered_completion_marker_valid; then
        LAST_ERROR="registered completion marker is absent or invalid"
        rc=1
    fi
    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 0 \
        && "$SESSION_FINALIZED" != 0 ]]; then
        LAST_ERROR="non-registered run claimed registered finalization"
        rc=1
    fi
    if [[ $rc -eq 0 && "$REGISTERED_RUN_STARTED" == 0 \
        && ( -e "$OUT_DIR/SESSION-COMPLETE" \
            || -L "$OUT_DIR/SESSION-COMPLETE" ) ]]; then
        LAST_ERROR="non-registered run left a completion marker"
        rc=1
    fi
    if [[ $rc -eq 0 && "$OUTPUT_CLAIMED" == 1 \
        && "$STRICT_CLEANUP_VERIFIED" != 1 ]]; then
        LAST_ERROR="successful exit lacked verified strict cleanup"
        rc=1
    fi

    if [[ $rc -ne 0 ]]; then
        rm -f -- "$OUT_DIR/SESSION-COMPLETE" "$OUT_DIR/SESSION-COMPLETE.tmp" \
            || CLEANUP_ERROR="${CLEANUP_ERROR:+$CLEANUP_ERROR; }could not remove completion marker"
        if [[ ! -s "$OUT_DIR/SESSION-VOID" ]]; then
            append_void_line "${LAST_ERROR:-unexpected harness failure rc=$rc}"
        fi
        CLEANUP_MODE=1
        if [[ -n "$win_daemon_pid" || -n "$win_cmd_pid" || -n "$current_block" ]]; then
            win_daemon_stop || true
        fi
        if [[ -n "$q_daemon_pid" ]]; then q_daemon_stop || true; fi
        if [[ -n "$CLEANUP_ERROR" ]]; then
            append_void_line "cleanup errors: $CLEANUP_ERROR"
        fi
        record_failure_evidence
        exit 1
    fi
    exit 0
}

main() {
    if [[ "$SELFTEST" == 1 ]]; then selftest; return; fi
    if ! claim_output_dir; then
        printf '%s\n' "FATAL: $OUTPUT_CLAIM_ERROR" >&2
        return 1
    fi
    trap on_exit EXIT
    install_signal_traps
    preflight
    if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
        strict_success_cleanup || session_void "preflight cleanup failed: ${LAST_ERROR:-unknown error}"
        log "PREFLIGHT_ONLY: no daemon started and no transfer timed"
        return
    fi

    REGISTERED_RUN_STARTED=1
    Q_SESSION_MAY_EXIST=1
    mkdir -p "$Q_MODULE/rigw-sessions/$SESSION_TAG" \
        || session_void "cannot create registered q session directory"
    printf '%s\n' 'block,trace_state,pass,cell,role,pair,role_order,transfer_ms,settled_ms,flush_ms,total_ms,landed_root,tree_manifest_sha256,exit,drain,valid,run_id,session_id,client_log' > "$RUNS_CSV"
    printf '%s\n' 'block,run_id,cell,pair,role,phase,sample,q_before_ns,windows_ns,q_after_ns,rtt_ns,offset_windows_minus_q_ns' > "$CLOCK_CSV"
    emit_schedule > "$OUT_DIR/schedule.csv"
    WIN_SESSION_MAY_EXIST=1
    wssh "New-Item -ItemType Directory -Force -Path '$WIN_SESSION' | Out-Null" \
        || session_void "cannot create registered Windows session directory"

    local block state pass first last
    while IFS=, read -r block state pass first last; do
        current_block="$block"
        run_block "$block" "$state" "$pass" "$first" "$last"
        current_block=""
    done < <(emit_schedule)

    end_gate
    python3 "$SCRIPT_DIR/otp12pf_rigw_analyze.py" "$OUT_DIR" \
        || session_void "phase/distribution analyzer rejected the session"
    LOCAL_EVIDENCE_COMPLETE=1
    log "ANALYZER ACCEPTED: exact local evidence inventory; finalizing session"
    finalize_registered_session \
        || session_void "registered finalization failed: ${LAST_ERROR:-unknown error}"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    main "$@"
fi
