#!/usr/bin/env bash
# PreCompact hook: steers Claude Code's compaction summary toward the facts that
# long sessions actually lose. JSON on stdout per the hooks reference.
cat <<'JSON'
{"hookSpecificOutput":{"hookEventName":"PreCompact","customInstructions":"Preserve verbatim: (1) the active finding/slice ID and its acceptance criteria; (2) full paths of every file modified or in flight; (3) all owner requirements and decisions stated this session that are not yet written to docs/; (4) unresolved reviewer feedback; (5) the next unexecuted step. After compaction, re-read docs/STATE.md and the active plan doc named there before taking any action."}}
JSON
exit 0
