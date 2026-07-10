# STATE — single entry point for "what is true right now"

Last updated: 2026-07-10

- 2026-07-04: Owner-approved dual push reached 3d8326b (origin: 10d89e0..3d8326b; gitea mirror: 2a77b9f..3d8326b). That push corrected a prior remote-name confusion; windows-latest CI on that push is the "meaningfully green" check referenced in prior notes.

- Current session (2026-07-10, this one): **otp-8 and otp-9 (a+b) landed and CLOSED through the codex loop** — otp-8 by assessment + wire pins (`5ffc9be`/`643294a`); otp-9 delegated-on-session (`7bf8ef8`/`607a924`, `b2fd876`/`1ce73b5` — codex caught two session-wide High findings, both fixed: require_complete_scan enforcement + cancellation-abortable mirror pass). Verdicts in `.review/results/otp-{8,9a,9b}.gpt-verdict.md`. ONE_TRANSFER_PATH otp-1..9 [x]. SMALL_FILE_CEILING paused (D-2026-07-05-1).

- Notes on push state (as of `1ce73b5`; basis: the prior session's `git ls-remote origin` check — not re-verified this session): origin/master was at `7f1c4b2`. Unpushed local commits: `7f1c4b2..HEAD`. windows-latest CI on the w9-3 harness fix rides the next push.

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
  - **otp-8 `[x]` — fallback byte-carrier, CLOSED by assessment +
    wire residue pins** (`5ffc9be`; codex fixes `643294a`: in-stream
    cancel HANG → fault race; unbounded `TarShardHeader` frame →
    splitter). Detail: DEVLOG 2026-07-10 14:15Z + `.review/`.
  - **otp-7b (1/2) `[x]` — resume over the TCP data plane + the D4
    fault-summary rider; otp-7 done** (`ecac9b0`, `071799a`, review
    fixes `d48351d`). Per-carrier block ceiling D-2026-07-10-2;
    `SessionFault.relative_path` (CONTRACT_VERSION → 2) +
    `end_of_operation_summary()` (verb print at otp-10); RELIABLE
    flush fix. Detail: DEVLOG 2026-07-10 07:30Z + `.review/`.
  - **otp-9 `[x]` (a: `7bf8ef8`+`607a924`; b: `b2fd876`+`1ce73b5`)
    — the delegated transfer rides the unified session**;
    `DelegatedPull` = trigger + progress relay. Codex 9b caught two
    session-wide Highs, both fixed: `require_complete_scan` ENFORCED
    (SCAN_INCOMPLETE refusal) and the mirror delete pass made
    cancellation-abortable. Suite → **1555**. Detail: DEVLOG
    2026-07-10 17:30Z + `.review/`.
  - **otp-2 — PER-DIRECTION rig baseline recorded; symmetric-fs
    verdict cells NOT satisfiable on this rig (codex F1, upheld)**.
    Owner opened Mac ↔ zoey (Thunderbolt 10GbE, zoey confined to
    `blit-temp`); harness `scripts/bench_otp2_baseline.sh` (cold
    caches, durable-at-dest windows both ends, pool drain,
    median-of-4); evidence
    `docs/bench/otp2-baseline-2026-07-10/README.md`. These endpoints
    are hardware-asymmetric, and D-2026-07-05-1's own rule says
    cross-direction comparisons are valid ONLY on symmetric
    endpoints — so this data anchors per-direction converge-up
    (old-vs-new, same cell) and CANNOT anchor the otp-12 bar's
    cross-direction half. July tmpfs data re-labeled wire-reference.
  - Current: **HOLD for the owner adjudication below** (otp-12
    acceptance bar on asymmetric rigs / whether a symmetric pair
    will be provided); **otp-10 (cutover + deletion)** follows.
    otp-5b-3 (pull cancel) optional.
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
   b-2), otp-8, otp-9 (a/b) `[x]`; otp-2 per-direction baseline
   recorded (symmetric-fs half = the owner question in Open
   questions). Current: **HOLD on that adjudication**, then
   **otp-10** (cutover + deletion).
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
   `dp.queue()` is not raced against control-lane events (deferred at
   codex otp-7b-1 F3; otp-8 F1 gave the in-stream sends a fault race —
   residual: the narrow CANCELLED→INTERNAL decay, verdict file).
   Delegated Checksum compare degrades to transfer-for-verification
   (otp-9b Known gap — session dest diff computes no local checksums).

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

- **otp-12 acceptance-bar adjudication (NEW — holds otp-10)**: see
  the first Open question. The rig itself is open (owner gave SSH +
  the 10GbE pair; per-direction baseline recorded). Standing owner
  instruction captured: the Windows 10GbE box + TrueNAS serve
  **remote↔remote (delegated) testing** when that stage comes
  (otp-12 matrix). otp-5b-3 question stands.
- **Three 10 GbE gate declarations**: ue-1 pass/fail, ue-2 pass/fail
  or re-scope, REV4 → Shipped. (Zero-copy a/b/c RESOLVED —
  D-2026-07-05-3; skippy CPU data stays in DEVLOG + DIAGNOSIS.md.)
- **Push go**: local commits `7f1c4b2..HEAD` (otp-7b through otp-9
  close) await the ref-listing + approval flow; windows-latest CI on
  the w9-3 harness fix rides it.

## Open questions

- **(OPEN, new 2026-07-10, otp-2 — owner call BEFORE otp-10 proceeds)**
  D-2026-07-05-1 already rules that cross-direction comparisons are
  valid only on symmetric endpoints; the Mac↔zoey rig's write ends
  are asymmetric (client SSD vs daemon pool — old-pull beat old-push
  1.3–1.8× in every recorded cell), so the otp-12 bar's
  cross-direction half ("every cell ≤ the better of that cell's two
  old directions") cannot be evaluated on this rig, and otp-2's
  "symmetric-fs verdict cells" deliverable is only partially
  satisfiable here. Owner options: (a) accept per-direction
  converge-up (new ≤ old, same cell, +10%) as this rig's verdict and
  proceed to otp-10, and/or (b) designate a hardware-symmetric pair
  (e.g. two like machines) for the cross-direction half before
  otp-12. Details: `docs/bench/otp2-baseline-2026-07-10/README.md`.
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

- **2026-07-10 (41st)** @ `d48351d` — otp-7b closed through the codex
  loop; otp-1..7 `[x]`; suite → 1550. Its stated first action (assess
  otp-8 before coding) was done this session. Process note: codex now
  runs gpt-5.6-sol; one round was delayed ~1 h by a codex usage limit.
- *(40th and earlier pruned to the cap — see DEVLOG 2026-07-06..10.)*
