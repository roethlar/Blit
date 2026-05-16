#!/usr/bin/env bash
set -euo pipefail

# Workflow-state lint. Verifies the invariants both agents rely on:
#
#   1. Every .review/ready/<id>.json has a matching
#      .review/findings/<id>.md.
#   2. A sentinel may only coexist with a verdict for the *previous*
#      round — i.e. when the coder re-armed for round N+1 after a
#      reopen, the round-N reopened.md is preserved as audit trail.
#      A sentinel pointing at the *same* sha as a verdict file means
#      the reviewer forgot to delete the sentinel after writing the
#      verdict (workflow bug).
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

    # Collision: a sentinel + verdict with the *same* sha means the
    # reviewer forgot to delete the sentinel after writing the
    # verdict. A sentinel + verdict with *different* shas is the
    # normal re-arm-after-reopen audit trail.
    verified=".review/results/${id}.verified.json"
    reopened=".review/results/${id}.reopened.md"
    if [[ -f "$verified" ]]; then
      vsha="$(grep -o '"sha":"[^"]*"' "$verified" | head -1 | sed 's/.*:"//;s/"$//' || true)"
      if [[ -n "$json_sha" && -n "$vsha" && "$json_sha" == "$vsha" ]]; then
        warn "$name: collides with $verified at same sha ${vsha:0:7} — reviewer must delete the sentinel after writing verdict"
      fi
    fi
    if [[ -f "$reopened" ]]; then
      # reopened.md has "Reviewed sha: \`<sha>\`" as free text.
      rsha="$(grep -o 'Reviewed sha: \`[a-f0-9]*\`' "$reopened" | head -1 | sed 's/.*\`//;s/\`$//' || true)"
      if [[ -n "$json_sha" && -n "$rsha" && "$json_sha" == "$rsha" ]]; then
        warn "$name: collides with $reopened at same sha ${rsha:0:7} — reviewer must delete the sentinel after writing verdict"
      fi
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
  #
  # `"${arr[@]+"${arr[@]}"}"` is the bash 3.2-safe way to iterate
  # a possibly-empty array under `set -u`. The naive
  # `"${arr[@]}"` form treats an empty nullglob-assigned array
  # as an unset variable and aborts (`arr[@]: unbound variable`).
  # Only one of the two arrays may be empty in a clean
  # post-verdict state, so we must guard both loops.
  seen_ids=""
  for f in ${verified_files[@]+"${verified_files[@]}"}; do
    name="$(basename "$f")"
    id="${name%.verified.json}"
    seen_ids="$seen_ids"$'\n'"$id"
    ok "$name"
  done
  for f in ${reopened_files[@]+"${reopened_files[@]}"}; do
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
