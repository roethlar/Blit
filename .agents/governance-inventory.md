# Existing Governance Inventory

Verdicts: **migrate** (content moves / file is edited toward the standard layout),
**supersede** (file stays with a banner pointing at a replacement), **leave**
(stays untouched).

This repo's governance was installed and owner-ratified on 2026-06-07 and is
mechanically enforced (CI docs gate, session hooks, lint script). The migration
therefore adds a thin standard-layout overlay instead of relocating working
files. No file is superseded; nothing earns a banner.

| Artifact | Role today | Verdict | Destination | Notes |
| --- | --- | --- | --- | --- |
| `AGENTS.md` | Canonical cross-agent contract (Claude Code, Codex, Antigravity) | migrate | `AGENTS.md` | Extended in place: new §10 Bootstrap Handoff + one plain-English line in §9. No existing rule is removed or weakened. |
| `CLAUDE.md` | Claude Code shim (`@AGENTS.md` import + harness specifics) | leave | - | Already a thin conformant shim. |
| `GEMINI.md` | Gemini/Antigravity shim pointing at `AGENTS.md` | leave | - | Already a thin conformant shim. |
| `docs/STATE.md` | Single entry point for current state (active work, queue, blockers) | leave | - | Stays canonical. CI docs gate, `scripts/agent/*` hooks, and PROTOCOL.md are wired to this exact path. New `.agents/state.md` is a pointer stub, never a second copy. |
| `docs/DECISIONS.md` | Append-only ledger of settled decisions | leave | - | Stays canonical, same reasoning. New `.agents/decisions.md` is a pointer stub. |
| `docs/agent/PROTOCOL.md` | Procedures behind the trigger vocabulary (catchup, plan, decision, handoff, drift, slice) | leave | - | Working, referenced by all harness wrappers. |
| `DEVLOG.md` | Append-only journal, newest first | leave | - | History, not state — exactly the standard treatment for journals. |
| `TODO.md` | Long-horizon backlog | leave | - | AGENTS.md already declares it backlog-only. |
| `REVIEW.md` + `.review/` | Two-agent coder/reviewer loop: status index, findings, sentinels, verdicts | leave | - | Working system with committed audit trail. Known wart: `.review/README.md` references paths on another machine (`/Users/michael/...`); already an open question in `docs/STATE.md`. |
| `.claude/commands/*.md` | Slash-command wrappers resolving to PROTOCOL.md | leave | - | Already the thin pointers the standard asks for. |
| `.claude/settings.json` | SessionStart context-inject + PreCompact hooks | leave | - | |
| `.agents/skills/{catchup,handoff}` | Antigravity workspace skills mirroring the trigger vocabulary | leave | - | |
| `scripts/agent/*` (`context.sh`, `catchup.sh`, `precompact.sh`, `check-docs.sh`, `migrate-status-headers.sh`) | Hook scripts + doc lint | leave | - | `check-docs.sh` hard-requires `docs/STATE.md`, `docs/DECISIONS.md`, `docs/agent/PROTOCOL.md`, `AGENTS.md`, `CLAUDE.md`. |
| `.github/workflows/docs-gate.yml` | CI: code changes must touch `docs/STATE.md`; doc lint | leave | - | The enforcement that makes `docs/STATE.md` the load-bearing path. |
| `docs/plan/*` | Plan docs with mandatory `**Status**:` headers | leave | - | Status vocabulary is lint-enforced. |
| `docs/audit/*`, `docs/reviews/*` | Audit and review reports (historical/reference) | leave | - | |
| `.serena/memories/` | Tool-local scratch | leave | - | Declared non-authoritative by AGENTS.md §1. |

New files added by this migration (no prior counterpart): `.agents/state.md`
(pointer), `.agents/decisions.md` (pointer), `.agents/repo-map.json`,
`.agents/artifact-manifest.json`, `.agents/governance-inventory.md` (this file),
`.agents/harvest.md` (harvest report; no dropbox configured on this machine).

## Supersession Banner

Not applicable — no artifact received a supersede verdict.
