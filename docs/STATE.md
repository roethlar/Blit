# STATE — single entry point for "what is true right now"

Last updated: 2026-06-12 (reviewer: design-4 and design-5 accepted) at
commit `b5cbb38`

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **Reviewer grading complete**: design-4-fallback-midmanifest-negotiation
  (`ddfeb58`) and design-5-send-failure-masks-rejection (`08d71a2`) were
  both accepted by `gemini-reviewer`. Verdict commits: `a841691`, `b5cbb38`.
  `REVIEW.md` rows are `[x]`; `.review/ready/` has no pending sentinels.
  Validation rerun: fmt + clippy green; `cargo test --workspace` green with
  1370 passed, 0 failed, 1 ignored (unsandboxed for loopback bind tests).
- No coder or reviewer work is in flight. Both 2026-06-12 session
  authorizations are single-session (AGENTS.md §9); neither carries forward.

## Queue (ordered)

1. **Owner gates remaining**: flip `docs/plan/MULTISTREAM_PULL.md`
   Draft → Active (w2-3); push approval for the local-only commits.
2. **Execute the rest of the design-review queue** — `REVIEW.md` order
   governs. Highest open ratified row is w4-1 (AbortOnDrop family, High);
   next visible rows include w4-3 and W1 socket-policy/timeout constants.
   Use `slice` only after a fresh owner authorization.
3. **Land adaptive-streams** (D-2026-06-07-2) — after w2-3 per
   MULTISTREAM_PULL.md sequencing; then w3-1. Then audit Round 1, TUI
   rework (Round 2), H10b streaming planner.
4. **10 GbE benchmark session — DEFERRED by owner (2026-06-12: rig
   assembly is real work; benchmarking pre-multi-stream is churn)**.
   Runs AFTER w2-2 → w2-3 → adaptive-streams land — the natural
   measurement point. It remains the zero-copy revisit gate
   (D-2026-06-12-1) and w2-3's sign-off measure; capture before/after
   baselines at that session, not earlier.

## Authoritative docs right now

- Design queue: `REVIEW.md` (13 design-queue rows `[x]`, 0 rows `[~]`) + the three
  `docs/audit/` 2026-06-11 deliverables
- Review loop: `REVIEW.md` + `.review/README.md` + `.review/findings/` +
  `.review/results/` (ready queue empty)
- Plans: `docs/plan/MULTISTREAM_PULL.md` (Draft — awaiting owner Active),
  `docs/plan/ZERO_COPY_RECEIVE_EVAL.md` (delete ratified D-2026-06-12-1,
  executes in w8-1), `docs/plan/TUI_REWORK.md` (gated on Round 1)
- Findings: `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md` (R3 governs)

## Blocked / waiting

- **Owner**: w2-3 Active flip; push approval for the Windows test-tuning
  commit (`439a2a7`, local-only — Windows CI red until it lands);
  re-authorization for coder work (w4-1 next).

## Open questions

- `docs/agent/SETUP.md` content — owner must supply (other machine);
  `.review/README.md` lines 8/101 still point at unreadable paths.
- Disposition of adaptive-streams branch refs after landing (D-2026-06-07-2).
- Windows: w9-1 ungated 27 tests; w9-5/w9-4/w4-2 added ungated
  daemon-spawn tests — unverified on Windows; next windows-latest CI run or
  run-blit-tests.ps1 triages real failures into findings.

## Handoff log (newest first, keep ≤ 3)

- **2026-06-12** @ `b5cbb38` — gemini-reviewer session: graded and accepted both pending sentinels (design-4 and design-5); verdicts committed, `REVIEW.md` rows `[x]`, ready/ queue empty. In-flight: none. **Exact first action next session**: owner decides the remaining gates (w2-3 Active flip, push approval).
- **2026-06-12** @ `0213896` — gpt-reviewer session: graded and accepted
  all 4 pending sentinels (w4-2, w5-2, w7-4, w7-6); verdicts committed,
  `REVIEW.md` rows `[x]`, ready queue empty. In-flight: none; owner gates
  remain design-4 ratification, w2-3 Active flip, and push approval.
  **Exact first action next session**: owner decides the gates; if coder
  work is re-authorized, run `slice` and start at the top open
  `REVIEW.md` row (currently w4-1).
- **2026-06-12** @ `559eb36` — Coder session (owner: "Continue with 1").
  Done: w4-2 (channel deletion + design-4 discovery/filing), w5-2
  (dead classifier deleted, retry policy moved to blit_core::remote::retry
  with contract test), w7-4 (checksum::hash_reader owns the 256 KiB loop,
  daemon's fifth copy gone), w7-6 (DEFAULT_PORT pub). 4 sentinels pending.
  Earlier same day: owner-approved push (`1adbe0c..bf63a6e` → origin+gitea)
  + named-branch prune (phase5/a1, phase5/blit-app-extract — gone from both
  remotes); D-2026-06-12-1 recorded (zero_copy delete); MULTISTREAM_PULL.md
  drafted (owner-delegated parameters). **Exact first action next session**:
  grade the 4 sentinels (reviewer loop), then owner decides design-4
  ratification + w2-3 Active flip.
