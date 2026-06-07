# STATE — single entry point for "what is true right now"

Last updated: 2026-06-07 (rule codification + merge) at commit `ca940a2`

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **Agent-protocol kit install** (completed). Merged branch and codified branchless rules in AGENTS.md.
- Repository note: `master` HEAD is `ca940a2`. The temporary `chore/agent-kit-install` branch has been merged and deleted.

## Queue (ordered)

1. **Resume audit-h3c.** Slice 1 (`bf4cc82`) is `[~]` pending reviewer verdict
   (`.review/ready/`); on the verdict, do **audit-h3c slice 2** — dynamic
   progress watchdog wrapping `recv_fallback_message` (the chokepoint slice 1
   created).
2. **Land adaptive-streams** (D-2026-06-07-2) — cherry-pick/rebase the stack up
   to `eafb187` (live-progress → PR1 telemetry → PR2 work-queue → PR2 review
   fix), excluding `d9d4ec7` (does-not-build WIP). Resolve the `data_plane.rs`
   StallGuard-vs-`Probe` conflict by hand. Write a `docs/plan/` doc first
   (no code before `**Status**: Active`).
3. Finish audit **Round 1** (data-loss / DoS class) per
   `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md` — R3 order governs.
4. **Round 2 — Phase 6 TUI rework** (`docs/plan/TUI_REWORK.md`):
   H4 → H5 → R3-H23 → H2 → H6 → H7 → H8 → M2 → M3 → M4 → M25.
   (R3-M28 source-of-truth sweep completed 2026-06-04.)
5. `greenfield_plan_v6.md` §1.1 streaming planner + 1 s heartbeat + 10 s stall
   detector — owner-ratified, not yet built (H10b); queued after Round 1 closes.

## Authoritative docs right now

- Findings: `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md` (read R2 + R3 delta;
  R3 overrides R2 on conflicts)
- Plan: `docs/plan/TUI_REWORK.md` (Phase 6; gated on Round 1 completion)
- Review loop: `REVIEW.md` + `.review/README.md`

## Blocked / waiting

- **Owner approval for git operations** (AGENTS.md §8), exact actions pending:
  - Stale branches pending deletion **by explicit name** (each verified ahead=0
    vs `master`, i.e. fully contained): `phase5/a1` and `phase5/blit-app-extract`,
    which exist **only** on the remotes (`origin` + `gitea`) — no local refs.
    Deletion is a remote `push --delete`. Owner names each branch before any
    deletion. (`claude/vigilant-mayer` is already gone as a ref; only the
    orphaned dir `.claude/worktrees/vigilant-mayer/` remains, untracked + ignored.)

## Open questions

- `docs/agent/SETUP.md` content — must be supplied by the owner (it lives on
  another machine). Until then `.review/README.md` still points at the
  unreadable `/Users/michael/Dev/SETUP.md` (line 8) and `cd /Users/michael/Dev/Blit`
  (line 101). Vendor + reference-fix is deferred to that input.
- Disposition of the adaptive-streams branch refs after the feature lands
  (D-2026-06-07-2): keep for history, or delete by name.

## Handoff log (newest first, keep ≤ 3)

- **2026-06-07** @ `ca940a2` — Merged agent-kit into master, deleted the branch, and codified branchless rules in AGENTS.md per owner command. Verified workspace builds and doc checks pass.
- **2026-06-07** @ `c793df2` — Installed the agent-protocol kit and reconciled
  the branch mess: documented the `-s ours` octopus trap (DECISIONS.md
  D-2026-06-07-1/-2, AGENTS.md §8), migrated plan-doc status headers, fixed
  `.gitignore`. Tree == `600023a`, builds. Awaiting owner approval for the first
  commit + named branch deletions; SETUP.md content still needed. First action
  next session: `catchup`, then check the audit-h3c slice 1 verdict.
- **2026-06-06** @ `600023a` — Agent-protocol kit authored against master.
  Prior state imported from `docs/plan/README.md`, the 2026-06-04 audit index,
  and `REVIEW.md`.
