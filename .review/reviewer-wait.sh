#!/usr/bin/env bash
set -euo pipefail

# Block until the next review sentinel exists *and is committed*,
# then print one machine-readable wake line and the sentinel
# payload. One-shot: reviewer grades, commits the verdict, then
# re-runs the helper for the next item.
#
# Round-2 of a0-pull-execution caught a race here: the original
# version woke as soon as a sentinel file appeared on disk,
# including untracked or staged-but-not-committed files. That
# let the reviewer wake on a state the coder hadn't yet
# committed — and write a verdict before the sentinel landed
# in HEAD. The contract is now: a sentinel only counts when
# `git ls-files` knows about it AND the working tree + index
# are clean for that path.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

interval="${REVIEW_POLL_INTERVAL_SECONDS:-5}"
timeout="${REVIEW_WAIT_TIMEOUT_SECONDS:-0}"
start="$(date +%s)"

# Return success if `$1` is a tracked path with no
# uncommitted/staged changes — i.e. the version on disk
# matches HEAD.
sentinel_is_committed() {
  local file="$1"
  git ls-files --error-unmatch -- "$file" >/dev/null 2>&1 || return 1
  git diff --quiet -- "$file" || return 1
  git diff --cached --quiet -- "$file" || return 1
  return 0
}

while true; do
  shopt -s nullglob
  ready_files=(.review/ready/*.json)
  shopt -u nullglob

  if ((${#ready_files[@]} > 0)); then
    IFS=$'\n' sorted=($(printf '%s\n' "${ready_files[@]}" | sort))
    for file in "${sorted[@]}"; do
      if sentinel_is_committed "$file"; then
        name="$(basename "$file")"
        echo "READY: $name"
        cat "$file"
        exit 0
      fi
    done
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
