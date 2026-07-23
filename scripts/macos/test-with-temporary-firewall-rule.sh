#!/bin/bash

set -u

mtfc_test_root=$(mktemp -d "${TMPDIR:-/tmp}/blit-firewall-helper-tests.XXXXXX")
mtfc_wrapper=$(
    cd "$(dirname "$0")" &&
        pwd
)/with-temporary-firewall-rule.sh
mtfc_fake_firewall="$mtfc_test_root/socketfilterfw"
mtfc_fake_sudo="$mtfc_test_root/sudo"
mtfc_passed=0

mtfc_cleanup_tests() {
    rm -rf "$mtfc_test_root"
}
trap mtfc_cleanup_tests EXIT

mtfc_fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

mtfc_assert_eq() {
    mtfc_expected=$1
    mtfc_actual=$2
    mtfc_message=$3
    [[ "$mtfc_actual" == "$mtfc_expected" ]] ||
        mtfc_fail "$mtfc_message: expected=$mtfc_expected actual=$mtfc_actual"
}

mtfc_assert_file_contains() {
    mtfc_file=$1
    mtfc_pattern=$2
    mtfc_message=$3
    grep -Fq "$mtfc_pattern" "$mtfc_file" ||
        mtfc_fail "$mtfc_message: missing '$mtfc_pattern' in $mtfc_file"
}

mtfc_assert_path_absent() {
    [[ ! -e "$1" ]] || mtfc_fail "$2: still exists: $1"
}

mtfc_mark_pass() {
    mtfc_passed=$((mtfc_passed + 1))
    printf 'ok %d - %s\n' "$mtfc_passed" "$1"
}

cat >"$mtfc_fake_firewall" <<'FAKE_FIREWALL'
#!/bin/bash
set -u

: "${FAKE_FIREWALL_DB:?}"
: "${FAKE_FIREWALL_LOG:?}"
touch "$FAKE_FIREWALL_DB" "$FAKE_FIREWALL_LOG"
mtfc_fake_action=${1:-}
mtfc_fake_path=${2:-}
printf '%s|%s\n' "$mtfc_fake_action" "$mtfc_fake_path" >>"$FAKE_FIREWALL_LOG"

case "$mtfc_fake_action" in
    --listapps)
        if [[ "${FAKE_MALFORMED_INVENTORY:-0}" == "1" ]]; then
            printf 'not an inventory\n'
            exit 0
        fi
        mtfc_fake_count=0
        while IFS= read -r mtfc_fake_entry; do
            if [[ -n "${FAKE_HIDE_PATH:-}" &&
                "$mtfc_fake_entry" == "$FAKE_HIDE_PATH" ]]; then
                continue
            fi
            mtfc_fake_count=$((mtfc_fake_count + 1))
        done <"$FAKE_FIREWALL_DB"
        printf 'Total number of apps = %s \n' "$mtfc_fake_count"
        if [[ "${FAKE_INVENTORY_STYLE:-}" == "spaced" ]]; then
            printf '\n'
        fi
        mtfc_fake_index=1
        while IFS= read -r mtfc_fake_entry; do
            if [[ -n "${FAKE_HIDE_PATH:-}" &&
                "$mtfc_fake_entry" == "$FAKE_HIDE_PATH" ]]; then
                continue
            fi
            if [[ "${FAKE_INVENTORY_STYLE:-}" == "spaced" ]]; then
                printf '  %s   :   %s   \n' \
                    "$mtfc_fake_index" "$mtfc_fake_entry"
                printf '             ( Allow   incoming connections )\n\n'
            else
                printf '%s : %s \n' "$mtfc_fake_index" "$mtfc_fake_entry"
                printf '             (Allow incoming connections)\n'
            fi
            mtfc_fake_index=$((mtfc_fake_index + 1))
        done <"$FAKE_FIREWALL_DB"
        ;;
    --add)
        [[ "${FAKE_ADD_FAIL:-0}" != "1" ]] || exit 31
        printf '%s\n' "$mtfc_fake_path" >>"$FAKE_FIREWALL_DB"
        if [[ "${FAKE_ADD_DUPLICATE:-0}" == "1" ]]; then
            printf '%s\n' "$mtfc_fake_path" >>"$FAKE_FIREWALL_DB"
        fi
        ;;
    --unblockapp)
        [[ "${FAKE_UNBLOCK_FAIL:-0}" != "1" ]] || exit 32
        grep -Fxq "$mtfc_fake_path" "$FAKE_FIREWALL_DB" || exit 33
        ;;
    --remove)
        [[ "${FAKE_REMOVE_FAIL:-0}" != "1" ]] || exit 34
        mtfc_fake_tmp="$FAKE_FIREWALL_DB.tmp.$$"
        awk -v wanted="$mtfc_fake_path" '$0 != wanted {print}' \
            "$FAKE_FIREWALL_DB" >"$mtfc_fake_tmp"
        mv "$mtfc_fake_tmp" "$FAKE_FIREWALL_DB"
        ;;
    *)
        exit 35
        ;;
esac
FAKE_FIREWALL

cat >"$mtfc_fake_sudo" <<'FAKE_SUDO'
#!/bin/bash
set -u

: "${FAKE_SUDO_LOG:?}"
printf '%s\n' "$*" >>"$FAKE_SUDO_LOG"

if [[ "${1:-}" == "-v" ]]; then
    [[ "${FAKE_SUDO_VALIDATE_FAIL:-0}" != "1" ]] || exit 41
    exit 0
fi
if [[ "${1:-}" == "-n" ]]; then
    shift
    if [[ "${1:-}" == "-v" ]]; then
        [[ "${FAKE_SUDO_KEEP_FAIL:-0}" != "1" ]] || exit 42
        exit 0
    fi
    if [[ "${FAKE_SUDO_REMOVE_FAIL:-0}" == "1" &&
        "${2:-}" == "--remove" ]]; then
        exit 44
    fi
    exec "$@"
fi
exit 43
FAKE_SUDO

chmod +x "$mtfc_fake_firewall" "$mtfc_fake_sudo"

mtfc_prepare_case() {
    mtfc_case_name=$1
    mtfc_case_root="$mtfc_test_root/$mtfc_case_name"
    mtfc_case_state="$mtfc_case_root/state"
    mtfc_case_db="$mtfc_case_root/firewall.db"
    mtfc_case_log="$mtfc_case_root/firewall.log"
    mtfc_case_sudo_log="$mtfc_case_root/sudo.log"
    mtfc_case_evidence="$mtfc_case_root/evidence"
    mtfc_case_app_dir="$mtfc_case_root/app dir"
    mtfc_case_app="$mtfc_case_app_dir/blit-daemon"
    mkdir -p "$mtfc_case_state" "$mtfc_case_app_dir"
    : >"$mtfc_case_db"
    : >"$mtfc_case_log"
    : >"$mtfc_case_sudo_log"
    : >"$mtfc_case_app"
    chmod +x "$mtfc_case_app"
    mtfc_case_canonical=$(
        /usr/bin/python3 -c \
            'import os, sys; print(os.path.realpath(sys.argv[1]))' \
            "$mtfc_case_app"
    )
}

mtfc_run_case() {
    env \
        BLIT_TEST_SOCKETFILTERFW="$mtfc_fake_firewall" \
        BLIT_TEST_SUDO="$mtfc_fake_sudo" \
        BLIT_TEST_PYTHON=/usr/bin/python3 \
        BLIT_TEST_FIREWALL_STATE_DIR="$mtfc_case_state" \
        BLIT_TEST_FIREWALL_DISABLE_KEEPALIVE=1 \
        FAKE_FIREWALL_DB="$mtfc_case_db" \
        FAKE_FIREWALL_LOG="$mtfc_case_log" \
        FAKE_SUDO_LOG="$mtfc_case_sudo_log" \
        "$@"
}

mtfc_test_success_with_spaces_and_unrelated_entry() {
    mtfc_prepare_case success
    printf '%s.backup\n' "$mtfc_case_canonical" >"$mtfc_case_db"
    FAKE_INVENTORY_STYLE=spaced mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session success \
        --evidence "$mtfc_case_evidence" \
        -- /bin/sh -c 'exit 0'
    mtfc_rc=$?
    mtfc_assert_eq 0 "$mtfc_rc" "successful wrapper exit"
    mtfc_assert_eq "$mtfc_case_canonical.backup" \
        "$(cat "$mtfc_case_db")" "unrelated entry preserved"
    mtfc_assert_path_absent "$mtfc_case_state/owned-rule.v1" \
        "successful ledger cleanup"
    mtfc_assert_file_contains "$mtfc_case_evidence/summary.txt" \
        "cleanup_verified=true" "successful cleanup summary"
    mtfc_assert_file_contains "$mtfc_case_evidence/summary.txt" \
        "invocation_started_at=" "successful cleanup start time"
    mtfc_assert_file_contains "$mtfc_case_evidence/summary.txt" \
        "invocation_finished_at=" "successful cleanup finish time"
    mtfc_assert_file_contains "$mtfc_case_evidence/summary.txt" \
        "cleanup_superseded_command=false" "successful cleanup result ownership"
    [[ -s "$mtfc_case_evidence/add.started-at" &&
        -s "$mtfc_case_evidence/add.finished-at" ]] ||
        mtfc_fail "add evidence is missing timestamps"
    mtfc_mark_pass "success owns exact path with spaces and preserves unrelated entry"
}

mtfc_test_command_failure_still_cleans() {
    mtfc_prepare_case command-failure
    mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session command-failure \
        --evidence "$mtfc_case_evidence" \
        -- /bin/sh -c 'exit 42'
    mtfc_rc=$?
    mtfc_assert_eq 42 "$mtfc_rc" "command failure preserved"
    mtfc_assert_eq "" "$(cat "$mtfc_case_db")" "command failure rule removed"
    mtfc_assert_path_absent "$mtfc_case_state/owned-rule.v1" \
        "command failure ledger cleanup"
    mtfc_mark_pass "nonzero command result still cleans and is preserved"
}

mtfc_test_existing_rule_is_never_adopted() {
    mtfc_prepare_case preexisting
    printf '%s\n' "$mtfc_case_canonical" >"$mtfc_case_db"
    mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session preexisting \
        --evidence "$mtfc_case_evidence" \
        -- /bin/sh -c 'exit 0'
    mtfc_rc=$?
    mtfc_assert_eq 65 "$mtfc_rc" "preexisting rule refusal"
    mtfc_assert_eq "$mtfc_case_canonical" \
        "$(cat "$mtfc_case_db")" "preexisting rule untouched"
    if grep -Fq -- '--remove' "$mtfc_case_log"; then
        mtfc_fail "preexisting rule was removed"
    fi
    mtfc_mark_pass "preexisting rule is refused and never removed"
}

mtfc_test_duplicate_rule_is_refused() {
    mtfc_prepare_case duplicate-preexisting
    printf '%s\n%s\n' "$mtfc_case_canonical" "$mtfc_case_canonical" \
        >"$mtfc_case_db"
    mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session duplicate-preexisting \
        --evidence "$mtfc_case_evidence" \
        -- /bin/sh -c 'exit 0'
    mtfc_rc=$?
    mtfc_assert_eq 65 "$mtfc_rc" "duplicate preexisting rule refusal"
    mtfc_assert_eq 2 "$(wc -l <"$mtfc_case_db" | tr -d ' ')" \
        "duplicate preexisting rules untouched"
    mtfc_mark_pass "duplicate preexisting path fails closed"
}

mtfc_test_add_failure_clears_empty_ledger() {
    mtfc_prepare_case add-failure
    FAKE_ADD_FAIL=1 mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session add-failure \
        --evidence "$mtfc_case_evidence" \
        -- /bin/sh -c 'exit 0'
    mtfc_rc=$?
    mtfc_assert_eq 69 "$mtfc_rc" "add failure result"
    mtfc_assert_eq "" "$(cat "$mtfc_case_db")" "add failure left no rule"
    mtfc_assert_path_absent "$mtfc_case_state/owned-rule.v1" \
        "add failure cleared reconciled ledger"
    mtfc_mark_pass "add failure proves absence and clears its ledger"
}

mtfc_test_unblock_failure_removes_added_rule() {
    mtfc_prepare_case unblock-failure
    FAKE_UNBLOCK_FAIL=1 mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session unblock-failure \
        --evidence "$mtfc_case_evidence" \
        -- /bin/sh -c 'exit 0'
    mtfc_rc=$?
    mtfc_assert_eq 70 "$mtfc_rc" "unblock failure result"
    mtfc_assert_eq "" "$(cat "$mtfc_case_db")" "unblock failure cleanup"
    mtfc_assert_path_absent "$mtfc_case_state/owned-rule.v1" \
        "unblock failure ledger cleanup"
    mtfc_mark_pass "unblock failure removes the rule it added"
}

mtfc_test_post_add_duplicate_fails_and_cleans() {
    mtfc_prepare_case duplicate-add
    FAKE_ADD_DUPLICATE=1 mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session duplicate-add \
        --evidence "$mtfc_case_evidence" \
        -- /bin/sh -c 'exit 0'
    mtfc_rc=$?
    mtfc_assert_eq 71 "$mtfc_rc" "post-add duplicate result"
    mtfc_assert_eq "" "$(cat "$mtfc_case_db")" "post-add duplicates removed"
    mtfc_mark_pass "duplicate post-add inventory fails and cleans"
}

mtfc_test_remove_failure_overrides_success_and_recovers() {
    mtfc_prepare_case remove-failure
    FAKE_REMOVE_FAIL=1 mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session remove-failure \
        --evidence "$mtfc_case_evidence" \
        -- /bin/sh -c 'exit 0'
    mtfc_rc=$?
    mtfc_assert_eq 90 "$mtfc_rc" "cleanup failure overrides success"
    [[ -f "$mtfc_case_state/owned-rule.v1" ]] ||
        mtfc_fail "cleanup failure did not retain ledger"
    mtfc_assert_eq "$mtfc_case_canonical" \
        "$(cat "$mtfc_case_db")" "cleanup failure retained live rule"
    mtfc_assert_file_contains "$mtfc_case_evidence/summary.txt" \
        "cleanup_verified=false" "cleanup failure summary"
    mtfc_assert_file_contains "$mtfc_case_evidence/summary.txt" \
        "cleanup_superseded_command=true" "cleanup failure supersession"

    mtfc_blocked_evidence="$mtfc_case_root/blocked-evidence"
    mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session blocked-by-ledger \
        --evidence "$mtfc_blocked_evidence" \
        -- /bin/sh -c 'exit 0'
    mtfc_blocked_rc=$?
    mtfc_assert_eq 65 "$mtfc_blocked_rc" \
        "unresolved ledger blocks new work"

    mtfc_recovery_evidence="$mtfc_case_root/recovery-evidence"
    mtfc_run_case "$mtfc_wrapper" \
        --recover \
        --evidence "$mtfc_recovery_evidence"
    mtfc_recovery_rc=$?
    mtfc_assert_eq 0 "$mtfc_recovery_rc" "exact recovery result"
    mtfc_assert_eq "" "$(cat "$mtfc_case_db")" "recovery removed exact rule"
    mtfc_assert_path_absent "$mtfc_case_state/owned-rule.v1" \
        "recovery cleared ledger"
    mtfc_mark_pass "cleanup failure overrides success and exact recovery clears it"
}

mtfc_test_lost_cleanup_authorization_retains_recovery() {
    mtfc_prepare_case cleanup-auth
    FAKE_SUDO_REMOVE_FAIL=1 mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session cleanup-auth \
        --evidence "$mtfc_case_evidence" \
        -- /bin/sh -c 'exit 0'
    mtfc_rc=$?
    mtfc_assert_eq 90 "$mtfc_rc" "lost cleanup authorization result"
    [[ -f "$mtfc_case_state/owned-rule.v1" ]] ||
        mtfc_fail "lost cleanup authorization did not retain ledger"
    mtfc_assert_eq "$mtfc_case_canonical" \
        "$(cat "$mtfc_case_db")" "lost authorization retained live rule"

    mtfc_recovery_evidence="$mtfc_case_root/recovery-evidence"
    mtfc_run_case "$mtfc_wrapper" \
        --recover \
        --evidence "$mtfc_recovery_evidence"
    mtfc_recovery_rc=$?
    mtfc_assert_eq 0 "$mtfc_recovery_rc" \
        "lost authorization recovery result"
    mtfc_assert_eq "" "$(cat "$mtfc_case_db")" \
        "lost authorization recovery removed rule"
    mtfc_assert_path_absent "$mtfc_case_state/owned-rule.v1" \
        "lost authorization recovery cleared ledger"
    mtfc_mark_pass "lost cleanup authorization fails closed and remains recoverable"
}

mtfc_test_initial_authorization_failure_is_nonmutating() {
    mtfc_prepare_case initial-auth
    FAKE_SUDO_VALIDATE_FAIL=1 mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session initial-auth \
        --evidence "$mtfc_case_evidence" \
        -- /bin/sh -c 'exit 0'
    mtfc_rc=$?
    mtfc_assert_eq 65 "$mtfc_rc" "initial authorization refusal"
    mtfc_assert_eq "" "$(cat "$mtfc_case_db")" \
        "initial authorization made no rule"
    mtfc_assert_path_absent "$mtfc_case_state/owned-rule.v1" \
        "initial authorization made no ledger"
    mtfc_mark_pass "initial authorization failure occurs before ledger or mutation"
}

mtfc_test_absent_stale_ledger_recovers_without_remove() {
    mtfc_prepare_case absent-ledger
    FAKE_REMOVE_FAIL=1 mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session absent-ledger \
        --evidence "$mtfc_case_evidence" \
        -- /bin/sh -c 'exit 0'
    mtfc_rc=$?
    mtfc_assert_eq 90 "$mtfc_rc" "setup cleanup failure"
    : >"$mtfc_case_db"
    : >"$mtfc_case_log"
    mtfc_recovery_evidence="$mtfc_case_root/recovery-evidence"
    mtfc_run_case "$mtfc_wrapper" \
        --recover \
        --evidence "$mtfc_recovery_evidence"
    mtfc_recovery_rc=$?
    mtfc_assert_eq 0 "$mtfc_recovery_rc" "absent stale-ledger recovery"
    if grep -Fq -- '--remove' "$mtfc_case_log"; then
        mtfc_fail "absent stale ledger invoked remove"
    fi
    mtfc_assert_path_absent "$mtfc_case_state/owned-rule.v1" \
        "absent stale ledger cleared"
    mtfc_mark_pass "stale ledger with absent rule reconciles without removal"
}

mtfc_test_absent_stale_ledger_blocks_new_work() {
    mtfc_prepare_case absent-ledger-blocks
    FAKE_REMOVE_FAIL=1 mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session absent-ledger-blocks \
        --evidence "$mtfc_case_evidence" \
        -- /bin/sh -c 'exit 0'
    mtfc_rc=$?
    mtfc_assert_eq 90 "$mtfc_rc" "setup cleanup failure"
    : >"$mtfc_case_db"

    mtfc_blocked_evidence="$mtfc_case_root/blocked-evidence"
    mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session must-recover-first \
        --evidence "$mtfc_blocked_evidence" \
        -- /bin/sh -c 'exit 0'
    mtfc_blocked_rc=$?
    mtfc_assert_eq 65 "$mtfc_blocked_rc" \
        "absent unresolved ledger blocks new work"
    [[ -f "$mtfc_case_state/owned-rule.v1" ]] ||
        mtfc_fail "blocked invocation cleared unresolved ledger"

    mtfc_recovery_evidence="$mtfc_case_root/recovery-evidence"
    mtfc_run_case "$mtfc_wrapper" \
        --recover \
        --evidence "$mtfc_recovery_evidence"
    mtfc_recovery_rc=$?
    mtfc_assert_eq 0 "$mtfc_recovery_rc" \
        "absent ledger recovery after block"
    mtfc_assert_path_absent "$mtfc_case_state/owned-rule.v1" \
        "absent blocked ledger cleared by recovery"
    mtfc_mark_pass "unresolved absent ledger blocks new work until recovery"
}

mtfc_test_malformed_inventory_fails_before_mutation() {
    mtfc_prepare_case malformed
    FAKE_MALFORMED_INVENTORY=1 mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session malformed \
        --evidence "$mtfc_case_evidence" \
        -- /bin/sh -c 'exit 0'
    mtfc_rc=$?
    mtfc_assert_eq 65 "$mtfc_rc" "malformed inventory refusal"
    mtfc_assert_eq "" "$(cat "$mtfc_case_db")" "malformed inventory no mutation"
    mtfc_assert_path_absent "$mtfc_case_state/owned-rule.v1" \
        "malformed inventory no ledger"
    mtfc_mark_pass "malformed inventory fails before authorization or mutation"
}

mtfc_test_post_add_undercount_retains_ledger() {
    mtfc_prepare_case undercount
    FAKE_HIDE_PATH="$mtfc_case_canonical" mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session undercount \
        --evidence "$mtfc_case_evidence" \
        -- /bin/sh -c 'exit 0'
    mtfc_rc=$?
    mtfc_assert_eq 90 "$mtfc_rc" \
        "unobserved successful add fails cleanup closed"
    mtfc_assert_eq "$mtfc_case_canonical" \
        "$(cat "$mtfc_case_db")" "under-count retained live rule"
    [[ -f "$mtfc_case_state/owned-rule.v1" ]] ||
        mtfc_fail "under-count cleared the owned-rule ledger"
    mtfc_assert_file_contains "$mtfc_case_evidence/summary.txt" \
        "cleanup_verified=false" "under-count cleanup summary"
    mtfc_assert_file_contains "$mtfc_case_evidence/summary.txt" \
        "cleanup_superseded_command=true" "under-count supersession"

    mtfc_recovery_evidence="$mtfc_case_root/recovery-evidence"
    mtfc_run_case "$mtfc_wrapper" \
        --recover \
        --evidence "$mtfc_recovery_evidence"
    mtfc_recovery_rc=$?
    mtfc_assert_eq 0 "$mtfc_recovery_rc" "under-count recovery"
    mtfc_assert_eq "" "$(cat "$mtfc_case_db")" \
        "under-count recovery removed exact rule"
    mtfc_assert_path_absent "$mtfc_case_state/owned-rule.v1" \
        "under-count recovery cleared ledger"
    mtfc_mark_pass "post-add under-count retains recovery state and exit 90"
}

mtfc_test_signal_cleanup() {
    mtfc_signal_name=$1
    mtfc_signal_rc=$2
    mtfc_prepare_case "signal-$mtfc_signal_name"
    mtfc_ready="$mtfc_case_root/ready"
    mtfc_run_case "$mtfc_wrapper" \
        --app "$mtfc_case_app" \
        --session "signal-$mtfc_signal_name" \
        --evidence "$mtfc_case_evidence" \
        -- /bin/sh -c \
        'touch "$1"; kill -"$2" "$PPID"; sleep 5' \
        sh "$mtfc_ready" "$mtfc_signal_name"
    mtfc_rc=$?
    mtfc_assert_eq "$mtfc_signal_rc" "$mtfc_rc" \
        "$mtfc_signal_name result"
    [[ -f "$mtfc_ready" ]] ||
        mtfc_fail "$mtfc_signal_name command did not start"
    mtfc_assert_eq "" "$(cat "$mtfc_case_db")" \
        "$mtfc_signal_name cleanup"
    mtfc_assert_path_absent "$mtfc_case_state/owned-rule.v1" \
        "$mtfc_signal_name ledger cleanup"
    mtfc_mark_pass "$mtfc_signal_name removes the exact owned rule"
}

mtfc_test_success_with_spaces_and_unrelated_entry
mtfc_test_command_failure_still_cleans
mtfc_test_existing_rule_is_never_adopted
mtfc_test_duplicate_rule_is_refused
mtfc_test_add_failure_clears_empty_ledger
mtfc_test_unblock_failure_removes_added_rule
mtfc_test_post_add_duplicate_fails_and_cleans
mtfc_test_remove_failure_overrides_success_and_recovers
mtfc_test_lost_cleanup_authorization_retains_recovery
mtfc_test_initial_authorization_failure_is_nonmutating
mtfc_test_absent_stale_ledger_recovers_without_remove
mtfc_test_absent_stale_ledger_blocks_new_work
mtfc_test_malformed_inventory_fails_before_mutation
mtfc_test_post_add_undercount_retains_ledger
mtfc_test_signal_cleanup INT 130
mtfc_test_signal_cleanup TERM 143

mtfc_assert_eq 16 "$mtfc_passed" "test count"
printf 'PASS: %s (%d cases)\n' "$mtfc_wrapper" "$mtfc_passed"
