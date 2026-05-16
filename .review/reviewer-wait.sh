#!/usr/bin/env bash
set -euo pipefail

# Block until the next review sentinel exists, then print one machine-readable
# wake line and the sentinel payload. This is intentionally one-shot: the
# reviewer grades the returned item, commits the verdict, then runs this helper
# again if they want to keep waiting.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

interval="${REVIEW_POLL_INTERVAL_SECONDS:-5}"
timeout="${REVIEW_WAIT_TIMEOUT_SECONDS:-0}"
start="$(date +%s)"

while true; do
  shopt -s nullglob
  ready_files=(.review/ready/*.json)
  shopt -u nullglob

  if ((${#ready_files[@]} > 0)); then
    IFS=$'\n' sorted=($(printf '%s\n' "${ready_files[@]}" | sort))
    file="${sorted[0]}"
    name="$(basename "$file")"
    echo "READY: $name"
    cat "$file"
    exit 0
  fi

  if [[ "$timeout" != "0" ]]; then
    now="$(date +%s)"
    if ((now - start >= timeout)); then
      echo "NO_READY"
      exit 2
    fi
  fi

  sleep "$interval"
done
