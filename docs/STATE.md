# STATE — single entry point for "what is true right now"

Last updated: 2026-06-20 (unified-transfer plan review freeze;
`UNIFIED_TRANSFER_ENGINE_REV2.md` drafted) at commit `7ecc355`
(pre-commit doc edits; uncommitted in working tree)

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **Transfer-core architecture conflict RESOLVED** (D-2026-06-20-1):
  convergence, not ground-up redesign. One src/dst-agnostic sequencer owns
  all four paths (local↔local, push, pull, daemon↔daemon); one live dial
  (streams + knobs) replaces the three static ladders, **bounded-unilateral**
  (receiver capacity profile + sender controls within it). **No probe
  phase** — the engine starts within ~1s at conservative defaults and the
  tuner adjusts dials live from the first byte; the planner is
  workload-shape-aware (file count vs bytes). Adaptive-streams PR1+PR2 (up
  to `eafb187`) salvaged as the substrate per D-2026-06-07-2; PR3 WIP
  `d9d4ec7` excluded.
- **Plan review IN PROGRESS — CODING FROZEN** (D-2026-06-20-4):
  `docs/plan/UNIFIED_TRANSFER_ENGINE.md` remains the original Active
  plan. `docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md` is the Draft review
  candidate; `docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md` (2026-06-20) is
  a further-drafted candidate = rev2 + restored Risks section + restored
  "C-ready by construction" acceptance criterion + corrected
  static-ladder references (two tables, not three) + explicit slice
  dependencies + labeled agent recommendations on the open questions.
  Owner asked to roll review findings into rev2 and freeze all
  unified-transfer-engine coding pending the owner's final plan
  decision. The candidates keep convergence and the four bound
  parameters from D-2026-06-20-2, but tighten slice shape: streaming
  initial planning gets its own slice, local fast paths become
  engine-owned strategies, work-stealing is treated as observable
  behavior, proto capacity/resize compatibility is designed before
  dependent code, and pull parity waits for multistream PullSync.
- **Reviewer grading complete** (2026-06-12): design-4 + design-5 accepted
  (`a841691`, `b5cbb38`); `REVIEW.md` rows `[x]`; ready queue empty.
  Validation: fmt + clippy green; `cargo test --workspace` 1370 passed, 0
  failed, 1 ignored. No coder/reviewer work in flight; 2026-06-12
  authorizations were single-session and do not carry forward.

## Queue (ordered)

1. **Owner final plan decision — CODING FROZEN UNTIL THEN**: choose
   whether `UNIFIED_TRANSFER_ENGINE_REV2.md` replaces/amends the original
   Active plan, or request another revision. No unified-transfer-engine
   code slice (`ue-1a` or rev2 equivalent) may start until the owner makes
   this final plan decision. Also pending: push approval for the Windows
   test-tuning commit (`439a2a7`, local-only — Windows CI red until it
   lands).
2. **After final plan decision only**: execute the accepted slice list.
   Current Draft candidate is `UNIFIED_TRANSFER_ENGINE_REV2.md` with
   `ue-r2-1a`–`ue-r2-1h` + `ue-r2-2`; original `UNIFIED_TRANSFER_ENGINE.md`
   has `ue-1a`–`ue-1e` + `ue-2`. Use `slice` per `.review/` only after a
   fresh owner authorization.
3. **Design-review queue (independent, survives the convergence)** —
   `REVIEW.md` order governs. Highest open ratified row is w4-1
   (AbortOnDrop family, High); next include w4-3 and W1 socket-policy /
   timeout constants. These are correctness findings, unaffected by the
   engine convergence — may be folded into `ue-1c` or fixed standalone,
   owner's call.
4. **10 GbE benchmark session — DEFERRED** (owner 2026-06-12: rig assembly
   is real work; benchmarking pre-convergence is churn). Now also the
   `ue-1` sign-off measure (loopback parity band: local↔local /
   local→daemon / daemon→local within a tight band) AND the gate for
   `ue-2` (continuous/C). Capture before/after baselines there, not earlier.
   Remains the zero-copy revisit gate (D-2026-06-12-1). After `ue-1`:
   audit Round 1, TUI rework (Round 2), H10b streaming planner.

## Authoritative docs right now

- Active original plan: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` —
  convergence engine; **Active but parked by D-2026-06-20-4**.
  Supersedes `MULTISTREAM_PULL.md` (now Superseded; goal absorbed as
  `ue-1d`).
- Draft review candidates: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md`
  and `docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md` (rev3 = rev2 + restored
  Risks/C-ready criterion + corrected ladder refs + slice dependencies +
  agent recommendations); not implementation authority until owner final
  decision.
- Design queue: `REVIEW.md` (13 design-queue rows `[x]`, 0 rows `[~]`) + the three
  `docs/audit/` 2026-06-11 deliverables
- Review loop: `REVIEW.md` + `.review/README.md` + `.review/findings/` +
  `.review/results/` (ready queue empty)
- Other plans: `docs/plan/ZERO_COPY_RECEIVE_EVAL.md` (delete ratified
  D-2026-06-12-1, executes in w8-1), `docs/plan/TUI_REWORK.md` (gated on
  Round 1), `docs/plan/BENCHMARK_10GBE_PLAN.md` (`ue-1` sign-off + `ue-2`
  gate)
- Findings: `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md` (R3 governs)
- Decisions: D-2026-06-20-1 (convergence direction), D-2026-06-20-4
  (ongoing plan review; coding frozen)

## Blocked / waiting

- **Owner**: final plan decision for unified transfer engine. Coding is
  frozen by D-2026-06-20-4 until owner decides whether rev2 replaces or
  amends the original plan. Also pending: push approval for the Windows
  test-tuning commit (`439a2a7`, local-only — Windows CI red until it
  lands).

## Open questions

- **(RESOLVED 2026-06-20, D-2026-06-20-1 / -2)** Transfer-core architecture
  — convergence per `UNIFIED_TRANSFER_ENGINE.md` (Active). Closed.
- **(RESOLVED — veto, D-2026-06-20-3)** Agent's flagged inference to fold
  the H10b streaming planner into the engine — **vetoed by owner.**
  D-2026-06-04-3 stands unchanged; engine's workload-shape-awareness +
  1s-start stand alone.
- **Engine type** — deferred to agent (recommends new `TransferEngine` +
  local adapter); ratified at `ue-1c`, owner may override.
- `UNIFIED_TRANSFER_ENGINE` plan-review decisions (2026-06-20, owner;
  candidate is now `REV3.md` = REV2 + restored Risks/C-ready criterion +
  corrected ladder refs + slice deps):
  - **(RESOLVED)** First-byte-within-~1s is a hard invariant for every
    mode except the modes where moving any byte before full knowledge
    would be unsafe (mirror/delete, resume, checksum-refusal). Novel vs
    known workload is a tuning-strategy choice (start-something-and-tune
    vs replay-optimal-last-run via the in-tree `perf_history`/
    `perf_predictor`), not an exception. Both meet 1s.
  - **(RESOLVED)** Deprecated `Pull` deletion stays in-plan as
    `ue-r2-1h`, gated on `ue-r2-1g` + `ue-r2-1b` compat tests.
  - **(OPEN)** Does REV3 replace `UNIFIED_TRANSFER_ENGINE.md`, or stay a
    review branch? Owner: no flip yet — planning review in progress;
    REV3 stays Draft, v1 stays Active-but-parked.
  - **(OPEN)** Edit D-2026-06-20-1 now to strip superseded
    warmup/size-gate wording, or let later decisions stand? Owner: not
    sure.
- `docs/agent/SETUP.md` content — owner must supply (other machine);
  `.review/README.md` lines 8/101 still point at unreadable paths.
- Disposition of adaptive-streams branch refs after `ue-1a` lands
  (D-2026-06-07-2).
- Windows: w9-1 ungated 27 tests; w9-5/w9-4/w4-2 added ungated
  daemon-spawn tests — unverified on Windows; next windows-latest CI run or
  run-blit-tests.ps1 triages real failures into findings.

## Handoff log (newest first, keep ≤ 3)

- **2026-06-20** @ `7ecc355` (doc edits uncommitted) — owner requested
  plan review findings be rolled into `docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md`
  and recorded a freeze (D-2026-06-20-4): unified-transfer-engine coding
  is frozen pending owner final plan decision. Rev2 keeps convergence and
  the D-2026-06-20-2 bound parameters but splits oversized slices and
  clarifies local fast paths, work-stealing tests, proto compatibility,
  and pull parity timing. In-flight: plan review only, no code. **Exact
  first action next session**: owner decides whether rev2 replaces/amends
  the original Active plan or requests another revision; push approval for
  `439a2a7` still pending separately.
- **2026-06-12** @ `b5cbb38` — gemini-reviewer session: graded and accepted both pending sentinels (design-4 and design-5); verdicts committed, `REVIEW.md` rows `[x]`, ready/ queue empty. In-flight: none. **Exact first action next session**: owner decides the remaining gates (w2-3 Active flip, push approval).
- **2026-06-12** @ `0213896` — gpt-reviewer session: graded and accepted
  all 4 pending sentinels (w4-2, w5-2, w7-4, w7-6); verdicts committed,
  `REVIEW.md` rows `[x]`, ready queue empty. In-flight: none; owner gates
  remain design-4 ratification, w2-3 Active flip, and push approval.
  **Exact first action next session**: owner decides the gates; if coder
  work is re-authorized, run `slice` and start at the top open
  `REVIEW.md` row (currently w4-1).
