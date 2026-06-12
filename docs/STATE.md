# STATE — single entry point for "what is true right now"

Last updated: 2026-06-12 (claude-reviewer: 7 design-review sentinels graded) at commit `88fdcdb`

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **7 design-review sentinels graded and accepted** (claude-reviewer per
  `.review/README.md` loop): w2-1, w5-1, w8-1b, w9-1, w9-2, w9-4, w9-5.
  All 7 `.review/ready/*.json` cleared; 7 `.verified.json` written; all
  corresponding rows in REVIEW.md flipped `[~]` → `[x]`. Validation
  re-run green for each (fmt/clippy/test --workspace); diffs matched
  findings exactly (no reopens). Commits local-only on master.
- **Unblocked**: the 7 verdicts unblock their skip sets in the design-review
  queue (w4-2 delete-push-upload-channel is first; w4-1, w5-2, others per
  overlap map). w8-1b analysis accepted (plan doc delivered; owner D-2026-06-12-1
  already ratified delete, now unblocked for w8-1 execution). w5-1 (lib.rs)
  graded, which was a cross-slice blocker.
- The 2026-06-11 session-authorization note (owner verbatim + scope) is in
  the handoff entry below; per AGENTS.md §9 it does NOT carry into the next
  session.

## Queue (ordered)

1. **Execute the rest of the design-review queue** — `REVIEW.md`
   "Design-review queue" order governs. The 7 sentinels (w5-1, w9-5, w2-1,
   w9-1, w9-2, w9-4, w8-1b) were graded+accepted by claude-reviewer
   2026-06-12; all rows `[x]`, ready/ empty. Next unblocked rows:
   w4-2 (push upload channel), w5-2 (dead classifier), w4-1 (AbortOnDrop
   family). w2-3 needs an owner plan interview first (multi-stream pull,
   authorized D-2026-06-11-2).
2. **Land adaptive-streams** (D-2026-06-07-2) — cherry-pick/rebase up to
   `eafb187`, excluding `d9d4ec7`. Plan doc first. Interacts with
   w2-3/w3-1 data_plane churn — sequence consciously.
3. Finish audit **Round 1** per `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md`
   (cross-check: several Round-1 items are now design-review slices).
4. **Round 2 — Phase 6 TUI rework** (`docs/plan/TUI_REWORK.md`).
5. `greenfield_plan_v6.md` §1.1 streaming planner + heartbeat + stall
   detector (H10b) — queued after Round 1. (w2-1 deleted the dead warmup
   machinery; H10b is the real adaptive design and also the revisit-gate
   context for `ZERO_COPY_RECEIVE_EVAL.md`.)

## Authoritative docs right now

- Design queue: `REVIEW.md` "Design-review queue" (7 w* rows now `[x]`;
  graded by claude-reviewer) + the three `docs/audit/` 2026-06-11 deliverables
- Review loop: `REVIEW.md` + `.review/README.md` (verdicts in
  `.review/results/w*.verified.json`; 7 findings in `.review/findings/w*.md`)
- New: `docs/plan/ZERO_COPY_RECEIVE_EVAL.md` (owner-ratified delete via
  D-2026-06-12-1; w8-1b slice delivered the analysis)
- Findings: `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md` (R3 governs)
- Plan: `docs/plan/TUI_REWORK.md` (gated on Round 1)

## Blocked / waiting

- **Owner approval to push**: local-only commits on `master` (coder slices
  + 7 claude-reviewer verdict commits + handoff records). Before any push:
  list exact local refs vs origin/gitea and wait for explicit owner
  approval (AGENTS.md §8). No agent may push.
- **Owner verdicts / next gates**: w2-3 plan interview (multi-stream pull);
  w8-1 delete of zero_copy (per D-2026-06-12-1 on the w8-1b eval doc).
- Stale remote branches pending deletion **by explicit name** (verified
  ahead=0): `phase5/a1`, `phase5/blit-app-extract` (remote-only refs on
  `origin` + `gitea`; deletion is a remote `push --delete`).

## Open questions

- `docs/agent/SETUP.md` content — owner must supply (lives on another
  machine); `.review/README.md` lines 8/101 still point at unreadable paths.
- Disposition of adaptive-streams branch refs after landing (D-2026-06-07-2).
- Windows: w9-1 ungated 27 tests and w9-5/w9-4 added ungated daemon-spawn
  tests — none verified on Windows from this macOS session; next
  windows-latest CI run (or `scripts/windows/run-blit-tests.ps1`) triages
  any real platform failures into their own findings.

## Handoff log (newest first, keep ≤ 3)

- **2026-06-12** @ `88fdcdb` — claude-reviewer persona session per
  `.review/README.md`. Woke via reviewer-wait.sh; graded all 7 design-review
  sentinels on master (w2-1-delete-warmup-machinery, w5-1-log-backend,
  w8-1b-zero-copy-fast-eval, w9-1-ungate-windows-tests, w9-2-revive-root-tests,
  w9-4-readonly-enforcement-tests, w9-5-jobs-lifecycle-e2e). All accepted
  (re-ran validation per slice; diffs + findings + commit messages aligned;
  no reopens or code changes by reviewer). Wrote 7 *.verified.json (reviewer:
  "claude-reviewer"), flipped REVIEW.md rows `[~]`→`[x]`, git-rm'd ready/
  sentinels. 7 local commits. ready/ now empty (NO_READY confirmed).
  Unblocks overlap-skipped coder rows (w4-2 first). **Exact first action
  next session**: owner may authorize coder on unblocked design-review
  slices (w4-2 etc.) or Windows parity (`scripts/windows/run-blit-tests.ps1`);
  any push still requires explicit owner gate (list refs first).
- **2026-06-12** @ `0af904e` — Autonomous overnight coder session under
  owner grant ("work on as much as you can. commit every slice as it
  lands. if anything gets questionable, stop." — master only, no branches,
  local commits, never push, faster-mode WIP iff files disjoint;
  single-session per §9). Done: 7 slices implemented/validated/sentineled
  (w5-1, w9-5, w2-1, w9-1, w9-2, w9-4, w8-1b — details in DEVLOG
  2026-06-12). Suite 1331→1368; fmt/clippy/test green after each slice.
  Stopped because all remaining rows are overlap-, owner-, or
  dependency-blocked (skip map in Now). **Exact first action next
  session**: owner reads this + DEVLOG, then either grades sentinels
  (reviewer loop) or re-authorizes coding — w4-2 is the next coder row
  once w5-1 is graded.
- **2026-06-11** @ `5b5ff5b` — Full session: h3c slice 1 verified; design-
  coherence review executed end-to-end (Phases A/B/C, ~9.3M agent tokens,
  3 docs in `docs/audit/`); design-1/2/3 filed; owner ratified all 38
  slices (D-2026-06-11-2). Commits `7d53107`,`6e8dfc4`,`55b1fca`,`5b5ff5b`
  LOCAL-ONLY pending push approval.
