# STATE — single entry point for "what is true right now"

Last updated: 2026-08-02 (TUI rework M1 landed: F3 picker mode + path-return plumbing, no caller wired yet; full gate green)

- **BLIT 0.1.1 IS RELEASED (D-2026-07-23-8):** annotated tag `v0.1.1`
  resolves to exact validated candidate `d1f1152d` on LAN Gitea and GitHub.
  The public GitHub release has all three validated archives and checksum
  sidecars. Canonical hashes and scope: `docs/RELEASE_READINESS.md`.
- **ONE TRANSFER PATH IS PROVED.** There is one `Transfer` RPC. When the caller is DESTINATION, it connects to the SOURCE daemon; that daemon sends through the same SOURCE pipeline. Push/pull-facing adapters only select roles. The connection initiator still opens sockets to the responder for NAT/firewall reachability; that topology does not select byte logic or worker policy.
- **ADAPTIVE ROLE PARITY IS ACCEPTED IN ldt-2.** Deterministic real-session traces in both socket layouts emit identical ADD epochs through 17, REMOVE 4→1, idle/hysteresis holds, and receiver bounds. The old exact-eight result remains historical static-policy evidence, not an adaptive target.
- **ldt-4 EVIDENCE IS FINAL FOR RELEASE** and no policy change follows: the
  live controller resized, but fixed order confounds the Windows→q ADD/REMOVE
  split with source warmth. Session ledger and write cost:
  `docs/bench/ldt4-evidence-audit-2026-07-22/`.
- **PRE-FIX P1 MEASUREMENT EVIDENCE IS CLOSED AND ARCHIVED.** MTU was killed
  as a material cause (pf-0, `docs/bench/otp12-jumbo-win-2026-07-13/`); the
  fast arm is bistable so any future counterfactual must measure its own
  paired within-session floor; P1 reproduced on a second Mac pre-fix
  (`docs/bench/otp12-q-baseline-2026-07-13/`); and the pf-final baseline
  re-record constraints are D-2026-07-14-1. All superseded for cause by
  D-2026-07-22-2 below — detail in DEVLOG 2026-07-14 and those bench dirs.
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

## Handoff — 2026-08-01
- Done: pfc SHIPPED (D-2026-08-01-2); clp-1..3; checkers 1.71×; audit-16
  closed (heartbeat verbose-gated). Reviews clean: ls-1, clp3-ls4. DEVLOG.
- **ls-5 + ls-6 SHIPPED: converged mirror 166.38 → 55.57 s (5.1× vs field
  complaint)** — directory-sweep stat + stream interrogation deleted
  (D-2026-08-01-4); cr-ls5-1/-2/-3 verified-closed; ls1-phase bench dir.
- **1.0 RELEASE EFFORT ACTIVE** (owner goal + discretion, 2026-08-01):
  gates in `docs/plan/RELEASE_1_0.md`. G1 closed (3 rounds, 5 findings);
  1.0.0 bump landed; 3 interim archives built+verified off `ef9a13b2`.
  **1.0 NOW GATED ON TUI REWORK (D-2026-08-02-1, owner option 3):**
  M1–M6 via subagents; M1 LANDED (F3 picker mode, no caller wired yet);
  M3a/b need §6 sign-off (1/3/5/6 + 6). Also open: G5b cross-OS smoke matrix.

## Now (active work)

- **MACOS TEST FIREWALL CLEANUP SHIPPED LOCALLY (D-2026-07-23-6):** helper,
  16 fake-backed cases, parser check and mutation guards complete; no review
  pending. Plan: `docs/plan/MACOS_TEST_FIREWALL_CLEANUP.md`.
- **THUNDERBOLT LIFECYCLE + SSD FOLLOW-UP COMPLETE (D-2026-07-23-3):** both
  closed, no repeats authorized; records in DEVLOG 2026-07-23 and
  `docs/bench/{end-to-end-transfer-latency,thunderbolt-ssd}-*/`.
- **RELEASE COMPLETION SHIPPED:** exact candidate `d1f1152d` passed every gate
  and is published as `v0.1.1`; ceiling/Thunderbolt tuning is post-release.
- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
  D-2026-07-05-4 "flip the plan and go").** The invariant (plan doc,
  verbatim): ONE block of transfer code; direction/initiator/verb can
  NEVER affect wall time by blit's doing — impossible by construction
  because the per-direction drivers and `Push`/`PullSync` are deleted
  at cutover. Slices otp-1..13; converge-up per cell (±10%);
  symmetric-fs disk-to-disk verdict cells. **D-2026-07-05-2:
  same-build peers only, refusal at session open.**
  Slice status, the closed-slice record, and the otp-12 worker-parity
  repair (historical static-policy proof, superseded as an adaptive target
  by ldt-2) live in Queue 2, `docs/history/state-archive.md`, and DEVLOG —
  not restated here.
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
  `[x]`; **sf-3a+ blocked** until ONE_TRANSFER_PATH ships, then
  resume/re-derive on the unified baseline. Principle: ceiling-driven,
  never competitor-relative (D-2026-07-04-4 — do not re-litigate).
## Queue (ordered)

1. **`docs/plan/LOCAL_SMALL_FILE_PATH.md` (ACTIVE, D-2026-07-31-4) —
   PRIORITY-1, the owner's ruling on the 2026-07-31 field check.** Local
   transfer is too slow; the owner ruled it "should have been resolved before
   this was declared release-worthy." Recorded 2026-07-13, shipped in 0.1.1
   anyway under D-2026-07-13-2's BEHIND sequencing, now superseded.
   **ls-1 step (0) IS RUN and attributed** — evidence and caveats in
   `docs/bench/ls1-phase-2026-07-31/`. A converged dry run of the owner's
   tree (46,041 files → SMB) reproduced their no-op at 273.57 s with **COMPARE
   at 100% of wall** and apply at ZERO, **falsifying L1–L4 here** (all
   apply-path).
   **SHIPPED, 273.57 s → 166.38 s (1.64×; 1.71× vs the owner's original
   283.92 s field run)**, in two parts. (a) Round-trip elimination: reuse the
   stat's attribute DWORD instead of re-reading it with `GetFileAttributesW`
   (metadata 3.653 → 2.556 ms/file). (b) **a DEDICATED comparison pool whose
   concurrency is discovered at runtime with NO user-facing flag**
   (D-2026-08-01-1; `AdaptiveCheckers` follows the `dial.rs` rule —
   conservative floor, no probe phase, one rung per chunk on measured
   throughput, settle on regression), landing within 0.4% of the best
   hand-tuned value on its own. **Correction: the earlier "destination
   saturates" conclusion was WRONG** — one datapoint on rayon's shared pool;
   8 dedicated threads beat 32 shared by 33 s. See `.../checkers.md`.
   **ls-4 SHIPPED: apply ran ONE worker; now 8 (`DEFAULT_SINK_WORKERS`).**
   The owner's local-to-local mirror re-opened what step (0) had closed —
   the bottleneck follows the DESTINATION, and on local NVMe
   **APPLY_BACKPRESSURE was 81.7% of wall** (L3, measured). Sweep: local
   17.68→6.58 s (2.69×, knee 4–8); **SMB completely flat with no penalty at
   16**, which is why this is a fixed default rather than the `--checkers`
   runtime treatment — nothing to discover, and an adaptive throttle on the
   WRITE path would be risk for no measured gain. Real tree: **17.68 → 8.70 s
   (2.03×)** on the shipped default. Evidence `.../apply-workers.md`.
   **ls-5 SHIPPED: directory-sweep destination stat, 166.38 → 114.73 s.**
   One `read_dir` per destination directory answers the diff's target
   resolutions (6,191 sweeps / 6,709 dirs, zero re-sweep waste); the
   per-file stat REMAINS authoritative for anything the sweep cannot judge
   exactly. Local NVMe A/B neutral. Evidence `.../dir-sweep.md`.
   **ls-6 SHIPPED (D-2026-08-01-4): stream interrogation deleted from the
   default compare, 114.73 → 55.57 s** (5.1× vs field complaint); remaining
   lever: sweep prefetch or wire-carrier concurrent-session measurement.
   **Review loop CLOSED: 4 rounds, 8 findings, all resolved, r4 CLEAN with
   `guard_confirmed: true`** — see `REVIEW.md`. **`ls-0` LANDED**: the
   summary separates copied from repaired files and labels the rate as a
   whole-run average. NOT the closed P1 defect (D-2026-07-22-2).
2. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4):**
   slices otp-1..13; any external review requires exact owner approval under
   D-2026-07-23-7. **otp-1 … otp-12c are all `[x]`** — closed-slice record and
   the otp-12a/b/c matrices live in `docs/history/state-archive.md`, DEVLOG,
   and `docs/bench/otp12*/`; the one historical FAIL cell (wm_tcp_mixed) is
   closed by D-2026-07-22-2. **otp-12d and otp-13 are POST-RELEASE
   (D-2026-07-22-1)**; no performance acceptance matrix is a shipping
   prerequisite.
3. **`docs/plan/PER_FILE_ERROR_CONTAINMENT.md` — SHIPPED** (owner declared
   2026-08-01, D-2026-08-01-2; every acceptance criterion met, including the
   owner-only re-run convergence checkpoint — their run 2 copied 0 files,
   0 B on the originating `D:\Apps → H:\apps` workload). pfc-1..6 closed the
   root posture defect: a single survivable per-file error no longer aborts
   a transfer. audit-17 closed with it. 7 range reviews, 11 findings, 10
   verified-closed + 1 declined. Detail: the plan and DEVLOG 2026-07-31.
4. **`docs/plan/CLI_LIVE_PROGRESS.md` (ACTIVE, D-2026-07-31-2): clp-1, clp-2
   and clp-3 all `[x]` landed.** **clp-3** adds Dracula colour to the live
   row and the failure block (owner-chosen palette, scope "both"): phase word
   carries the colour, deleting is orange not red so red keeps its meaning
   for failures, counts and failed paths stay unstyled. Colour is applied
   AFTER layout — escapes are zero-width on screen but not in a `String` —
   and the plain forms are `#[cfg(test)]` references so production output
   cannot drift from them. `NO_COLOR` + TTY detection are the whole control
   surface; no flag (D-2026-08-01-1). **Not yet reviewed.** clp-1/clp-2 gave one live row with truthful phases
   (enumerating → comparing/up-to-date → copying → deleting; dry runs
   never claim deletion), width-safe current-file segment, `-v` per-file
   lines through the row handle, and the `log` backend routed through
   the row while it lives (warns scroll above, never scroll it away).
   Owner confirmed on a real TTY (2026-08-01) that the row holds one line
   while `-v` lines scroll above it. Residue (d) (remote-route attachment)
   deferred to remote render work. TUI-scale redesign stays a queued TODO.
5. **POST-RELEASE performance declarations:** ue-1, ue-2, and the REV4
   performance status flip are not release gates (D-2026-07-22-1).
6. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
   resumes/re-derives after ONE_TRANSFER_PATH ships.
7. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
   Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
   cutover as a runtime-selected write strategy in the unified receive
   sink (design: eval doc §If-FAST-evidence; dead module deletes in
   w8-1). Rig facts + build recipe: DEVLOG 2026-07-05 10:00.
   **Standing owner safety rule**: ALL activity on rig `zoey` stays
   inside its `…/blit-temp/` folder — nothing written outside it, ever;
   no daemon runs on zoey without a fresh go.
8. **Post-REV4 residue** (unowned, 5 items) — list in DEVLOG 2026-07-13 21:00Z.

## Authoritative docs right now

- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Active**,
  D-2026-07-09-1 — otp-7 slice design; governs otp-7a/7b);
  **`docs/plan/PER_FILE_ERROR_CONTAINMENT.md` (ACTIVE, D-2026-07-30-1 —
  governs pfc-1..5, the current implementation work).**
- Shipped release record: **`docs/RELEASE_READINESS.md`** and
  **`docs/plan/RELEASE_COMPLETION.md`**.
- Historical live-tuning record: **`docs/plan/LIVE_DIAL_TUNING.md`**; exact
  session audit: **`docs/bench/ldt4-evidence-audit-2026-07-22/`**.
- Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**paused** at
  sf-2) and **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** (code-
  complete; measurement gates remain). REV4 superseded v1/REV2/REV3
  (history only).
- Process: `.agents/playbooks/openreview.md` — synchronous unprimed review only
  after exact owner approval under D-2026-07-23-7 (formal review uses Claude
  Opus 4.8/max; Grok advisory; landed-slice codex dispatch is standing under
  D-2026-07-31-3); `.agents/playbooks/codereview.md` supplies finding intake
  and triage only. `docs/agent/GPT_REVIEW_LOOP.md` is historical;
  `.review/README.md` is retired as the grading mechanism (its
  `findings/`/`results/` records and the REVIEW.md index remain live).
- Review loop: `REVIEW.md` (no open rows) + `.review/findings|results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (module delete ratified
  D-2026-06-12-1, executes w8-1; **capability unparked D-2026-07-05-3** —
  post-cutover write strategy), `TUI_REWORK.md` (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).

## Blocked / waiting (owner declarations and explicitly dated external blockers; checkpoints are owner-only)

- **Rig facts:** `.agents/machines.md` is canonical; no host pairings here.
- **Two stale test firewall entries await separately approved cleanup;** the
  helper is shipped, but these historical entries remain untouched. Exact
  paths and the no-reuse/no-removal gate: `.agents/machines.md`.

## Open questions

- None outstanding. (pfc Shipped by owner declaration, D-2026-08-01-2.)
