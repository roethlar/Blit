# STATE — single entry point for "what is true right now"

Last updated: 2026-07-15 (pf-1 rig-W G14 recovery fixed/guard-proved; Claude review next; no accepted rig datum)

- **NEXT ACTION — PF-1 RIG-W G14 CLAUDE REVIEW:** Claude-accepted exact `d7345f1` passed q Bash 3.2 self-test, launcher, and preflight after fresh additive staging. Two fresh registered sessions each completed exactly 16 arms, then became `SESSION-VOID` at the pair-3 entry gate when delayed Spotlight work following q destination-tree writes spiked from quiet pre-round samples to 58.7% and 87.3%. G14 at `942c88e` keeps the fixed 10% ceiling: preflight stays immediate, while runtime checks outside timed arms may use the existing 5 s × 24 recovery budget for either owned load history or Spotlight; every contaminant is rechecked and persistent/malformed states fail closed. Current-byte validation and three targeted mutation proofs are green; run the required Claude Fable 5/max review before a fresh additive build/stage and retry. Worker parity remains closed and is not a blocker.
- **ONE TRANSFER PATH IS PROVED.** There is one `Transfer` RPC. When the caller is DESTINATION, it connects to the SOURCE daemon; that daemon sends through the same SOURCE pipeline. Push/pull-facing adapters only select roles. The connection initiator still opens sockets to the responder for NAT/firewall reachability; that topology does not select byte logic or worker policy.
- **WORKER PARITY IS CLOSED.** The identical 10,000-file fixture now reaches exactly 8 workers under both initiator layouts (old guard: 3 vs 2; destination-initiator `max_streams=0`: 1). Payload starts while resize ACKs are pending, refusal is terminal, and resize arbitration is atomic. Final Codex re-review: PASS; workspace gate: 1,490 passed, 2 ignored.
- **WHY NO MAC↔MAC DATA YET:** the current verdict engine can label a 1.092 cell both `PASS` and `REPRODUCES`, and the end-fabric gate can grade after a 10GbE→1GbE renegotiation because it rechecks MSS/IP but not link speed. Those are measurement blockers, not transfer-path blockers. P1 remains real on macOS↔Windows; no Mac↔Mac data exists.

- **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.
- **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
- **THE FAST ARM IS BISTABLE — the trap named in pf-1's gate above.** `win_init` runs are **bimodal** (~730/~840 ms): S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. A counterfactual that merely shifts the mixture would **fake a recovery**.
- **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption **H1** rests on (corrected 2026-07-14: H5/H6/H7 are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong), never tested until now. **And jumbo does NOT dissolve P1** — pf-0 has now measured the matched 1500 arm and killed MTU outright (above).
- **THE MAC IS A BENCH END — the codex loop and a rig-W session CANNOT run concurrently** (`.agents/machines.md`). A 53-min A-B-B-A attempt was destroyed by codex load on the Mac and discarded; the contamination is *asymmetric* (it inflates `mac_init` and MANUFACTURES P1). **Rig-W now runs on `q`** (dedicated M4 mini, quiet, faster than nagatha), which decouples the two for good.
- Recent code state: every transfer rides the ONE session. The otp-12 worker-parity repair is reviewed at `42b9b38`; full workspace fmt/clippy/test is green (1,490 passed, 2 ignored, as of that commit).
- **P1 (the headline invariance criterion) — the one thing between blit and shipping.** Fails rig W (`wm_tcp_mixed` 1.237 and 1.300 — do NOT read that as a regression, it is **two different Mac NICs**), but **PASSES 8/8 with Linux on both ends** (`docs/bench/otp12-perf-2026-07-13/`; P1's own cell 1.092/1.003). So it is **platform-INTERACTING, not pure layout** — yet **NOT exonerated**: a code path that only bites on one platform (H1's Windows accept branch) looks identical. **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a *cross-direction* miss for a cell that ALREADY passes invariance — P1 *is* the invariance failure. So: **fix it to ≤1.10, or the owner amends acceptance criterion 1.** Neither is assumed.
- **⚠ THREE of my claims were reported and RETRACTED on 2026-07-13**, all the same root cause — trusting an instrument I had not validated: (1) "P1 is code" (a harness that keyed durability to the *initiator*, not the destination); (2) "P1 is acceptable platform residue" (D-2026-07-12-1 does not cover it); (3) "macOS can't send jumbo / the switch is broken" (it was `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — it cost the owner a pointless adapter swap). **Verify the instrument before believing the measurement.**

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
  D-2026-07-05-4 "flip the plan and go").** The invariant (plan doc,
  verbatim): ONE block of transfer code; direction/initiator/verb can
  NEVER affect wall time by blit's doing — impossible by construction
  because the per-direction drivers and `Push`/`PullSync` are deleted
  at cutover. Slices otp-1..13; converge-up per cell (±10%);
  symmetric-fs disk-to-disk verdict cells. **D-2026-07-05-2:
  same-build peers only, refusal at session open.**
  - **Slices otp-1 … otp-11 are all `[x]` CLOSED** — the session
    machine, the baselines, the cutover deletion (−13.8k lines) and
    otp-11b's deletion of the old orchestration (−6.2k). The
    deletion-proof acceptance line COMPLETES. The closed-slice record
    was rotated verbatim to `docs/history/state-archive.md`
    (2026-07-14 drift); per-slice detail lives in DEVLOG + `.review/`.
  - **Open: otp-12d and otp-13** — both DEFERRED behind pf-final, see
    Queue 1.
  - **otp-12 worker-parity repair `[x]`** — both initiator layouts reach the same exact target; zero receiver capacity means unknown/default in both; payload proceeds while resize ACKs are pending; resize refusal is terminal. Final Codex re-review PASS. This is code/integration proof, not hardware acceptance.
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
  `[x]`; **sf-3a+ blocked** until ONE_TRANSFER_PATH ships, then
  resume/re-derive on the unified baseline. Principle: ceiling-driven,
  never competitor-relative (D-2026-07-04-4 — do not re-litigate).
- **Background**: REV4 code-complete, gates DATA-COMPLETE (declarations
  in Blocked); the synchronous reviewloop governs all changes
  (D-2026-07-04-1), with Claude Fable 5/max selected by D-2026-07-15-1.

## Queue (ordered)

1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
   the only work item until it ships**: slices otp-1..13 through the
   synchronous reviewloop per slice (reviewer selection D-2026-07-15-1).
   otp-1, otp-3, otp-4a,
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
1a. **`docs/plan/OTP12_PERF_FINDINGS.md` (ACTIVE, D-2026-07-13-1).**
    pf-0 is complete: MTU was killed as a dominant cause. **The owner selected
    pf-1 instrumentation on rig W on 2026-07-15.** The TCP phase-trace slice
    and reduced paired q↔netwatch-01 harness must each clear review before rig
    time. Round-10 candidate `5a7e7ec` passed launcher and preflight; its first
    registered arm then voided at 1247 ms settle before durability or CSV
    append because three sequential clock-probe SSH startups consumed about
    1.1 s. G11 batches the unchanged three independently bracketed samples,
    fails closed on channel/process errors, and preflight-times the complete
    path against 750 ms headroom. Round-11 independent Grok discarded an
    unsupported no-tool response, then proved the old sampler red, restored
    green, and accepted exact candidate `aa0785c`. Its launcher and preflight
    passed at 396/403 ms. The registered run appended one completed-arm row,
    then G12 failed before the paired trace-off q client launched because Bash
    3.2 nounset rejects an empty array expansion. The entire session is void;
    no row was analyzed or graded. G12 is fixed at `cd78ab9`, Bash-3.2 guard-proved,
    and accepted by round-12 Grok at exact identity `d5e9dda`. Its additive live run
    completed eight arms, then G13 exposed the mid-run load gate counting the
    benchmark's own one-minute load history; that session is also void. G13 at
    `0cbb16a` keeps the
    fixed load ceiling with bounded runtime recovery and is Bash-3.2 guard-proved.
    Claude Fable 5/max accepted exact `d7345f1`. Two additive live sessions
    cleared G13 through 16 arms, then G14 exposed delayed Spotlight work at the
    same pair-3 gate; both sessions are void. G14 preserves the 10% bar with
    bounded runtime recovery and is Bash-3.2 guard-proved at `942c88e`; Claude
    review is next. The phase report and
    `0f922de` historical control remain part of the
    pf-1 HARD GATE. No Mac↔Mac data has been taken, and worker parity is no
    longer a blocker. Then: pf-1 → pf-final (all rigs) → otp-12d → otp-13.
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
- Process: `.agents/playbooks/reviewloop.md` — the synchronous loop for
  **all code and plan changes** (D-2026-07-04-1), using Claude CLI
  `--model claude-fable-5 --effort max` for current dispatches
  (D-2026-07-15-1). `docs/agent/GPT_REVIEW_LOOP.md` is historical;
  `.review/README.md` is retired as the grading mechanism (its
  `findings/`/`results/` records and the REVIEW.md index remain live).
- Review loop: `REVIEW.md` (all `ue-r2-*` rows `[x]`; design-queue
  rows) + `.review/findings/` + `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (module delete ratified
  D-2026-06-12-1, executes w8-1; **capability unparked
  D-2026-07-05-3** — post-cutover write strategy), `TUI_REWORK.md`
  (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).

## Blocked / waiting (all owner declarations; checkpoints are owner-only)

- **The Mac↔Mac measurement is blocked on the open round-12 instrument-correctness findings and both Macs being bench-quiet.** Round 11 is fixed at `bfae311`; worker parity is fixed and reviewed at `42b9b38`. This is no longer a push/pull-path or worker-count blocker. No Mac↔Mac data exists. Detail: `.review/results/macmac-r12.codex-design.md` and `.review/results/macmac-r12.codex-harness.md`.
- **Rig facts:** `.agents/machines.md` is canonical; do not restate host pairings here.
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
  Linux daemon-spawn flakiness; **windows-latest CI has never been
  observed green — check it live, do not record push state here.**
  NOTE 2026-07-12: the macOS `blit_utils` residual (pre-existing,
  reproduced at `6d37a22`) ran ELEVATED under heavy load (~3/12 vs 2/8
  historical) — own finding if it persists on a quiet machine.
- *(Resolved 2026-07-12/13: SizeMtime SKIP, `725aa07`, the `./NAME` foot-gun,
  otp-5b-3 cancel, the change-journal premise — all landed; see DEVLOG.)*

## Handoff log (newest first, keep ≤ 3)

- **2026-07-15 (52nd)** — **Round 11 FIXED (7 commits, each guard-proved; instrument at `bfae311`,
  prereg rev 11), round 12 REVIEWED, and the reframe changed the plan.** Framed per D-2026-07-14-5
  around the end goal, codex + grok BOTH said **DO NOT RUN** Mac↔Mac; codex raised a would-be
  BLOCKER (P1 measured with an un-settled harness — re-measure first). **I relayed that as a pivot
  WITHOUT checking the data; owner refused and demanded consensus.** Adjudication (both, from the
  CSVs): **P1 IS REAL** — flush symmetric (72/73 ms) vs a ~300 ms effect in *transfer* time, gRPC
  control passes at 1.020, Linux no-P1. The release blocker is genuine. D-2026-07-14-4 (`B≥T/2`
  refuses) and D-2026-07-14-5 (first-review reframe) recorded. Also: rebuilt nagatha's missing
  `f35702a` worktree+binaries. No crates/proto, no rig time, no data. Full: **DEVLOG 02:30Z**.
- **2026-07-14 (51st)** — **BOTH MACS CONFIRMED READY (owner); DEVLOG backfilled for rounds
  7–11. No code, no rig time, no data.** TM autobackup = 0 on both; zero `blit-daemon`. A ready rig
  is not a ready instrument — round 11's BLOCKER stood (now fixed, 52nd). Full: **DEVLOG 22:45Z**.
- **2026-07-14 (50th, `f933097`)** — **`drift`: STATE hygiene.** Handoff log was four rounds stale.
  Created `docs/history/state-archive.md`, anchored `Suite 1488 as of bb28ddd`. Full: **DEVLOG 21:10Z**.
- *(49th and earlier pruned to the cap — full entries in DEVLOG 2026-07-06..15.)*
