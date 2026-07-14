# STATE — single entry point for "what is true right now"

Last updated: 2026-07-13

- **NEXT ACTION — run the A-B-B-A MTU experiment on rig `q` (Queue 1a).** Everything is staged and validated; it is ~55 min of unattended rig time. Exact command + design + decision rule: `docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md` (**rev 4**; codex rounds 1+2 = 15 findings, **15/15 accepted**). Four sessions **9000, 1500, 1500, 9000** on `q`, RUNS=8, `CELLS=wm_tcp_mixed,wm_tcp_large,mw_tcp_mixed,wm_grpc_mixed`; the same-MTU replicates supply the **measured noise floor** every threshold is expressed against.
- **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption H1/H5/H6/H7 all rest on, never tested until now. **And jumbo does NOT dissolve P1.** What is still unmeasured is how much MTU *contributes* (a FAIL only proves jumbo is *insufficient*) — that needs the matched 1500 arm, i.e. the A-B-B-A run.
- **THE MAC IS A BENCH END — the codex loop and a rig-W session CANNOT run concurrently** (`.agents/machines.md`). A 53-min A-B-B-A attempt was destroyed by codex load on the Mac and discarded; the contamination is *asymmetric* (it inflates `mac_init` and MANUFACTURES P1). **Rig-W now runs on `q`** (dedicated M4 mini, quiet, faster than nagatha), which decouples the two for good.
- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
- **P1 (the headline invariance criterion) — the one thing between blit and shipping.** Fails rig W (`wm_tcp_mixed` 1.237 and 1.300 — do NOT read that as a regression, it is **two different Mac NICs**), but **PASSES 8/8 with Linux on both ends** (`docs/bench/otp12-perf-2026-07-13/`; P1's own cell 1.092/1.003). So it is **platform-INTERACTING, not pure layout** — yet **NOT exonerated**: a code path that only bites on one platform (H1's Windows accept branch) looks identical. **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a *cross-direction* miss for a cell that ALREADY passes invariance — P1 *is* the invariance failure. So: **fix it to ≤1.10, or the owner amends acceptance criterion 1.** Neither is assumed.
- **⚠ THREE of my claims were reported and RETRACTED on 2026-07-13**, all the same root cause — trusting an instrument I had not validated: (1) "P1 is code" (a harness that keyed durability to the *initiator*, not the destination); (2) "P1 is acceptable platform residue" (D-2026-07-12-1 does not cover it); (3) "macOS can't send jumbo / the switch is broken" (it was `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — it cost the owner a pointless adapter swap). **Verify the instrument before believing the measurement.**

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
  - **Closed `[x]`: otp-1 … otp-11** — the whole session machine, the
    baselines (otp-2/2w), the **CUTOVER DELETION** (4 drivers +
    `Push`/`PullSync` + 13 messages out of tree AND proto, −13.8k lines,
    no bridge; relay removed D-2026-07-11-1), and **otp-11b's deletion of
    the entire old orchestration** (−6.2k lines: orchestrator, engine,
    local_worker, auto_tune, change_journal — the last an UNSOUND fast
    path that silently lost data). The deletion-proof acceptance line
    COMPLETES. Detail: DEVLOG 2026-07-10/11/12; evidence
    `docs/bench/otp2{,w}-baseline-2026-07-10/`, `otp11-local-2026-07-11/`.
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
  `[x]`; **sf-3a+ blocked** until ONE_TRANSFER_PATH ships, then
  resume/re-derive on the unified baseline. Principle: ceiling-driven,
  never competitor-relative (D-2026-07-04-4 — do not re-litigate).
- **Background**: REV4 code-complete, gates DATA-COMPLETE (declarations
  in Blocked); the codex loop governs all changes (D-2026-07-04-1).

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
1a. **`docs/plan/OTP12_PERF_FINDINGS.md` — THE REAL NEXT ITEM**
   (**ACTIVE**, D-2026-07-13-1 — owner: "just write the code and
   reviewloop slice by slice"; implementation proceeds, each slice
   through the codex loop).
   Two experiments come BEFORE any code; both docs own their detail.
   **(i) The A-B-B-A MTU run on `q`** —
   `docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md` (rev 4;
   codex 15/15 accepted). Answers how much MTU *contributes*; we already
   know jumbo does not FIX P1 (q baseline, 1.385 at 9000).
   **(ii) THE MAC↔MAC RIG — the missing cell, and it discriminates the
   hypotheses** (owner, 2026-07-13; UNTESTED, now possible: nagatha `.92`
   + `q` `.54`, both 10GbE/MTU 9000). Linux↔Linux = **no P1** (8/8 PASS);
   macOS↔Windows = **P1** (1.237/1.300/1.385); macOS↔macOS = **?**
   - reproduces → P1 needs **no Windows peer**; it is macOS-side and
     **H1 DIES** (H1 accuses the *Windows* accept branch);
   - vanishes → P1 **requires** the Windows peer → H1 strongly supported.
   Needs a 3rd harness variant (rig-W's is Windows-specific; the Linux
   one is Linux-specific) — macOS durability (fsync walk) + `purge` both
   ends; through the codex loop. **Schedule for nagatha idle time.**
   **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a
   *cross-direction* miss for a cell that ALREADY passes invariance — P1
   *is* the invariance failure. **Fix it to ≤1.10, or the owner amends
   acceptance criterion 1.** Not assumed either way. P2
   (`push_tcp_small` 1.105–1.201) is a converge bar vs the OLD build,
   UNTESTED on the Linux rig. Sequence: **MTU run + Mac↔Mac → pf-1 → fix
   → pf-final (ALL rigs) → otp-12d → otp-13.**
1b. **AFTER otp-12 — the Windows/local pair, planned TOGETHER** (same tar
   path, opposite directions: a fidelity fix ADDS per-file work to a path
   already losing to robocopy, so planning them apart optimises one against
   the other). Both docs own their detail; do not restate it here.
   - **`docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md` (D-2026-07-13-3)**
     — Windows attributes + ADS silently dropped, exit 0, **both routes
     (measured)**; loss is **conditional on file count**
     (`transfer_plan.rs:103-109`). Unlanded Windows support, NOT a regression.
     **Fix = WIRE CONTRACT change** → amend `TRANSFER_SESSION.md` first.
   - **`docs/plan/LOCAL_SMALL_FILE_PATH.md` (Draft, D-2026-07-13-2)** — local
     apply **does not scale** (8 workers buy 1.05×; robocopy gets ~2.2× from 8
     threads) and ships **one** worker. At EQUAL concurrency blit BEATS
     robocopy; at 8-vs-8 it loses 1.9×. `docs/bench/win-local-ab-2026-07-13/`.
2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
   Shipped (zero-copy resolved — D-2026-07-05-3). Follow-ups largely
   absorbed by otp-2/otp-12's rig matrices.
3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
   resumes/re-derives after ONE_TRANSFER_PATH ships.
4. **PAUSED: design-review queue** (`REVIEW.md`; w7-1 topmost open row —
   likely landed inside otp-6's one-delete-rule slice; re-check first).
5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
   Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
   cutover as a runtime-selected write strategy in the unified receive
   sink (design: eval doc §If-FAST-evidence; dead module deletes in
   w8-1). Rig facts + build recipe: DEVLOG 2026-07-05 10:00.
   **Standing owner safety rule**: ALL activity on rig `zoey` stays
   inside its `…/blit-temp/` folder — nothing written outside it, ever;
   no daemon runs on zoey without a fresh go.
6. **Post-REV4 residue** (unowned, 5 items) — list in DEVLOG 2026-07-13 21:00Z.

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

- **Rigs**: owner go standing through otp-12. zoey (12a), netwatch-01
  (12b), netwatch-01↔skippy (12c) done; **magneto↔skippy = the same-OS
  rig** (new 2026-07-13). Rig facts + the macOS ping/MTU trap:
  `.agents/machines.md`.
- **otp-12c RECORDED 2026-07-13** (pre-fix rows = replication/control
  evidence, NOT acceptance evidence; Queue 1a):
  `docs/bench/otp12c-win-2026-07-13/` (198 runs) and
  `otp12c-delegated-2026-07-13/` (**rig D 7/7 PASS**). Codex: FAIL →
  **7/7 accepted**. Detail: DEVLOG 2026-07-13.
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
- *(Resolved 2026-07-12/13 — SizeMtime data-safe SKIP, the `725aa07`
  snapshot, the CLI `./NAME` foot-gun, otp-5b-3 mid-copy cancel, the
  change-journal premise: all landed; see DEVLOG.)*

## Handoff log (newest first, keep ≤ 3)

- **2026-07-13/14 (47th)** — HEAD `0f52e6a` + this handoff. **P1 REPRODUCES ON
  A SECOND MAC** (`q`, 1.385 FAIL at MTU 9000; three controls PASS 1.002–1.043
  in-session → rig noise 2–4%): P1 is the macOS↔Windows **pairing**, not one
  machine — the untested assumption under **H1** (corrected 2026-07-14: H5/H6/H7
  are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong) — and **jumbo does
  not dissolve it**. **New dedicated bench Mac `q`** (M4 mini, quiet, 1.18 GB/s;
  `.agents/machines.md`) — because **the Mac is a bench END and the codex loop
  cannot run during a rig-W session**: a 53-min A-B-B-A run was contaminated by
  codex on the Mac and DISCARDED (asymmetric — it inflates `mac_init` and
  MANUFACTURES P1). Also landed: **a Windows correctness bug** (attrs + ADS
  silently dropped, both routes, conditional on FILE COUNT — D-2026-07-13-3);
  **the local blit-vs-robocopy headline was WRONG** (8-thread robocopy vs
  1-worker blit — owner caught it; at equal concurrency blit WINS, and the real
  defect is that blit does not SCALE — D-2026-07-13-2); MTU prereg rev 1→4
  (codex 15/15 accepted; every threshold I first wrote was *invented* because
  the design had no noise model).
  **In-flight: none. Rigs clean, Windows MTU 9000, 4 commits unpushed.**
  **NEXT: run the A-B-B-A on `q`** (~55 min; command in the prereg).
- **2026-07-13 (46th)** — otp-12c closed (rig D 7/7); same-OS Linux rig built
  (8/8 PASS → P1 is platform-INTERACTING); perf plan ACTIVE (D-2026-07-13-1);
  **three claims retracted, all from trusting an unvalidated instrument**.
  Full entry: **DEVLOG 2026-07-13 20:00Z**.
- *(45th and earlier pruned to the cap — see DEVLOG 2026-07-06..13.)*
