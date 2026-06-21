# STATE — single entry point for "what is true right now"

Last updated: 2026-06-20 (owner: "rev4 replaces v1" — `REV4` flipped to
**Active**, v1/REV2/REV3 Superseded; D-2026-06-20-5) at commit `09268eb`
(doc edits uncommitted in working tree)

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
- **Plan decided — REV4 is the Active convergence plan** (owner: "rev4
  replaces v1"; D-2026-06-20-5). `UNIFIED_TRANSFER_ENGINE_REV4.md` is
  **Active**; v1, REV2, and REV3 are **Superseded**. The D-2026-06-20-4
  coding freeze is lifted **as to the plan decision** — but per
  AGENTS.md §9 no code slice starts without a fresh per-slice owner
  authorization (next: `ue-r2-1a`). REV4 is REV3 with its
  code-reality section corrected against the tree at `HEAD`. REV3's
  headline "two static tables, not three" correction was itself **wrong**
  — all three stream-count ladders are live (`determine_remote_tuning`,
  `desired_streams` `push/control.rs:476`, `pull_stream_count`
  `pull.rs:904`); v1's three-ladder count was right; `tuning.rs`'s own
  doc comment confirms "the daemon's push negotiation runs its own ladder
  and wins". REV4 also re-scopes the pull single-stream claim to PullSync
  (deprecated `Pull` is already multistream), fixes strategy names
  (`journal_no_work`, not `journal_skip`; no distinct `single_file`),
  grounds every symbol with `file:line`, and carries v1's
  lineage/absorption header forward so it can stand as plan-of-record.
  (The D-2026-06-20-4 freeze that gated this is now resolved by the
  owner's decision.) REV4 keeps convergence and the four bound
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

1. **Owner authorization to begin `ue-r2-1a` (salvage substrate)** —
   REV4 is Active (D-2026-06-20-5), but per AGENTS.md §9 the plan
   decision does not authorize coding. Next step is a fresh owner
   go-ahead to run `slice` and start `ue-r2-1a` (cherry-pick adaptive
   PR1+PR2 up to `eafb187`, resolve the `data_plane.rs`
   StallGuard-vs-`Probe` conflict). Also pending: push approval for the
   Windows test-tuning commit (`439a2a7`, local-only — Windows CI red
   until it lands).
2. **After owner authorization**: execute the REV4 slice list in order —
   `ue-r2-1a` → `1b` → `1c` → `1d`/`1e`/`1f` → `1g` → `1h` → `ue-r2-2`
   (deps in REV4 §"Slice dependencies"). Use `slice` per `.review/` only
   after a fresh owner authorization.
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

- **Active plan: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** —
  convergence engine; flipped Active by D-2026-06-20-5 ("rev4 replaces
  v1"). Carries forward v1's absorption of `MULTISTREAM_PULL.md` (the
  pull-multistream goal lands as slice `ue-r2-1g`).
- Superseded by REV4 (history only, do not implement):
  `UNIFIED_TRANSFER_ENGINE.md` (v1), `…_REV2.md`, `…_REV3.md`. REV4 = REV3
  with the code-reality section corrected against `HEAD` (REV3's "two
  tables, not three" ladder claim was wrong — all three ladders are
  live), pull single-stream re-scoped to PullSync, strategy names fixed,
  every symbol grounded with `file:line`.
- Design queue: `REVIEW.md` (13 design-queue rows `[x]`, 0 rows `[~]`) + the three
  `docs/audit/` 2026-06-11 deliverables
- Review loop: `REVIEW.md` + `.review/README.md` + `.review/findings/` +
  `.review/results/` (ready queue empty)
- Other plans: `docs/plan/ZERO_COPY_RECEIVE_EVAL.md` (delete ratified
  D-2026-06-12-1, executes in w8-1), `docs/plan/TUI_REWORK.md` (gated on
  Round 1), `docs/plan/BENCHMARK_10GBE_PLAN.md` (`ue-1` sign-off + `ue-2`
  gate)
- Findings: `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md` (R3 governs)
- Decisions: D-2026-06-20-1 (convergence direction), D-2026-06-20-5
  (REV4 replaces v1 as Active; v1/REV2/REV3 Superseded; plan-decision
  freeze lifted)

## Blocked / waiting

- **Owner**: (1) fresh per-slice authorization to begin `ue-r2-1a` —
  REV4 is Active, but AGENTS.md §9 means the plan decision alone does not
  greenlight coding; (2) push approval for the Windows test-tuning commit
  (`439a2a7`, local-only — Windows CI red until it lands).

## Open questions

- **(RESOLVED 2026-06-20, D-2026-06-20-1 / -2)** Transfer-core architecture
  — convergence per `UNIFIED_TRANSFER_ENGINE.md` (Active). Closed.
- **(RESOLVED — veto, D-2026-06-20-3)** Agent's flagged inference to fold
  the H10b streaming planner into the engine — **vetoed by owner.**
  D-2026-06-04-3 stands unchanged; engine's workload-shape-awareness +
  1s-start stand alone.
- **Engine type** — deferred to agent (recommends new `TransferEngine` +
  local adapter); ratified at `ue-r2-1c`, owner may override.
- `UNIFIED_TRANSFER_ENGINE` plan-review decisions (2026-06-20, owner;
  `REV4.md` is now the **Active** plan — D-2026-06-20-5; REV3's ladder
  "correction" was wrong, all three ladders are live):
  - **(RESOLVED)** First-byte-within-~1s is a hard invariant for every
    mode except the modes where moving any byte before full knowledge
    would be unsafe (mirror/delete, resume, checksum-refusal). Novel vs
    known workload is a tuning-strategy choice (start-something-and-tune
    vs replay-optimal-last-run via the in-tree `perf_history`/
    `perf_predictor`), not an exception. Both meet 1s.
  - **(RESOLVED)** Deprecated `Pull` deletion stays in-plan as
    `ue-r2-1h`, gated on `ue-r2-1g` + `ue-r2-1b` compat tests.
  - **(RESOLVED 2026-06-20, owner — D-2026-06-20-5)** REV4 **replaces**
    v1. `UNIFIED_TRANSFER_ENGINE.md` Superseded; REV2/REV3
    Superseded-by-REV4; REV4 is the one Active plan.
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

- **2026-06-20** @ `09268eb` (doc edits uncommitted) — reviewed all three
  unified-transfer candidates against the code, produced REV4 (= REV3 +
  corrected code reality, every symbol grounded with `file:line`), and —
  on owner's "rev4 replaces v1" — recorded **D-2026-06-20-5** and
  propagated it: REV4 flipped to **Active**, v1/REV2/REV3 marked
  **Superseded**, STATE + DECISIONS updated. Decisive finding: REV3's
  "two static tables, not three" ladder correction was itself wrong —
  all three ladders (`determine_remote_tuning`, `desired_streams`,
  `pull_stream_count`) are live. No git commit (owner gate). In-flight:
  none; coding still gated by AGENTS.md §9 absent a per-slice go-ahead.
  **Exact first action next session**: owner authorizes `ue-r2-1a` (or
  edits D-2026-06-20-1 / approves the `439a2a7` push — both still open).
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
