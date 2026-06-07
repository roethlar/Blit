#!/usr/bin/env bash
# Doc-consistency lint. Run locally before pushing; CI runs it in docs-gate.yml.
set -u
cd "$(git rev-parse --show-toplevel 2>/dev/null || pwd)" || exit 1
fail=0

req() {
  if [ ! -f "$1" ]; then
    echo "MISSING: $1"
    fail=1
  fi
}
req docs/STATE.md
req docs/DECISIONS.md
req docs/agent/PROTOCOL.md
req AGENTS.md
req CLAUDE.md

if [ -f docs/STATE.md ]; then
  grep -q '^Last updated:' docs/STATE.md || {
    echo "docs/STATE.md: missing 'Last updated:' line"
    fail=1
  }
  lines=$(wc -l < docs/STATE.md | tr -d ' ')
  if [ "$lines" -gt 200 ]; then
    echo "docs/STATE.md is $lines lines (cap 200) — prune handoffs into DEVLOG.md"
    fail=1
  fi
fi

for f in docs/plan/*.md; do
  [ -e "$f" ] || continue
  case "$f" in */README.md) continue ;; esac
  first=$(grep -m1 -E '^\*\*Status\*\*:' "$f" || true)
  if [ -z "$first" ]; then
    echo "$f: missing '**Status**:' header (Draft|Active|Shipped|Superseded|Historical)"
    fail=1
  elif ! printf '%s\n' "$first" | grep -qE '^\*\*Status\*\*:[[:space:]]*(Draft|Active|Shipped|Superseded|Historical)([[:space:]]|$)'; then
    echo "$f: first '**Status**:' line is off-vocabulary ($first) — expected Draft|Active|Shipped|Superseded|Historical"
    fail=1
  fi
done

if [ "$fail" -eq 0 ]; then
  echo "check-docs: OK"
fi
exit "$fail"
