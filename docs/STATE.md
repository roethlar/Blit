# STATE — single entry point for "what is true right now"

Last updated: 2026-06-11 (h3c slice 1 verified; repo design-coherence review being planned) at commit `1be16bc`

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **Design-review queue ratified in full** (D-2026-06-11-2). All 38 new slices
  entered in `REVIEW.md` ("Design-review queue" section, execution order).
  Embedded decisions taken: Pull RPC deleted after w2-3 harvest; `zero_copy.rs`
  excluded from deletion → FAST evaluation slice `w8-1b`; multi-stream-pull
  plan doc authorized. `DESIGN_COHERENCE_REVIEW.md` → **Shipped**.
- **Next coder action**: pick `w5-1-log-backend` (topmost `[ ]` in the
  design-review queue) per the `slice` procedure.

## Queue (ordered)

1. **Execute the design-review queue** — `REVIEW.md` "Design-review queue"
   section is the authoritative order (w5-1 first). The former queued
   "audit-h3c slice 2" transport work is now slice family W1 inside it
   (W1.1 bundle lands with w1-2/design-3 at position 6-9; the cadence-watchdog
   re-evaluation happens after W1 lands).
2. **Land adaptive-streams** (D-2026-06-07-2) — cherry-pick/rebase the stack up
   to `eafb187` (live-progress → PR1 telemetry → PR2 work-queue → PR2 review
   fix), excluding `d9d4ec7` (does-not-build WIP). Resolve the `data_plane.rs`
   StallGuard-vs-`Probe` conflict by hand. Write a `docs/plan/` doc first
   (no code before `**Status**: Active`). NOTE: interacts with w2-3/w3-1
   (data_plane churn) — sequence consciously when reached.
3. Finish audit **Round 1** (data-loss / DoS class) per
   `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md` — R3 order governs. Several
   Round-1-class items are now covered by design-review slices (w4-1, w4-3,
   design-2/3); cross-check before starting.
4. **Round 2 — Phase 6 TUI rework** (`docs/plan/TUI_REWORK.md`):
   H4 → H5 → R3-H23 → H2 → H6 → H7 → H8 → M2 → M3 → M4 → M25.
   (R3-M28 source-of-truth sweep completed 2026-06-04.)
5. `greenfield_plan_v6.md` §1.1 streaming planner + 1 s heartbeat + 10 s stall
   detector — owner-ratified, not yet built (H10b); queued after Round 1 closes.
   (w2-1 deletes the dead warmup machinery; H10b is the real adaptive design.)

## Authoritative docs right now

- Findings: `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md` (read R2 + R3 delta;
  R3 overrides R2 on conflicts)
- Design queue: `REVIEW.md` "Design-review queue" section (ratified
  D-2026-06-11-2) + the three `docs/audit/` 2026-06-11 deliverables
  (`DESIGN_MAP`, `DESIGN_FINDINGS…PHASE_B`, `AUDIT_REPORT…DESIGN`)
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

- **2026-06-11** @ `5b5ff5b` — Full session: h3c slice 1 verified; design-
  coherence review executed end-to-end (Phases A/B/C, ~9.3M agent tokens,
  3 docs in `docs/audit/`); design-1/2/3 filed; owner ratified all 38 slices
  (D-2026-06-11-2). Pushed through `ab0d8a0`; commits `7d53107`, `6e8dfc4`,
  `55b1fca`, `5b5ff5b` are LOCAL-ONLY pending push approval. First action next
  session: `catchup`, then pick `w5-1-log-backend` per the `slice` procedure.
- **2026-06-11** @ `1be16bc` — audit-h3c slice 1 graded and accepted (owner
  verdict; validation re-run green, test-fn count flat at 344). Assessment
  facts recorded in DEVLOG 2026-06-11. Owner directed: plan a repo-wide
  design-coherence review next; slice-2 re-scope waits on its findings.
- **2026-06-07** @ `ca940a2` — Merged agent-kit into master, deleted the branch, and codified branchless rules in AGENTS.md per owner command. Verified workspace builds and doc checks pass.
