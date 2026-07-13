# STATE — single entry point for "what is true right now"

Last updated: 2026-07-13

- Recent sessions (2026-07-11/13, 44th–46th): **otp-10, otp-11 closed through the codex loop; otp-12c RECORDED** — every transfer (local included) rides the ONE session; the separate local orchestration no longer exists (−6.2k lines at 11b); the old journal fast path was proven UNSOUND (data-loss repro recorded) and died with it. Both otp-12c rigs measured 2026-07-13 (evidence in Blocked; adjudication belongs to otp-13, and the slice still owes a codex round). Suite **1484** (the otp-13 ≥1483 floor met at the deletion slice). SMALL_FILE_CEILING paused (D-2026-07-05-1).

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
  D-2026-07-05-4 "flip the plan and go") — otp-4a landed.** The
  invariant (plan doc, verbatim): ONE block of transfer code;
  direction/initiator/verb can NEVER affect wall time by blit's doing
  — impossible by construction because the per-direction drivers and
  `Push`/`PullSync` are deleted at cutover. Slices otp-1..13;
  converge-up per cell (±10%); symmetric-fs disk-to-disk verdict
  cells. **D-2026-07-05-2: same-build peers only, refusal at session
  open.** Progress (each slice through the codex loop; per-slice
  detail lives in DEVLOG + `.review/`, NOT here):
  - **Closed `[x]`: otp-1, otp-3, otp-4 (a, b-1/2/3), otp-5 (a,
    b-1/2), otp-6 (a/b), otp-7 (a, b-1/2), otp-8, otp-9 (a/b)** —
    the full session machine: contract, role drivers, daemon
    serving, both data planes + sf-2 resize + cancel, mirror/filters
    (one delete rule), resume both carriers (wire bounds
    D-2026-07-10-1/-2), fallback byte-carrier, delegated-on-session.
    Suite → **1555** (as of `1ce73b5`; later commits are
    bench/docs-only). SizeMtime = data-safe skip (open Q below).
    Per-slice detail: DEVLOG 2026-07-10 entries + `.review/`.
  - **otp-2 `[x]` (both halves).** zoey = PER-DIRECTION reference;
    Mac↔Windows = cross-direction rig (otp-2w). Harnesses
    `scripts/bench_otp2{,w}_baseline.sh`, evidence
    `docs/bench/otp2{,w}-baseline-2026-07-10/README.md`. Key reading:
    old push trails old pull on BOTH rigs — otp-12's interleaved
    old-vs-new discriminates code cost from platform write-path cost.
  - **otp-10 `[x]` CLOSED (a, b-1/2, c-1/2)** — verb cutover + THE
    CUTOVER DELETION: one chokepoint per verb shape (`blit_app
    run_remote_push`/`run_remote_pull`), ONE args→compare mapping,
    move maps IgnoreTimes/Checksum-only on every route; relay removed
    (D-2026-07-11-1); 4 drivers + `Push`/`PullSync` + 13 messages out
    of tree AND proto (−13.8k lines, no bridge); DelegatedPull
    no-payload proof recorded. Suite 1555 → … → **1488**. Per-slice
    detail: DEVLOG 2026-07-11 entries + `.review/`.
  - **otp-11 `[x]` CLOSED (a + addendum + b)** — local transfers ride
    the session (`run_local_session` over `in_process_pair`; the
    LOCAL byte-carrier = process-local `LocalApply`, no wire shape,
    clonefile/block-clone preserved — slice design
    `docs/plan/OTP11_LOCAL_SESSION.md`, every round codex-reviewed:
    design 10 + slice 9 + addendum 4 + deletion 6 findings, all
    adjudicated in `.review/results/otp-11*`). Perf gate PASS against
    SOUND baselines (1 GiB local = 22 ms both binaries; the old 21 ms
    journal no-op was proven UNSOUND — silent data loss on deep
    modifications, repro in `docs/bench/otp11-local-2026-07-11/`).
    **11b deleted the whole old orchestration** (−6.2k lines:
    orchestrator/engine/local_worker/auto_tune/change_journal +
    the compare_manifests sweep; dial re-homed verbatim; types →
    `transfer_session/local.rs`); the acceptance criteria's
    deletion-proof line for "the separate local orchestration path"
    COMPLETES. Suite 1488 → 1513 → **1484** (≥1483 floor met at the
    deletion slice, margin +1). Detail: DEVLOG 2026-07-12 entries.
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
  `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`); **sf-3a+ blocked**
  until ONE_TRANSFER_PATH ships, then resume/re-derive on the unified
  baseline. Principle stands: ceiling-driven, never competitor-relative
  (D-2026-07-04-4; a ≥25% margin answer was retracted — do not
  re-litigate). Evidence `docs/bench/10gbe-2026-07-05/`.
- **Background (2026-07-04/05, all `[x]`)**: REV4 code-complete, gates
  DATA-COMPLETE (declarations pending in Blocked); codex loop governs
  all changes (D-2026-07-04-1; DEVLOG 07-04/05).

## Queue (ordered)

1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
   the only work item until it ships**: slices otp-1..13 through the
   codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
   otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b), otp-7 (a, b-1,
   b-2), otp-8, otp-9 (a/b), otp-2 (+ otp-2w), otp-10 (a, b-1/2,
   c-1/2), **otp-11 (a + b)**, **otp-12a (zoey)**, **otp-12b
   (Mac↔Windows)** `[x]`. 12a: 10 PASS, 2 to the walk. 12b — THE
   INVARIANCE CRITERION: 11/12 PASS (1.003–1.057); wm_tcp_mixed 1.237
   (TCP×mixed×dest-initiator, code-shaped); push_tcp_small 1.149
   (both rigs); Win→Mac beats the better old direction 6/6; Mac→Win
   gap shapes recorded for the walk
   (`docs/bench/otp12-{zoey,win}-2026-07-12/`). **otp-12c `[x]`
   RECORDED 2026-07-13**: direct-path baseline at the cutover sha
   (`docs/bench/otp12c-win-2026-07-13/`) + the delegated rig-D
   matrix (`docs/bench/otp12c-delegated-2026-07-13/`, 5/7 PASS at
   RUNS=4; both FAIL cells PASS at RUNS=8 — see Blocked; rig D 7/7).
   **otp-12d and otp-13 are DEFERRED, not next** — otp-12c's rows are
   PRE-FIX, and `docs/plan/OTP12_PERF_FINDINGS.md` (pf-final) voids
   pre-fix new arms for acceptance. Assembling the acceptance matrix now
   would build otp-13's artifact from void rows.
1a. **`docs/plan/OTP12_PERF_FINDINGS.md` — THE REAL NEXT ITEM** (Draft;
   owner 2026-07-12: "fix the code before devoting another block of time
   to testing. plan, reviewloop codex, then fix once converged").
   **P1 misses the plan's HEADLINE criterion** (initiator/verb
   invariance): `wm_tcp_mixed` FAILs in two independent sessions and got
   WORSE at the cutover sha (1.237 → **1.300**) on tight spreads
   (6.4/8.4%) far below D2's 25% escalation trigger — it cannot be
   re-run away. P2 (`push_tcp_small`) likewise worsened (1.149 →
   **1.201**; zoey 1.105). Codex: r2 REVISE, **r3 NEEDS ANOTHER ROUND**
   (6/6 accepted); r4 fixes in, r4 review pending. **Blocked on: owner
   flip to Active** once codex converges → pf-1 (investigate) → pf-2..n
   (fix) → pf-final (rerun ALL THREE rigs) → otp-12d → otp-13.
2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
   Shipped (zero-copy resolved — D-2026-07-05-3). Optional follow-ups
   largely absorbed by otp-2/otp-12's rig matrices; skippy env facts
   moved to Blocked → Rig availability.
3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
   resumes/re-derives after ONE_TRANSFER_PATH ships.
4. **PAUSED: design-review queue** (`REVIEW.md` order; w7-1 topmost
   open row) — same directive; w7-1 likely landed for free inside
   otp-6's one-delete-rule slice; re-check before picking it up.
5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
   Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
   cutover as a runtime-selected write strategy in the unified receive
   sink (design: eval doc §If-FAST-evidence; dead module deletes in
   w8-1). Rig facts + build recipe: DEVLOG 2026-07-05 10:00.
   **Standing owner safety rule**: ALL activity on rig `zoey` stays
   inside its `…/blit-temp/` folder — nothing written outside it, ever;
   no daemon runs on zoey without a fresh go.
6. **Post-REV4 residue** (unowned): epoch-0/early-ADD hardening; remote
   perf-history lanes (1e gap); receive-side dial tuning residue (w3-1
   scoped it out); the source send half's bounded `dp.queue()` is not
   raced against control-lane events (codex otp-7b-1 F3; residual: the
   narrow CANCELLED→INTERNAL decay); the CLI progress monitor lives
   through the in-session mirror purge (display-only; fix = the M-C
   `AppProgressEvent` phase reshape — codex otp-10b-2 F5).

## Authoritative docs right now

- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Active**,
  D-2026-07-09-1 — otp-7 slice design; governs otp-7a/7b).
- Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**paused** at
  sf-2) and **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** (code-
  complete; measurement gates remain). REV4 superseded v1/REV2/REV3
  (history only).
- Process: `docs/agent/GPT_REVIEW_LOOP.md` (Active) — the codex loop
  for **all code and plan changes** (D-2026-07-04-1); `.review/README.md`
  is retired as the grading mechanism (its `findings/`/`results/`
  records and the REVIEW.md index remain live).
- Review loop: `REVIEW.md` (all `ue-r2-*` rows `[x]`; design-queue
  rows) + `.review/findings/` + `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (module delete ratified
  D-2026-06-12-1, executes w8-1; **capability unparked
  D-2026-07-05-3** — post-cutover write strategy), `TUI_REWORK.md`
  (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).

## Blocked / waiting (all owner declarations; checkpoints are owner-only)

- **Rigs**: owner go GIVEN 2026-07-12 (standing through otp-12). zoey
  (12a), netwatch-01 (12b), netwatch-01↔skippy (12c) all done. Rig
  plumbing facts: DEVLOG 2026-07-13.
- **otp-12c RECORDED 2026-07-13** (pre-fix rows → replication + control
  evidence, NOT acceptance evidence; see Queue 1a):
  `docs/bench/otp12c-win-2026-07-13/` (198 runs; 93 PASS / 12 FAIL /
  3 FAIL-SAME-SESSION) and `docs/bench/otp12c-delegated-2026-07-13/`
  (**rig D 7/7 PASS** — 5 at RUNS=4, 2 via D2's escalation at RUNS=8).
  Codex: FAIL → **7/7 accepted** (`.review/results/otp-12c.*`).
  Detail: DEVLOG 2026-07-13.
- **Three 10 GbE gate declarations**: ue-1, ue-2 (pass/fail or
  re-scope), REV4 → Shipped. (Zero-copy RESOLVED — D-2026-07-05-3.)

## Open questions

- **(OPEN, ripe — data in hand)** REV4 → Shipped flip: awaits the
  three declarations in Blocked (zero-copy resolved, D-2026-07-05-3).
- **(SLOTTED 2026-07-12 — owner ack)** `docs/WHITEPAPER.md` §8 (~line
  592) describes the deleted `determine_remote_tuning` — fix folded
  into **w10-docs-batch**; no one-off edit.
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: w9-3 fixed the
  Linux daemon-spawn flakiness; windows-latest CI pending a push.
  NOTE 2026-07-12: the macOS `blit_utils` residual (pre-existing,
  reproduced at `6d37a22`) ran ELEVATED under heavy load (~3/12 vs 2/8
  historical) — own finding if it persists on a quiet machine.
- *(Resolved 2026-07-12/13 — the SizeMtime data-safe SKIP semantic,
  the `725aa07` worktree snapshot, the CLI `./NAME` foot-gun, otp-5b-3
  mid-copy cancel, and the change-journal premise: all landed; see
  DEVLOG.)*

## Handoff log (newest first, keep ≤ 3)

- **2026-07-13 (46th, this session)** — **otp-12c RECORDED**: rig prep,
  the direct-path re-baseline (`d12534d`) and the delegated rig-D
  session (`68bb490`); three harness bugs found live and fixed
  (`b49413d`, `a2dea3f`). Also landed earlier in the session: the
  mid-copy cancel e2e + the plan-D4 mid-record fault fix (`920c6a7`)
  and the CLI `./NAME` hint (`ace91de`). In-flight: none; tree clean.
  **Next**: the codex round otp-12c owes, then **otp-12d assembly** →
  otp-13 owner walk.
- **2026-07-12 (45th)** — **otp-11 CLOSED WHOLE** (11a route +
  journal-hole addendum + 11b deletion, four codex rounds; suite
  1488 → 1484, the ≥1483 floor met; the separate local orchestration
  no longer exists).
- *(44th and earlier pruned to the cap — see DEVLOG 2026-07-06..13.)*
