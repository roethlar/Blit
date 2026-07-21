# STATE — single entry point for "what is true right now"

Last updated: 2026-07-21 (Windows client-path f8 tactical review clean; exact staging/live retry next)

- **HANDOFF 2026-07-17, HEAD `d53b5fd`:** `a39f0c5` surfaced the generated
  `start.cmd` split; `d53b5fd` fixed and mutation-proved both array-concatenation
  faults with retained harness evidence and full local gates green.
  - Done: live evidence remains retained; no endpoint or daemon deletion/overwrite.
  - In-flight: no completed/timed live transfer row yet.
  - Next: run tactical Grok/Opus 4.8 on exact `d53b5fd`, then additively stage and run one quiet
    fresh `q`↔`netwatch-01` retry.

- **NEXT ACTION — ADDITIVELY STAGE EXACT `c2e1284`, THEN RUN:** exact `55fc5d5` cleared Windows console-host classification live and completed/retained arm 1. Session `ldt4-20260721T210445Z-55fc5d5ff456` then voided before arm-2 client creation because unparenthesized `$dir + '/client-launch.ok'` split the prospective-file array. Exact `c2e1284` parenthesizes that one path, structurally forbids the live-failing form, is mutation-proved/full-gate green, and is tactical Grok-clean.
- **ONE TRANSFER PATH IS PROVED.** There is one `Transfer` RPC. When the caller is DESTINATION, it connects to the SOURCE daemon; that daemon sends through the same SOURCE pipeline. Push/pull-facing adapters only select roles. The connection initiator still opens sockets to the responder for NAT/firewall reachability; that topology does not select byte logic or worker policy.
- **ADAPTIVE ROLE PARITY IS ACCEPTED IN ldt-2.** Deterministic real-session traces in both socket layouts emit identical ADD epochs through 17, REMOVE 4→1, idle/hysteresis holds, and receiver bounds. The old exact-eight result remains historical static-policy evidence, not an adaptive target.
- **WHY NO ldt-4 RIG-W DATA YET:** earlier retained sessions failed closed on fixtures, generated paths, endpoint DHCP, q hostname, and Windows console-host classification. Exact `55fc5d5` cleared those gates and completed arm 1, but session `ldt4-20260721T210445Z-55fc5d5ff456` is void because arm 2 failed before Windows client creation on split launch-gate path syntax. Its one provisional row is invalid and ungraded. Both ports are closed, no Blit process remains, and the prior active Windows daemon is restored byte-for-byte. `ldt-4-live-f8` owns the path correction.

- **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.
- **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
- **THE FAST ARM IS BISTABLE — the trap named in pf-1's gate above.** `win_init` runs are **bimodal** (~730/~840 ms): S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. A counterfactual that merely shifts the mixture would **fake a recovery**.
- **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption **H1** rests on (corrected 2026-07-14: H5/H6/H7 are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong), never tested until now. **And jumbo does NOT dissolve P1** — pf-0 has now measured the matched 1500 arm and killed MTU outright (above).
- **RIG-W HOST AND QUIETNESS RULES:** `.agents/machines.md` is canonical. ldt-4 must establish quietness live on `q` and `netwatch-01`; recorded readiness is never substituted for the run gate.
- Recent code state: every transfer rides the ONE session. ldt-2 is accepted at `65a0f9f`; ldt-3 lifecycle/observer closure is accepted at review fix `406a7e5` after clean neutral r2 (`.review/findings/ldt-3.md`).
- **P1 (the headline invariance criterion) — the one thing between blit and shipping.** Historical acceptance sessions fail rig W (`wm_tcp_mixed` 1.237 and 1.300 — do NOT read that as a regression, it is **two different Mac NICs**), but **PASS 8/8 with Linux on both ends** (`docs/bench/otp12-perf-2026-07-13/`; P1's own cell 1.092/1.003). Exact current-build `8e019ef` reverses and point-passes in the reduced rig-W diagnostic, but its 329 ms floor and failing gRPC control make that non-reproduction non-acceptance evidence. P1 remains **platform-INTERACTING, not pure layout**, and **NOT exonerated**. **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a *cross-direction* miss for a cell that ALREADY passes invariance. So: meet the formal ≤1.10 bar on a gradeable run, or the owner amends acceptance criterion 1. Neither is assumed.
- **⚠ THREE of my claims were reported and RETRACTED on 2026-07-13**, all the same root cause — trusting an instrument I had not validated: (1) "P1 is code" (a harness that keyed durability to the *initiator*, not the destination); (2) "P1 is acceptable platform residue" (D-2026-07-12-1 does not cover it); (3) "macOS can't send jumbo / the switch is broken" (it was `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — it cost the owner a pointless adapter swap). **Verify the instrument before believing the measurement.**

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and ≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff` procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **LIVE_DIAL_TUNING ACTIVE (D-2026-07-16-2):** ldt-1..3 are accepted; repairs through stable-q-identity `f6` are fixed/reviewed/staged at `21fe468`; Windows console-host `f7` is fixed/reviewed/staged at `55fc5d5` and cleared live. That retry completed arm 1, then exposed Windows client launch-gate path `f8` before arm-2 client creation; exact `c2e1284` is mutation-proved/full-gate green and tactical Grok-clean before additive staging. Formal Fable openreview is held for capacity.
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
    Queue 2.
  - **otp-12 worker-parity repair `[x]` (historical static-policy proof)** — both initiator layouts reached the same then-current target; zero receiver capacity meant unknown/default in both; payload proceeded while resize ACKs were pending; resize refusal was terminal. ldt-2 replaces that target with one live controller. This remains code/integration history, not adaptive hardware acceptance.
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
  `[x]`; **sf-3a+ blocked** until ONE_TRANSFER_PATH ships, then
  resume/re-derive on the unified baseline. Principle: ceiling-driven,
  never competitor-relative (D-2026-07-04-4 — do not re-litigate).
- **Background**: REV4 code-complete, gates DATA-COMPLETE (declarations in Blocked);
  D-2026-07-16-3/-4 makes review risk-based: Grok is advisory for ordinary
  second eyes/slice checks; every formal `openreview` uses Claude Fable 5/max.

## Queue (ordered)

1. **`docs/plan/LIVE_DIAL_TUNING.md` (ACTIVE, D-2026-07-16-2).** ldt-1..3 are accepted. Live repairs through Windows console-host `f7` are fixed/reviewed/staged at exact `55fc5d5`, which cleared those gates and completed arm 1 live. Its retained retry exposed Windows client launch-gate path finding `ldt-4-live-f8` before arm-2 client creation. Exact `c2e1284` is mutation-proved/full-gate green and tactical Grok-clean. Formal Fable openreview is held; additively stage the exact reviewed harness, then execute fresh quiet rig-W adaptive and role-invariance evidence.
2. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4):**
   slices otp-1..13 with risk-selected neutral `openreview`
   (reviewer authority D-2026-07-16-4).
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
2a. **`docs/plan/OTP12_PERF_FINDINGS.md` (ACTIVE, D-2026-07-13-1).**
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
    benchmark's own one-minute load history; that session is also void. G13 at `0cbb16a`
    keeps the fixed load ceiling with bounded runtime recovery and is Bash-3.2 guard-proved.
    Claude Fable 5/max accepted exact `d7345f1`. Two additive live sessions
    cleared G13 through 16 arms, then G14 exposed delayed Spotlight work at the
    same pair-3 gate; both sessions are void. G14 preserves the 10% bar with
    bounded runtime recovery and is Bash-3.2 guard-proved at `942c88e`.
    Round-14 Claude accepted exact `1f62ce5`. Its additive run completed all 32
    block-1 arms and exercised G14, then G15 exposed Windows SSH consuming
    blocks 2–4 from the loop's stdin; the analyzer correctly voided 32/128.
    G15 isolates SSH stdin and is mutation-proved at `7bdaf8b`; round-15 Claude accepted exact `8e019ef`. Its additive registered run completed all 128 arms and the analyzer accepted the exact evidence now recorded under `docs/bench/otp12-pf1-rigw-2026-07-15/`. Its pre-ldt-2 target traces prove historical 8/8 static stream parity under both initiator layouts. The target's historical P1 direction did not reproduce, but the 329 ms resolution floor and failing gRPC control forbid a causal grade. Grok supplementary and Claude Fable 5/max authoritative reviews accepted exact record `7ecc2f9`. Continue the separately required P2 small-fixture instrumentation and `0f922de` historical control. Further P1 rig work requires a plan amendment; no Mac↔Mac data has been taken, and worker parity is no longer a blocker. Then: pf-1 → pf-final (all rigs) → otp-12d → otp-13.
2b. **AFTER otp-12 — the Windows/local pair, planned TOGETHER** (same tar
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
3. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
   Shipped (zero-copy resolved — D-2026-07-05-3). Follow-ups largely
   absorbed by otp-2/otp-12's rig matrices.
4. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
   resumes/re-derives after ONE_TRANSFER_PATH ships.
5. **PAUSED: design-review queue** (`REVIEW.md`; w7-1 topmost open row —
   likely landed inside otp-6's one-delete-rule slice; re-check first).
6. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
   Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
   cutover as a runtime-selected write strategy in the unified receive
   sink (design: eval doc §If-FAST-evidence; dead module deletes in
   w8-1). Rig facts + build recipe: DEVLOG 2026-07-05 10:00.
   **Standing owner safety rule**: ALL activity on rig `zoey` stays
   inside its `…/blit-temp/` folder — nothing written outside it, ever;
   no daemon runs on zoey without a fresh go.
7. **Post-REV4 residue** (unowned, 5 items) — list in DEVLOG 2026-07-13 21:00Z.

## Authoritative docs right now

- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Active**,
  D-2026-07-09-1 — otp-7 slice design; governs otp-7a/7b).
- Active correction: **`docs/plan/LIVE_DIAL_TUNING.md` (D-2026-07-16-2)** — restore live telemetry ADD/REMOVE before further transfer acceptance measurements.
- Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**paused** at
  sf-2) and **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** (code-
  complete; measurement gates remain). REV4 superseded v1/REV2/REV3
  (history only).
- Process: `.agents/playbooks/openreview.md` — synchronous unprimed review when
  risk-selected under D-2026-07-16-3/-4: Grok may advise, but every formal
  `openreview` uses Claude Fable 5/max;
  `.agents/playbooks/codereview.md` supplies finding intake and triage only.
  `docs/agent/GPT_REVIEW_LOOP.md` is historical;
  `.review/README.md` is retired as the grading mechanism (its
  `findings/`/`results/` records and the REVIEW.md index remain live).
- Review loop: `REVIEW.md` (all `ue-r2-*` rows `[x]`; design-queue
  rows) + `.review/findings/` + `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (module delete ratified
  D-2026-06-12-1, executes w8-1; **capability unparked
  D-2026-07-05-3** — post-cutover write strategy), `TUI_REWORK.md`
  (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).

## Blocked / waiting (owner declarations and explicitly dated external blockers; checkpoints are owner-only)

- **Rig facts:** `.agents/machines.md` is canonical; do not restate host pairings here.
- **ldt-4 rig-W endpoint (as of `31c12c9`, 2026-07-21):** q's registered `en8`/`.54` link is restored at MTU 9000/10Gbase-T and routes to Windows on `en8`. Stale `.177` has incomplete ARP/no TCP 22; DNS plus strict-host-key SSH prove `NETWATCH-01` now owns `.173` with the same trusted keys. D-2026-07-21-1 directs the exact pin to follow `.173`; review/staging remain before live retry. Never bypass host-key checking.
- **otp-12c RECORDED 2026-07-13** (pre-fix rows = replication/control
  evidence, NOT acceptance evidence; Queue 2a):
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
