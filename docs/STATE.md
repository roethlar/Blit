# STATE — single entry point for "what is true right now"

Last updated: 2026-07-06 (**otp-6a landed + graded (codex FAIL → 1
Medium accepted + fixed)** — the unified session now honors
`SessionOpen.filter` on the SOURCE scan, applied through the universal
`FilteredSource` chokepoint so filtering is impl-independent (the fix
closed a latent gap for otp-9's remote source). otp-6 split into 6a
(filters, done) + 6b (mirror, next). ONE_TRANSFER_PATH otp-1 + otp-3 +
otp-4a + otp-4b (1/2/3) + otp-5a + otp-5b (1/2) + otp-6a `[x]`, current
slice **otp-6b** (mirror on the session, one delete rule).
SMALL_FILE_CEILING stays paused, D-2026-07-05-1.)
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
    serves `Transfer` as Responder, client push over gRPC; A/B
    byte-identical vs old push; SizeMtime = data-safe skip (open Q below).
  - **otp-4b (1/2/3) `[x]` — push data plane fully on the session, closed**:
    single-stream TCP data plane, mid-transfer resize/multi-stream + sf-2
    shape correction, deterministic mid-transfer cancel. Detail: DEVLOG.
  - **otp-5a `[x]`** (`84be1cc`, codex PASS) — the one served `Transfer`
    RPC serves BOTH roles via `run_responder` (SOURCE-init→daemon
    DESTINATION = push; DEST-init→daemon SOURCE = pull, in-stream).
  - **otp-5b (1/2) `[x]`** — the SOURCE-responder data plane, closed:
    5b-1 (`e6a0b3b`+`13485ee`) decoupled connection role (RESPONDER
    binds+accepts, INITIATOR dials) from byte role; 5b-2 (`d579365`+
    `773a877`) lifted the single-stream cap — the pull data plane resizes
    via sf-2 (same resize frames as push). Defaults to TCP; A/B
    byte-identical vs old `pull_sync`. Suite → **1522**.
  - **otp-6a `[x]`** (`c026692`+`0bb27f5`, codex FAIL→1 Med fixed) — the
    session honors `SessionOpen.filter` on the SOURCE scan, applied via the
    universal `FilteredSource` chokepoint (not the per-impl `scan` arg, which
    only `FsTransferSource` honors). Bad peer globs refused at OPEN. → 1524.
  - Current: **otp-6b** (mirror on the session — the one delete rule).
    otp-5b-3 (pull mid-transfer cancel) is optional — the CANCELLED framing
    is role-agnostic. (otp-2 symmetric baseline is rig-gated; before otp-10.)
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
   otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6a `[x]`. Current: **otp-6b**
   (mirror on the session — the one delete rule: the DESTINATION computes
   deletions from the completed source manifest, filter-scoped +
   scan-complete-guarded, and executes them locally; `entries_deleted` in
   the summary). otp-2 (symmetric baseline) is RIG-GATED — before otp-10
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

- **2026-07-06 (34th)** @ `c026692`+`0bb27f5` — **otp-6a (filters on the
  session, codex FAIL → 1 Medium accepted + fixed)** (DEVLOG 22:30;
  `.review/…/otp-6a-session-filters.md`, `.review/results/otp-6a.*`). The
  session honors `SessionOpen.filter` on the SOURCE scan:
  `source_open_validator` validates globs (peer-notified refusal at OPEN)
  instead of refusing all filters. F1 (Med, `0bb27f5`): routed filtering
  through the universal `FilteredSource` decorator, not the per-impl
  `scan(filter)` arg — only `FsTransferSource` honored the arg;
  `RemoteTransferSource` ignores it (latent silent-no-filter for otp-9's
  remote relay). Two guard tests. Suite 1522 → **1524/0**. **Exact first
  action next session**: otp-6b (mirror — DESTINATION accumulates the
  complete source manifest, scan-complete-guarded, filter-scoped extraneous
  delete, `entries_deleted`; relax `destination_open_validator`'s mirror
  refusal, the `mirror_request_is_refused_until_its_slice_lands` test flips).
  In-flight: none. 10 GbE gates + push go remain in Blocked;
  `f6e592e`..HEAD unpushed. (`Cargo.lock` drift unstaged.)
- **2026-07-06 (33rd)** @ `d579365`+`773a877` — **otp-5b-2 (pull data-plane
  resize, codex NEEDS FIXES → 1 Low accepted + fixed)** (DEVLOG 20:15;
  `.review/…/otp-5b-2-pull-data-plane-resize.{md,gpt-verdict.md}`). The pull
  data plane grows mid-transfer via the same sf-2 shape correction as push;
  resize frames identical both ways, only socket acquisition flips (SOURCE
  responder accepts each epoch-N socket, DEST initiator dials it). F1 (Low):
  ceiling reads the advertised `receiver_capacity`, not a fresh local read.
  Suite → **1522**.
