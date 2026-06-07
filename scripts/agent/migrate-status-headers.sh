#!/usr/bin/env bash
# One-shot migration: normalize **Status** headers in docs/plan/*.md to the
# controlled vocabulary (Draft|Active|Shipped|Superseded|Historical) required by
# check-docs.sh. Mapping derived from docs/plan/README.md's own classification
# (2026-06-04 M28 sweep). Review the diff and adjust any value you disagree with.
# Safe to delete after running. Portable across GNU/macOS (uses perl).
set -u
cd "$(git rev-parse --show-toplevel 2>/dev/null || pwd)" || exit 1

status_for() {
  case "$(basename "$1")" in
    TUI_REWORK.md)                  echo "Active" ;;     # live per README; sign-off note moves to body
    greenfield_plan_v6.md)          echo "Active" ;;     # §1.1 ratified 2026-06-04 (D-2026-06-04-3)
    MASTER_WORKFLOW.md)             echo "Active" ;;     # live per README
    RELEASE_PLAN_v2_2026-05-04.md)  echo "Shipped" ;;    # 0.1.0 tagged 2026-05-31
    REMOTE_TRANSFER_PARITY.md)      echo "Shipped" ;;    # README: shipped
    WORKFLOW_PHASE_2.md)            echo "Shipped" ;;    # README: complete
    WORKFLOW_PHASE_2.5.md)          echo "Shipped" ;;
    WORKFLOW_PHASE_3.md)            echo "Shipped" ;;
    WORKFLOW_PHASE_4.md)            echo "Shipped" ;;
    TUI_DESIGN.md)                  echo "Superseded" ;; # by TUI_REWORK.md (D-2026-05-31-2)
    *)                              echo "Historical" ;; # default for retained-for-rationale docs
  esac
}

for f in docs/plan/*.md; do
  [ -e "$f" ] || continue
  case "$f" in */README.md|*/TEMPLATE.md) continue ;; esac

  # Already valid? Judge by the FIRST Status line only — embedded section-level
  # Status lines deeper in a doc must not mask an off-vocabulary header.
  first="$(grep -m1 -E '^\*\*Status\*\*:' "$f" || true)"
  if [ -n "$first" ] && printf '%s\n' "$first" | \
     grep -qE '^\*\*Status\*\*:[[:space:]]*(Draft|Active|Shipped|Superseded|Historical)([[:space:]]|$)'; then
    continue
  fi

  val="$(status_for "$f")"
  if [ -n "$first" ]; then
    # Rewrite the first existing Status line, preserving its old value as a note.
    old="$(printf '%s\n' "$first" | sed 's/^\*\*Status\*\*:[[:space:]]*//')"
    STATUS_VAL="$val" STATUS_OLD="$old" perl -0pi -e \
      's/^\*\*Status\*\*:[^\n]*/**Status**: $ENV{STATUS_VAL} (was: $ENV{STATUS_OLD})/m' "$f"
  else
    # Insert after the first line (the title).
    STATUS_VAL="$val" perl -0pi -e \
      's/\A([^\n]*\n)/$1\n**Status**: $ENV{STATUS_VAL}\n/' "$f"
  fi
  echo "stamped: $f -> $val"
done

echo "Done. Review with: git diff docs/plan/  — then run scripts/agent/check-docs.sh"
