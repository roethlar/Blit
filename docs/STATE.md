# STATE — single entry point for "what is true right now"

Last updated: 2026-06-11 (h3c slice 1 verified; repo design-coherence review being planned) at commit `1be16bc`

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **audit-h3c slice 1 verified** (2026-06-11, owner accept, verdict in
  `.review/results/`). The review assessment found 4 facts feeding the slice-2
  re-scope — see DEVLOG 2026-06-11 entry: 1 MiB cap is correctness vs tonic's
  4 MiB decode default; client channels lack HTTP/2 keepalive (the real H3
  hang); fallback throughput is h2 flow-control-window-bound; the clamp
  collapses to a constant (sinks' `chunk_bytes` now inert).
- **Design-coherence review: Phases A + B COMPLETE** (2026-06-11). Map at
  `docs/audit/DESIGN_MAP_2026-06-11.md`; verified findings at
  `docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md` — 70 confirmed
  (4 high / 40 medium / 26 low after adversarial severity correction),
  6 refuted-and-recorded. Three bug-class findings already filed as `[ ]`
  Open (design-1/2/3 in REVIEW.md). **At the Phase B checkpoint: owner
  go/no-go for Phase C** (synthesis: dedup the 70 into ranked slices,
  candidate finding docs, queue proposal — owner ratifies each individually
  per D-2026-06-11-1).

## Queue (ordered)

1. **Repo design-coherence review** — `docs/plan/DESIGN_COHERENCE_REVIEW.md`
   **Active** (D-2026-06-11-1). Phase A done → map in
   `docs/audit/DESIGN_MAP_2026-06-11.md`. Awaiting owner: Phase B go/no-go,
   and whether to file the map's 4 bug-class candidates as findings now.
2. **audit-h3c slice 2 (re-scoped, pending review findings + plan doc):**
   transport-policy first — single shared client channel builder with HTTP/2
   keepalive, explicit `max_decoding_message_size`, adaptive flow-control
   windows; error-chain preservation (tonic Status → eyre, retry classifier);
   delete inert sink `chunk_bytes` params; then re-evaluate whether a cadence
   watchdog is still needed (wedged-but-alive peers only). Held until the
   review's transport findings are in, so the re-scope is designed once.
3. **Land adaptive-streams** (D-2026-06-07-2) — cherry-pick/rebase the stack up
   to `eafb187` (live-progress → PR1 telemetry → PR2 work-queue → PR2 review
   fix), excluding `d9d4ec7` (does-not-build WIP). Resolve the `data_plane.rs`
   StallGuard-vs-`Probe` conflict by hand. Write a `docs/plan/` doc first
   (no code before `**Status**: Active`). Held until the coherence map exists
   (agent-proposed sequencing; owner has not ratified this hold explicitly).
4. Finish audit **Round 1** (data-loss / DoS class) per
   `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md` — R3 order governs.
5. **Round 2 — Phase 6 TUI rework** (`docs/plan/TUI_REWORK.md`):
   H4 → H5 → R3-H23 → H2 → H6 → H7 → H8 → M2 → M3 → M4 → M25.
   (R3-M28 source-of-truth sweep completed 2026-06-04.)
6. `greenfield_plan_v6.md` §1.1 streaming planner + 1 s heartbeat + 10 s stall
   detector — owner-ratified, not yet built (H10b); queued after Round 1 closes.

## Authoritative docs right now

- Findings: `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md` (read R2 + R3 delta;
  R3 overrides R2 on conflicts)
- Plan: `docs/plan/DESIGN_COHERENCE_REVIEW.md` (**Active**, D-2026-06-11-1 —
  the current active plan; Phase A authorized)
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

- **2026-06-11** @ `1be16bc` — audit-h3c slice 1 graded and accepted (owner
  verdict; validation re-run green, test-fn count flat at 344). Assessment
  facts recorded in DEVLOG 2026-06-11. Owner directed: plan a repo-wide
  design-coherence review next; slice-2 re-scope waits on its findings.
  First action next session if interrupted: read the repo-review plan doc in
  `docs/plan/` (may not exist yet — then re-read DEVLOG 2026-06-11 + this Now).
- **2026-06-07** @ `ca940a2` — Merged agent-kit into master, deleted the branch, and codified branchless rules in AGENTS.md per owner command. Verified workspace builds and doc checks pass.
- **2026-06-07** @ `c793df2` — Installed the agent-protocol kit and reconciled
  the branch mess: documented the `-s ours` octopus trap (DECISIONS.md
  D-2026-06-07-1/-2, AGENTS.md §8), migrated plan-doc status headers, fixed
  `.gitignore`. Tree == `600023a`, builds. Awaiting owner approval for the first
  commit + named branch deletions; SETUP.md content still needed. First action
  next session: `catchup`, then check the audit-h3c slice 1 verdict.
