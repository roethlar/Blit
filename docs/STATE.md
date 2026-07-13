# STATE — single entry point for "what is true right now"

Last updated: 2026-07-13

- **NEXT ACTION — run the MTU experiment BEFORE any code (Queue 1a).** Windows sat at **MTU 1500 for every benchmark ever recorded**, so jumbo was never once exercised; the fleet is now at **9000** and the negotiated MSS on the rig-W path is **8948 both directions** (measured — `getsockopt(TCP_MAXSEG)` + Linux `ss -ti`; 1448 at MTU 1500). Design + decision rule are PRE-REGISTERED: `docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md` (rev 2, codex 7/7 accepted). **Both MTU conditions get measured** (9000 and 1500, identical scope, RUNS=8, same NIC/sha) — a single jumbo run cannot attribute anything, because no prior session is a matched control and a FAIL would prove only that jumbo is *insufficient*, not that MTU contributes nothing. **CORRECTION (codex F6): `mixed` is NOT "the most packet-heavy fixture" — `large` is, by ~2× (~741k segments vs ~378k at MSS 1448).** `mixed` is P1's cell because that is where the failure was *observed*; `wm_tcp_large` is now the bulk-packet positive control.
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
  - **Closed `[x]`: otp-1, otp-3 … otp-9** — the whole session machine
    (contract, role drivers, daemon serving, both data planes + resize +
    cancel, mirror/filters, resume, fallback carrier, delegated).
    SizeMtime = data-safe skip. Detail: DEVLOG 2026-07-10.
  - **otp-2 `[x]`** — baselines. zoey = per-direction reference;
    Mac↔Windows = cross-direction rig (otp-2w). Evidence
    `docs/bench/otp2{,w}-baseline-2026-07-10/`. Key reading: old push
    trails old pull on BOTH rigs.
  - **otp-10 `[x]`** — verb cutover + **THE CUTOVER DELETION**: 4
    drivers + `Push`/`PullSync` + 13 messages out of tree AND proto
    (−13.8k lines, no bridge); relay removed (D-2026-07-11-1).
    Detail: DEVLOG 2026-07-11.
  - **otp-11 `[x]`** — local transfers ride the session; **11b deleted
    the whole old orchestration** (−6.2k lines: orchestrator, engine,
    local_worker, auto_tune, change_journal — the last one an UNSOUND
    fast path that silently lost data, repro in
    `docs/bench/otp11-local-2026-07-11/`). The deletion-proof acceptance
    line COMPLETES. Detail: DEVLOG 2026-07-12.
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
   **RUN THIS FIRST — the MTU experiment.** Design + decision rule are
   PRE-REGISTERED before any data (the doc owns the detail; do not restate
   it here): `docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md`
   rev 2 — codex NOT READY (4 BLOCKER + 3 HIGH), **7/7 accepted**;
   adjudication `.review/results/pf-0-prereg.gpt-verdict.md`. Headlines:
   **both** MTU conditions get measured (9000 AND 1500, identical CELLS,
   RUNS=8, same NIC + sha) — a lone jumbo run attributes nothing, since
   12b differs by sha and 12c by NIC. **CORRECTION: `mixed` is NOT "the
   packet-heaviest fixture" — `large` is, by ~2×** (~741k vs ~378k
   segments at MSS 1448); that old rationale was factually wrong.
   **P1 misses the plan's HEADLINE criterion on rig W** (initiator/verb
   invariance): `wm_tcp_mixed` FAILs twice — 1.237 and 1.300 — on tight
   spreads, so not re-runnable away. (Do NOT read 1.237→1.300 as a
   regression: **different Mac NICs**, see machines.md.) **But it does
   NOT reproduce on a same-OS rig**: Linux both ends = **8/8 PASS**, P1's
   cell at 1.092/1.003 (`docs/bench/otp12-perf-2026-07-13/`) → it is
   platform-INTERACTING, not pure layout. **P1 HAS NO ESCAPE HATCH**
   (codex r5 F1): D-2026-07-12-1 waives only a *cross-direction* miss for
   a cell that ALREADY passes invariance — P1 *is* the invariance
   failure. So: **fix it to ≤1.10, or the owner amends acceptance
   criterion 1.** Not assumed either way. P2 (`push_tcp_small` 1.105–
   1.201, both rigs) is a converge bar vs the OLD build and is UNTESTED
   on the Linux rig. Sequence: **jumbo re-run → pf-1 → fix → pf-final
   (ALL THREE rigs) → otp-12d → otp-13.**
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

- **2026-07-13 (46th)** — **otp-12c closed** (rig D 7/7; codex 7/7
  accepted). **Same-OS rig built** (magneto↔skippy): Linux both ends =
  **8/8 PASS**, so P1 is platform-INTERACTING, not pure layout. Perf plan
  → **ACTIVE** (D-2026-07-13-1). Also landed: mid-copy cancel e2e + the
  D4 mid-record fault fix (`920c6a7`), CLI `./NAME` hint (`ace91de`), CI
  fmt fix (`bb28ddd`, suite **1488**).
  **THREE claims of mine were reported and RETRACTED this session** —
  all from trusting an unvalidated instrument: (1) "P1 is code" (1.78),
  from a harness that keyed durability to the *initiator*, not the
  destination (fixed `2c0af86`); (2) "P1 is acceptable platform residue"
  (D-2026-07-12-1 does not cover an invariance failure — codex r5 F1);
  (3) "macOS can't send jumbo / the switch is broken" (it was
  `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — cost the
  owner an adapter swap for nothing). **Verify the instrument before the
  measurement.** In-flight: none; tree clean.
  **Next**: the jumbo re-run (Queue 1a) → pf-1.
- *(45th and earlier pruned to the cap — see DEVLOG 2026-07-06..13.)*
