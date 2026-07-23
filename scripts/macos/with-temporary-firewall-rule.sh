#!/bin/bash
#
# Run one macOS test command while owning one temporary Application Firewall
# rule. This is test infrastructure only: it installs and starts nothing.

set -u

mtfc_usage() {
    printf '%s\n' \
        "usage:" \
        "  $0 --app ABSOLUTE_PATH --session ID --evidence NEW_DIR -- COMMAND [ARG ...]" \
        "  $0 --recover --evidence NEW_DIR" >&2
}

mtfc_die() {
    printf 'macOS test firewall cleanup: %s\n' "$*" >&2
    exit 65
}

mtfc_reject_line_unsafe() {
    mtfc_value=$1
    mtfc_label=$2
    case "$mtfc_value" in
        *$'\n'* | *$'\r'*)
            mtfc_die "$mtfc_label may not contain a newline"
            ;;
    esac
    if [[ "$mtfc_value" =~ [[:space:]]$ ]]; then
        mtfc_die "$mtfc_label may not end in whitespace"
    fi
}

mtfc_app_input=
mtfc_session_id=
mtfc_evidence_dir=
mtfc_recover=0
mtfc_command=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --app)
            [[ $# -ge 2 ]] || {
                mtfc_usage
                exit 64
            }
            mtfc_app_input=$2
            shift 2
            ;;
        --session)
            [[ $# -ge 2 ]] || {
                mtfc_usage
                exit 64
            }
            mtfc_session_id=$2
            shift 2
            ;;
        --evidence)
            [[ $# -ge 2 ]] || {
                mtfc_usage
                exit 64
            }
            mtfc_evidence_dir=$2
            shift 2
            ;;
        --recover)
            mtfc_recover=1
            shift
            ;;
        --)
            shift
            mtfc_command=("$@")
            break
            ;;
        *)
            mtfc_usage
            exit 64
            ;;
    esac
done

[[ -n "$mtfc_evidence_dir" ]] || {
    mtfc_usage
    exit 64
}
[[ ! -e "$mtfc_evidence_dir" ]] ||
    mtfc_die "evidence path already exists: $mtfc_evidence_dir"
mkdir "$mtfc_evidence_dir" ||
    mtfc_die "cannot create evidence directory: $mtfc_evidence_dir"

mtfc_firewall_tool=${BLIT_TEST_SOCKETFILTERFW:-/usr/libexec/ApplicationFirewall/socketfilterfw}
mtfc_sudo_tool=${BLIT_TEST_SUDO:-sudo}
mtfc_python_tool=${BLIT_TEST_PYTHON:-/usr/bin/python3}
mtfc_state_dir=${BLIT_TEST_FIREWALL_STATE_DIR:-"$HOME/Library/Caches/blit-test-firewall"}
mtfc_state_file="$mtfc_state_dir/owned-rule.v1"
mtfc_test_disable_keeper=${BLIT_TEST_FIREWALL_DISABLE_KEEPALIVE:-0}
mtfc_test_allow_root=${BLIT_TEST_FIREWALL_ALLOW_ROOT:-0}
mtfc_invocation_started_at=$(date -u '+%Y-%m-%dT%H:%M:%SZ')

[[ -x "$mtfc_firewall_tool" ]] ||
    mtfc_die "firewall tool is not executable: $mtfc_firewall_tool"
command -v "$mtfc_sudo_tool" >/dev/null 2>&1 ||
    mtfc_die "sudo tool is unavailable: $mtfc_sudo_tool"
[[ -x "$mtfc_python_tool" ]] ||
    mtfc_die "python is not executable: $mtfc_python_tool"
[[ "$EUID" -ne 0 || "$mtfc_test_allow_root" == "1" ]] ||
    mtfc_die "run the wrapper as the invoking user, not through sudo"

mkdir -p "$mtfc_state_dir" ||
    mtfc_die "cannot create firewall state directory: $mtfc_state_dir"
chmod 700 "$mtfc_state_dir" ||
    mtfc_die "cannot protect firewall state directory: $mtfc_state_dir"

mtfc_run_capture() {
    mtfc_capture_name=$1
    shift
    date -u '+%Y-%m-%dT%H:%M:%SZ' \
        >"$mtfc_evidence_dir/$mtfc_capture_name.started-at"
    "$@" >"$mtfc_evidence_dir/$mtfc_capture_name.out" 2>&1
    mtfc_capture_rc=$?
    date -u '+%Y-%m-%dT%H:%M:%SZ' \
        >"$mtfc_evidence_dir/$mtfc_capture_name.finished-at"
    printf '%s\n' "$mtfc_capture_rc" >"$mtfc_evidence_dir/$mtfc_capture_name.exit"
    return "$mtfc_capture_rc"
}

mtfc_inventory_count=0
mtfc_inventory_allow=0
mtfc_inventory_block=0

mtfc_capture_inventory() {
    mtfc_inventory_phase=$1
    mtfc_inventory_path=$2
    mtfc_inventory_file="$mtfc_evidence_dir/firewall-$mtfc_inventory_phase.txt"
    mtfc_metrics_file="$mtfc_evidence_dir/firewall-$mtfc_inventory_phase.metrics"

    date -u '+%Y-%m-%dT%H:%M:%SZ' \
        >"$mtfc_evidence_dir/firewall-$mtfc_inventory_phase.started-at"
    "$mtfc_firewall_tool" --listapps >"$mtfc_inventory_file" 2>&1
    mtfc_inventory_rc=$?
    date -u '+%Y-%m-%dT%H:%M:%SZ' \
        >"$mtfc_evidence_dir/firewall-$mtfc_inventory_phase.finished-at"
    printf '%s\n' "$mtfc_inventory_rc" \
        >"$mtfc_evidence_dir/firewall-$mtfc_inventory_phase.exit"
    [[ "$mtfc_inventory_rc" -eq 0 ]] || return 1

    "$mtfc_python_tool" - "$mtfc_inventory_path" "$mtfc_inventory_file" \
        >"$mtfc_metrics_file" <<'PY'
import re
import sys

wanted = sys.argv[1]
lines = open(sys.argv[2], encoding="utf-8").read().splitlines()
if not lines:
    raise SystemExit("empty firewall inventory")
header = re.fullmatch(r"Total number of apps = ([0-9]+)\s*", lines[0])
if not header:
    raise SystemExit("malformed firewall inventory header")
declared = int(header.group(1))
entries = []
index = 1
while index < len(lines):
    path_match = re.fullmatch(r"\s*[0-9]+\s+:\s(.*?)\s*", lines[index])
    if not path_match or index + 1 >= len(lines):
        raise SystemExit("malformed firewall inventory entry")
    status_match = re.fullmatch(
        r"\s+\((Allow|Block) incoming connections\)\s*", lines[index + 1]
    )
    if not status_match:
        raise SystemExit("malformed firewall inventory status")
    entries.append((path_match.group(1), status_match.group(1)))
    index += 2
if len(entries) != declared:
    raise SystemExit(
        f"firewall inventory count mismatch: declared={declared} parsed={len(entries)}"
    )
matching = [status for path, status in entries if path == wanted]
print(f"count={len(matching)}")
print(f"allow={matching.count('Allow')}")
print(f"block={matching.count('Block')}")
PY
    mtfc_parse_rc=$?
    [[ "$mtfc_parse_rc" -eq 0 ]] || return 1

    mtfc_inventory_count=$(
        awk -F= '$1 == "count" {print $2}' "$mtfc_metrics_file"
    )
    mtfc_inventory_allow=$(
        awk -F= '$1 == "allow" {print $2}' "$mtfc_metrics_file"
    )
    mtfc_inventory_block=$(
        awk -F= '$1 == "block" {print $2}' "$mtfc_metrics_file"
    )
    [[ "$mtfc_inventory_count" =~ ^[0-9]+$ ]] &&
        [[ "$mtfc_inventory_allow" =~ ^[0-9]+$ ]] &&
        [[ "$mtfc_inventory_block" =~ ^[0-9]+$ ]]
}

mtfc_ledger_schema=
mtfc_ledger_lexical=
mtfc_ledger_canonical=
mtfc_ledger_session=
mtfc_ledger_created=
mtfc_ledger_pid=

mtfc_read_ledger() {
    [[ -f "$mtfc_state_file" ]] || return 1
    {
        IFS= read -r mtfc_ledger_schema &&
            IFS= read -r mtfc_ledger_lexical &&
            IFS= read -r mtfc_ledger_canonical &&
            IFS= read -r mtfc_ledger_session &&
            IFS= read -r mtfc_ledger_created &&
            IFS= read -r mtfc_ledger_pid &&
            ! IFS= read -r mtfc_extra_line
    } <"$mtfc_state_file" || return 1
    [[ "$mtfc_ledger_schema" == "1" ]] || return 1
    [[ "$mtfc_ledger_canonical" == /* ]] || return 1
    [[ -n "$mtfc_ledger_session" ]] || return 1
    return 0
}

mtfc_sync_path_and_parent() {
    "$mtfc_python_tool" - "$1" <<'PY'
import os
import sys

path = sys.argv[1]
fd = os.open(path, os.O_RDONLY)
try:
    os.fsync(fd)
finally:
    os.close(fd)
parent_fd = os.open(os.path.dirname(path), os.O_RDONLY)
try:
    os.fsync(parent_fd)
finally:
    os.close(parent_fd)
PY
}

mtfc_write_ledger() {
    mtfc_ledger_tmp="$mtfc_state_file.tmp.$$"
    mtfc_ledger_created=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
    umask 077
    {
        printf '1\n'
        printf '%s\n' "$mtfc_app_input"
        printf '%s\n' "$mtfc_app_path"
        printf '%s\n' "$mtfc_session_id"
        printf '%s\n' "$mtfc_ledger_created"
        printf '%s\n' "$$"
    } >"$mtfc_ledger_tmp" || return 1
    chmod 600 "$mtfc_ledger_tmp" || return 1
    mv "$mtfc_ledger_tmp" "$mtfc_state_file" || return 1
    mtfc_sync_path_and_parent "$mtfc_state_file"
}

mtfc_clear_ledger() {
    rm -f "$mtfc_state_file" || return 1
    "$mtfc_python_tool" - "$mtfc_state_dir" <<'PY'
import os
import sys

fd = os.open(sys.argv[1], os.O_RDONLY)
try:
    os.fsync(fd)
finally:
    os.close(fd)
PY
}

mtfc_keeper_pid=
mtfc_start_keeper() {
    [[ "$mtfc_test_disable_keeper" == "1" ]] && return 0
    (
        while sleep 30; do
            "$mtfc_sudo_tool" -n -v >/dev/null 2>&1 || exit 1
        done
    ) &
    mtfc_keeper_pid=$!
}

mtfc_stop_keeper() {
    [[ -n "$mtfc_keeper_pid" ]] || return 0
    kill "$mtfc_keeper_pid" >/dev/null 2>&1 || true
    wait "$mtfc_keeper_pid" >/dev/null 2>&1 || true
    mtfc_keeper_pid=
}

mtfc_recover_owned_rule() {
    mtfc_read_ledger ||
        mtfc_die "owned-rule ledger is malformed: $mtfc_state_file"
    mtfc_app_path=$mtfc_ledger_canonical
    mtfc_session_id=$mtfc_ledger_session

    mtfc_capture_inventory recovery-before "$mtfc_app_path" ||
        mtfc_die "cannot parse the live firewall inventory; ledger retained"

    if [[ "$mtfc_inventory_count" -gt 0 ]]; then
        mtfc_run_capture sudo-validate "$mtfc_sudo_tool" -v ||
            mtfc_die "administrator authorization failed; ledger retained"
        mtfc_run_capture recovery-remove "$mtfc_sudo_tool" -n \
            "$mtfc_firewall_tool" --remove "$mtfc_app_path" ||
            mtfc_die "exact recovery removal failed; ledger retained"
        mtfc_capture_inventory recovery-after "$mtfc_app_path" ||
            mtfc_die "cannot verify recovery inventory; ledger retained"
        [[ "$mtfc_inventory_count" -eq 0 ]] ||
            mtfc_die "exact recovery path remains in firewall inventory; ledger retained"
    fi

    mtfc_clear_ledger ||
        mtfc_die "firewall is clean but the owned-rule ledger could not be cleared"
    {
        printf 'mode=recovery\n'
        printf 'session=%s\n' "$mtfc_session_id"
        printf 'path=%s\n' "$mtfc_app_path"
        printf 'ledger_created_at=%s\n' "$mtfc_ledger_created"
        printf 'ledger_helper_pid=%s\n' "$mtfc_ledger_pid"
        printf 'invocation_started_at=%s\n' "$mtfc_invocation_started_at"
        printf 'invocation_finished_at=%s\n' \
            "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
        printf 'cleanup_verified=true\n'
    } >"$mtfc_evidence_dir/summary.txt"
}

if [[ "$mtfc_recover" -eq 1 ]]; then
    [[ -z "$mtfc_app_input" && -z "$mtfc_session_id" &&
        "${#mtfc_command[@]}" -eq 0 ]] || {
        mtfc_usage
        exit 64
    }
    mtfc_recover_owned_rule
    exit 0
fi

[[ -n "$mtfc_app_input" && -n "$mtfc_session_id" &&
    "${#mtfc_command[@]}" -gt 0 ]] || {
    mtfc_usage
    exit 64
}
[[ "$mtfc_app_input" == /* ]] ||
    mtfc_die "daemon path must be absolute: $mtfc_app_input"
[[ -f "$mtfc_app_input" ]] ||
    mtfc_die "daemon path is not a regular file: $mtfc_app_input"
mtfc_reject_line_unsafe "$mtfc_app_input" "daemon path"
mtfc_reject_line_unsafe "$mtfc_session_id" "session ID"

mtfc_app_path=$(
    "$mtfc_python_tool" -c \
        'import os, sys; print(os.path.realpath(sys.argv[1]))' \
        "$mtfc_app_input"
) || mtfc_die "cannot canonicalize daemon path: $mtfc_app_input"
[[ "$mtfc_app_path" == /* ]] ||
    mtfc_die "canonical daemon path is not absolute: $mtfc_app_path"
mtfc_reject_line_unsafe "$mtfc_app_path" "canonical daemon path"

[[ ! -e "$mtfc_state_file" ]] ||
    mtfc_die "an owned-rule ledger already exists; run --recover first: $mtfc_state_file"

mtfc_capture_inventory before-add "$mtfc_app_path" ||
    mtfc_die "cannot parse the pre-add firewall inventory"
[[ "$mtfc_inventory_count" -eq 0 ]] ||
    mtfc_die "refusing to adopt an existing firewall entry: $mtfc_app_path"

mtfc_run_capture sudo-validate "$mtfc_sudo_tool" -v ||
    mtfc_die "administrator authorization failed before firewall mutation"
mtfc_start_keeper

mtfc_cleanup_needed=0
mtfc_child_pid=
mtfc_command_started=false
mtfc_command_rc=not-run

mtfc_cleanup_rule() {
    mtfc_capture_inventory before-remove "$mtfc_app_path" || return 1
    if [[ "$mtfc_inventory_count" -gt 0 ]]; then
        mtfc_run_capture remove "$mtfc_sudo_tool" -n \
            "$mtfc_firewall_tool" --remove "$mtfc_app_path" || return 1
    fi
    mtfc_capture_inventory after-remove "$mtfc_app_path" || return 1
    [[ "$mtfc_inventory_count" -eq 0 ]] || return 1
    mtfc_clear_ledger || return 1
    return 0
}

mtfc_finish() {
    mtfc_original_rc=$?
    trap - EXIT HUP INT TERM
    mtfc_cleanup_ok=false
    mtfc_cleanup_superseded=false
    mtfc_final_rc=$mtfc_original_rc
    if [[ "$mtfc_cleanup_needed" -eq 1 ]]; then
        if mtfc_cleanup_rule; then
            mtfc_cleanup_ok=true
        else
            mtfc_final_rc=90
        fi
    fi
    if [[ "$mtfc_final_rc" -ne "$mtfc_original_rc" ]]; then
        mtfc_cleanup_superseded=true
    fi
    mtfc_stop_keeper
    {
        printf 'mode=run\n'
        printf 'session=%s\n' "$mtfc_session_id"
        printf 'lexical_path=%s\n' "$mtfc_app_input"
        printf 'canonical_path=%s\n' "$mtfc_app_path"
        printf 'ledger_created_at=%s\n' "$mtfc_ledger_created"
        printf 'helper_pid=%s\n' "$$"
        printf 'invocation_started_at=%s\n' "$mtfc_invocation_started_at"
        printf 'invocation_finished_at=%s\n' \
            "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
        printf 'command_started=%s\n' "$mtfc_command_started"
        printf 'command_exit=%s\n' "$mtfc_command_rc"
        printf 'cleanup_verified=%s\n' "$mtfc_cleanup_ok"
        printf 'cleanup_superseded_command=%s\n' "$mtfc_cleanup_superseded"
        printf 'final_exit=%s\n' "$mtfc_final_rc"
    } >"$mtfc_evidence_dir/summary.txt"
    if [[ "$mtfc_cleanup_ok" != true ]]; then
        printf 'macOS test firewall cleanup: unresolved owned rule: %s\n' \
            "$mtfc_app_path" >&2
    fi
    exit "$mtfc_final_rc"
}

mtfc_signal() {
    mtfc_signal_name=$1
    mtfc_signal_rc=$2
    trap - "$mtfc_signal_name"
    if [[ -n "$mtfc_child_pid" ]]; then
        kill -s "$mtfc_signal_name" "$mtfc_child_pid" >/dev/null 2>&1 || true
        wait "$mtfc_child_pid" >/dev/null 2>&1 || true
        mtfc_child_pid=
    fi
    mtfc_command_rc=$mtfc_signal_rc
    exit "$mtfc_signal_rc"
}

trap mtfc_finish EXIT
trap 'mtfc_signal HUP 129' HUP
trap 'mtfc_signal INT 130' INT
trap 'mtfc_signal TERM 143' TERM

mtfc_cleanup_needed=1
mtfc_write_ledger ||
    mtfc_die "cannot write durable owned-rule ledger: $mtfc_state_file"

mtfc_run_capture add "$mtfc_sudo_tool" -n \
    "$mtfc_firewall_tool" --add "$mtfc_app_path" || exit 69
mtfc_run_capture unblock "$mtfc_sudo_tool" -n \
    "$mtfc_firewall_tool" --unblockapp "$mtfc_app_path" || exit 70
mtfc_capture_inventory after-add "$mtfc_app_path" || exit 71
[[ "$mtfc_inventory_count" -eq 1 &&
    "$mtfc_inventory_allow" -eq 1 &&
    "$mtfc_inventory_block" -eq 0 ]] || exit 71

"${mtfc_command[@]}" &
mtfc_command_started=true
mtfc_child_pid=$!
wait "$mtfc_child_pid"
mtfc_command_rc=$?
mtfc_child_pid=
exit "$mtfc_command_rc"
