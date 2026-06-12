# STATE — single entry point for "what is true right now"

Last updated: 2026-06-12 (reviewer: 4 sentinels accepted; ready queue empty) at
commit `0213896`

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **Reviewer grading complete**: w4-2-delete-push-upload-channel
  (`03bcb1d`), w5-2-retry-classifier-consolidation (`9c960dc`),
  w7-4-hash-reader-helper (`6b2f433`), and w7-6-default-port-pub
  (`de04054`) were all accepted by `gpt-reviewer`. Verdict commits:
  `1246398`, `8c32eb3`, `5910e29`, `0213896`. `REVIEW.md` rows are
  `[x]`; `.review/ready/` has no pending sentinels. Validation was rerun
  per sentinel: fmt + clippy green; `cargo test --workspace` green with
  1369 passed, 0 failed, 1 ignored (unsandboxed for loopback bind tests).
- **NEW BUG FILED: design-4-fallback-midmanifest-negotiation (High)** —
  found building w4-2's regression net: forced-gRPC pushes fail at ≥128
  files (exactly FILE_LIST_EARLY_FLUSH_ENTRIES; ~100 timing-flaky with
  partial transfer, ≤80 reliable on loopback). Pre-existing — verified by
  stash-bisect on the unmodified tree. It made w4-2's 262k wedge
  unreachable in practice. Repro: `#[ignore]` test in
  remote_tcp_fallback.rs (joint acceptance test for design-4 + w4-2).
  NOT among the ratified 38 — needs owner ratification before a fix slice.
- No coder or reviewer work is in flight. Both 2026-06-12 session
  authorizations are single-session (AGENTS.md §9); neither carries forward.

## Queue (ordered)

1. **Owner gates**: ratify (or reject) a design-4 fix slice; flip
   `docs/plan/MULTISTREAM_PULL.md` Draft → Active (w2-3); push approval for
   the local-only commits when desired.
2. **Execute the rest of the design-review queue** — `REVIEW.md` order
   governs. Highest open ratified row is w4-1 (AbortOnDrop family, High);
   next visible rows include w4-3 and W1 socket-policy/timeout constants.
   Use `slice` only after a fresh owner authorization.
3. **10 GbE benchmark session** (owner; macOS/Windows/Linux/TrueNAS matrix,
   all transfer paths) — doubles as the zero-copy revisit-gate measurement
   (D-2026-06-12-1) and w2-3's sign-off measure. NOTE: design-4 means any
   forced-gRPC bench leg >~100 files will fail — bench TCP paths, or fix
   design-4 first.
4. **Land adaptive-streams** (D-2026-06-07-2) — after w2-3 per
   MULTISTREAM_PULL.md sequencing; then w3-1. Then audit Round 1, TUI
   rework (Round 2), H10b streaming planner.

## Authoritative docs right now

- Design queue: `REVIEW.md` (11 design-queue rows `[x]`, 0 rows `[~]`;
  design-4 filed `[ ]` and awaiting owner ratification) + the three
  `docs/audit/` 2026-06-11 deliverables
- Review loop: `REVIEW.md` + `.review/README.md` + `.review/findings/` +
  `.review/results/` (ready queue empty)
- Plans: `docs/plan/MULTISTREAM_PULL.md` (Draft — awaiting owner Active),
  `docs/plan/ZERO_COPY_RECEIVE_EVAL.md` (delete ratified D-2026-06-12-1,
  executes in w8-1), `docs/plan/TUI_REWORK.md` (gated on Round 1)
- Findings: `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md` (R3 governs)

## Blocked / waiting

- **Owner**: design-4 ratification; w2-3 Active flip; next push approval
  (all commits after `bf63a6e` are local-only; current pre-handoff HEAD is
  `0213896`).
- **10 GbE session** hardware/time (owner, possibly today).

## Open questions

- `docs/agent/SETUP.md` content — owner must supply (other machine);
  `.review/README.md` lines 8/101 still point at unreadable paths.
- Disposition of adaptive-streams branch refs after landing (D-2026-06-07-2).
- Windows: w9-1 ungated 27 tests; w9-5/w9-4/w4-2 added ungated
  daemon-spawn tests — unverified on Windows; next windows-latest CI run or
  run-blit-tests.ps1 triages real failures into findings.
- design-4 mechanism is a hypothesis (mid-manifest fallback negotiation vs
  the daemon manifest loop's premature-FileData rejection) — fix slice must
  verify before changing behavior.

## Handoff log (newest first, keep ≤ 3)

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
- **2026-06-12** @ `88fdcdb` — claude-reviewer session: graded all 7
  overnight sentinels (w2-1, w5-1, w8-1b, w9-1, w9-2, w9-4, w9-5), all
  accepted, no reopens; REVIEW.md rows `[x]`; ready/ emptied.
