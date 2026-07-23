# STATE — single entry point for "what is true right now"

Last updated: 2026-07-23 (Blit 0.1.1 released)

- **BLIT 0.1.1 IS RELEASED (D-2026-07-23-8):** annotated tag `v0.1.1`
  resolves to exact validated candidate `d1f1152d` on LAN Gitea and GitHub.
  The public GitHub release has all three validated archives and checksum
  sidecars. Canonical hashes and scope: `docs/RELEASE_READINESS.md`.
- **ONE TRANSFER PATH IS PROVED.** There is one `Transfer` RPC. When the caller is DESTINATION, it connects to the SOURCE daemon; that daemon sends through the same SOURCE pipeline. Push/pull-facing adapters only select roles. The connection initiator still opens sockets to the responder for NAT/firewall reachability; that topology does not select byte logic or worker policy.
- **ADAPTIVE ROLE PARITY IS ACCEPTED IN ldt-2.** Deterministic real-session traces in both socket layouts emit identical ADD epochs through 17, REMOVE 4→1, idle/hysteresis holds, and receiver bounds. The old exact-eight result remains historical static-policy evidence, not an adaptive target.
- **ldt-4 EVIDENCE IS FINAL FOR RELEASE:** the first complete horizon session
  `ldt4-20260722T013314Z-a0c3e3f18afd` was valid after corrected-analyzer
  reanalysis; fresh `7050a29` was redundant confirmation. The live controller
  resized, but fixed order confounds the Windows→q ADD/REMOVE split with source
  warmth. No policy change follows. Full session ledger and write cost:
  `docs/bench/ldt4-evidence-audit-2026-07-22/`.

- **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.
- **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
- **THE FAST ARM IS BISTABLE — the trap named in pf-1's gate above.** `win_init` runs are **bimodal** (~730/~840 ms): S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. A counterfactual that merely shifts the mixture would **fake a recovery**.
- **P1 REPRODUCED ON A SECOND MAC BEFORE THE FIX (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 at MTU 9000, while all three controls passed at 1.002–1.043. That proved the historical result was not tied to one Mac; D-2026-07-22-2 now closes its initiator-dependent product mechanism from exact code/guard evidence below.
- **RIG-W HOST AND QUIETNESS RULES:** `.agents/machines.md` is canonical. ldt-4 must establish quietness live on `q` and `netwatch-01`; recorded readiness is never substituted for the run gate.
- Recent code state: every transfer rides the ONE session. ldt-2 is accepted at `65a0f9f`; ldt-3 lifecycle/observer closure is accepted at review fix `406a7e5` after clean neutral r2 (`.review/findings/ldt-3.md`).
- **P1 IS CLOSED WITHOUT ANOTHER TRANSFER (D-2026-07-22-2).** The failing
  builds used the old-red worker path: its deterministic guard settled SOURCE
  initiation at 3 workers and DESTINATION initiation at 2, while a second
  destination-only zero-capacity branch could cap at 1.
  `a76b785..42b9b38` fixed and mutation-proved parity;
  post-fix `8e019ef` passed the target point bar, and ldt-2 retains adaptive
  role parity. Evidence: `docs/bench/p1-evidence-reconciliation-2026-07-22/`.
- **⚠ THREE of my claims were reported and RETRACTED on 2026-07-13**, all the same root cause — trusting an instrument I had not validated: (1) "P1 is code" (a harness that keyed durability to the *initiator*, not the destination); (2) "P1 is acceptable platform residue" (D-2026-07-12-1 does not cover it); (3) "macOS can't send jumbo / the switch is broken" (it was `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — it cost the owner a pointless adapter swap). **Verify the instrument before believing the measurement.**

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and ≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff` procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Handoff — 2026-07-23 at `b8371d90`
- Done: Blit 0.1.1 is tagged on both remotes and publicly released with all six
  verified assets; the publication pushed no `master` branch.
- Next: resume the post-release Queue 1. Two stale rules move only on new approval.

## Now (active work)

- **MACOS TEST FIREWALL CLEANUP SHIPPED LOCALLY (D-2026-07-23-6):** the exact
  lifecycle helper, 16 fake-backed cases, real read-only parser check, and
  mutation guards are complete. No external review is pending under
  D-2026-07-23-7. Plan: `docs/plan/MACOS_TEST_FIREWALL_CLEANUP.md`.
- **THUNDERBOLT LIFECYCLE ATTRIBUTED (D-2026-07-23-3):** exact instrumented
  candidate `a3be4a64` completed one 8 GiB Q-to-Nagatha RAM transfer with all
  hashes matching. Only 13.645 ms was observed outside its 3.587845-second data
  span: connect + RPC open + establishment totaled 4.687 ms, while rendering
  through terminal took 0.041 ms. The historical 0.448-second gap did not recur
  and is outside these fixed product stages. This sample's body rate was 19.153
  Gb/s versus the prior 35.578 Gb/s, but one observation cannot classify that
  difference or select a policy change. Cleanup is complete and no repeat is
  authorized. Evidence: `docs/bench/end-to-end-transfer-latency-2026-07-23/`.
- **THUNDERBOLT SSD FOLLOW-UP IS COMPLETE:** exact candidate `d1f1152d`
  moved 12 GiB SSD-to-SSD in 7.73 s (13.335 Gb/s); Apple openrsync took
  33.81 s (3.049 Gb/s), so Blit was 4.37x faster. All 36 files hash-match;
  allocated writes were 38.884 GB. A later no-write cold read put Q's source
  SSD at 1.931 GB/s; Blit's 1.667 GB/s is 86.3% of that source-only rate.
  Cleanup is complete; no repeat is authorized. Evidence:
  `docs/bench/thunderbolt-ssd-2026-07-22/`.
- **RELEASE COMPLETION SHIPPED:** exact candidate `d1f1152d` passed every gate
  and is published as `v0.1.1`. Optional ceiling and Thunderbolt tuning remain
  post-release work.
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
  - **otp-12 worker-parity repair `[x]` (historical static-policy proof)** — both initiator layouts reached the same then-current target; zero receiver capacity meant unknown/default in both; payload proceeded while resize ACKs were pending; resize refusal was terminal. ldt-2 replaces that target with one live controller. This remains code/integration history, not adaptive hardware acceptance.
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
  `[x]`; **sf-3a+ blocked** until ONE_TRANSFER_PATH ships, then
  resume/re-derive on the unified baseline. Principle: ceiling-driven,
  never competitor-relative (D-2026-07-04-4 — do not re-litigate).
## Queue (ordered)

1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4):**
   slices otp-1..13; any external review requires exact owner approval under
   D-2026-07-23-7.
   otp-1, otp-3, otp-4a,
   otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b), otp-7 (a, b-1,
   b-2), otp-8, otp-9 (a/b), otp-2 (+ otp-2w), otp-10 (a, b-1/2,
   c-1/2), **otp-11 (a + b)**, **otp-12a (zoey)**, **otp-12b
   (Mac↔Windows)** `[x]`. 12a: 10 PASS, 2 to the walk. 12b — THE
   INVARIANCE CRITERION: 11/12 PASS (1.003–1.057); wm_tcp_mixed 1.237
   (TCP×mixed×dest-initiator, historical pre-fix result now closed by
   D-2026-07-22-2); push_tcp_small 1.149
   (both rigs); Win→Mac beats the better old direction 6/6; Mac→Win
   gap shapes recorded for the walk
   (`docs/bench/otp12-{zoey,win}-2026-07-12/`). **otp-12c `[x]`
   RECORDED 2026-07-13**: direct-path baseline at the cutover sha
   (`docs/bench/otp12c-win-2026-07-13/`) + the delegated rig-D
   matrix (`docs/bench/otp12c-delegated-2026-07-13/`, 5/7 PASS at
   RUNS=4; both FAIL cells PASS at RUNS=8 — see Blocked; rig D 7/7).
   **otp-12d and otp-13 are POST-RELEASE (D-2026-07-22-1).** Their retained
   pre-fix evidence remains usable for what it records; no performance
   acceptance matrix is a shipping prerequisite.
2. **`docs/plan/LOCAL_SMALL_FILE_PATH.md` (Draft, D-2026-07-13-2):** local
   apply ships one worker and does not scale; resume only under an active plan.
3. **POST-RELEASE performance declarations:** ue-1, ue-2, and the REV4
   performance status flip are not release gates (D-2026-07-22-1).
4. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
   resumes/re-derives after ONE_TRANSFER_PATH ships.
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
- Shipped release record: **`docs/RELEASE_READINESS.md`** and
  **`docs/plan/RELEASE_COMPLETION.md`**.
- Historical live-tuning record: **`docs/plan/LIVE_DIAL_TUNING.md`**; exact
  session audit: **`docs/bench/ldt4-evidence-audit-2026-07-22/`**.
- Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**paused** at
  sf-2) and **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** (code-
  complete; measurement gates remain). REV4 superseded v1/REV2/REV3
  (history only).
- Process: `.agents/playbooks/openreview.md` — synchronous unprimed review only
  after exact owner approval under D-2026-07-23-7; formal review uses Claude
  Opus 4.8/max and Grok remains advisory;
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
- **Two stale test firewall entries await separately approved cleanup;** the
  helper is shipped, but these historical entries remain untouched. Exact
  paths and the no-reuse/no-removal gate: `.agents/machines.md`.

## Open questions

- None. Post-release work is ordered above.
