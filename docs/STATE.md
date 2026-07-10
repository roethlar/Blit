# STATE — single entry point for "what is true right now"

Last updated: 2026-07-10

- 2026-07-04: Owner-approved dual push reached 3d8326b (origin: 10d89e0..3d8326b; gitea mirror: 2a77b9f..3d8326b). That push corrected a prior remote-name confusion; windows-latest CI on that push is the "meaningfully green" check referenced in prior notes.

- Current session (2026-07-10, this one): **otp-7b landed and CLOSED through the codex loop — otp-7 is done** (`ecac9b0` 7b-1 data-plane resume, `071799a` 7b-2 fault-summary rider + cancel e2e, `d48351d` review fixes; both codex verdicts adjudicated in `.review/results/otp-7b-{1,2}.gpt-verdict.md`). ONE_TRANSFER_PATH otp-1..7 [x]. SMALL_FILE_CEILING remains paused (D-2026-07-05-1).

- Notes on push state (re-verified via `git ls-remote origin` at session start, as of `d48351d`): origin/master is at `7f1c4b2` — the owner pushed since the 40th handoff's "unpushed f6e592e..HEAD" note, which is now stale. Unpushed local commits: `7f1c4b2..HEAD` (this session's four). windows-latest CI on the w9-3 harness fix rides the next push.

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
  - **Closed `[x]`: otp-1, otp-3, otp-4a, otp-4b (1/2/3), otp-5a,
    otp-5b (1/2), otp-6 (a/b), otp-7a** — contract + role drivers +
    daemon serving; push and pull data planes with sf-2 resize +
    cancel; mirror/filters (one delete rule); in-stream resume with
    wire bounds D-2026-07-10-1. SizeMtime = data-safe skip (open Q
    below).
  - **otp-7b (1/2) `[x]` — resume over the TCP data plane + the D4
    fault-summary rider, CLOSED; otp-7 done.** 7b-1 (`ecac9b0`):
    composite `ResumeFile` work item = strict per-file socket
    serialization; shared `ResumeBlockDiff`; DEST claim state shared
    with `NeedListSink`; per-carrier ceiling D-2026-07-10-2;
    session-client resume options. 7b-2 (`071799a`): structured
    `SessionFault.relative_path` (wire optional field,
    CONTRACT_VERSION → 2) + `end_of_operation_summary()` (verb print
    lands at otp-10); cancel-during-resume e2e; RELIABLE fix — resume
    block writes now flush (unflushed tokio file writes had made a 7a
    pin ~50% flaky under suite load). Codex: 7b-1 FAIL → 3 fixed / 1
    pre-fixed / 1 deferred (residue list) / 1 rejected; 7b-2 NEEDS
    FIXES → 4/4 fixed (`d48351d`; keepalive ticks vs the receiver
    StallGuard on silent hash scans, resume batches drive sf-2 resize,
    64 MiB ceiling pinned, single-file-root "" identity). Suite →
    **1550**. Detail: DEVLOG 2026-07-10 07:30Z + `.review/`.
  - Current: **otp-8 (fallback byte-carrier)** — NOTE for the next
    session: the in-stream carrier already exists and is exercised as
    the fallback by every slice since otp-3 (both directions, resume
    included); assess whether otp-8 is substantially satisfied and
    what residue remains (e.g. `--force-grpc`-shaped option plumbing)
    before writing new code. otp-5b-3 (pull cancel) optional; otp-2
    rig-gated before otp-10.
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
  `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`); **sf-3a+ blocked**
  until ONE_TRANSFER_PATH ships, then resume/re-derive on the unified
  baseline. Principle stands: ceiling-driven, never competitor-relative
  (D-2026-07-04-4; a ≥25% margin answer was retracted — do not
  re-litigate). Evidence `docs/bench/10gbe-2026-07-05/`.
- **Background (2026-07-04/05, all `[x]`)**: REV4 code-complete +
  measurement gates DATA-COMPLETE (push/pull ≈ 9.5 of 9.88 Gbit/s; owner
  declarations pending in Blocked); 10 GbE session done; w9-3 + review rows
  landed. Codex loop governs all changes (D-2026-07-04-1; DEVLOG 07-04/05).

## Queue (ordered)

1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
   the only work item until it ships**: slices otp-1..13 through the
   codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
   otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b), otp-7 (a, b-1,
   b-2) `[x]`. Current: **otp-8** (fallback byte-carrier — see the
   Now section's assess-first note: the in-stream carrier already
   runs as every slice's fallback). otp-2 (symmetric baseline) is
   RIG-GATED — before otp-10 cutover.
2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
   Shipped (zero-copy resolved — D-2026-07-05-3). Optional owner-gated
   measurement follow-ups (Win 11 bare-metal; disk-path variants;
   >ARC-size push) — disk-path items largely absorbed by otp-2/otp-12's
   symmetric-rig matrices. Env: bench binaries at
   `skippy:/mnt/generic-pool/video/blit-bin/` (/tmp, /home noexec there).
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
   `dp.queue()` is not raced against control-lane events — shape since
   otp-4b, deferred at codex otp-7b-1 F3 (keepalive bounds the window;
   both cancel e2es pin the required behavior).

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

- **Three 10 GbE gate declarations**: ue-1 pass/fail (evidence: band
  holds), ue-2 pass/fail or re-scope (no organic resize at 10 GbE),
  REV4 → Shipped. (The zero-copy revisit verdict and the a/b/c
  question are RESOLVED — D-2026-07-05-3, unparked; measured skippy
  data 1.43 cores daemon-receive / 0.45 client at 9.5 Gbit/s stays
  recorded in DEVLOG + DIAGNOSIS.md.)
- **Push go**: local commits `7f1c4b2`..HEAD (this session's four)
  await the ref-listing + approval flow; windows-latest CI on the
  w9-3 harness fix rides it. (The 40th handoff's `f6e592e..HEAD`
  basis was falsified at session start — origin already sits at
  `7f1c4b2`; see the push-state note above.)

## Open questions

- **(OPEN — owner ack, 2026-07-05, otp-4a)** Unified SizeMtime semantic:
  same-size + dest-NEWER — old push clobbers, session adopts **data-safe
  SKIP** (converge-up; `--force` still overwrites; pinned by
  `same_size_newer_destination_is_skipped_not_clobbered`). Owner: confirm
  or ask for old-push clobber. Reasoning: `.review/findings/otp-4-daemon-serves-transfer.md`.
- **(OPEN)** Historical docs embed `/Users/...` paths — agent rec: leave.
- **(OPEN, 2026-07-04)** `725aa07` tracked a 236-file stale worktree snapshot
  (`.claude/worktrees/vigilant-mayer/`). Agent rec: `git rm -r`; awaits go.
- **(OPEN, 2026-07-04)** `docs/WHITEPAPER.md` §§~309/606/641 still describe
  the deleted `determine_remote_tuning`/`TuningParams` — fold into
  w10-docs-batch (agent rec) or rewrite sooner?
- **(OPEN, ripe — data in hand)** REV4 → Shipped flip: the 10 GbE
  session delivered the measurement evidence; flip awaits the three
  declarations in Blocked (was four — zero-copy resolved,
  D-2026-07-05-3).
- **(OPEN, new 2026-07-05)** CLI foot-gun found during the session:
  `blit copy src_large dst` with an existing local dir, no `./`,
  parses the bare name as an mDNS discovery endpoint and errors
  "remote source must include a module or root"
  (blit-app endpoints.rs). Should local-path existence win over the
  discovery interpretation, or at least improve the error? Candidate
  review-queue row; owner to slot.
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: full suite green
  locally; daemon-spawn e2e flakiness root-caused + fixed on Linux (w9-3:
  port-TOCTOU race + cargo-lock contention). Remaining: windows-latest CI
  on the next push (10d89e0 predates the w9-3 fix).

## Handoff log (newest first, keep ≤ 3)

- **2026-07-10 (41st)** @ `d48351d` — **otp-7b landed and CLOSED through
  the codex loop (both sub-slices reviewed + fixes adjudicated); otp-7
  is done, otp-1..7 `[x]`.** Commits: `ecac9b0` (7b-1 data-plane
  resume), `071799a` (7b-2 fault-summary rider + cancel e2e + the
  tokio-flush RELIABLE fix), `d48351d` (review fixes; verdicts in
  `.review/results/otp-7b-{1,2}.gpt-verdict.md`). Suite 1540 → **1550**,
  fmt/clippy clean, guard proofs by temporary revert throughout.
  **Exact first action next session**: assess **otp-8** (fallback
  byte-carrier) against what already exists — the in-stream carrier has
  been the live fallback since otp-3 and is exercised in both
  directions incl. resume; determine the actual residue before coding.
  In-flight: none; tree clean at handoff commit. Process note: codex
  now runs model gpt-5.6-sol (config default moved past the loop doc's
  gpt-5.5 note); one review round was delayed ~1 h by a codex account
  usage limit (resets hourly-ish; scriptable around).
- **2026-07-10 (40th)** @ `3fa4ec9` — otp-7 Active (owner Q1–Q3,
  D-2026-07-09-1); otp-7a landed and CLOSED through the codex loop
  (`4e5ff58`, fixes `1919410`, wire bounds D-2026-07-10-1, suite
  1530→1540 — the recorded 1529 was a miscount). Its stated first
  action (implement otp-7b from the plan's implementation map) was
  done this session per the 41st. The Blocked "Cargo.lock drift" item
  was dropped — basis falsified (lock last changed at `16237e2`).
- *(39th and earlier pruned to the cap — see DEVLOG 2026-07-06 entries.)*
