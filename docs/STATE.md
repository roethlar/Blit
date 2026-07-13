# STATE — single entry point for "what is true right now"

Last updated: 2026-07-12

- Recent sessions (2026-07-11/12, 44th–45th): **otp-10 AND otp-11 fully closed through the codex loop** — every transfer (local included) rides the ONE session; the separate local orchestration no longer exists (−6.2k lines at 11b); the old journal fast path was proven UNSOUND (data-loss repro recorded) and died with it. Suite **1484** (the otp-13 ≥1483 floor met at the deletion slice). SMALL_FILE_CEILING paused (D-2026-07-05-1). Push state: see Blocked.

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
   (`docs/bench/otp12-{zoey,win}-2026-07-12/`). Current: **otp-12c
   (delegated, netwatch-01↔skippy)** — direct-path baseline at
   `f35702a` recorded 2026-07-13
   (`docs/bench/otp12c-win-2026-07-13/`); delegated session next.
   Then 12d, otp-13.
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

- **Rigs**: owner go GIVEN 2026-07-12; zoey (12a) + netwatch-01 (12b)
  sessions done. Remaining: 12c delegated = netwatch-01↔skippy
  (`admin@skippy`, x86_64, pool paths only; fresh staging needed).
- **otp-12c prep DONE 2026-07-12**: same-build `f35702a` on both ends;
  corpus restaged; cross-machine smokes pass both directions. Rig
  foot-gun (cost one debugging round): Win32-OpenSSH reaps
  `Start-Process` children when the spawning SSH session closes — the
  netwatch daemon silently died and 9031 stealth-dropped (mimicked a
  firewall block; rules were never the problem). Start Windows daemons
  detached via WMI `Invoke-CimMethod Win32_Process Create` (Linux
  side unaffected — skippy daemons survive SSH close fine). The prep
  daemons were stopped before the 12c recorded runs; harnesses own
  their daemon lifecycles end-to-end.
- **otp-12c direct-path baseline RECORDED 2026-07-13**
  (`docs/bench/otp12c-win-2026-07-13/`): rig-W matrix re-run, new arm
  at the cutover sha `f35702a` (12b's new arm was `e21cf84`), old arm
  `0f922de`. 198 timed runs, 24/24 cells complete, 3 DRAIN-TIMEOUT
  pairs voided+re-run, 0 CR residue. Verdicts: 93 PASS / 12 FAIL /
  3 FAIL-SAME-SESSION / 12 RECORDED. Texture: wm_tcp_mixed invariance
  **1.300** (12b: 1.237 — the same code-shaped cell, not washed out at
  the cutover sha); new-vs-old_session losses concentrate in
  TCP×{small,mixed}×push + pull_tcp_mixed (1.14–1.25) while the new
  arm wins the small-pull side; mw_* cross rows restate the push>pull
  gap vs the committed 12b baselines. Adjudication belongs to otp-13.
  Delegated harness: `scripts/bench_otp12_delegated.sh` (committed as
  a draft; before a live run it needs `EXPECT_SHA=f35702a` + skippy
  binary/module path confirmation, and its first session is a
  shakeout — see script header).
- **Three 10 GbE gate declarations**: ue-1, ue-2 (pass/fail or
  re-scope), REV4 → Shipped. (Zero-copy RESOLVED — D-2026-07-05-3.)
- ~~Push go~~ **RESOLVED 2026-07-12 (owner: push approved via the
  ref-listing flow)**: pushed `f19776c..fbef546` (9 commits) →
  origin/master = `fbef546`. Note: a second partial push outside these
  sessions had already moved the remote `6d37a22` → `f19776c`
  (fast-forward ancestor of HEAD, no divergence), carrying the w9-3
  windows-latest CI fix. Local and remote are now in sync.
- ~~otp-5b-3~~ **RESOLVED 2026-07-12 (owner: "write the test")** —
  re-scoped: the pull-specific framing is mooted by direction-invariance
  (one session, D-2026-07-05-1); what's missing is a direction-agnostic
  **mid-copy cancel e2e** (cancel while file data is in flight; existing
  coverage only hits mirror-purge cancel + jobs lifecycle). Near-term
  standalone slice; codex loop applies. **LANDED `920c6a7`**: the test
  exposed a real bug — a peer `Frame::Error` arriving mid-record
  (file or tar-shard) was misreported as a ProtocolViolation about
  frame position instead of surfacing the peer's own fault (plan D4
  says a CANCELLED must stay CANCELLED). Both record receivers now
  match the block-record handling; suite 39/39. Companion commit
  `ace91de` shipped the CLI foot-gun `./NAME` suggestion + unified
  remote-refusal wording. Pushed 2026-07-12 (owner: "commit then
  push"): `fbef546..8f15ce2` (6 commits, incl. otp12 review
  artifacts) → origin/master = `8f15ce2`.
- ~~The change-journal question~~ **RESOLVED 2026-07-12 (owner:
  "neither option passes — figure out a real fix"; the premise was
  false)**: the old 21 ms journal skip was UNSOUND — `NoChanges`
  decays to root-dir mtime equality, so deep modifications silently
  never synced (REPRODUCED against the pre-otp-11 binary; transcript
  in `docs/bench/otp11-local-2026-07-11/README.md`). Sound-vs-sound
  the session no-op wins 2.2× (226 vs 507 ms/10k, 5-run medians) →
  gate passes;
  11b's journal deletion removes a data-loss bug. Pinned:
  `deep_modification_after_warm_runs_syncs`. Sound O(changes) no-op
  (journal REPLAY as a session phase, both carriers) filed as future
  capability — slice doc D3. **otp-11b is UNBLOCKED.**

## Open questions

- **(RESOLVED 2026-07-12 — owner confirmed SKIP)** Unified SizeMtime
  semantic: same-size + dest-NEWER = **data-safe SKIP** (converge-up;
  `--force` still overwrites; pinned by
  `same_size_newer_destination_is_skipped_not_clobbered`). Owner ack
  after trade-off review. Reasoning: `.review/findings/otp-4-daemon-serves-transfer.md`.
- **(RESOLVED 2026-07-12 — owner go)** `725aa07` stale worktree
  snapshot removed via `git rm -r .claude/worktrees/vigilant-mayer`
  (236 files; dir was not a registered worktree). Historical docs
  embedding `/Users/...` paths: leave (owner-accepted rec).
- **(SLOTTED 2026-07-12 — owner ack)** `docs/WHITEPAPER.md` §8 (~line
  592) describes the deleted `determine_remote_tuning` — fix folded
  into **w10-docs-batch** (rewrite the stale sentence to current
  `auto_tune` reality); no one-off edit now.
- **(OPEN, ripe — data in hand)** REV4 → Shipped flip: awaits the
  three declarations in Blocked (zero-copy resolved, D-2026-07-05-3).
- **(RESOLVED 2026-07-12 — owner: better error, parsing unchanged)**
  CLI foot-gun: a bare local dir name with no `./` still parses as an
  mDNS endpoint (no local-wins ambiguity introduced), but when the
  lookup fails AND a local path of that name exists, the error must
  suggest it: "'NAME' exists here as a folder — did you mean ./NAME?"
  (blit-app endpoints.rs). Small standalone slice.
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: w9-3 fixed the
  Linux daemon-spawn flakiness; windows-latest CI pending the next
  push. NOTE 2026-07-12: the macOS `blit_utils` residual (pre-existing,
  reproduced at `6d37a22`) ran ELEVATED under heavy load (~3/12 vs 2/8
  historical) — own finding if it persists on a quiet machine.

## Handoff log (newest first, keep ≤ 3)

- **2026-07-12 (45th, this session)** — **otp-11 CLOSED WHOLE (11a
  route + journal-hole addendum + 11b deletion, four codex rounds;
  suite 1488 → 1484 with the ≥1483 floor met by real pins; the
  separate local orchestration no longer exists)**. In-flight: none;
  tree clean. **Next**: otp-12 (rig-gated, Blocked) → otp-13.
- **2026-07-11 (44th)** — otp-10c closed (relay removal + the cutover
  deletion); suite 1605 → 1488. Owner ask pending: `725aa07` snapshot.
- **2026-07-11 (43rd)** — otp-10a/10b closed; verb cutover complete.
- *(42nd and earlier pruned to the cap — see DEVLOG 2026-07-06..12.)*
