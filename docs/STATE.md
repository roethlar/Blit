# STATE — single entry point for "what is true right now"

Last updated: 2026-07-31 (pfc-1..6 + clp-1..2 landed; owner field check returned)

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

## Handoff — 2026-07-31 at `a0b5d83d`
- Done: pfc-1..6 and clp-1..2 landed (8 slices, 2 plans), review program
  closed (7 range reviews, 11 findings resolved). Owner ran the field check.
- Next: answer the owner's four summary-coherence objections (Open questions)
  — no code change without their ruling; the Shipped flip is theirs alone.

## Handoff — 2026-07-23 at `544adf81`
- Done: Blit 0.1.1 is publicly released from exact validated candidate
  `d1f1152d`; current post-release diagnostics and firewall-test work are closed.
- Next: re-ground `ONE_TRANSFER_PATH`, identify its first unblocked post-release
  slice, and stop for an approved plan/go before code or hardware work.

## Now (active work)

- **MACOS TEST FIREWALL CLEANUP SHIPPED LOCALLY (D-2026-07-23-6):** helper,
  16 fake-backed cases, real parser check, and mutation guards complete; no
  external review pending. Plan: `docs/plan/MACOS_TEST_FIREWALL_CLEANUP.md`.
- **THUNDERBOLT LIFECYCLE ATTRIBUTED (D-2026-07-23-3), SSD FOLLOW-UP
  COMPLETE:** both closed, no repeats authorized; records in DEVLOG
  2026-07-23 and `docs/bench/end-to-end-transfer-latency-2026-07-23/` +
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
   D-2026-07-23-7. **otp-1 … otp-12c are all `[x]`** — closed-slice record
   and the otp-12a/b/c matrices live in `docs/history/state-archive.md`,
   DEVLOG, and `docs/bench/otp12*/`; the one historical FAIL cell
   (wm_tcp_mixed) is closed by D-2026-07-22-2. **otp-12d and otp-13 are
   POST-RELEASE (D-2026-07-22-1)**; retained pre-fix evidence remains usable
   for what it records, and no performance acceptance matrix is a shipping
   prerequisite.
2. **`docs/plan/PER_FILE_ERROR_CONTAINMENT.md` (ACTIVE, D-2026-07-30-1) —
   CODE-COMPLETE, ONE ACCEPTANCE CRITERION LEFT (the owner's).** Root
   posture defect: first-error-wins pipeline + no failure vocabulary in
   `TransferSummary`, so one survivable per-file error killed a session;
   SMB-synthesized HIDDEN on dot-named files could never converge. Found
   via the owner's `D:\Apps → H:\apps` (Samba) mirror abort. Q1 settled
   (mirror deletes proceed; move source-deletion refuses while failures
   exist). **pfc-1..6 all `[x]` landed on `master`**, one commit per
   slice, each red/green guard-proven with a full gate at landing:
   shared `attributes_converge` predicate; bounded per-file failure
   report in `SinkOutcome`; tar-shard per-member containment with exact
   failed identity; `TransferSummary.files_failed`/`failures` on the
   wire (CONTRACT_VERSION 5→6); CLI failure block + exit 2 + the real
   `refuse_source_delete_on_failures` gate on all eight move routes;
   attributes-only in-place repair (`files_repaired`, zero bytes
   re-sent). The pfc-2 interim mirror-only interlock is GONE — containment
   applies to every session kind. audit-17 closed end to end.
   **Review program complete** under D-2026-07-31-3's standing codex
   dispatch: 7 range reviews (pfc-1, clp-1, pfc-5, pfc-6 clean; pfc-2/3/4
   raised 11 findings — 10 verified-closed with guard-confirmed codex
   verdicts, cr-clp2-4 declined with record). Per-slice detail: DEVLOG
   2026-07-31 and the plan's landing notes.
   **The one open acceptance criterion is re-run convergence, and it is
   the OWNER'S to declare** (checkpoints are owner-only). Owner ran it
   2026-07-31 and the data satisfies it — run 1: 9578 files / 393.01 MiB
   in 355.23s, 5445 files metadata-repaired with zero bytes re-sent; run
   2: 0 files, 0 B in 283.92s (a true no-op). **No declaration was given;
   the owner instead raised four coherence objections — see Open
   questions.** Plan does NOT flip to Shipped until the owner says so.
3. **`docs/plan/CLI_LIVE_PROGRESS.md` (ACTIVE, D-2026-07-31-2): clp-1 and
   clp-2 both `[x]` landed** — one live row with truthful phases
   (enumerating → comparing/up-to-date → copying → deleting; dry runs
   never claim deletion), width-safe current-file segment, `-v` per-file
   lines through the row handle, and the `log` backend routed through
   the row while it lives (warns scroll above, never scroll it away).
   Owner confirmed on a real TTY that the row holds one line (2026-07-31,
   pre-clp-2) and used it through both field-check runs. Residue (d)
   (remote-route attachment) deferred to remote render work — see plan
   landing notes. TUI-scale redesign stays a separate queued TODO.
4. **`docs/plan/LOCAL_SMALL_FILE_PATH.md` (Draft, D-2026-07-13-2):** local
   apply ships one worker and does not scale; resume only under an active plan.
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
- Review loop: `REVIEW.md` (no open rows) + `.review/findings/` +
  `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (module delete ratified
  D-2026-06-12-1, executes w8-1; **capability unparked D-2026-07-05-3** —
  post-cutover write strategy), `TUI_REWORK.md` (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).

## Blocked / waiting (owner declarations and explicitly dated external blockers; checkpoints are owner-only)

- **Rig facts:** `.agents/machines.md` is canonical; do not restate host pairings here.
- **Two stale test firewall entries await separately approved cleanup;** the
  helper is shipped, but these historical entries remain untouched. Exact
  paths and the no-reuse/no-removal gate: `.agents/machines.md`.

## Open questions

- **The owner's four objections to the pfc field-check summary (2026-07-31),
  unanswered and blocking the Shipped flip.** Against run 1's output
  (`Mirror complete: 9578 files, 393.01 MiB in 355.23s` / `• Throughput:
  1.11 MiB/s | Workers used: 1` / `• Repaired metadata on 5445 file(s) — no
  bytes re-sent`): (a) why `Workers used: 1`; (b) 1.11 MiB/s is too slow;
  (c) is throughput an accurate/useful metric when the run re-sent no bytes;
  (d) the byte total next to "no bytes re-sent" reads as incoherent.
  Established from code, not yet ruled on: `copied_files`/`total_bytes` and
  `files_repaired` are DISJOINT sets (`local_session.rs` re-run test pins
  a repaired file at `copied_files: 0`, `total_bytes: 0`), so the two lines
  are consistent but the adjacency invites the misread; `Workers used` is a
  hard-coded `1` outside the debug limiter (`blit-cli/src/transfers/
  local.rs:876`) because local apply genuinely runs one sink worker — the
  known gap already queued as item 4; and the throughput divisor is TOTAL
  wall time, so run 2's 283.92s all-scan no-op implies ~71s of actual copy
  window in run 1 (≈5.5 MiB/s, not 1.11). Whether any of this becomes a
  code change is the owner's call.
