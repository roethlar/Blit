# STATE — single entry point for "what is true right now"

Last updated: 2026-07-15 (52nd handoff — round 11 fixed + round-12 consensus: P1 IS REAL, the Mac↔Mac run is parked; owner to pick direction)

- **NEXT ACTION — OWNER DECISION, then execute it. NO DATA HAS EVER BEEN TAKEN and none is queued.** Round 11 is fully fixed (instrument at `bfae311`, prereg **rev 11**), and the round-12 review — reframed per **D-2026-07-14-5** to ask "is this the best experiment", not "is the code correct per my plan" — reached a **two-reviewer consensus that changes the plan**: read `.review/results/macmac-r12.{codex-design,codex-harness,grok-design}.md` and `.review/results/p1-adjudication-r1.{codex,grok}.md`.
  - **P1 IS REAL — settled by independent adjudication of the RECORDED data (codex + grok, high confidence).** A prior review claimed P1 might be a free-writeback timing artifact of the old harness (`bench_otp12_win.sh` flushes with no settle) and should be re-measured first. **The data refute that:** on `wm_tcp_mixed` the flush is **symmetric** (72 vs 73 ms) against a **~300 ms** effect, the effect is entirely in **transfer time** (remove flush and the ratio *rises*, 1.385→1.417, with zero arm overlap), the **same-fixture gRPC control passes at 1.020** (a writeback artifact would hit it identically), and Linux's identical immediate-flush method shows **no P1**. The precedent both cite: a *real* accounting artifact was caught here once (`2c0af86`) because it polluted the gRPC control — P1 is carrier-specific, so it passes that test. **The release blocker is genuine, not measurement error.**
  - **BOTH REVIEWERS: the Mac↔Mac run is NOT the next move — no outcome of it changes the release-critical action.** It answers only "can P1 occur without a Windows peer?", and every outcome still routes to fixing P1 on the pair where it lives (macOS↔Windows). Grok's power analysis: with four independent full-range controls that must ALL be clean and rig W's fast arm known-bimodal, the *most likely* successful outcome is `CONTROLS-NOT-CLEAN` — a re-run, not an answer.
  - **THE DECISION FOR THE OWNER**: (1) **instrument the TCP dial/accept transfer path on rig W** — both reviewers' recommendation; P1 is now pinned to TCP + destination-initiated + mixed, so add timing spans to `SourceSockets` Dial/Accept, `add_dialed_stream`, the dial-before-ACK at `transfer_session/mod.rs:3113`; the fastest route to a fix. Or (2) **run Mac↔Mac anyway** for the 2×2 map — instrument is READY (BLOCKER closed and proved: pointing it at the 1GbE NIC trips three independent gates), but see the consensus above. **No agent may pick this; owner call.**
  - **The Mac↔Mac instrument is DONE and REVIEWED** — engine 40 cases / 19 mutations, harness self-test 0-blind on both Macs, fabric gates proved by mutation. If run: nagatha↔`q`, 10GbE MTU 9000, build `f35702a` (nagatha's worktree + build were MISSING and were rebuilt this session), both Macs codex-quiet and Time Machine off. Host facts: `.agents/machines.md`.
  - **⚠ ROUND-12 STILL-OPEN correctness findings (real, not yet fixed — apply before any Mac↔Mac run):** the threshold `min(src/10, 230)` can report `REPRODUCES` on a cell whose ratio (1.092) *passes* the 1.10 bar (codex BLOCKER — the `min` gives EITHER standard, the prose says BOTH); the end-fabric gate re-checks MSS/IP but **not link speed** (a 10GbE→1GbE renegotiation keeping MTU 9000 grades — my own duplicate-site bug); the `B ≥ T/2` refusal guards only the positive margin, not the smaller `src/11` negative one; two mutations "kill" for the wrong reason. Detail in `macmac-r12.codex-design.md`.
- **THE INSTRUMENT IS THE RISK — ~110 findings across TEN reviews of this ONE harness, all accepted, none rejected, and it has still never run.** Three project claims were already retracted to harness bugs. **TWO DEFECT CLASSES recur in EVERY round; the next review must assume both are present.** (1) **"Fixed the branch I was shown, not the class"** — the same materiality bug escaped **four** rounds; a fail-open `pgrep` was fixed in one gate and left in its duplicate; the drain was fixed by VALUE and left failing by STATUS; Spotlight coerced a non-number to 0 exactly as the drain once accepted `"."`. **And a deletion regressed the build pin**: cutting the escalation block out took the adjacent `EXPECT_SHA` check with it, so any sha — including `.dirty` — was accepted. (2) **"A protection that never executes, or cannot fail"** — `SETTLE_MS` **had never run in any revision** (a quoting bug killed the `sleep` and its status was discarded), while the prereg asserted it for three revisions; the ssh-dispatch **bound** was measured once at preflight and never enforced on a run. Earned rules: **verify the instrument before believing the measurement**; **`bash -n` is not an execution**; **a protection that cannot be observed is not a protection**; **a mutation that cannot be killed is not a proof.**
- **⚠ THE MAC↔MAC RIG IS *NOT* AN H1 DISCRIMINATOR — retracted 2026-07-14.** "Reproduces ⇒ H1 dies" was **WRONG**: H1 accuses **blit's own code paths**, not Windows, and that code runs on macOS too — so a reproduction is *consistent with* H1. It answers one thing, scoped to this pair: **can P1 occur WITHOUT a Windows peer?** A reproduction ⇒ P1 is not waivable as "Windows residue" (it does **not** prove a platform-*general* cost, and leaves macOS/APFS and host×role open). A null ⇒ it did not reproduce *on this pair* — consistent with "Windows required", **not proof** of it, and reportable only if the run could have SEEN the effect. Detail: the pre-registration.
- **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.
- **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
- **THE FAST ARM IS BISTABLE — the trap named in pf-1's gate above.** `win_init` runs are **bimodal** (~730/~840 ms): S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. A counterfactual that merely shifts the mixture would **fake a recovery**.
- **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption **H1** rests on (corrected 2026-07-14: H5/H6/H7 are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong), never tested until now. **And jumbo does NOT dissolve P1** — pf-0 has now measured the matched 1500 arm and killed MTU outright (above).
- **THE MAC IS A BENCH END — the codex loop and a rig-W session CANNOT run concurrently** (`.agents/machines.md`). A 53-min A-B-B-A attempt was destroyed by codex load on the Mac and discarded; the contamination is *asymmetric* (it inflates `mac_init` and MANUFACTURES P1). **Rig-W now runs on `q`** (dedicated M4 mini, quiet, faster than nagatha), which decouples the two for good.
- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488 as of `bb28ddd`** — the last commit to touch `crates/`+`proto/`; every commit since is docs/scripts, so the count stands unre-run. SMALL_FILE_CEILING paused (D-2026-07-05-1).
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
   **(i) The A-B-B-A MTU run on `q` — `[x]` DONE 2026-07-14: MTU KILLED**
   (`r = −3.1%`; `docs/bench/otp12-jumbo-win-2026-07-13/`). See the pf-0
   bullet at the top for the two limits it puts on pf-1.
   **(ii) THE MAC↔MAC RIG — the missing cell of the 2×2** (owner,
   2026-07-13). Linux↔Linux = **no P1** (8/8 PASS); macOS↔Windows = **P1**
   (1.237/1.300/1.385/1.362); macOS↔macOS = **?** Design, decision rule and
   the retraction of the "H1 dies" framing: **see NEXT ACTION at the top**
   and the rev-2 pre-registration. **Both Macs are bench ENDS: the codex
   loop CANNOT run during the session** (the gate enforces it).
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

- **The Mac↔Mac run is BLOCKED and NOT clearable by an agent** — round 11's
  findings are unfixed (engine 2 HIGH, harness 1 BLOCKER + 4 HIGH) and both
  Macs must be codex-quiet. Basis and detail: NEXT ACTION at the top of this
  file; never restated here (re-verified 2026-07-14 against
  `.review/results/macmac-harness-r11.*` and `git log -- scripts/bench_otp12pf_mac.sh`,
  whose newest commit is still round 10's `8997f92`).
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
