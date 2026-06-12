# STATE — single entry point for "what is true right now"

Last updated: 2026-06-12 (coder: 4 more slices sentineled; design-4 filed) at
commit `559eb36`

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **4 sentinels await review** (coder session 2026-06-12 under owner grant
  "Continue with 1"; commits local-only): w4-2-delete-push-upload-channel
  (`03bcb1d`), w5-2-retry-classifier-consolidation (`9c960dc`),
  w7-4-hash-reader-helper (`6b2f433`), w7-6-default-port-pub (`de04054`).
  Validation green per slice; suite 1368 → 1369 (w5-2 deleted the dead
  module's 4 tests, called out in its finding doc).
- **NEW BUG FILED: design-4-fallback-midmanifest-negotiation (High)** —
  found building w4-2's regression net: forced-gRPC pushes fail at ≥128
  files (exactly FILE_LIST_EARLY_FLUSH_ENTRIES; ~100 timing-flaky with
  partial transfer, ≤80 reliable on loopback). Pre-existing — verified by
  stash-bisect on the unmodified tree. It made w4-2's 262k wedge
  unreachable in practice. Repro: `#[ignore]` test in
  remote_tcp_fallback.rs (joint acceptance test for design-4 + w4-2).
  NOT among the ratified 38 — needs owner ratification before a fix slice.
- **Coder stopped because**: remaining `[ ]` rows overlap the 4 pending
  sentinels (w4-1/w2-2/w4-4 ← w4-2's push files; w4-3/w3-1 ← w7-4's
  daemon pull.rs; w6-1/w7-1/w8-1 ← w5-2's core lib/remote files), need
  the owner (w2-3 plan flip, design-4 ratification), or deserve a fresh
  session (w7-5, w5-5, w9-6, w10 — disjointness unverified or
  judgment-heavy). Grading the sentinels unblocks the skip sets.
- Both 2026-06-12 session authorizations are single-session (AGENTS.md §9);
  neither carries forward.

## Queue (ordered)

1. **Grade the 4 pending sentinels** (reviewer loop per `.review/README.md`).
2. **Owner gates**: ratify (or reject) a design-4 fix slice; flip
   `docs/plan/MULTISTREAM_PULL.md` Draft → Active (w2-3); push approval for
   the local-only commits when desired.
3. **Execute the rest of the design-review queue** — REVIEW.md order
   governs; after grading, next rows are w4-1 (AbortOnDrop family, High),
   w4-3 (disconnect racing), w2-2 (stream-ladder owner), w8-1 (foundation
   sweep incl. zero_copy delete per D-2026-06-12-1).
4. **10 GbE benchmark session** (owner; macOS/Windows/Linux/TrueNAS matrix,
   all transfer paths) — doubles as the zero-copy revisit-gate measurement
   (D-2026-06-12-1) and w2-3's sign-off measure. NOTE: design-4 means any
   forced-gRPC bench leg >~100 files will fail — bench TCP paths, or fix
   design-4 first.
5. **Land adaptive-streams** (D-2026-06-07-2) — after w2-3 per
   MULTISTREAM_PULL.md sequencing; then w3-1. Then audit Round 1, TUI
   rework (Round 2), H10b streaming planner.

## Authoritative docs right now

- Design queue: `REVIEW.md` (7 rows `[x]` graded; 4 rows `[~]` pending;
  design-4 filed `[ ]`) + the three `docs/audit/` 2026-06-11 deliverables
- Review loop: `REVIEW.md` + `.review/README.md` + `.review/findings/`
- Plans: `docs/plan/MULTISTREAM_PULL.md` (Draft — awaiting owner Active),
  `docs/plan/ZERO_COPY_RECEIVE_EVAL.md` (delete ratified D-2026-06-12-1,
  executes in w8-1), `docs/plan/TUI_REWORK.md` (gated on Round 1)
- Findings: `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md` (R3 governs)

## Blocked / waiting

- **Reviewer grading** of w4-2 / w5-2 / w7-4 / w7-6 (sentinels in
  `.review/ready/`).
- **Owner**: design-4 ratification; w2-3 Active flip; next push approval
  (all commits after `bf63a6e` are local-only).
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
- **2026-06-12** @ `0af904e` — Autonomous overnight coder session (owner
  grant): 7 slices implemented/validated/sentineled (w5-1, w9-5, w2-1,
  w9-1, w9-2, w9-4, w8-1b — DEVLOG 2026-06-12). Suite 1331→1368.
