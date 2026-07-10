# STATE — single entry point for "what is true right now"

Last updated: 2026-07-10

- 2026-07-04: Owner-approved dual push reached 3d8326b (origin: 10d89e0..3d8326b; gitea mirror: 2a77b9f..3d8326b). That push corrected a prior remote-name confusion; windows-latest CI on that push is the "meaningfully green" check referenced in prior notes.

- Current session (2026-07-10, this one): **otp-8, otp-9 (a+b), and otp-2 (zoey per-direction + otp-2w Windows cross-direction) all landed and CLOSED through the codex loop** — code slices first, then the owner opened three rigs mid-session (zoey, the Windows box, skippy) and both benchmark baselines were recorded, reviewed (8- and 7-finding rounds, incl. a timing-overhead bug that forced a re-measure of both matrices), and committed. Verdicts in `.review/results/otp-{8,9a,9b,2,2w}.gpt-verdict.md`. ONE_TRANSFER_PATH otp-1..9 + otp-2 [x]; **otp-10 is next and nothing holds it**. SMALL_FILE_CEILING paused (D-2026-07-05-1).

- Notes on push state (re-verified via `git ls-remote origin` at this handoff, as of `cccd89a`): origin/master is at `7f1c4b2`. Unpushed local commits: `7f1c4b2..HEAD` = **24** (otp-7b close through otp-2w close). windows-latest CI on the w9-3 harness fix rides the next push.

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
  - **otp-2 `[x]` (both halves).** zoey rig = PER-DIRECTION
    reference (hardware-asymmetric ends; D-2026-07-05-1 forbids
    cross-direction verdicts there — codex F1 upheld); owner then
    designated Mac↔Windows = cross-direction rig (**otp-2w**).
    Harnesses `scripts/bench_otp2{,w}_baseline.sh` (cold caches,
    SELF-TIMED durable-at-dest windows — an in-window ssh flush had
    inflated push medians ~1.2 s on both rigs, both matrices
    re-measured), evidence
    `docs/bench/otp2{,w}-baseline-2026-07-10/README.md`. July tmpfs
    data re-labeled wire-reference. Key reading: old push trails old
    pull on BOTH rigs (Windows ×1.46–×2.38), carrier-insensitive on
    large — otp-12's interleaved old-vs-new discriminates code cost
    from platform write-path cost.
  - Current: **otp-10 (cutover + deletion)** — staging sketch from
    the survey: 10a push-shaped verb rides `run_push_session` (+ the
    deferred verb wiring: PushSessionOptions mirror/filter,
    `--force-grpc`, progress line via ByteProgressSink,
    `end_of_operation_summary` print, resume flags; A/B parity pins
    vs old push); 10b pull-shaped verb likewise (options exist since
    9a); 10c deletion — 4 drivers + `Push`/`PullSync` RPCs out of
    tree AND proto, no bridge, ported-test accounting + file-by-file
    deletion proof (incl. DelegatedPull no-payload-bytes assertion).
    Dispatch chokepoint: `blit-app/src/transfers/dispatch.rs`
    (`TransferRoute`). otp-5b-3 (pull cancel) optional.
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
   b-2), otp-8, otp-9 (a/b), otp-2 (zoey per-direction + otp-2w
   Windows cross-direction) `[x]`. Current: **otp-10 (cutover +
   deletion)**.
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

- **Rig availability (owner, 2026-07-10, verified by ssh)**: for the
  otp-12 matrix — remote↔remote (delegated) uses the Windows box
  (`michael@10.1.10.173`) + TrueNAS `skippy` (`admin@skippy`,
  x86_64; existing test folder `/mnt/generic-pool/video/blit-bin/`
  with July binaries + bench.toml; /tmp and /home are noexec there);
  skippy also available for Mac↔Linux cells "if needed" (owner).
  zoey = per-direction rig; Windows pair = cross-direction rig.
- **Three 10 GbE gate declarations**: ue-1 pass/fail, ue-2 pass/fail
  or re-scope, REV4 → Shipped. (Zero-copy a/b/c RESOLVED —
  D-2026-07-05-3; skippy CPU data stays in DEVLOG + DIAGNOSIS.md.)
- **Push go**: local commits `7f1c4b2..HEAD` (24 — otp-7b close
  through otp-2w close, as of `cccd89a`) await the ref-listing +
  approval flow; windows-latest CI on the w9-3 harness fix rides it.
- **otp-5b-3** (pull mid-transfer cancel e2e, marked optional): pick
  up while otp-10 runs, or drop? — standing question.

## Open questions

- **(RESOLVED 2026-07-10 — owner)** Asymmetric-rig acceptance
  question: the owner designated Mac↔Windows as the closer-spec pair
  for the cross-direction half (recorded as otp-2w, see Now); zoey
  stays per-direction-only per D-2026-07-05-1. otp-12 runs both:
  per-direction converge-up on zoey, both halves on the Windows pair
  (interleaved A/B for push cells on both rigs).
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

- **2026-07-10 (42nd)** @ `cccd89a` — **otp-8, otp-9, otp-2/otp-2w
  all closed through the codex loop; otp-1..9 + otp-2 `[x]`; both
  benchmark baselines recorded on owner-opened rigs.** In-flight:
  none; tree clean. **Exact first action next session**: implement
  **otp-10a** — the push-shaped verb rides `run_push_session` per the
  staging sketch in Now (dispatch chokepoint
  `blit-app/src/transfers/dispatch.rs`); codex loop per sub-slice.
  Machine-local (this Mac): rig SSH keys installed (zoey root,
  Windows michael@10.1.10.173, skippy admin); NOPASSWD purge sudoers
  rule; zig/cargo-zigbuild toolchain; ssh ControlMaster sockets.
  Windows box keeps the blit-bench-daemon firewall rule + staged
  purge script; zoey keeps `e757dcc` binaries in blit-temp (for
  otp-12 interleaved A/B), Windows repo checkout is DETACHED at
  `0f922de` with the owner's prior state stashed (`bench-cargo-lock`).
- **2026-07-10 (41st)** @ `d48351d` — otp-7b closed through the codex
  loop; otp-1..7 `[x]`; suite → 1550. Process note: codex runs
  gpt-5.6-sol; one round was delayed ~1 h by a codex usage limit.
- *(40th and earlier pruned to the cap — see DEVLOG 2026-07-06..10.)*
