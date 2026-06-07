# CLAUDE.md

@AGENTS.md

## Claude Code specifics

- **Session context is auto-injected.** A `SessionStart` hook in
  `.claude/settings.json` runs `scripts/agent/context.sh`, which prints
  `docs/STATE.md` plus a git summary into context — on startup, resume, clear,
  and after every compaction. If that banner is missing for any reason, read
  `docs/STATE.md` yourself before acting.
- **Compaction is lossy.** A `PreCompact` hook tells the summarizer what to
  preserve (active finding ID, in-flight file paths, unexecuted steps, reviewer
  feedback), but treat any post-compaction summary as unreliable: re-read
  `docs/STATE.md` and the active plan doc before the next action.
- **Slash commands:** `/catchup`, `/plan`, `/decision`, `/handoff`, `/drift`,
  `/slice` — thin wrappers over `docs/agent/PROTOCOL.md`.
- **Plan mode:** use it for non-trivial work, but an approved in-chat plan must
  still be written to `docs/plan/` (or a finding doc) before implementation.
  Plan-mode output is not durable.
- **Auto memory:** personal scratch only. Where memory disagrees with
  `docs/STATE.md` or `docs/DECISIONS.md`, the files win.
