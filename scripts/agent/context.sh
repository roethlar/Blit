#!/usr/bin/env bash
# Prints session-grounding context. Wired into Claude Code's SessionStart hook
# (stdout is injected into context on startup/resume/clear/compact). Codex and
# Antigravity users: AGENTS.md directs the agent to read docs/STATE.md directly,
# or run this script manually.
set -u
cd "$(git rev-parse --show-toplevel 2>/dev/null || pwd)" || exit 0

echo "=== SESSION CONTEXT (auto-injected; see AGENTS.md) ==="
echo "--- git ---"
echo "branch: $(git branch --show-current 2>/dev/null || echo '?')"
git log --oneline -3 2>/dev/null
dirty=$(git status --short 2>/dev/null | head -15)
if [ -n "$dirty" ]; then
  echo "uncommitted:"
  echo "$dirty"
fi
echo
echo "--- docs/STATE.md ---"
if [ -f docs/STATE.md ]; then
  cat docs/STATE.md
else
  echo "MISSING — create it from the handoff procedure in docs/agent/PROTOCOL.md"
fi
echo "=== END SESSION CONTEXT ==="
exit 0
