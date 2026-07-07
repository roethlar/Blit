# STATE — single entry point for "what is true right now"

Last updated: 2026-07-06 (**otp-6 CLOSED; otp-7 in DESIGN**. otp-6 (a/b)
mirror + filters landed + graded. Current slice **otp-7** (resume block
phase, the plan's RELIABLE exception): owner chose "plan doc first" — slice
design drafted at `docs/plan/OTP7_RESUME.md` (**Draft**), NO CODE until the
owner answers its Q1–Q3 and flips it to Active. ONE_TRANSFER_PATH otp-1..6
`[x]`. SMALL_FILE_CEILING stays paused, D-2026-07-05-1. This session: filed
`audit-17`/`audit-18` (`TODO.md` only); drafted+reviewed
`docs/plan/LOCAL_ERROR_TELEMETRY.md` (Draft, held — see Open questions).
No code, no Queue change (D-2026-07-05-4).)
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
  - **otp-6 (a/b) `[x]`** — mirror + filters on the session, closed.
    6a (`c026692`+`0bb27f5`) honors `SessionOpen.filter` via the universal
    `FilteredSource` chokepoint. 6b (`01d9c41`+`3c99557`) is the one delete
    rule: DESTINATION diffs the complete source manifest at SourceDone,
    scan-complete-guarded + filter-scoped. Codex High: keep-set now folds
    case on macOS too (case-insensitive-FS data-loss). Suite → **1529**.
  - Current: **otp-7 IN DESIGN** — Draft `docs/plan/OTP7_RESUME.md`
    (`9fb5e4a`) awaiting owner review (see Open questions); no code until
    Active. otp-5b-3 (pull cancel) optional; otp-2 rig-gated before otp-10.
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
   otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b) `[x]`. Current:
   **otp-7 IN DESIGN** (`docs/plan/OTP7_RESUME.md` Draft, owner review;
   no code until Active). otp-2 (symmetric baseline) is RIG-GATED —
   before otp-10 cutover.
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
   tuning residue (w3-1 scoped it out).

## Authoritative docs right now

- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Draft** — otp-7 slice
  design, awaiting owner review before any code).
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
- `Cargo.lock`: fresh transitive drift (crossbeam-*, cc, etc.), same class
  as `04c9c6d` — not this session's; owner's call to commit or revert.

## Open questions

- **(OPEN — owner review, 2026-07-07, otp-7)** `docs/plan/OTP7_RESUME.md`
  (Draft) awaits the owner's Q1–Q3 (graceful stale fallback; in-place-patch
  mid-failure model; 7a-then-7b staging — all agent-rec yes) and the flip to
  Active. That flip unblocks otp-7 implementation.
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
- **(OPEN, 2026-07-06, timing not scope)** `docs/plan/LOCAL_ERROR_TELEMETRY.md`
  (Draft, reviewed+fixed) — owner: pick up at ONE_TRANSFER_PATH's next
  natural pause (not a D-2026-07-05-4 exception); trigger = current
  otp-7 slice done.
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

- **2026-07-06 (37th)** @ `deb3800` — **Two owner bug reports triaged,
  filed as `audit-17`/`audit-18` (TODO.md only, D-2026-07-05-4); no
  code, otp-7 untouched.** Both: a `blit copy` that aborted entirely on
  one bad filename mid-copy. `audit-17` (`5628c03`): dest FAT-family fs
  rejects a `:` in a NuGet cache path (`sink.rs:605`, `os error 22`) — no
  invalid-filename handling exists. `audit-18` (`deb3800`): a non-UTF-8
  filename is irreversibly mangled by `relative_path_to_posix`'s
  `to_string_lossy()` before I/O (`payload.rs:360`) — shared by local
  mirror + remote push. Both need an owner `plan` before code.
  **Exact first action next session**: otp-7 — owner's Q1–Q3 on
  `docs/plan/OTP7_RESUME.md`, flip Active, codex-review, implement
  otp-7a. In-flight: none. Done since 36th: nothing on
  ONE_TRANSFER_PATH — pure bug triage.
- **2026-07-07 (36th)** @ `9fb5e4a` — **otp-7 slice design drafted; owner
  review pending (NO CODE)**. Owner picked "plan doc first" for the RELIABLE
  resume slice; `docs/plan/OTP7_RESUME.md` (Draft) records the strict-ordering
  block-hash choreography, the reuse map, design decisions (graceful stale
  fallback, source block-diff as a session helper, in-place-patch mid-failure),
  7a-in-stream/7b-data-plane staging, and the four guard-proof targets.
  **Exact first action next session**: get the owner's Q1–Q3 answers + flip to
  Active, then codex-review the plan change (D-2026-07-04-1) and implement
  otp-7a. In-flight: none. Done since 35th: otp-6 fully closed; `f6e592e`..HEAD
  unpushed (incl. otp-6a/6b + this design doc).
