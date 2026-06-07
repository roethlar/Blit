#!/usr/bin/env bash
# Prints the catchup report directly from docs/STATE.md and REVIEW.md.
# The agent's only job: run this, show the output unchanged, add one
# "Proposed first action:" line. The report cannot drift, because no one
# composes it — this script does.
set -u
cd "$(git rev-parse --show-toplevel 2>/dev/null || pwd)" || exit 1
[ -f docs/STATE.md ] || { echo "catchup: docs/STATE.md missing" >&2; exit 1; }

section() {
  awk -v want="$1" '
    /^## /   { on = (tolower($0) ~ "^## " tolower(want)); next }
    on       { print }
  ' docs/STATE.md
}

items() {
  section "$1" | awk '
    function flush() { if (buf != "") { gsub(/  +/, " ", buf); print buf }; buf = "" }
    /^[-*] /        { flush(); buf = substr($0, 3); next }
    /^[0-9]+\. /    { flush(); sub(/^[0-9]+\. /, ""); buf = $0; next }
    /^[[:space:]]/  { line = $0; sub(/^[[:space:]]+/, "", line);
                      if (buf != "" && line != "") buf = buf " " line; next }
    NF              { flush(); buf = $0; next }
    END             { flush() }
  '
}

emit() {
  label="$1"; sect="$2"
  list=$(items "$sect")
  if [ -z "$list" ]; then
    echo "$label: (no $sect section found in STATE.md)"
  elif [ "$(printf '%s\n' "$list" | wc -l)" -eq 1 ]; then
    echo "$label: $list"
  else
    echo "$label:"
    printf '%s\n' "$list" | sed 's/^/  - /'
  fi
}

now=$(items "Now" | head -1);   [ -n "$now" ]  || now="(no Now section found in STATE.md)"
next=$(items "Queue" | head -1); [ -n "$next" ] || next="(no Queue section found in STATE.md)"
echo "Now: $now"
echo "Next: $next"
emit "Blocked" "Blocked"
emit "Open questions" "Open questions"

if [ -f REVIEW.md ]; then
  open=$(grep -c '^|.*\[ \]' REVIEW.md || true)
  pend=$(grep -c '^|.*\[~\]' REVIEW.md || true)
  pend_ids=$(grep '^|.*\[~\]' REVIEW.md | awk -F'|' '{gsub(/^ +| +$/,"",$2); print $2}' | paste -sd ', ' -)
  echo "Review loop: $open open, $pend pending review${pend_ids:+ (pending: $pend_ids)}"
else
  echo "Review loop: (REVIEW.md not found)"
fi
