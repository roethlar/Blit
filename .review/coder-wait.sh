#!/usr/bin/env bash
set -euo pipefail

# Block until the reviewer has graded the given finding id, then print one
# machine-readable wake line + the verdict payload. Symmetric to
# `.review/reviewer-wait.sh`: where the reviewer's helper blocks on the
# `ready/` directory, this one blocks on `results/`.
#
# Usage:
#   .review/coder-wait.sh <id>
#   REVIEW_POLL_INTERVAL_SECONDS=2 .review/coder-wait.sh <id>
#   REVIEW_WAIT_TIMEOUT_SECONDS=300 .review/coder-wait.sh <id>
#
# Exits:
#   0  — VERDICT printed; reviewer accepted (verified.json) or reopened
#         (reopened.md). Coder reads the file to decide next action.
#   1  — usage error.
#   2  — REVIEW_WAIT_TIMEOUT_SECONDS reached with no verdict.
#
# REVIEW_WAIT_TIMEOUT_SECONDS=0 (the default) waits forever.

if [[ $# -ne 1 ]]; then
  echo "usage: $(basename "$0") <id>" >&2
  exit 1
fi

id="$1"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

interval="${REVIEW_POLL_INTERVAL_SECONDS:-5}"
timeout="${REVIEW_WAIT_TIMEOUT_SECONDS:-0}"
start="$(date +%s)"

verified=".review/results/${id}.verified.json"
reopened=".review/results/${id}.reopened.md"

while true; do
  if [[ -f "$verified" ]]; then
    echo "VERIFIED: ${id}"
    cat "$verified"
    exit 0
  fi
  if [[ -f "$reopened" ]]; then
    echo "REOPENED: ${id}"
    cat "$reopened"
    exit 0
  fi

  if [[ "$timeout" != "0" ]]; then
    now="$(date +%s)"
    if ((now - start >= timeout)); then
      echo "NO_VERDICT"
      exit 2
    fi
  fi

  sleep "$interval"
done
