#!/usr/bin/env bash
set -euo pipefail

# Block until the reviewer has graded the given finding id *at the
# current ready-sentinel sha*, then print one machine-readable wake
# line + the verdict payload. Symmetric to
# `.review/reviewer-wait.sh`: where the reviewer's helper blocks on
# the `ready/` directory, this one blocks on a sha-matched verdict
# in `results/`.
#
# Trigger condition: a verdict file
# (`.review/results/<id>.verified.json` or `.review/results/<id>.reopened.md`)
# exists *and* its embedded sha matches the sha named by the
# sentinel `.review/ready/<id>.json`. This makes the wait
# round-aware: round-N+1 won't false-trigger on round-N's preserved
# verdict (audit trail across re-arms).
#
# Sha extraction:
#   verified.json — `"sha":"<sha>"` (40-hex)
#   reopened.md   — `Reviewed sha: \`<sha>\``
#
# Usage:
#   .review/coder-wait.sh <id>
#   .review/coder-wait.sh <id> <expected-sha>   # override
#   REVIEW_POLL_INTERVAL_SECONDS=2 .review/coder-wait.sh <id>
#   REVIEW_WAIT_TIMEOUT_SECONDS=300 .review/coder-wait.sh <id>
#
# Exits:
#   0  — VERDICT printed; verdict's sha matches the expected sha.
#         Caller reads the payload to decide next action.
#   1  — usage error, or no sentinel + no <expected-sha> arg.
#   2  — REVIEW_WAIT_TIMEOUT_SECONDS reached with no matching verdict.
#
# REVIEW_WAIT_TIMEOUT_SECONDS=0 (the default) waits forever.

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $(basename "$0") <id> [<expected-sha>]" >&2
  exit 1
fi

id="$1"
expected_sha_override="${2:-}"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

interval="${REVIEW_POLL_INTERVAL_SECONDS:-5}"
timeout="${REVIEW_WAIT_TIMEOUT_SECONDS:-0}"
start="$(date +%s)"

sentinel=".review/ready/${id}.json"
verified=".review/results/${id}.verified.json"
reopened=".review/results/${id}.reopened.md"

# Snapshot expected sha at startup (sentinel may be deleted by the
# reviewer when they write the verdict; we still need to know what
# sha we were waiting for).
if [[ -n "$expected_sha_override" ]]; then
  expected_sha="$expected_sha_override"
elif [[ -f "$sentinel" ]]; then
  expected_sha="$(grep -o '"sha":"[a-f0-9]*"' "$sentinel" | head -1 | sed 's/.*:"//;s/"$//')"
  if [[ -z "$expected_sha" ]]; then
    echo "sentinel $sentinel missing 'sha' field" >&2
    exit 1
  fi
else
  echo "no sentinel at $sentinel and no <expected-sha> arg — arm a sentinel or pass an explicit sha" >&2
  exit 1
fi

extract_verified_sha() {
  # Tolerate both compact (`"sha":"…"`) and pretty
  # (`"sha": "…"` or `"sha" : "…"`) JSON. Earlier `coder-wait.sh`
  # rounds assumed compact only and exited 1 when the reviewer
  # produced pretty JSON (sentinel deleted → no sha-matched
  # verdict found).
  grep -Eo '"sha"[[:space:]]*:[[:space:]]*"[a-f0-9]+"' "$1" \
    | head -1 \
    | sed -E 's/.*"([a-f0-9]+)"/\1/'
}

extract_reopened_sha() {
  grep -o 'Reviewed sha: `[a-f0-9]*`' "$1" | head -1 | sed 's/^Reviewed sha: `//; s/`$//'
}

while true; do
  if [[ -f "$verified" ]]; then
    vsha="$(extract_verified_sha "$verified")"
    if [[ "$vsha" == "$expected_sha" ]]; then
      echo "VERIFIED: ${id}"
      cat "$verified"
      exit 0
    fi
  fi
  if [[ -f "$reopened" ]]; then
    rsha="$(extract_reopened_sha "$reopened")"
    if [[ "$rsha" == "$expected_sha" ]]; then
      echo "REOPENED: ${id}"
      cat "$reopened"
      exit 0
    fi
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
