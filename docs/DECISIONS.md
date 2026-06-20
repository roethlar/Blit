# DECISIONS — settled choices

**Status**: Active

Append-only ledger of decisions that future sessions must not relitigate or miss.
Add entries via the `decision` procedure in `docs/agent/PROTOCOL.md`. Newest last.
When a decision supersedes plan text, the plan text gets edited in the same
session — this file is the index, not a substitute for fixing the doc.

Format:

```
## D-<YYYY-MM-DD>-<n> — <short title>
- Decision: <one line>
- Why: <one line>
- Supersedes: <doc §/decision ID, or "nothing">
```

---

## D-2026-05-31-1 — v0.1.0 shipped; release plan frozen
- Decision: `RELEASE_PLAN_v2_2026-05-04.md` is a frozen reference, no longer the active source of truth.
- Why: 0.1.0 tagged 2026-05-31; the plan served its purpose.
- Supersedes: RELEASE_PLAN_v2_2026-05-04.md as active plan.

## D-2026-05-31-2 — Pick-not-Type TUI direction
- Decision: `TUI_REWORK.md` (dual-pane, M1–M6) supersedes `TUI_DESIGN.md` §6 trigger-modal text inputs and the F3 free-text destination prompt.
- Why: any field requiring the operator to recall and type an off-screen path is an interface failure.
- Supersedes: TUI_DESIGN.md §6 (portions).

## D-2026-06-04-1 — R3 overrides R2 in the audit chain
- Decision: where R2 and R3 disagree on a finding's severity or content, R3 wins; see the ID-override table in `AUDIT_REPORT_2026-06-04_INDEX.md`.
- Why: R3 incorporates the GPT R2 critique and severity rebalance.
- Supersedes: conflicting R2 entries.

## D-2026-06-04-2 — Env vars are out for app + diagnostic config
- Decision: no environment-variable configuration carve-out (R3-L39); purge completed via `audit-l39-m27-env-var-purge`.
- Why: owner policy — config surfaces stay explicit.
- Supersedes: nothing (clarifies prior ambiguity).

## D-2026-06-04-3 — Streaming planner ratified, build deferred
- Decision: `greenfield_plan_v6.md` §1.1 (streaming planner + 1 s heartbeat + 10 s stall detector) is canonical but not yet built; multi-slice implementation queued after audit Round 1 (H10b).
- Why: data-loss/DoS hardening takes priority; the plan claim is ratified rather than retired.
- Supersedes: nothing.

## D-2026-06-06-1 — STATE.md precedence model adopted
- Decision: `docs/STATE.md` is the single entry point for current state, with the precedence order in `AGENTS.md` §1; DEVLOG.md is write-only history, TODO.md is backlog-only, tool-local memories are scratch.
- Why: state smeared across TODO/DEVLOG/plan-README/Serena was the drift mechanism the 2026-06-04 audit documented (drift-* findings, M28).
- Supersedes: "Agent-Specific Expectations" in the previous AGENTS.md (Serena memories as session persistence).

## D-2026-06-07-1 — Keep the `c793df2` octopus on master; no history rewrite
- Decision: `c793df2` (a `git merge -s ours` octopus whose parents are `600023a` + `eafb187` + `d9d4ec7`) stays on `origin/master`; we do **not** rewrite history or force-push to remove it.
- Why: its tree is byte-identical to `600023a` (`git diff 600023a c793df2` is empty) and the workspace builds, so it is cosmetically ugly but harmless; rewriting already-pushed shared history is riskier than the wart. The merge was pushed without owner approval — the corrective is the new AGENTS.md §8 Git-safety contract, not a second unsafe operation.
- Consequence (the trap): because `eafb187` and `d9d4ec7` are now *ancestors* of master, `git branch --merged` falsely reports them merged and a plain `git merge` of either no-ops without landing code. `d9d4ec7` (adaptive-streams-pr3-resizable) does **not** build and its files are not in master's tree. Branch cleanup in this repo is by explicit name only, never `--merged`.
- Supersedes: nothing.

## D-2026-06-07-2 — Adaptive-streams lands via cherry-pick/rebase, excluding the WIP
- Decision: the adaptive-streams stack (live-progress → PR1 telemetry → PR2 work-queue → PR2 review fix, up to `eafb187`) lands later as a planned `docs/plan/` slice via cherry-pick or rebase onto fresh commits — never via `git merge` of the branch (see D-2026-06-07-1 trap). `d9d4ec7` (PR3 WIP, "DOES NOT BUILD") is explicitly excluded until it is finished and compiles.
- Why: the `-s ours` octopus recorded those tips as parents without landing their code, so the feature is not actually in master; a real merge would no-op. The one real conflict (`data_plane.rs`: `StallGuardWriter` vs the `Probe` generic) must be resolved by hand, which only a cherry-pick/rebase surfaces.
- Supersedes: nothing.

## D-2026-06-11-1 — Design-coherence review plan Active; ratification covers Phase A only
- Decision: `docs/plan/DESIGN_COHERENCE_REVIEW.md` flipped Draft → Active. Owner approval authorizes **Phase A only** (concept-ownership map + per-crate stratum inventory); Phases B and C each need a fresh go/no-go at the preceding checkpoint. Interview decisions bound into the plan: blit-tui light pass, owner ratifies each Phase C finding, wire-breaking recommendations in scope (proto not frozen).
- Why: the repo was built by many models across several greenfield restarts and the owner judges it too inconsistently designed to trust as-is; mapping concept ownership precedes any re-scope (audit-h3c slice 2) or feature landing (adaptive-streams) so the fixes get designed once.
- Supersedes: nothing.

## D-2026-06-11-2 — Design-review queue ratified in full; Pull-RPC delete; zero_copy gets a FAST evaluation
- Decision: All Phase C slices (`AUDIT_REPORT_2026-06-11_DESIGN.md`) ratified as proposed and entered into REVIEW.md in the proposed order. Embedded decisions: (a) **W2.4** — the deprecated Pull RPC is deleted once W2.3 has harvested its multi-stream pattern; criterion applied: not needed for FAST/SIMPLE/RELIABLE in any scenario. (b) **W8.1** — `zero_copy.rs` is **excluded** from the dead-code deletion sweep; owner judges it has FAST potential; disposition is an evaluation slice (`w8-1b`) that either produces a plan doc to wire splice into the receive pipeline or concludes deletion. (c) **W2.3** — writing the multi-stream-pull plan doc is authorized (no code before Status: Active).
- Why: review program (D-2026-06-11-1) delivered all three phases; owner is the gate for queue entry and exercised it in full.
- Supersedes: nothing (completes D-2026-06-11-1; `DESIGN_COHERENCE_REVIEW.md` flips Active → Shipped).

## D-2026-06-12-1 — zero_copy.rs: delete (w8-1b verdict)
- Decision: `zero_copy.rs` is deleted rather than wired in. The w8-1b evaluation (`docs/plan/ZERO_COPY_RECEIVE_EVAL.md`) recommended deletion and the owner agreed (2026-06-12 session). The deletion executes inside w8-1 once the w5-1 sentinel (lib.rs) is graded — it is no longer excluded from that sweep.
- Why: the dead draft busy-waits on EAGAIN (would be rewritten, not revived); wiring needs a raw-fd special case beside a permanent buffered fallback; the CPU saving is a fraction of one core, Linux-only, and unmeasured. Revisit gate: 10 GbE benchmarks showing receive-side CPU saturation — design notes preserved in the eval doc.
- Supersedes: D-2026-06-11-2 item (b) (zero_copy exclusion from W8.1 was pending this evaluation; the evaluation is done).

## D-2026-06-20-1 — Transfer-core architecture conflict resolved: convergence, not ground-up redesign
- Decision: The 2026-06-14 "redesign the transfer subsystem from the ground up" framing is resolved as **convergence**, not a rebuild. One src/dst-agnostic sequencer owns all four paths (local↔local, push, pull, daemon↔daemon); the dial (stream count + all transfer knobs) is a single live object adjusted from measured telemetry; the already-shared byte-moving leaf stays. Dials are **bounded-unilateral** (receiver advertises a capacity ceiling; sender owns the dial within it) and **size-gated** (small transfers skip the probe entirely). The adaptive-streams stack (PR1 telemetry + PR2 work-stealing queue, up to `eafb187`) is salvaged as the substrate per D-2026-06-07-2; PR3 WIP (`d9d4ec7`) stays excluded. Built A-first (warmup), C-ready by construction (mutable dial + elastic stream-set exist from A, so continuous adjustment is a later feed, not a retrofit). Plan: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (Draft — awaiting owner Draft→Active flip).
- Why: owner (30-year IT veteran, not a developer) judges the fragmentation — one engine for local, hand-wired loops for push/pull, three competing static stream-count tables, no live tuning — is the root of the "local↔local 10× slower than local→daemon" class of drift; a single engine makes that class impossible by construction and gives the LLM agent one place to update. Ground-up rebuild was judged too much; convergence on the existing shared leaf is the FAST/SIMPLE/RELIABLE fit. The adaptive substrate was purpose-built by an earlier Fable session as C's foundation, so building A on it does not paint the design into a corner.
- Scope consequence: this **moots the standalone premise** of the queued incremental work and absorbs the goals — w2-2 (three ladders → one dial) is `ue-1b`; w2-3 multi-stream pull (`MULTISTREAM_PULL.md`) is `ue-1d` via the unified sequencer; w2-4 (delete deprecated Pull RPC) is `ue-1e`; adaptive-streams cherry-pick is `ue-1a`. `MULTISTREAM_PULL.md` is superseded as a standalone plan (kept as reference); its goal survives inside this plan. The design-review queue's correctness findings (w4-1 etc.) are independent and unaffected.
- Supersedes: the "ground-up redesign" framing of the 2026-06-14 open question recorded in STATE.md (that open question is now closed); `MULTISTREAM_PULL.md` as a standalone plan (goal absorbed into `UNIFIED_TRANSFER_ENGINE.md` slice `ue-1d`).

## D-2026-06-20-2 — UNIFIED_TRANSFER_ENGINE.md flipped Draft → Active; four bound parameters
- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` is **Active**. Owner approved with four parameters that bind the design: (q1) **no probe-then-go phase** — the engine starts moving within ~1s at conservative defaults bounded by the receiver ceiling and the tuner adjusts dials live from the first byte; the "small-transfer threshold" is obviated (no probe to skip), and the **planner** carries the workload-shape judgment (file count vs bytes) that the old size gate proxied. (q2) the receiver advertises a **rich capacity profile** (CPU cores, disk class, load, max streams, drain estimate) — "more data serves the ubergoal"; do not minimize the negotiation payload. (q3) engine type **deferred to the agent**, who recommends a new src/dst-agnostic `TransferEngine` + a local adapter over renaming `TransferOrchestrator` in place — ratified at `ue-1c`. (q4) `ue-2` (mid-transfer stream add/drop via PR3's resize proto) is **in scope at Active**, sequenced last; 11 months of owner benchmarking is the justification, the 10 GbE rig is sign-off not a gate.
- Why: owner answered the four gating questions (the stated Draft→Active condition) and said "active now." q1 materially improved the design — live-from-first-byte removes the fragile size threshold and collapses the A/B/C probe staging into "adjust what is cheap in `ue-1b`, add stream resize in `ue-2`."
- Inference flagged for owner (now vetoed — see D-2026-06-20-3): the agent had proposed folding the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b) in as the planner half and superseding its "after audit Round 1" timing. **Owner vetoed 2026-06-20.** The absorption is dropped; D-2026-06-04-3 stands unchanged. The engine's workload-shape-awareness + first-byte-within-~1s requirements remain, stated on their own merits, not as the H10b concept.
- Supersedes: the "A-first warmup probe" and "size-gated skip-probe" framings in the Draft version of `UNIFIED_TRANSFER_ENGINE.md` (already edited in-place). *(The proposed supersession of D-2026-06-04-3's streaming-planner timing is withdrawn per the owner veto — see D-2026-06-20-3.)*

## D-2026-06-20-3 — Veto: do NOT fold the streaming planner (H10b) into the unified engine
- Decision: The flagged inference in D-2026-06-20-2 is **vetoed by the owner.** The unified engine does **not** absorb the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b), and D-2026-06-04-3's "after audit Round 1" sequencing **stands unchanged** — the convergence plan does not supersede it. What survives from the vetoed inference: the engine's planner is **workload-shape-aware** (file count vs bytes; 100k×10B ≠ 1×20MB) and must meet the **first-byte-within-~1s** commitment by yielding an initial plan from a partial scan and refining. That is an engine-internal requirement stated on its own merits, **not** the H10b streaming-planner concept and **not** a supersession of D-2026-06-04-3. Whether the engine's fast-start enumeration and the separate H10b streaming planner overlap is left to the owner at audit Round 1, not pre-resolved here.
- Why: owner did not intend to revive H10b by way of the convergence plan; the inference was the agent's, flagged for confirmation, and the owner declined it. The workload-shape-awareness goal was always standalone and stands.
- Supersedes: nothing. Reverts the conditional H10b supersession that D-2026-06-20-2 had proposed (that entry is edited in-place to drop the inference and point here).

## D-2026-06-20-4 — Unified transfer engine plan review freeze
- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md` is a Draft review candidate next to the original plan, and all unified-transfer-engine coding is frozen until the owner makes a final plan decision.
- Why: review found the Active plan's direction is sound but several slices need tightening before code starts: streaming initial planning was hidden inside `ue-1c`, local fast paths need to become engine-owned strategies, work-stealing is observable behavior, wire compatibility needs concrete shape, and pull parity gates must wait for multistream pull.
- Supersedes: D-2026-06-20-2 only as an implementation greenlight; it does not supersede the convergence direction or the owner's four bound parameters.
