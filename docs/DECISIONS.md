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
- Decision: The 2026-06-14 "redesign the transfer subsystem from the ground up" framing is resolved as **convergence**, not a rebuild. One src/dst-agnostic sequencer owns all four paths (local↔local, push, pull, daemon↔daemon); the dial (stream count + all transfer knobs) is a single live object adjusted from measured telemetry; the already-shared byte-moving leaf stays. Dials are **bounded-unilateral** (receiver advertises a capacity ceiling; sender owns the dial within it) ~~and **size-gated** (small transfers skip the probe entirely)~~ **(size-gate framing superseded by D-2026-06-20-2 q1 — there is no probe phase to skip; the engine moves within ~1s and tunes live)**. The adaptive-streams stack (PR1 telemetry + PR2 work-stealing queue, up to `eafb187`) is salvaged as the substrate per D-2026-06-07-2; PR3 WIP (`d9d4ec7`) stays excluded. ~~Built A-first (warmup), C-ready by construction (mutable dial + elastic stream-set exist from A, so continuous adjustment is a later feed, not a retrofit).~~ **(A/warmup staging superseded by D-2026-06-20-2 q1 — conservative start + live tuning from the first byte; C shipped as `ue-r2-2` under REV4/D-2026-06-20-5.)** Plan: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (Draft — awaiting owner Draft→Active flip). *(Stale wording struck 2026-07-04 on owner direction — "follow the existing pattern": the in-place-annotation pattern of D-2026-06-20-3/-6. The convergence direction itself stands unchanged.)*
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

## D-2026-06-20-5 — REV4 replaces UNIFIED_TRANSFER_ENGINE.md as the Active convergence plan
- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md` is the **Active** unified-transfer-engine plan (owner: "rev4 replaces v1"). `UNIFIED_TRANSFER_ENGINE.md` (v1) flips Active → Superseded; the intermediate review candidates `REV2.md` and `REV3.md` flip Draft → Superseded — all three superseded by REV4. REV4 carries v1's lineage/absorption header forward, so the supersessions v1 recorded (MULTISTREAM_PULL absorbed as the pull-multistream slice `ue-r2-1g`; PIPELINE_UNIFICATION/UNIFIED_RECEIVE_PIPELINE Historical) remain in force. The plan-review freeze (D-2026-06-20-4) is lifted as to the **plan decision**; coding still requires a fresh per-slice owner authorization (AGENTS.md §9) — no slice (`ue-r2-1a` first) starts on this decision alone.
- Why: REV4 is the only candidate whose code-reality section was verified against the tree (`HEAD` `09268eb`). REV3's headline "two static tables, not three" correction was itself wrong — all three stream-count ladders are live (`remote/tuning.rs::determine_remote_tuning`, `push/control.rs::desired_streams:476`, `pull.rs::pull_stream_count:904`), v1's three-ladder count was substantially right, and `tuning.rs`'s own doc comment confirms the daemon "runs its own ladder and wins". REV3 also wrongly said `determine_remote_tuning` drives local (it drives push + daemon pull) and conflated single-stream PullSync with the already-multistream deprecated Pull. REV4 = REV3 + corrected code reality, every symbol grounded with `file:line`, v1 lineage preserved. One Active plan avoids drift between candidates.
- Supersedes: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (v1, Active → Superseded) and the review candidates `REV2.md` / `REV3.md` (Draft → Superseded) — all by `REV4.md`. Lifts D-2026-06-20-4's implementation freeze (the plan decision is now made). Does **not** supersede the convergence direction (D-2026-06-20-1), the four bound parameters (D-2026-06-20-2), or the H10b veto (D-2026-06-20-3). ~~The D-2026-06-20-1 warmup/size-gate cleanup remains an open owner question, untouched here.~~ *(Resolved 2026-07-04 — cleanup applied in place; see the edited D-2026-06-20-1.)*

## D-2026-06-20-6 — Code→GPT-review→fix loop for the unified engine; ungated per-slice commits
- Decision: Adopt a synchronous code→review→fix loop for the `ue-r2-*` slices (`docs/agent/GPT_REVIEW_LOOP.md`, Active). Claude codes + commits each slice, invokes GPT-5.5 via `codex` (headless here via the local `headroom` proxy) to review that commit, adjudicates every finding against source/tests, fixes the accepted ones, and proceeds. Three standing authorizations the owner gave this session: (a) **per-slice commits to `master` are ungated** for this loop — no agent branches, never push (push stays owner-only); (b) **per-slice code-quality acceptance is delegated** to the loop + validation suite — the owner is not a developer and will NOT be asked to bless code that passed validation+review ("that would just be theater"); (c) the agent proceeds autonomously and pauses only for genuine decisions/issues/blockers/plan-changes and the remaining owner gates (push; 10 GbE sign-off).
- Why: the owner wants forward progress without rubber-stamp checkpoints. An external reviewer (GPT-5.5) catches what a single author misses, while Claude's adjudication guards against the reviewer's false positives — demonstrated necessary the same day: a codex-class review's confident "two static tables, not three" claim was wrong (all three ladders are live). Commits are low-risk and reversible (nothing publishes until the owner pushes), so per-commit gating was pure friction.
- Supersedes: nothing. ~~Scopes `.review/` usage for `ue-r2-*` only~~ **(scope clause superseded by D-2026-07-04-1 — the loop is now repo-wide for all code and plan changes)** — the async sentinel (`ready/`) + `reviewer-wait.sh` hand-off is not used (records `findings/` + `results/` are reused). Records the owner's explicit relaxation of the §9 per-slice-code checkpoint (code acceptance delegated to this loop); the §8 push gate and all other §9 owner gates stand.

## D-2026-07-04-1 — Codex review loop for ALL code and plan changes; async sentinel loop retired
- Decision: The synchronous code→codex-review→fix loop (`docs/agent/GPT_REVIEW_LOOP.md`) now governs **every code change and every plan change** in this repo — owner, 2026-07-04: "use codex review loop for all code and plan changes", "NO EXCEPTIONS". The `.review/README.md` async two-agent hand-off (`ready/` sentinels + `reviewer-wait.sh` + a separate reviewer agent) is retired as the grading mechanism for new work; its record formats (`.review/findings/`, `.review/results/`, the `REVIEW.md` status index) remain in use by the codex loop. Reviewer identity on verdicts: `gpt-5.5` (codex), adjudicated by the coding agent per the loop's adjudication step. For docs/plan-only changes the validation gate is `bash scripts/agent/check-docs.sh` (the cargo suite is not required, per `.agents/repo-guidance.md` Verification); the review step still runs.
- Why: the codex loop demonstrably catches real defects (every `ue-r2-*` slice) while the async reviewer role sat structurally unfilled — w4-1 landed 2026-07-04 and immediately stalled at "awaiting reviewer verdict" with no reviewer in existence; a review mechanism that actually runs beats one that waits for an agent nobody spawns.
- Supersedes: the scope clause of D-2026-06-20-6 ("Scopes `.review/` usage for `ue-r2-*` only" — the loop is now repo-wide; D-2026-06-20-6's standing authorizations (a)/(b)/(c) carry over unchanged to the widened scope). Also supersedes `.review/README.md`'s sentinel/reviewer-wake sections and `docs/agent/PROTOCOL.md` `slice` step 2's sentinel requirement (both edited in place, annotated).

## D-2026-07-04-2 — Keep the `9f37a7a`/`48c5a11` staging-slip commits; no history rewrite
- Decision: The two Windows-session commits that don't build in isolation (`9f37a7a` clippy baseline carrying a stray `pull.rs` deletion, `48c5a11` win-1) stay on `master` as pushed; no rebase, no force-push. `git bisect` runs must skip them (both are documented in the ue-r2-1h finding doc and DEVLOG). This closes the erratum question opened 2026-07-04.
- Why: owner call 2026-07-04 ("leave as-is"). HEAD is fully gated and every later commit builds; the only cost is two skippable commits in bisect. Rewriting already-pushed shared history is the riskier operation — same calculus as D-2026-06-07-1, which is this repo's precedent for keeping a pushed wart over a second unsafe git operation.
- Supersedes: nothing (closes the STATE.md "commit erratum" blocked item).

## D-2026-07-04-3 — Flip `supports_cancellation` for Push/PullSync: CancelJob works on attached transfers
- Decision: The `CancelJob` dispatch policy stops refusing attached Push/PullSync jobs. After the flip, `blit jobs cancel` (and the TUI F2 cancel) fires the row's cancel token for those kinds and the handlers — which race that token since w4-3 — tear down cleanly; the CLI contract changes from exit 2 / `FailedPrecondition` ("unsupported") to exit 0 on success, and the TUI's Unsupported surface for these kinds disappears. Implementation is a queued review-loop slice (`w4-5-supports-cancellation-flip` in REVIEW.md) through the codex loop, with tests pinning the new contract.
- Why: owner call 2026-07-04 ("flip it"). The original "disconnect is the cancel" rationale predates w4-3's race wiring; the flip is now policy-only, and cancel-from-anywhere (second terminal, TUI) is strictly more operable than find-and-kill-the-client.
- Supersedes: the DelegatedPull-only cancellation policy recorded in `active_jobs.rs`'s `supports_cancellation` rustdoc (edited when the slice lands) and the corresponding "policy deliberately unchanged" scope note in the w4-3 finding doc (which anticipated exactly this flip).

## D-2026-07-04-4 — SMALL_FILE_CEILING.md flipped Draft → Active
- Decision: `docs/plan/SMALL_FILE_CEILING.md` is **Active** (owner "go", 2026-07-04). sf-1 (tripwire harness) starts now; the in-plan gates stand unchanged — sf-6's wire-design owner sign-off before any code, and the sf-4/sf-7 acceptance reviews with the owner.
- Why: the codex plan review is complete (5/5 accepted + fixed, records `219cecf`) and the plan binds the measured small-file/mixed ceiling gaps (`docs/bench/10gbe-2026-07-05/`) to the owner's ceiling-driven principle. The other four 10 GbE gate declarations (ue-1, ue-2, zero-copy a/b/c, REV4 → Shipped) were NOT part of this go and stay in STATE.md Blocked.
- Supersedes: nothing (the plan's "(pending owner approval)" decision ref now points here).

## D-2026-07-05-1 — One transfer path; direction-invariance by construction; SMALL_FILE_CEILING paused
- Decision: All byte transfer in blit must flow through ONE `TransferSession` implementation — direction, initiator, and CLI verb select *roles* (SOURCE/DESTINATION), never code. The per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the `Push`/`PullSync` RPCs are deleted when the migration completes — owner, 2026-07-05: "ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did." Benchmark methodology corollary: cross-direction performance comparisons are valid only on symmetric endpoints ("tmp on one side, spinning rust on the other is not a valid test"); tmpfs cells are wire-reference only. Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Draft; no code until the owner flips it Active). `docs/plan/SMALL_FILE_CEILING.md` is **paused** effective immediately at sf-2 (done): sf-3a and later slices are blocked until ONE_TRANSFER_PATH ships, then resume/re-derive against the unified baseline (owner delegated this sequencing: "I DO NOT CARE. FIX IT.").
- Why: the measured push/pull disparity recurred because direction symmetry was discipline spread across four driver loops, not structure — the sf-2 stream-count bug existed only in the push driver, the slow-start defect only in the pull driver. Deleting the alternatives is the only arrangement in which the owner's invariant cannot regress.
- Supersedes: the post-REV4 residue item "pull 1s-start restructuring" (STATE Queue item 4 — absorbed by ONE_TRANSFER_PATH's streaming-manifest choreography); SMALL_FILE_CEILING's queue position (paused, not superseded — its principle D-2026-07-04-4 stands); ~~and, effective only at ONE_TRANSFER_PATH's cutover slice (otp-10), REV4 §Constraints' "mixed old/new peers must negotiate down" rule (annotated in place; until that slice lands the rule governs)~~ **(the "only at cutover" scoping is superseded by D-2026-07-05-2 — no version compatibility, ever, effective immediately)**. The bounded-unilateral dial contract (D-2026-06-20-1/-2) is NOT superseded — it carries into the unified session unchanged.

## D-2026-07-05-2 — No version compatibility, ever: same-build peers only
- Decision: Blit has NO version-compatibility obligation of any kind, in any direction, at any time — owner standing rule, restated with force 2026-07-05: "backward compatibility is NOT a consideration. I expect blit 1.2.3 not to be able to talk to blit-daemon 1.2.3.1. period. same build only. do not engineer tech debt into an unshipped product." Client and daemon interoperate only when built from the same source; the wire handshake must REFUSE a mismatched peer outright at session open (exact protocol/build identity — mechanism specified in ONE_TRANSFER_PATH otp-1 and pinned by test). Feature-capability bits that exist to tolerate version skew ("advisory until both peers advertise support", `supports_stream_resize`-style flags) are dead weight and go away with the unified session. NOT affected: the receiver capacity profile (runtime capacity of the receiving machine, D-2026-06-20-1/-2) — that is hardware negotiation, not version negotiation.
- Why: REV4 §Constraints carried a written "mixed old/new peers must negotiate down" rule while the owner's contrary rule lived only in chat; the ONE_TRANSFER_PATH plan review then resolved the document conflict in favor of the written rule ("governs until cutover"). Wrong direction — recording the owner's rule as a decision ends the unrecorded-intent-loses-to-stale-paper failure mode.
- Supersedes: REV4 §Constraints mixed-version clause (annotated in place, effective immediately — not at cutover); SMALL_FILE_CEILING §Constraints "mixed-version peers keep working via existing negotiation" clause and sf-6's mixed-version-test deliverable (annotated); the "effective only at ONE_TRANSFER_PATH's cutover slice" scoping inside D-2026-07-05-1's Supersedes line (the supersession is immediate and total); ONE_TRANSFER_PATH's Non-goals compat wording (rewritten same commit).

## D-2026-07-05-3 — Zero-copy receive unparked: revisit gate declared met (UNAS rig)
- Decision: The D-2026-06-12-1 revisit gate ("receive-side CPU saturation") is **declared met by the owner** (2026-07-05): a UniFi UNAS 8 Pro daemon target whose CPU cannot saturate 10 GbE even from SSD cache. Zero-copy receive is unparked as sanctioned FAST work. Two clarifications: (a) the dead `zero_copy.rs` module still gets deleted as ratified — its EAGAIN busy-wait draft is a rewrite, not a revival (eval doc); (b) the capability returns the one-path way (owner exchange 2026-07-05): a **runtime-selected write strategy inside the unified receive sink** — the eval doc's revisit design (`AsyncFd`-readiness splice loop beside the buffered relay, selected when the reader is a raw TcpStream and the payload is a file record, buffered relay as universal fallback), capability-gated by kernel/fs support, identical in both roles — never a side path. Sequenced after ONE_TRANSFER_PATH's cutover (otp-10) as its own slice set; the UNAS is the measurement rig and the symmetric-endpoint benchmark rule (D-2026-07-05-2 era methodology) applies to its cells.
- Why: the 10 GbE session showed skippy's 32-core receiver at 1.43 cores — gate not met on that rig — but the gate was always about CPU-bound receivers, and the owner now operates one. On a CPU-bound receiver, cutting the userspace copy is exactly the FAST lever the eval preserved design notes for.
- Supersedes: the STATE Blocked "zero-copy option a/b/c" question and the "zero-copy revisit verdict" item among the four 10 GbE owner declarations (both resolved by this entry); amends D-2026-06-12-1's revisit-gate framing from "10 GbE benchmarks showing receive-side CPU saturation" to "a CPU-bound receiver exists" (annotated in the eval doc). D-2026-06-12-1's deletion of the dead module stands.

## D-2026-07-05-4 — ONE_TRANSFER_PATH flipped Draft → Active
- Decision: `docs/plan/ONE_TRANSFER_PATH.md` is **Active** (owner: "flip the plan and go", 2026-07-05). Slice execution starts at otp-1 (wire+session contract, doc + proto, no behavior). The owner re-affirmed the per-slice codex review loop in the same message ("reviewloop codex for each slice") — already binding via D-2026-07-04-1; recorded here as an explicit re-affirmation. All in-plan gates stand: converge-up baseline pins (otp-2), deletion proof + DelegatedPull no-payload-bytes assertion (otp-10), symmetric-rig acceptance (otp-12), owner checklist walk (otp-13). Standing constraints in force: D-2026-07-05-2 (same-build only), zoey activity restricted to the blit-temp test folder with the zero-copy test pre-authorized there (STATE queue item 5).
- Why: the codex plan review completed (5 findings accepted + fixed, `496357d`); D-2026-07-05-2/-3 propagated; the owner's flip is the approval the plan procedure requires.
- Supersedes: nothing (the plan's "Active flip gets its own entry" placeholder now points here).

## D-2026-07-09-1 — OTP7_RESUME flipped Draft → Active (Q1–Q3 settled)
- Decision: `docs/plan/OTP7_RESUME.md` is **Active** (owner, 2026-07-09). The three open questions are settled by the owner's principle — "FAST, SIMPLE, RELIABLE file transfer. if we abort the whole thing when we could have fixed or surfaced a single error, we are violating all of those." — plus an explicit "confirmed. no collapse.": **Q1** stale/mismatched partial ⇒ graceful full-file fallback (contract wins over the old data-plane hard error, D1 as drafted). **Q2** in-place patch stays (no temp+rename atomicity, parity with the code being replaced), with an owner rider: a mid-resume fault must appear in the CLI's **end-of-operation summary**, naming the file(s) and suggesting a re-run to converge — not only as a scrolling mid-stream line; this small CLI deliverable lands within otp-7 (plan D4). No atomicity follow-up filed — convergence-on-retry is the reliability model. **Q3** staging is 7a (in-stream) then 7b (data plane), one slice per codex loop pass ("keep the reviewloop codex playbook going slice by slice").
- Why: owner answered Q1–Q3 in session 2026-07-09; the flip is the approval the plan procedure requires. In the same exchange the owner re-confirmed the broader progress-display redesign (persistent stats block + scrolling file frame, "probably a TUI") — that stays a queued TODO.md item ("CLI transfer output redesign"), NOT otp-7 scope, and needs its own plan.
- Supersedes: nothing (the plan doc's Open-questions section is rewritten as resolved in the same commit).

## D-2026-07-10-1 — Resume wire bounds on the in-stream carrier (amends OTP7_RESUME D5)
- Decision: The session's resume block phase is bounded so no legal open can produce a frame the gRPC-served in-stream carrier cannot deliver, nor an amplified hash list (codex otp-7a F1). The DESTINATION clamps `ResumeSettings.block_size` into **[64 KiB, 2 MiB]** (`MIN_RESUME_BLOCK_SIZE`, `MAX_IN_STREAM_RESUME_BLOCK_SIZE`; `0` ⇒ 1 MiB default) — floor kills block_size=1's 32× hash-list amplification, ceiling keeps a one-block `BlockTransfer` frame under tonic's default 4 MiB decode limit — and caps any one `BlockHashList` at **65_536 hashes** (2 MiB of hashes); a partial with more blocks degrades to the empty list, i.e. the plan-D1 graceful full-transfer fallback, never an oversized frame. The SOURCE range-validates the wire block size at frame arrival (same-build peers, D-2026-07-05-2: out-of-range is a protocol violation, not a negotiation). otp-7b revisits the ceiling for the TCP data plane, whose binary block records carry no protobuf envelope.
- Why: plan D5 as drafted clamped only to `MAX_BLOCK_SIZE` (64 MiB), which is fine for local block copies but 16× over the unraised tonic frame limit the served in-stream carrier actually has — a legal open would fail mid-transfer (RELIABLE violation), and a hostile-or-buggy tiny block size would OOM-amplify the hash list. Pinned by `resume_block_size_floor_clamps_tiny_requests`, `resume_block_size_ceiling_clamps_oversized_requests` (guard-proven by clamp removal), and the pure-fn cap boundary test.
- Supersedes: OTP7_RESUME.md D5's "clamped to `MAX_BLOCK_SIZE`" wording (amended in place, same commit).

## D-2026-07-10-2 — Resume block-size ceiling is per carrier (completes the D-2026-07-10-1 revisit)
- Decision: The resume block-size ceiling the DESTINATION clamps to (and the SOURCE range-validates at `BlockHashList` arrival) is **the carrier's**: **2 MiB** on the in-stream carrier (unchanged, D-2026-07-10-1) and **64 MiB** on the TCP data plane (`MAX_DATA_PLANE_RESUME_BLOCK_SIZE` = the receive pipeline's `MAX_WIRE_BLOCK_BYTES` = the old resume path's `MAX_BLOCK_SIZE`). Both ends decide by grant presence — grant ⇒ data plane — so same-build peers agree without negotiation. The floor (64 KiB) and the 65_536-hash `BlockHashList` cap are carrier-independent (the hash list always rides the control lane as protobuf); a partial with more blocks than the cap still degrades to the D1 full-transfer fallback. Session-wide block size stays; per-file block-size auto-scaling for very large partials (>4 TiB at 64 MiB blocks) remains future work.
- Why: binary data-plane `BLOCK` records carry no protobuf envelope, so the 2 MiB tonic-frame rationale does not apply there; the wire already enforces `MAX_WIRE_BLOCK_BYTES` = 64 MiB on the receive side. A larger ceiling lets a data-plane session keep block-wise resume for partials up to 4 TiB (65_536 × 64 MiB) instead of degrading to full transfer at 128 GiB (the 2 MiB-ceiling limit).
- Supersedes: nothing — completes the revisit D-2026-07-10-1 explicitly deferred to otp-7b (OTP7_RESUME.md D5 amended in place, same commit).

## D-2026-07-11-1 — `--relay-via-cli` removed; remote→remote is delegated-only
- Decision: The `--relay-via-cli` escape hatch is **removed** (owner, 2026-07-11, otp-10c-1). Remote→remote transfers are delegated-only; the CLI is never in the byte path. The relay's read half was the PullSync client's on-demand per-file remote read — a capability the unified session deliberately does not have — so PullSync's deletion (otp-10c) makes a streaming relay unrebuildable; offered the choice, the owner picked removal over a stage-to-temp-dir reimplementation. The topology the flag served (destination cannot reach source, CLI can reach both) is handled by two manual commands — pull to a local path, then push it — and the delegated CONNECT_SOURCE error hint now says exactly that. `RemoteTransferSource`, its `remote_transfer_source_constructed` counter, and every relay-combination gate (mirror/move/detach/resume × relay) die with the flag; the delegated no-CLI-byte-path pin (`cli_data_plane_outbound_bytes == 0`) remains the byte-path-isolation proof and doubles as the otp-10 deletion proof's CLI half.
- Why: unshipped product (no compat bar, D-2026-07-05-2 era posture); the ONE_TRANSFER_PATH directive is deletion of bespoke side paths, and a staged relay would merely automate what two commands already do — not worth a maintained transfer-adjacent code path.
- Supersedes: `REMOTE_REMOTE_DELEGATION_PLAN.md` (already Historical) §§relay-fallback/escape-hatch design — dated header note added, body kept verbatim per that doc's own precedent; the relay-combination gates R50-F1/R51-F2 (move×relay), audit-h1 rounds 1–2 (mirror×relay), codex otp-10a F4 (resume×relay), and the detach×relay gate — deleted with the flag they guarded (their data-loss reasoning is moot once no relay path exists to combine with); `scripts/bench_remote_remote.sh`'s relay leg (removed same commit).

## D-2026-07-12-1 — otp-12 cross-direction bar: platform-residue cells count as satisfied
- Decision: On the owner-designated cross-direction rig (Mac↔Windows), an otp-12 cell that (a) meets per-direction converge-up (new ≤ old, same direction, ±10%) and (b) is initiator/verb-invariant within ±10%, but (c) exceeds `min(old_push, old_pull) × 1.10` with the platform-residue discriminator attributing the gap to the destination write path (the old arm shows the same direction gap in the same interleaved session), **counts as satisfying the cross-direction half of ONE_TRANSFER_PATH acceptance criterion 2** (owner "yes", 2026-07-12, to the plain-English framing: "if a direction is exactly as fast as the old code, and the time is identical no matter which machine starts the copy, but it still misses that bar purely because of the Windows write path — does it count as a pass?"). The evidence README still records BOTH computations per cell (the F4 arithmetic and the discriminator); the otp-13 walk reviews the numbers, but a platform-residue cell is not a blocker.
- Why: the plan's Non-goals already exclude making different hardware perform identically, and D-2026-07-05-1 restricts cross-direction verdicts to symmetric endpoints; no truly fs-identical pair exists in the fleet, so on the designated closest-spec rig the "better of the two old directions" bar can only bind net of the destination write-path residue the discriminator isolates. Settling the rule before the run prevents re-litigating it with numbers in hand.
- Supersedes: nothing — refines how acceptance criterion 2 is evaluated on the designated rig (`docs/plan/ONE_TRANSFER_PATH.md` criterion annotated in place, same commit); resolves `docs/plan/OTP12_ACCEPTANCE_RUN.md` Q1 (rewritten as resolved, same commit).

## D-2026-07-13-1 — OTP12_PERF_FINDINGS goes Active after one final codex round; implementation proceeds slice-by-slice
- Decision: `docs/plan/OTP12_PERF_FINDINGS.md` flips **Draft → Active** after ONE final codex round, and implementation then proceeds regardless of whether that round returns a "converged" verdict — owner, 2026-07-13, verbatim: **"one more round with codex on the plan then just write the code and reviewloop slice by slice. that converges faster than plans with no ground truth to test."** Each code slice still goes through the codex review loop (D-2026-07-04-1, unchanged); what is retired is *plan-only* iteration as the gate on starting work. The plan's own Status line ("the flip to Active happens at codex convergence") is amended by this decision: the round happens, its accepted findings are fixed, and then code starts — a non-converged verdict is no longer a blocker, it is input to the first slice.
- Why: rounds 2–4 each returned real findings, but they were increasingly findings about the *plan text* (falsifiability wording, thresholds, bar phrasing) rather than about reality, and the plan's central factual claim was settled not by review but by *measurement* — the same-OS rig, which refuted a claim four review rounds had left standing (`docs/bench/otp12-perf-2026-07-13/`; a wrong "P1 is code" claim was reported and retracted the same day). Ground truth comes from instrumented code and rigs, not from more prose; pf-1 exists precisely to generate it. Continuing to polish the plan has diminishing returns against the cost of not yet having a single measured counterfactual.
- Supersedes: the "flip to Active at codex convergence" gate in `OTP12_PERF_FINDINGS.md`'s Status line (rewritten in place, same commit). Does NOT supersede D-2026-07-04-1 — every code slice is still codex-reviewed before the next begins.

## D-2026-07-13-2 — the local small-file finding queues BEHIND OTP12_PERF_FINDINGS
- Decision: `docs/plan/LOCAL_SMALL_FILE_PATH.md` (Draft) is sequenced **behind** the ACTIVE `docs/plan/OTP12_PERF_FINDINGS.md` — the MTU experiment, then pf-1, then its fix slices. Owner, 2026-07-13, verbatim: **"well, odds that one affects the other? if this is contributory, would we know? probably irrelevant. behind."** No local-path code lands until otp-12's investigation has its attribution. The finding itself (blit vs robocopy, local `D: -> E:`, `docs/bench/win-local-ab-2026-07-13/`) is recorded now; only the *fix* waits.
- Why: two reasons, one causal and one procedural. **Causal**: the local finding is very unlikely to explain either otp-12 finding. P1 is an *initiator-invariance* failure — both arms run identical code and differ only in who dials, so a worker-count or per-file cost cancels between them, and a local copy has no initiator axis at all. P2 is a *new-vs-old* regression, whereas the local cost is *old*: otp-11's own gate measured old-vs-new local `small` at 1684 -> 1750 ms (+3.9% PASS, `docs/bench/otp11-local-2026-07-11/`) and otp-11 D1 explicitly preserved the old pipeline's payload shapes (`PreparedPayload::File`/`TarShard` "exactly as the old local pipeline"). A long-standing cost cannot produce a new regression. **Procedural**: fixing local *first* would touch code shared with the wire sink, perturb P1/P2 mid-investigation, and void the pre-fix baselines pf-final depends on — destroying the attribution rather than adding to it. Sequencing behind keeps every counterfactual legible, and pf-final's full-matrix rerun would still surface any shared-code effect as a number.
- Carried into pf-1 as a cheap check (the one way the two could touch): the local apply pipeline runs **one** worker by default (`transfer_session/local.rs:602`, `sink_workers` is 1 unless the hidden `--workers` flag sets `debug_mode`). If the unified session likewise changed the **remote receive** side's worker count versus old push, that WOULD be new, per-file, and a live P2 candidate. Establish it by reading the executed old path, not by assuming.
- Supersedes: nothing. Adds `LOCAL_SMALL_FILE_PATH.md` to the `docs/STATE.md` queue behind item 1a.

## D-2026-07-13-3 — Windows attribute/ADS loss is a real gap; fix it AFTER otp-12
- Decision: `blit` silently drops Windows file attributes (ReadOnly/Hidden/System) and alternate data streams on the tar-shard path — **on both the local and the remote route**, exit code 0, no warning — and it will be **fixed after the current phase (otp-12) completes**, not now. Owner, 2026-07-13, verbatim: **"well that, while funny, makes sense. we started this as a linux alternative to robocopy, and full windows support was always a goal... but obviously not landed. so, good, let's address that. after this current phase is complete."** Finding, repro, and root cause: `docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md`.
- Framing (owner's, and it is the correct one): this is **unlanded Windows support**, NOT a regression. blit began as a Linux alternative to robocopy; full Windows parity was always a goal and the metadata half never shipped. It predates the unified session and is not P1, P2, or otp-11 fallout.
- What makes it more than a missing feature: the loss is **conditional on file count**, so it is silent and non-obvious. `transfer_plan.rs:103-109` sends a transfer down the tar path when there are ≥2 small files AND (≥32 of them OR average ≤128 KiB); otherwise files go through `CopyFileExW`, which carries attributes and ADS for free. So the SAME file keeps its metadata when copied alone and loses it when copied alongside 39 siblings. Proven with identical 200 KiB files where only the count varied (40 → LOST, 3 → PRESERVED), locally and over the wire.
- **Fixing it is a WIRE CONTRACT change.** The tar shard is the wire payload format for small files, so carrying attributes/ADS means extending the shard header or the manifest — a frame change, which trips the stop-and-amend rule: `docs/TRANSFER_SESSION.md` is amended through the codex loop BEFORE any code. Same-build-both-ends (D-2026-07-05-2) means no compatibility surface is created, but the contract doc still governs. The header-vs-manifest choice is a design decision reserved for the owner.
- Sequencing: behind otp-12, and **planned together with `LOCAL_SMALL_FILE_PATH.md`** (D-2026-07-13-2) — they touch the same tar path and pull in opposite directions (a fidelity fix ADDS per-file work to a path already losing 1.9× to robocopy at equal thread count). Planning them separately would optimise one against the other.
- Not in scope / not a bug: **empty directories**. Their absence is a documented design position — `blit check`'s help (`crates/blit-cli/src/cli.rs:20-35`) states the equivalence model skips empty directories and points at `diff -r` for full tree equivalence. blit models files, not directories. (`test_push_empty_directory` only asserts the command succeeds; it never checks the directory arrived — a crash smoke test, not a fidelity test.) **ACLs** are likewise out: robocopy does not copy them either without `/COPY:S`.
- Supersedes: nothing. Adds `docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md` to the `docs/STATE.md` queue behind otp-12, alongside D-2026-07-13-2.

## D-2026-07-14-1 — the committed baselines are RE-RECORDED at MTU 9000 (amends OTP12_ACCEPTANCE_RUN D5's pin, not its freeze)
- Decision: the frozen committed baselines that `pf-final` grades against are **re-recorded with their OLD builds at MTU 9000**, so acceptance compares old and new like-for-like on the fabric the fleet actually runs. Owner, 2026-07-14, choosing between three presented options, verbatim: **"Re-record the baseline at 9000"**. The 2026-07-10 baselines are **retained as historical MTU-1500 records** — superseded as the acceptance reference, never deleted or rewritten.
- **REVISED 2026-07-14 after codex review (`.review/results/pf-0-rebaseline-decision.*`; NOT READY, 6 findings, 6/6 accepted).** The owner's choice stands unchanged; the first draft of this entry was **not executable** and its rationale was over-applied. Corrections are folded in below and marked. The revision does not reopen the decision — it makes it performable.
- Why (**corrected — the first draft over-applied pf-0**): pf-0's "3–4% faster at jumbo" is measured on **one cell (`wm_tcp_large`), one rig (W), and both arms of the NEW build** — it is **not** a measured old-vs-new leniency, and pf-0 measured **no** small cells, **no** rig-Z cells, and **no** OLD-build MTU response at all (its own committed-reference rows were VOID at jumbo). So the justification is **not** "the ceiling is loose by 3–4%" — that number cannot be generalized across cells or rigs. It is **methodological**: *an acceptance reference and the sessions graded against it must share the MTU of the fabric under test.* pf-0 proves the mismatch is real and that MTU moves wall time on at least one cell; that alone makes a mismatched ceiling unsound in an unknown direction. The **known** direction, where measured, is lenient — which is the wrong error for a bar guarding the one class of finding (P1/P2) between blit and shipping.
- Scope — **BOTH rigs, not just rig W.** Each harness hardcodes its own committed reference, and both predate the 2026-07-13 fabric-wide jumbo raise: rig W `scripts/bench_otp12_win.sh:105` → `docs/bench/otp2w-baseline-2026-07-10/`; rig Z `scripts/bench_otp12_zoey.sh:102` → `docs/bench/otp2-baseline-2026-07-10/`. **Verified, not assumed** (2026-07-14): netwatch-01 "ran at MTU 1500 for EVERY benchmark ever recorded" (`.agents/machines.md`), and zoey's pre-jumbo `systemd-networkd` configs — backed up as `*.premtu`, dated 2026-04-30 — carry **no `MTUBytes` stanza**, i.e. the default 1500; the 9000 configs were written 2026-07-13. Rig D (delegated) has **no** old baseline and is unaffected.
- **THE NON-LOOSENING GUARD (added on review — without it this decision breaks the very control it amends).** `OTP12_ACCEPTANCE_RUN.md` D2 exists precisely so that *"the fixed pre-cutover bar must not be loosened by a slower old rerun"* (its design finding F2). A re-record re-rolls **hardware, OS/disk state and day** as well as MTU — rig W's Mac end is now `q`, not the nagatha that recorded 2026-07-10 — so an unguarded re-record could **loosen** the bar, which is exactly what F2 forbids. Therefore, applying F2 (not inventing a new rule): **the acceptance reference for each cell is the per-cell MINIMUM of {the 2026-07-10 committed median, the re-recorded 9000 median}.** It can only tighten, never loosen. Any cell whose re-record is **slower** than 2026-07-10 is **flagged for investigation, never silently adopted** — the old build getting slower on faster hardware would mean the rig or the method drifted, and that must be explained before any acceptance run is graded.
- Implementation constraints (for the re-baseline slice, which goes through the codex loop like any code change):
  * **Rig W** re-records on `0f922de` (its original old build), provenance manifest-verified.
  * **Rig Z has NO clean "original old build" to reuse** (caught on review): the otp-2 baseline's *client* was a clean `e757dcc` but the *daemon* it actually ran was a **dirty** `731023b` build — which D1/D6's clean-matched-pair discipline forbids reusing. Resolution: rig Z re-records on a **CLEAN `e757dcc` pair**, which is sound because `git diff 731023b e757dcc -- crates proto Cargo.toml Cargo.lock` is **empty** (the committed daemon code is identical — otp-2 README correction), and because otp-12a **already** staged a clean `e757dcc` rebuild for its old arm, so this is precedent, not a new reference build.
  * `BASELINE_SUMMARY` is hardcoded **by design** (no override) so a run cannot quietly re-point its own ceiling. Re-pointing it is therefore a reviewed source edit, not an env var — and the new value must be a **committed** dated dir.
  * The MSS gate that pf-0 used (record MSS at session start AND end; VOID the session if it is not the expected value at both) applies to the re-baseline sessions: a baseline recorded at an unverified MTU is exactly the defect being fixed.
- Supersedes: the *pin* in `OTP12_ACCEPTANCE_RUN.md` **D2 and D5** (both name the committed **2026-07-10** median; D2 was missed in the first draft, leaving the two sections contradicting each other — caught on review). The **freeze principle stands**: a baseline is immutable once recorded, no run may re-point its own reference, and **the bar can never be loosened** (the guard above). What changes is only *which* frozen record the harness grades against, once. The 2026-07-10 baselines are **retained, unmodified, as historical MTU-1500 records** and their READMEs are re-labelled accordingly. Closes the OPEN item raised in `OTP12_PERF_FINDINGS.md` §pf-0.

## D-2026-07-14-2 — a SECOND reviewer (grok) may be added to the loop for hard calls; codex remains the default
- Decision: the review loop may run a **second, independent model (`grok`)** alongside codex on high-stakes slices. Owner, 2026-07-14, verbatim: **"Reviewloop grok for another opinion"**. Codex remains the **default and mandatory** reviewer; grok is **additive, never a substitute**, and never runs alone.
- Why the original rule said otherwise, and why this does not break it: `docs/agent/GPT_REVIEW_LOOP.md` says "Codex is the only reviewer... do not add same-model self-review panels, Claude subagent reviewers, or any other substitute". That rule exists to stop **the author's own model grading its own work** (the Identity rule). Grok is neither the author's model nor a substitute for codex, so a second *independent* reviewer serves the rule's purpose rather than defeating it. **Claude subagent reviewers remain forbidden.**
- Evidence it earns its keep (the first use, same day): on the Mac↔Mac instrument, grok reviewed independently, **CONFIRMED both of codex's blockers with its own measurements** (a 500 ms sleep reading as ~3 ms through the broken two-process timer; a rig-W-sized effect still reporting `VANISHES`), and found **three defects codex missed** — including a **RIG-VOID gate that fails open, which grok reproduced** (controls at ratio 1.200/bar FAIL while the session still emitted `VANISHES`). Two independent models converging on a blocker is far stronger than one; a defect only one of them finds is exactly the value of the second. Records: `.review/results/macmac-harness-r2.{gpt,grok}-verdict.md`.
- When to use it: high-stakes slices — a **benchmark instrument** (this project has retracted three claims to harness bugs), a decision rule that will be applied to data, or any adjudication the owner flags. Not every slice; the cost is real (each review is minutes).
- Adjudication is unchanged: **both reviewers are claim sources, not authorities.** Every finding is verified against source before it is accepted, and rejections must cite the file:line that disproves them.
- Supersedes: the "Codex is the only reviewer" sentence in `docs/agent/GPT_REVIEW_LOOP.md` §Shape, which is amended in the same commit to point here.

## D-2026-07-14-3 — the Mac↔Mac decision rule is SIMPLIFIED: one statistic, one threshold, four cell states
- Decision: the mechanized decision rule for the Mac↔Mac rig is **cut back to the smallest thing that still prevents post-hoc rationalization**. Owner, 2026-07-14, verbatim: **"simplify"**, chosen over "harden" after seven review rounds.
- The problem it settles: the instrument has two halves — the **measurement** (harness: transfers, timing, rig gates) and the **decision rule** (engine: what the numbers mean). The measurement half is close to done and is verifiable by running it (`SELFTEST=1`). The decision rule had grown to ~10 outcomes, five thresholds, a certification tier and a precedence stack, and **four of the last five BLOCKERs were in the rule, not the measurement** — each a corner where the branches interacted to produce a confidently wrong verdict. The complexity was buying nothing the owner uses: he reads the table of numbers regardless.
- What is KEPT (this is what pre-registration is actually for): the question, the statistic, and the thresholds are all **fixed before any data exists**, and the harness **computes the verdict** — so no one can look at the numbers and then invent a favourable reading.
- What the rule now IS (rev 8): per cell, the paired ABBA differences `d_i = destinit_i − srcinit_i`, their median, and one **exact order-statistic CI** (coverage ≥95%). One threshold `T = min(10% of the srcinit median, Δ_ref = 230 ms)` — the effect must matter by the project's own invariance bar **and** be the size of the one rig W measured. Four cell states, mutually exclusive **by construction** (no labels a new case can walk past): `EFFECT` (CI_lo ≥ +T), `INVERTED` (CI_hi ≤ −T), `NONE` (CI strictly inside ±T — a genuine equivalence result), `UNCLEAR` (anything else). **Controls must all be `NONE` at the tighter `T/2`**, or no verdict about the measurand is read at all.
- What was DELETED, and why it is safe: the 1.10 bar takes no part in inference (it is the acceptance criterion — computed and reported, never consulted); the sign test is reported, not decided on (at n=8 the CI already implies it); and `UNSTABLE`, `PARTIAL`, `BAR-FAIL-INCONSISTENT`, `UNDERPOWERED` and the precedence stack are gone — **a wide CI absorbs bimodality automatically and lands in `UNCLEAR`**, which is exactly what those branches were hand-coding. All eight runs of every arm are still printed, so bimodality stays visible.
- Cost accepted: `UNCLEAR` and a failed control certification are now the same kind of answer — "not enough power" — and there is **NO escalation**: a noisy rig is fixed by a **quieter rig**, not more pairs. (**CORRECTED 2026-07-14**: this line originally registered `RUNS=16` as the remedy for both. The owner removed the escalation the same day — `n` is **exactly 8**, and the harness refuses any other value (`bench_otp12pf_mac.sh` preflight) because a null is judged on the **full range**, which only *widens* with `n`: more pairs can never rescue an `UNCLEAR` rig nor certify a control. The stale line was found by `drift` while fixing round 11.)
- Supersedes: the rev-4/5/6/7 decision rules in `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md`. **Legitimate only because NO DATA HAS EVER BEEN TAKEN** — before the first run is the only honest time to change a pre-registered rule.

## D-2026-07-14-4 — a rig whose permitted bias reaches HALF the effect threshold is NOT CLEAN: the session refuses to grade
- Decision: on any measurand cell, if the residual arm bias the controls could not exclude (`B`) reaches **`T/2`**, the session verdict is **`CONTROLS-NOT-CLEAN`** and **no verdict is read** — not a reproduction, and not a null. Owner, 2026-07-14, choosing between "refuse to grade" and "grade it anyway, flag it", verbatim: **"Refuse to grade"**.
- The defect it closes (round-11 codex, HIGH, `.review/results/macmac-harness-r11.codex-engine.md`; grok found the same dead-zone independently): `T` is **capped** at Δ_ref = 230 ms, but the bias a *clean* control is permitted to carry is a **fraction of its arm** (≤5%), and that fraction is scaled onto whatever arm it is applied to. On a slow measurand the two diverge: with clean controls at `+49 ms` on a 1000 ms arm (4.9%), a measurand at `src = 10000 ms` gets `T = 230` but `B = 490`. Then `T − B < 0`, so **a null is arithmetically impossible**, while `T + B` still licenses an `EFFECT` of which **up to 68% is permitted rig bias** — at a ratio of only 1.072, i.e. inside the project's own invariance bar. A confidently wrong reproduction, off a rig certified clean.
- Why refusing beats flagging: it is the same principle the rule already applies to a dirty control — *a noisy rig is fixed by a quieter rig, not by grading it with an asterisk*. `B ≥ T/2` means the rig's own permitted noise is at least half the effect being hunted; nothing read off it can be attributed. Cost accepted: a marginal rig yields **no answer** and must be quietened and re-run.
- Also fixed in the same round (mechanical, no owner choice — `B` must only ever HARDEN a verdict): (a) `B` could make the **session** verdict *easier* via `MIXED` precedence — a bigger `B` pushed one cell out of `INVERTED`, the `MIXED` branch stopped firing, and an inconclusive session reported `REPRODUCES`. The `MIXED` test is now taken on the **unhardened** cell states, so extra control noise can never turn an inconclusive session into a reproduction. (b) The **arm** medians used the low-median convention registered only for the paired differences; a bimodal arm therefore pulled `srcinit_med` *down*, shrinking `T`, making an `EFFECT` **easier**. The arms now use the conventional even-`n` median.
- Supersedes: nothing. Amends the rule of D-2026-07-14-3 (rev 10 → rev 11 of the pre-registration); **legitimate only because NO DATA HAS EVER BEEN TAKEN.**

## D-2026-07-14-5 — the FIRST review of anything asks "is this the best way to do it", not "is this correct per the plan"
- **Prompt form superseded by D-2026-07-16-1.** The best-way question remains;
  the prescribed end-goal framing, alternative questions, explicit
  wrong-question invitation, and later-round correctness narrowing below are
  historical and must not be copied into a Claude prompt.
- Decision: the first codex/grok round on a slice, a plan or an instrument is framed around **the end goal**, not around the agent's own plan. It states the goal in plain terms, hands over the artefacts, and asks: will this achieve it? what would you do instead, or first? what does each possible outcome buy? is the *shape* right? — and **only then** correctness. Owner, 2026-07-14, verbatim: **"less 'is this code correct per the plan' and more 'is this the best way to do it'"**, and **"You keep finding problems in the plan you wrote, so it's likely codex will have a more coherent idea."**
- The failure it closes: a prompt that says *"verify these fixes closed the findings"* can only ever return **a longer list of findings**. It grants the plan, the design and the approach and audits the diff against them — so a wrong *approach* survives every round while the *code* gets steadily more correct. The agent wrote the plan; asking the reviewer to grade the code against that plan is **the author grading their own work with extra steps**, which is precisely what the Identity rule (`.review/README.md`) forbids and what codex exists to avoid.
- Evidence: the Mac↔Mac instrument. **ELEVEN review rounds, ~110 findings, all accepted, and it has still never run.** Every round was framed as "here are the previous findings, verify they are closed and find the next ones" — and every round obliged. Not one round was ever asked whether the experiment was worth running at all.
- The prompt must say explicitly that **"this is a well-built thing pointed at the wrong question" is the most valuable answer available and will not offend.** A reviewer told to find bugs finds bugs; a reviewer told to challenge the approach can still report the approach is sound — and *that* report is then worth something, because refusal was available.
- Later rounds may narrow to correctness against the (now-reviewed) spec.
- Supersedes: nothing. Extended `docs/agent/GPT_REVIEW_LOOP.md` historically;
  its prompt details are superseded by D-2026-07-16-1.

## D-2026-07-15-1 — Future reviewloop dispatches use Claude Fable 5 at max effort
- Decision: The already-dispatched otp12 pf-1 rig-W G12 Grok review is the sole grandfathered review and remains valid; the owner clarified that it must finish in flight. Every later synchronous reviewloop dispatch uses Claude CLI with the exact reviewer flags `--model claude-fable-5 --effort max`. Codex and Grok are not substitutes unless a later explicit owner instruction changes the reviewer. Owner, 2026-07-15: **"going forward use reviewloop claude with --model claude-fable-5 and --effort max for reviews"** and, clarifying the boundary, **"going forward, meaning let the in-flight grok review finish."**
- Why: The coding harness is Codex, so Codex reviewing its own work is not an independent second set of eyes. The owner selected a specific external model and effort level for future reviews while preserving the already-running Grok review rather than discarding completed proof.
- Unchanged: D-2026-07-04-1's repo-wide scope, the synchronous fixed-SHA review contract, validation and red-to-green guard proof, structured fail-closed verdict, coder adjudication, commits on `master`, and the no-push rule all stand. D-2026-07-14-5's best-way question stands only in the neutral form required by D-2026-07-16-1.
- Supersedes: only the reviewer identity/dispatch clauses of D-2026-06-20-6 and D-2026-07-04-1, D-2026-07-14-2's Codex-mandatory/Grok-additive selection, and the Codex/Grok model-name examples in D-2026-07-14-5. It does not invalidate historical review results.

## D-2026-07-16-1 — Claude review prompts ask the best-way question without steering
- Decision: every Claude review round receives only a one-sentence neutral goal,
  immutable artifact/base/head identity, and the substantive question **"Is the
  code as implemented the best way to achieve this goal?"** For a plan or other
  non-code artifact, substitute only its noun. Do not ask Claude to validate
  code against the plan. Do not provide an author-written issue list, prior
  findings to confirm, expected outcome, preferred design, suggested
  alternative, concern checklist, persuasive framing, or any other steering.
  This applies to first reviews and re-reviews alike. Mechanical safety bounds,
  the fixed-SHA JSON verdict schema, and a neutral request for an independently
  chosen guard proof remain allowed because they constrain execution and record
  identity, not the substantive answer.
- Why: the owner reports that Fable performs best when it is allowed to decide
  what matters without leading or framing. A prompt that names the author's
  diagnosis or expected fix turns the second opinion back into plan-conformance
  checking and biases the result. Owner, 2026-07-16: **"ask it if the code as
  implemented is 'the best way to achieve the goal'- no leading, no framing. no
  steering. that's how fable performs best."**
- Supersedes: D-2026-07-14-5's prescribed prompt framing, explicit
  wrong-question invitation, alternative-question list, and later-round
  correctness exception. It preserves only that decision's underlying
  best-way-not-plan-conformance question. D-2026-07-15-1's Claude Fable 5/max
  reviewer selection and every fixed-SHA/guard/adjudication safety rule stand.

## D-2026-07-16-2 — Activate the live dial tuning correction
- Decision: `docs/plan/LIVE_DIAL_TUNING.md` moves from Draft to Active. Implement
  its ldt-1 through ldt-4 slices in order through the synchronous reviewloop;
  the owner's activation instruction was **"go"** after the reviewed Draft and
  neutral Claude acceptance were presented.
- Why: production still follows a static ADD-only shape ramp even though the
  settled design requires one SOURCE-owned controller to adjust TCP workers up
  and down from live telemetry in both connection layouts. Claude round 1
  reopened two details, both were corrected one per commit, and a neutral
  D-2026-07-16-1 Claude Fable 5/max round 2 accepted the complete Draft with an
  independent red/green guard.
- Supersedes: only the Draft status and owner-activation checkpoint in
  `docs/plan/LIVE_DIAL_TUNING.md`. The reviewed design, no-push rule, fixed-SHA
  review contract, and any plan-defined endpoint safety gates remain unchanged.

## D-2026-07-16-3 — Use Fable selectively and Grok for tactical slice review
- Decision: formal review is risk-selected rather than an unconditional gate on
  every code and plan change. Use Grok for independent second eyes and slice
  reviews when they add value. Reserve Claude CLI with
  `--model claude-fable-5 --effort max` for final acceptance passes and
  tactical high-risk or contested questions. The current ldt-4 harness gets a
  Grok slice review; the final ldt-4 package gets the Fable pass unless a
  tactical need calls for Fable earlier. Owner, 2026-07-16: **"let's cut back
  on fable reviews. it's getting costly. use fable for final passes and
  tactically when needed. use grok for second-eyes and slice reviews when
  needed."**
- Why: the recorded live-dial Fable spend reached about $90.62 with strong
  value on the design race/cap proof and ldt-3 trace-order defect, but poor
  marginal value on mechanical documentation, clean re-reviews, and a
  declined style-only suggestion. Reviewer cost should follow expected risk
  reduction rather than change count.
- Supersedes: D-2026-07-15-1's Fable-for-every-dispatch selection,
  D-2026-07-04-1's unconditional every-code/every-plan review clause, and the
  corresponding review-frequency clause in D-2026-07-16-2. It preserves the
  synchronous fixed-SHA `openreview` machinery whenever review is selected,
  D-2026-07-16-1's neutral no-steering prompt for Claude, independent
  red/green guard proof, fail-closed structured verdict, coder adjudication,
  one-finding-per-commit fixes, no-agent-branch rule, no-push rule, and all
  historical review results.

## D-2026-07-16-4 — Fable owns formal openreview; Grok remains advisory
- Decision: Every formal `openreview` dispatch uses Claude CLI with `--model claude-fable-5 --effort max`; Grok may provide an advisory second eye or tactical slice check, but a Grok result is never the formal acceptance verdict. Owner, 2026-07-16: **"no don't use grok for openreview. that should be fable."**
- Why: The cost reduction comes from selecting fewer formal reviews, not from substituting Grok at the acceptance boundary; the owner wants Fable's unprimed judgment for every review that carries `openreview` authority.
- Supersedes: D-2026-07-16-3 only where it permitted the ldt-4 harness or another ordinary slice review to use Grok as formal `openreview`. It preserves risk-based review frequency, Grok advisory use, Fable tactical/final use, D-2026-07-16-1's neutral prompt, and every fixed-SHA/guard/adjudication/git-safety rule.

## D-2026-07-21-1 — ldt-4 follows netwatch-01's verified current DHCP identity
- Decision: Change the registered ldt-4 Windows endpoint from stale
  `10.1.10.177` to verified-current `10.1.10.173` and proceed without forcing
  the machine back onto its prior lease. Owner, 2026-07-21: **"no. just adapt
  to reality and go."**
- Why: DNS and strict-host-key SSH identify `.173` as the same `NETWATCH-01`,
  with host keys identical to q's trusted `.177` records; the NIC MAC,
  interface, MTU, and 10 GbE topology remain independently pinned. A DHCP
  address change is not a reason to mutate the rig or discard current reality.
- Supersedes: the `.177` literal introduced by ldt-4 endpoint correction
  `9926bf7` and repeated in the harness/analyzer. It does not weaken endpoint
  identity, host-key verification, MAC/interface/link gates, exact-artifact
  review, or the additive evidence contract.

## D-2026-07-22-1 — Release correctness before further hardware tuning
- Decision: Stop all pre-release data-moving performance experiments. Treat a
  harness or analyzer rejection as a tool outcome, not automatic invalidation
  of complete raw evidence: repair the interpreter and reanalyze immutable,
  unambiguous evidence instead of rerunning it. Move ldt-4 causal tuning,
  Mac↔Mac Thunderbolt testing, P1 performance closure, and other hardware
  ceilings after release. Any later large SSD-write test requires explicit
  owner approval.
- Why: the complete first horizon session was unnecessarily repeated after an
  analyzer expected `ADD`/`REMOVE` instead of production's exact protobuf enum
  spelling, consuming another 160 GiB of destination writes. Performance
  research has delayed a complete release without exposing a data-correctness
  failure. Release work is now limited to correctness, supported-platform CI,
  packaging, installation, startup, and explicit scope decisions.
- Supersedes: D-2026-07-16-2 only where it requires ldt-4 live continuation;
  the P1 performance criterion as a shipping gate in `docs/STATE.md`; the
  pre-release scheduling of `OTP12_PERF_FINDINGS`, performance acceptance
  residue, and the Thunderbolt experiment. It preserves accepted ldt-1..3
  code, all retained evidence, and every endpoint identity and safety rule.

## D-2026-07-22-2 — Resolve P1 from existing evidence without more transfers
- Decision: Identify and close the P1 initiator discrepancy from retained raw
  evidence, exact historical/current code, and deterministic mutation guards;
  do not run another physical transfer for it. Owner, 2026-07-22, verbatim:
  **"no. without doing more pointless transfers, identify and fix the
  discrepancy"**.
- Why: the failing builds' product path is identical to the old-red worker
  guard (SOURCE initiation 3 workers, DESTINATION initiation 2, plus a
  destination-only zero-capacity cap); `a76b785..42b9b38` fixes and
  mutation-proves parity, post-fix `8e019ef` no longer shows the target-cell
  failure, and ldt-2 preserves role parity under the current controller.
- Supersedes: D-2026-07-22-1 only where it deferred P1 closure and
  `OTP12_PERF_FINDINGS` where it required another P1 rig/counterfactual/final
  run. It preserves the ban on unapproved data-moving hardware work and does
  not close the separate P2 finding.

## D-2026-07-22-3 — Every known broken behavior blocks release
- Decision: Activate `docs/plan/RELEASE_COMPLETION.md`. Every known broken
  product behavior must be fixed before release, including P2, Windows metadata
  loss, incomplete progress, CI failures, move hangs, packaging, installation,
  and startup gaps. Classification is agent bookkeeping, not an owner decision
  or a deferral mechanism. Owner, 2026-07-22, verbatim: **"I don't care how you
  classify the broken things. all of them need to be fixed. accounting is your
  problem. you are the only consumer of that. fix it. I need a working app."**
- Why: the owner needs one working cross-platform application, not an internal
  taxonomy or a release assembled by waiving known failures. Optional ceiling
  research remains distinct from a measured product regression or broken
  behavior.
- Supersedes: D-2026-07-22-1 only where it deferred a known broken behavior
  such as P2, every unresolved release-scope question in STATE/readiness, and
  the Draft gate in `RELEASE_COMPLETION.md`. It preserves the ban on
  unapproved data-moving hardware tests and all outward-git approval gates.

## D-2026-07-22-4 — Run one conservative Thunderbolt ceiling probe before publication
- Decision: Move the Mac-to-Mac direct Thunderbolt probe ahead of publication
  as a narrow exception to D-2026-07-22-1. SSD writes are approved only
  conservatively; certify the physical link and routing first, measure the
  bidirectional RAM/network ceiling, then compare exact-candidate Blit against
  unencrypted rsync once on the same byte direction and medium. Owner,
  2026-07-22: **"would like to do the thunderbolt test first to probe the
  ceiling vs rsync, ssd writes approved but stay conservative"**, followed by
  **"go"** for the isolated Thunderbolt configuration and zero-disk test.
- Why: a direct 40 Gb/s path can expose engine headroom hidden by the 10 GbE
  rigs, while a RAM destination and APFS-cloned source fixture avoid another
  large write cycle. The completed probe is recorded in
  `docs/bench/thunderbolt-macmac-2026-07-22/README.md`.
- Supersedes: D-2026-07-22-1 only for this completed, explicitly approved
  probe's pre-publication ordering. It does not turn optional tuning into a
  release gate, approve a repeated or formal matrix, weaken the large-write
  approval rule, authorize code changes, or authorize publication.

## D-2026-07-22-5 — Cap the SSD-backed Thunderbolt follow-up at 40 GB
- Decision: Activate `docs/plan/THUNDERBOLT_SSD_PROBE.md` with up to 40 GB of
  total plan-created SSD writes when the larger fixture materially improves
  the evidence. Realize the ceiling as one physical 12 GiB source plus one
  12 GiB destination per tool: 36 GiB / 38.7 decimal GB of benchmark payload,
  at most 100 MiB of tooling/config/log overhead, one arm per tool, and no
  automatic retry or repeat. Owner, 2026-07-22: **"you can write up to 40gb if
  that will give you more data"**.
- Why: twelve 1 GiB files lengthen the Blit arm by 50% versus the RAM probe,
  exceed Q's comfortable file-cache working set, and remain below the owner's
  ceiling across source creation and both destination writes.
- Supersedes: D-2026-07-22-4 only where its conservative approval did not name
  a numeric budget. It does not approve more than this one plan, any failed-arm
  retry, a repeated/formal matrix, a product change, or publication.

## D-2026-07-23-1 — Profile the remaining Thunderbolt RAM-path gap once
- Decision: Activate `docs/plan/THUNDERBOLT_RAM_PROFILE.md` for one
  exact-candidate Q-to-Nagatha run using the same warm 8 GiB APFS-cloned source
  shape, a RAM-disk destination, existing session-phase telemetry, and process
  CPU/memory accounting. No SSD payload, rsync or iperf arm, reverse direction,
  retry, repeat, product change, or publication is authorized. Owner approved
  the stated RAM-only profiling next action with **"go"** on 2026-07-23.
- Why: the SSD-backed reduction is already attributed chiefly to Q's physical
  source reads. The unresolved 28.6 versus 37.9 Gb/s gap is the warm engine
  path; one observed run can distinguish CPU, fixed-phase, backpressure, and
  controller-lifetime limits without another wear-heavy transfer.
- Supersedes: D-2026-07-22-1 only for this one pre-publication diagnostic and
  D-2026-07-22-4 only where it required a separate approval for later tuning
  observation. It preserves every code-plan, large-write, repeat, git-publication,
  tag, and release gate.

## D-2026-07-23-2 — Retire Fable and use Claude Opus 4.8/max for reviews
- Decision: The already-running end-to-end transfer latency plan review was the
  final Fable review. Every future formal `openreview` uses Claude CLI with
  `--model claude-opus-4-8 --effort max`; Fable is not a review fallback. If
  that exact reviewer is unavailable, the review fails closed and is reported
  rather than silently rerouted. Owner, 2026-07-23: **"after this run, stop
  using fable for reviews."** This applies with the owner's earlier approval to
  switch reviews to Claude Opus 4.8/max.
- Why: the owner explicitly retired Fable after the in-flight run and selected
  Opus 4.8/max as its replacement. The in-flight Fable attempt ended without a
  verdict after reaching its session limit, so it carries no acceptance.
- Supersedes: D-2026-07-15-1 and D-2026-07-16-4 on reviewer identity, plus the
  Fable selection in D-2026-07-16-3. It preserves risk-based review frequency,
  Grok's advisory-only role, D-2026-07-16-1's neutral prompt, exact-SHA and
  independent-guard requirements, finding adjudication, and git safety.

## D-2026-07-23-3 — Activate end-to-end transfer latency attribution
- Decision: Activate `docs/plan/END_TO_END_TRANSFER_LATENCY.md`. Add default-off
  diagnostic lifecycle timing without changing transfer policy, then perform
  exactly one approved 8 GiB Q-to-Nagatha RAM-destination validation under the
  plan's SSD-write and no-repeat limits. Owner, 2026-07-23: **"go"**.
- Why: the clean Opus 4.8/max review verified the plan and its evidence; the
  remaining 0.448-second external interval must be observed before any further
  performance change can be justified.
- Supersedes: nothing. It preserves the separate release, publication, tag,
  transfer-policy-change, SSD-write, retry, and repeat gates.

## D-2026-07-23-4 — Activate terminal data-plane attribution
- Decision: Activate `docs/plan/TERMINAL_DATA_PLANE_ATTRIBUTION.md`. Preserve
  the existing final SOURCE-stream payload and blocked-write counters for
  traced sessions and emit their terminal aggregate after the send pipeline
  joins. No hardware transfer or transfer-policy change is authorized. Owner,
  2026-07-23: **"Continue."**
- Why: short high-speed transfers can finish before the periodic sampler emits
  a record. Terminal aggregation reuses counters already maintained on the
  payload path and supplies the missing attribution without another transfer.
- Supersedes: nothing. It preserves every hardware-run, SSD-write, release,
  publication, tag, transfer-policy, retry, and repeat gate.

## D-2026-07-23-5 — Run one terminal data-plane hardware validation
- Decision: Activate `docs/plan/TERMINAL_DATA_PLANE_VALIDATION.md`. Run the
  accepted terminal observer exactly once over Q-to-Nagatha Thunderbolt with an
  8 GiB read-only Q source and a fresh Nagatha RAM destination. Q SSD writes
  remain below 32 decimal MB and exclude payload; no retry or comparison run is
  authorized. Owner, 2026-07-23: **"go"**.
- Why: the retained 35.578 and 19.153 Gb/s samples share the same data-path
  code, while the accepted terminal observer can now distinguish socket
  backpressure from time outside socket writes in one bounded observation.
- Supersedes: D-2026-07-23-4 only where it withheld later physical validation.
  It preserves every transfer-policy, repeat, larger-write, release,
  publication, tag, and push gate.
