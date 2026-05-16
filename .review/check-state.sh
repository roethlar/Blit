#!/usr/bin/env bash
set -euo pipefail

# Workflow-state lint. Verifies the invariants both agents rely on:
#
#   1. Every .review/ready/<id>.json has a matching
#      .review/findings/<id>.md.
#   2. No .review/results/<id>.{verified.json,reopened.md} collides
#      with an existing .review/ready/<id>.json (the reviewer must
#      delete the sentinel when writing a verdict).
#   3. Both verdict files for the same id must not coexist.
#   4. Sentinel JSON has the four required fields (id, branch, sha,
#      ts) and the id matches the filename.
#
# Exit 0 if state is consistent; exit 1 if any invariant is
# violated. Output is human-readable; intended for ad-hoc invocation
# by either agent (or a human) to confirm nothing is wedged.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

problems=0

warn() {
  echo "  ✗ $*" >&2
  problems=$((problems + 1))
}

ok() {
  echo "  ✓ $*"
}

shopt -s nullglob

echo "== ready/ sentinels =="
ready_files=(.review/ready/*.json)
if ((${#ready_files[@]} == 0)); then
  ok "no pending sentinels"
else
  for f in "${ready_files[@]}"; do
    name="$(basename "$f")"
    id="${name%.json}"
    finding=".review/findings/${id}.md"

    if [[ ! -f "$finding" ]]; then
      warn "$name: missing finding doc $finding"
    fi

    # Schema: id, branch, sha, ts must all be present + id must match filename.
    json_id="$(grep -o '"id":"[^"]*"' "$f" | head -1 | sed 's/.*:"//;s/"$//' || true)"
    json_branch="$(grep -o '"branch":"[^"]*"' "$f" | head -1 | sed 's/.*:"//;s/"$//' || true)"
    json_sha="$(grep -o '"sha":"[^"]*"' "$f" | head -1 | sed 's/.*:"//;s/"$//' || true)"
    json_ts="$(grep -o '"ts":"[^"]*"' "$f" | head -1 | sed 's/.*:"//;s/"$//' || true)"

    [[ -z "$json_id" ]] && warn "$name: missing 'id' field"
    [[ -z "$json_branch" ]] && warn "$name: missing 'branch' field"
    [[ -z "$json_sha" ]] && warn "$name: missing 'sha' field"
    [[ -z "$json_ts" ]] && warn "$name: missing 'ts' field"
    if [[ -n "$json_id" && "$json_id" != "$id" ]]; then
      warn "$name: filename id ($id) doesn't match payload id ($json_id)"
    fi

    if [[ "$problems" == "0" ]]; then
      ok "$name (branch=$json_branch sha=${json_sha:0:7})"
    fi

    # Collision: a sentinel with a verdict file already present is wedged.
    verified=".review/results/${id}.verified.json"
    reopened=".review/results/${id}.reopened.md"
    if [[ -f "$verified" ]]; then
      warn "$name: collides with existing $verified — reviewer must delete the sentinel after writing verdict"
    fi
    if [[ -f "$reopened" ]]; then
      warn "$name: collides with existing $reopened — reviewer must delete the sentinel after writing verdict"
    fi
  done
fi

echo ""
echo "== results/ verdicts =="
verified_files=(.review/results/*.verified.json)
reopened_files=(.review/results/*.reopened.md)

if ((${#verified_files[@]} == 0 && ${#reopened_files[@]} == 0)); then
  ok "no verdict files yet"
else
  # macOS ships bash 3.2 which lacks `declare -A`; use a
  # newline-separated list instead. Verdict counts are tiny, so
  # the O(n) scan per insert is fine.
  seen_ids=""
  for f in "${verified_files[@]}"; do
    name="$(basename "$f")"
    id="${name%.verified.json}"
    seen_ids="$seen_ids"$'\n'"$id"
    ok "$name"
  done
  for f in "${reopened_files[@]}"; do
    name="$(basename "$f")"
    id="${name%.reopened.md}"
    if printf '%s\n' "$seen_ids" | grep -Fxq "$id"; then
      warn "$id: both verified and reopened verdict files present — pick one"
    fi
    ok "$name"
  done
fi

shopt -u nullglob

echo ""
if ((problems == 0)); then
  echo "STATE: consistent"
  exit 0
else
  echo "STATE: $problems problem(s) — see above"
  exit 1
fi
