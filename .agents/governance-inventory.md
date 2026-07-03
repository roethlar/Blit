# Existing Governance Inventory

Verdicts: **migrate** (content moves / file is edited toward the standard layout),
**supersede** (file stays with a banner pointing at a replacement), **leave**
(stays untouched).

This repo's governance was installed and owner-ratified on 2026-06-07 and is
mechanically enforced (CI docs gate, session hooks, lint script). The 2026-06-07
migration added a thin standard-layout overlay instead of relocating working
files; no file was superseded.

This entry records the **2026-07-03 reconciliation**: the toolkit's
`AGENTS.template.md` gained a Prime Invariants block and a `playbook` operator
since 2026-06-07, and moved to a stricter model where `AGENTS.md` must be a
byte-identical copy of the template with all repo-specific content carved into
`.agents/repo-guidance.md`. This repo's `AGENTS.md` had never adopted that
carve-out (it carried repo-specific content — project map, style, git safety —
directly), so `agentsTemplate.reconcileRecommended` came back true. Still no
file is superseded; the working system (`docs/STATE.md`, `docs/DECISIONS.md`,
`docs/agent/PROTOCOL.md`, `REVIEW.md`/`.review/`) is unchanged in role, just
now referenced from `.agents/repo-guidance.md` instead of `AGENTS.md` directly.

| Artifact | Role today | Verdict | Destination | Notes |
| --- | --- | --- | --- | --- |
| `AGENTS.md` | Canonical cross-agent contract (Claude Code, Codex, Antigravity) | migrate | `AGENTS.md` | Replaced whole with the current template verbatim (2026-07-02.1): adds the Prime Invariants block and `playbook` operator. All repo-specific content it used to carry (project map, style, git safety specifics, source-of-truth order, operator procedures) moved to `.agents/repo-guidance.md`, generalized and re-verified against current repo evidence. No existing rule is removed or weakened — see the "Earned Practices" section of the new `.agents/repo-guidance.md` for git safety, carried forward verbatim in substance. |
| `.agents/repo-guidance.md` | New file — carve-out target for everything specific to this repo | migrate | `.agents/repo-guidance.md` | Did not exist before this round; the toolkit only requires it as of the current template. Holds mission detail, reading order, the repo's `slice` operator (no generic-template equivalent), verification commands, remotes, git-safety earned practices, style, and project map. |
| `.agents/push-policy.md` | New file — narrow push-only policy the current template's Prime Invariants block points to | migrate | `.agents/push-policy.md` | Set to `ask`, consistent with this repo's existing (stricter) git-safety rules, which stay the authoritative detail in `.agents/repo-guidance.md`. |
| `CLAUDE.md` | Claude Code shim (`@AGENTS.md` import + harness specifics) | leave | - | Already a thin conformant shim; still correct after the `AGENTS.md` rewrite since it just imports the file. |
| `GEMINI.md` | Gemini/Antigravity shim pointing at `AGENTS.md` | leave | - | Already a thin conformant shim. |
| `docs/STATE.md` | Single entry point for current state (active work, queue, blockers) | leave | - | Stays canonical. CI docs gate, `scripts/agent/*` hooks, and PROTOCOL.md are wired to this exact path. `.agents/state.md` is a pointer stub, never a second copy. |
| `docs/DECISIONS.md` | Append-only ledger of settled decisions | leave | - | Stays canonical, same reasoning. `.agents/decisions.md` is a pointer stub. |
| `docs/agent/PROTOCOL.md` | Procedures behind the trigger vocabulary (catchup, plan, decision, handoff, drift, slice) | leave | - | Working, referenced by all harness wrappers and now by `.agents/repo-guidance.md`'s Operator Vocabulary section. |
| `DEVLOG.md` | Append-only journal, newest first | leave | - | History, not state — exactly the standard treatment for journals. |
| `TODO.md` | Long-horizon backlog | leave | - | AGENTS.md already declares it backlog-only. |
| `REVIEW.md` + `.review/` | Two-agent coder/reviewer loop: status index, findings, sentinels, verdicts | leave | - | Working system with committed audit trail. |
| `.claude/commands/*.md` | Slash-command wrappers resolving to PROTOCOL.md | migrate (one addition) | `.claude/commands/playbook.md` | Existing six wrappers (catchup/decision/drift/handoff/plan/slice) are already the thin pointers the standard asks for and are unchanged. `playbook.md` is new: the current template defines a `playbook` operator this repo didn't have a wrapper for. |
| `.claude/settings.json` | SessionStart context-inject + PreCompact hooks | migrate (additive merge) | `.claude/settings.json` | Existing `SessionStart` (`context.sh`, prints `docs/STATE.md` on every start/resume/clear/compact) and `PreCompact` (`precompact.sh`, steers the compaction summarizer) hooks already exceed the toolkit's generic "SessionStart matcher:compact echo pointing at Prime Invariants" re-ground hook in this repo — that generic hook is intentionally **not** added; it would fire redundantly alongside `context.sh` on every compaction. The toolkit's `PreToolUse` AGENTS.md pre-edit tripwire is genuinely new (this repo had no equivalent) and is merged in as an additional hook entry alongside the existing two. |
| `.claude/agents-md-tripwire.py` | New file — advisory pre-edit reminder for AGENTS.md | migrate | `.claude/agents-md-tripwire.py` | Copied verbatim from the toolkit template; now directly relevant since `AGENTS.md` is a gated-write-only file after this reconciliation. |
| `.agents/skills/{catchup,handoff}` | Antigravity workspace skills mirroring the trigger vocabulary | leave | - | |
| `scripts/agent/*` (`context.sh`, `catchup.sh`, `precompact.sh`, `check-docs.sh`, `migrate-status-headers.sh`) | Hook scripts + doc lint | leave | - | `check-docs.sh` hard-requires `docs/STATE.md`, `docs/DECISIONS.md`, `docs/agent/PROTOCOL.md`, `AGENTS.md`, `CLAUDE.md`. |
| `.github/workflows/docs-gate.yml` | CI: code changes must touch `docs/STATE.md`; doc lint | leave | - | The enforcement that makes `docs/STATE.md` the load-bearing path. |
| `docs/plan/*` | Plan docs with mandatory `**Status**:` headers | leave | - | Status vocabulary is lint-enforced. |
| `docs/audit/*`, `docs/reviews/*` | Audit and review reports (historical/reference) | leave | - | |
| `.serena/memories/` | Tool-local scratch | leave | - | Declared non-authoritative by `AGENTS.md` Universal Invariants. |

Files updated but not newly created by this round: `.agents/state.md`,
`.agents/decisions.md`, `.agents/repo-map.json`,
`.agents/artifact-manifest.json` (custody/paths refreshed to include the two
new files and the current commit).

## Supersession Banner

Not applicable — no artifact received a supersede verdict.

## Out-of-scope observation (flagged, not acted on)

The commit `725aa07` ("chore: track claude worktrees?") added 236 files under
`.claude/worktrees/vigilant-mayer/` to git tracking — a full copy of a Claude
Code worktree, including its own nested `.serena/memories/`, `Cargo.toml`,
source tree, etc. This is unrelated to governance and outside this
reconciliation's scope; noted here only because discovery surfaced it as a
governance-adjacent path under `.claude/`. Worth a deliberate owner decision
(intentional tracking vs. accidental `git add`), but not touched by this run.
