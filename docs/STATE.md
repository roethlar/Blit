# STATE — single entry point for "what is true right now"

Last updated: 2026-07-06 (**otp-4b-3 landed + graded (codex 3 passes,
PASS)** — a mid-transfer `CancelJob` now surfaces `SessionFault{CANCELLED}`
to the client over the data plane, no hang; **otp-4b fully closed**.
ONE_TRANSFER_PATH otp-1 + otp-3 + otp-4a + otp-4b (1/2/3) `[x]`, current
slice **otp-5** (daemon-as-SOURCE / pull-equivalent). SMALL_FILE_CEILING
stays paused, D-2026-07-05-1.)
**Owner pushed `master` → GitHub at `10d89e0`**; `f6e592e`..HEAD are
local on top, unpushed — windows-latest CI check rides the next push.

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
  open.** Progress (each through the codex loop; closed-slice detail in
  DEVLOG + `.review/` + REVIEW.md):
  - **otp-1 / otp-3 / otp-4a `[x]`** — wire+session contract
    (`docs/TRANSFER_SESSION.md`); role-parameterized drivers over the
    in-process transport (invariance property in the role suite); daemon
    SERVES `Transfer` as Responder, client `run_source`s as SOURCE over
    a gRPC `FrameTransport`; A/B byte-identical vs old push; SizeMtime =
    data-safe skip (owner-ack open question, below).
  - **otp-4b (1/2/3) `[x]` — data plane fully on the session**: 4b-1
    single-stream TCP data plane (`881d412`+`e1aafcc`+`777dfc5`, codex 3
    passes); 4b-2 mid-transfer resize/multi-stream + sf-2 shape
    correction (`dce56de`, codex PASS); 4b-3 below.
  - **otp-4b-3 `[x]`** (`3ae0a5f`+`a530005`+`46cc4bb`, codex 3 passes,
    PASS) — deterministic mid-transfer cancel: `source_send_half` races
    the payload drain against a peer-framed control-lane fault, so a
    `CancelJob` surfaces `SessionFault{CANCELLED}` (not the data-plane
    `Broken pipe`) and a blocked reader no longer hangs (dropping the
    raced `finish()` future aborts in-flight workers). `recv_peer_fault`
    strict on the live drain; `prefer_peer_fault` lenient on the
    `queue()`/`finish()` error paths. **otp-4b fully closed.** Suite →
    **1516/0**.
  - Current: **otp-5** (daemon-as-SOURCE / pull-equivalent — the same
    session code with roles flipped; the parity suite reruns with no
    per-direction test code). (otp-2 symmetric baseline is rig-gated;
    before otp-10.)
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
  `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`); **sf-3a+
  blocked** until ONE_TRANSFER_PATH ships, then resume/re-derive on
  the unified baseline. Principle stands: ceiling-driven, never
  competitor-relative (D-2026-07-04-4; a ≥25% margin answer was
  retracted — do not re-litigate). Evidence `docs/bench/10gbe-2026-07-05/`.
- **Background (2026-07-04/05, all `[x]`)**: REV4 code-complete +
  measurement gates DATA-COMPLETE (push/pull ≈ 9.5 of 9.88 Gbit/s; owner
  declarations pending in Blocked); 10 GbE session done; w9-3 + eleven
  review-queue rows landed. Codex loop governs all code + plan changes
  (D-2026-07-04-1). Details: DEVLOG 2026-07-04/05.

## Queue (ordered)

1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
   the only work item until it ships**: slices otp-1..13 through the
   codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
   otp-4b (1/2/3) `[x]`. Current: **otp-5** (roles swapped — client
   initiates as DESTINATION, the pull-equivalent; the same session code
   with roles flipped, parity suite reruns with no per-direction test
   code). otp-2 (symmetric baseline) is RIG-GATED — before otp-10
   cutover.
2. **10 GbE owner declarations (still pending)**: ue-1, ue-2,
   REV4 → Shipped (zero-copy resolved — D-2026-07-05-3). Optional
   owner-gated measurement follow-ups (Win 11 bare-metal datapoint;
   disk-path variants; >ARC-size push) — note the disk-path items
   are largely absorbed by otp-2/otp-12's symmetric-rig matrices. Env: bench
   binaries staged at `skippy:/mnt/generic-pool/video/blit-bin/`
   (/tmp and /home on skippy are noexec).
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
   tuning residue (w3-1 scoped it out).

## Authoritative docs right now

- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
  D-2026-07-05-4)**.
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
- **Push go**: local commits `f6e592e`..HEAD await the ref-listing +
  approval flow; windows-latest CI on the w9-3 harness fix rides it.
- `Cargo.lock`: dependency-refresh drift committed at `04c9c6d` (was
  unavoidable — blit-core gained `rand`); revert selectively if
  unwanted, otherwise settled.

## Open questions

- **(OPEN — owner ack requested, 2026-07-05, otp-4a)** Unified
  SizeMtime semantic: same-size + dest-NEWER — old push clobbers, the
  session adopts the **data-safe SKIP** (converge-up; `--force` still
  overwrites; pinned by
  `same_size_newer_destination_is_skipped_not_clobbered`). So "byte-
  identical trees vs old push" is intentionally not literal in that one
  cell. Owner: confirm, or ask for old-push clobber (one-line change).
  Full reasoning: `.review/findings/otp-4-daemon-serves-transfer.md`.
- **(OPEN)** Historical docs embed `/Users/...` paths — agent rec: leave.
- **(OPEN, new 2026-07-04)** `725aa07` tracked a 236-file stale
  worktree snapshot (`.claude/worktrees/vigilant-mayer/`, incl. a
  full `crates/` copy). Keep or `git rm -r`? Agent rec: remove;
  deletion awaits an owner go.
- **(OPEN, new 2026-07-04)** `docs/WHITEPAPER.md` §§~309/606/641 still
  describe `determine_remote_tuning`/`TuningParams` (stale since
  ue-r2-1e, `TuningParams` now deleted) — fold into w10-docs-batch or
  rewrite sooner? Agent rec: w10.
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

- **2026-07-06 (30th)** @ `3ae0a5f`+`a530005`+`46cc4bb`+`db9b63d` —
  **otp-4b-3 landed and graded (codex 3 passes, PASS); otp-4b fully
  closed** (details: DEVLOG 05:37, finding doc otp-4b-3 section,
  `.review/results/otp-4b3-*`). A mid-transfer `CancelJob` now surfaces
  `SessionFault{CANCELLED}` to the client over the data plane and no
  longer hangs a blocked-reader source: `source_send_half` races the
  payload drain against a peer-framed control-lane fault (strict
  `recv_peer_fault` on the live `finish()` select arm — dropping the
  raced future aborts in-flight workers; lenient `prefer_peer_fault` on
  the `queue()`/`finish()` error paths, skipping in-flight needs). Codex
  caught F1 queue-not-preferred + F2 e2e-gate-before-TCP + F3
  drop-drain-events (pass 1), then the F1+F3 interaction spuriously
  raising `PROTOCOL_VIOLATION` (pass 2) — both rounds fixed, pass 3 PASS.
  Suite 1513 → **1516/0**. In-flight: none. **Exact first action next
  session**: otp-5 (daemon-as-SOURCE / pull-equivalent — roles flipped,
  parity suite reruns) through the codex loop. Owner declarations: three
  10 GbE gates + push go remain in Blocked; local `f6e592e`..HEAD
  unpushed.
- **2026-07-06 (29th)** @ `dce56de`+records — **otp-4b-2 landed and
  graded** (mid-transfer stream growth / sf-2 shape correction on the
  session data plane; details: DEVLOG 00:30, `.review/results/otp-4b2-*`).
  SOURCE owns the live dial + proposes `DataPlaneResize{ADD}` per epoch;
  DESTINATION arms + acks + accepts one more socket; settled count on
  `DestinationOutcome.data_plane_streams`. **Codex PASS.** Suite → 1513/0.
- **2026-07-05 (28th)** @ `777dfc5` — otp-4b-1 landed + graded
  (single-stream TCP data plane; codex 3 passes, PASS). Suite → 1512/0.
  (Older: otp-4a @ `fe4ad6d`; details in DEVLOG.)
