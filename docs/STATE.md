# STATE — single entry point for "what is true right now"

Last updated: 2026-07-12

- Recent sessions (2026-07-11/12, 44th–45th): **otp-10 fully closed (cutover + deletion) and otp-11a closed through the codex loop** — local transfers ride the session (in-process transport + local byte-carrier); the deletion slice 11b is BLOCKED on one owner question (change-journal retirement cost, Blocked below). Suite **1512**. SMALL_FILE_CEILING paused (D-2026-07-05-1). Push state: see Blocked.

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
  - **otp-11a `[x]` CLOSED (the local route; deletion is 11b)** —
    slice design `docs/plan/OTP11_LOCAL_SESSION.md` (D1–D3,
    codex-reviewed): local transfers ride the session
    (`run_local_session`, both role drivers over `in_process_pair`);
    the LOCAL byte-carrier = process-local `LocalApply` (no wire
    shape — the destination plans + applies needs in-process through
    `FsTransferSink`, clonefile/block-clone preserved); the `blit_app`
    local chokepoint re-pointed, CLI/TUI untouched, all verb + move
    data-loss regression pins green; old orchestration in-tree but
    production-caller-less (11b deletes it). Design codex 10 + slice
    codex 9 findings adjudicated (`.review/results/otp-11{-design,a}.*`).
    Perf gate: huge/tree/small PASS (1 GiB local = 22 ms both — clone
    kept); **noop10k FAIL → the journal question in Blocked**.
    Suite 1488 → **1512**. Detail: DEVLOG 2026-07-12.
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
   c-1/2), **otp-11a** `[x]`. Current: **otp-11b (the local
   orchestration deletion + compare_manifests sweep + retirement
   accounting, ≈+44 pins)** — BLOCKED on the journal owner question
   (Blocked below).
2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
   Shipped (zero-copy resolved — D-2026-07-05-3). Optional follow-ups
   largely absorbed by otp-2/otp-12's rig matrices; skippy env facts
   moved to Blocked → Rig availability.
3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
   resumes/re-derives after ONE_TRANSFER_PATH ships.
4. **PAUSED: design-review queue** (`REVIEW.md` order; w7-1 topmost
   open row; filed w6-2a/b/c + relay-1) — same directive; note w7-1
   (mirror-executor consolidation) likely lands for free inside
   otp-6's one-delete-rule slice; re-check before picking it up.
5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
   Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
   cutover as a runtime-selected write strategy in the unified receive
   sink (design: eval doc §If-FAST-evidence; dead module deletes in
   w8-1). Rig facts + the aarch64-musl static build recipe: DEVLOG
   2026-07-05 10:00. **Standing owner safety rule**: ALL activity on
   rig `zoey` is confined to its `…/blit-temp/` folder — module roots,
   test data, everything; nothing written outside it, ever. Zero-copy
   is pre-authorized to be tested there when the post-cutover slice set
   reaches it; no daemon runs on zoey before then without a fresh go.
6. **Post-REV4 residue** (unowned): ~~pull 1s-start restructuring~~
   (absorbed by ONE_TRANSFER_PATH choreography, D-2026-07-05-1);
   epoch-0/early-ADD hardening; remote perf-history lanes (1e gap);
   `derive_local_plan_tuning` fold-or-retire; receive-side dial
   tuning residue (w3-1 scoped it out); the source send half's bounded
   `dp.queue()` is not raced against control-lane events (deferred at
   codex otp-7b-1 F3; otp-8 F1 gave the in-stream sends a fault race —
   residual: the narrow CANCELLED→INTERNAL decay, verdict file);
   CLI progress monitor lives through the in-session mirror purge
   (display-only ticks/avg dilution; fix = the M-C `AppProgressEvent`
   phase reshape — deferred at codex otp-10b-2 F5).

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

- **Rig availability (owner, 2026-07-10, verified by ssh)**: for the
  otp-12 matrix — remote↔remote (delegated) uses the Windows box
  (`michael@10.1.10.173`) + TrueNAS `skippy` (`admin@skippy`,
  x86_64; existing test folder `/mnt/generic-pool/video/blit-bin/`
  with July binaries + bench.toml; /tmp and /home are noexec there);
  skippy also available for Mac↔Linux cells "if needed" (owner).
  zoey = per-direction rig; Windows pair = cross-direction rig.
- **Three 10 GbE gate declarations**: ue-1, ue-2 (pass/fail or
  re-scope), REV4 → Shipped. (Zero-copy RESOLVED — D-2026-07-05-3.)
- **Push go**: origin/master = `6d37a22` (re-verified via `ls-remote`
  2026-07-11 — a partial push landed outside these sessions); unpushed
  `6d37a22..HEAD` (12 at the 10c-1 record). Awaits the ref-listing +
  approval flow; windows-latest CI on the w9-3 fix rides it.
- **otp-5b-3** (pull mid-transfer cancel e2e, marked optional): pick
  up while otp-10 runs, or drop? — standing question.
- **NEW (otp-11a, 2026-07-12): the change-journal question — blocks
  otp-11b.** Measured (`docs/bench/otp11-local-2026-07-11/README.md`):
  repeated no-op mirror, 10k files — old path ~21 ms (its journal skip
  engages after 1–2 runs) vs session ~219 ms (full enumerate+diff,
  which beats the old NON-journal pass, 610 ms). Retiring the journal
  (slice doc D3) is what the delta measures. Options: **(a)** accept —
  repeated no-ops cost a full re-stat (rsync-class, ~2 s/100k files),
  `change_journal/` dies at 11b; **(b)** keep it as a pre-session
  no-op short-circuit (~600 LOC; a local-only fast path in front of
  the one path — the class of side apparatus the directive kills);
  **(c)** = (a) now + file journal-assisted no-op detection as a
  future SESSION capability (both carriers). Rec: (a) or (c).

## Open questions

- **(OPEN — owner ack, 2026-07-05, otp-4a)** Unified SizeMtime semantic:
  same-size + dest-NEWER — old push clobbers, session adopts **data-safe
  SKIP** (converge-up; `--force` still overwrites; pinned by
  `same_size_newer_destination_is_skipped_not_clobbered`). Owner: confirm
  or ask for old-push clobber. Reasoning: `.review/findings/otp-4-daemon-serves-transfer.md`.
- **(OPEN)** Historical docs embed `/Users/...` paths (rec: leave);
  `725aa07` tracked a stale worktree snapshot (rec `git rm -r`, awaits go).
- **(OPEN, 2026-07-04)** `docs/WHITEPAPER.md` §§~309/606/641 describe
  the deleted `determine_remote_tuning` — fold into w10-docs-batch?
- **(OPEN, ripe — data in hand)** REV4 → Shipped flip: awaits the
  three declarations in Blocked (zero-copy resolved, D-2026-07-05-3).
- **(OPEN, new 2026-07-05)** CLI foot-gun: `blit copy src_large dst`
  with an existing local dir and no `./` parses the bare name as an
  mDNS discovery endpoint and errors (blit-app endpoints.rs). Should
  local-path existence win, or the error improve? Owner to slot.
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: full suite green
  locally; daemon-spawn e2e flakiness root-caused + fixed on Linux (w9-3:
  port-TOCTOU race + cargo-lock contention). Remaining: windows-latest CI
  on the next push (10d89e0 predates the w9-3 fix).

## Handoff log (newest first, keep ≤ 3)

- **2026-07-12 (45th, this session)** — **otp-11a CLOSED through the
  codex loop (design doc + slice + fix round; suite 1488 → 1512;
  perf gate huge/tree/small PASS, 1 GiB clone kept)**. In-flight:
  none; tree clean. **Next**: owner answers the change-journal
  question (Blocked) → otp-11b. (Mid-session full-suite "failures" =
  dirty-tree BUILD_MISMATCH sampling artifacts; clean rebuild
  converged them.)
- **2026-07-11 (44th)** — otp-10c closed (relay removal
  D-2026-07-11-1 + the cutover deletion); suite 1605 → 1488. Owner
  ask pending: the `725aa07` snapshot `git rm -r` go (otp-10c-2 F6).
- **2026-07-11 (43rd)** — otp-10a/10b-1/10b-2 closed (1555 → 1605);
  verb cutover complete.
- *(42nd and earlier pruned to the cap — see DEVLOG 2026-07-06..12.)*
