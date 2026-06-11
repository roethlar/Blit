# Repo-wide design-coherence review

**Status**: Shipped
**Created**: 2026-06-11
**Supersedes**: nothing
**Decision ref**: D-2026-06-11-1 (activation, Phase A); D-2026-06-11-2 (full
ratification of the Phase C queue — program complete, all three phases
delivered 2026-06-11)

## Goal

A comprehensive, evidence-backed map of where the codebase's *design* (not its
bug count) diverges from the project's three principles — FAST (maximum
hardware efficiency in every config), SIMPLE (no user tuning; the tool adapts
at runtime), RELIABLE (works, and fails plainly with clear text) — plus a
ranked findings report that feeds the existing `.review/` loop. The repo was
built by many models (GPT 4/5/5.5, Opus 3/4–4.8, Gemini, Grok) across several
greenfield restarts; the owner judges it "too schizophrenically designed to be
trusted as-is" but does not want another restart. The review's job is to find
the strata those models left behind: concept-ownership violations, duplicated
subsystems, boundary leaks, inconsistent error handling, hardcoded constants
that violate the no-tuning principle, dead code and abandoned migrations, test
quality variance, and docs-vs-code drift.

Seed evidence (from the audit-h3c slice-1 review assessment, DEVLOG
2026-06-11): three independently-written gRPC channel builders already drifted
apart (`pull.rs:239`, `push/client/mod.rs:307`, `blit-app/src/client.rs:38`);
a TCP transport parameter (`chunk_bytes`) embedded in the transport-agnostic
`PlannedPayloads`; fallback correctness silently resting on tonic's default
4 MiB decode limit; missing client-side HTTP/2 keepalive while the daemon side
has it. The review should find the rest of this class.

## Non-goals

- **No code changes.** The review only reads; every change it motivates goes
  through the normal plan/review loop afterward.
- **Not a greenfield/rewrite proposal.** The owner has restarted this project
  multiple times and explicitly does not want another restart.
- **Not a second bug-class audit.** The 2026-06-04 audit (data-loss / DoS,
  Rounds 1–3) continues separately; overlapping findings get cross-referenced,
  not duplicated.
- **Not a TUI feature review.** Phase 6 TUI rework (`TUI_REWORK.md`) is already
  planned; see Open questions for review depth on `blit-tui`.

## Constraints

- Review-only: the working tree is untouched for the duration; output is docs.
- Every reported finding carries file:line evidence and survives independent
  adversarial verification before it reaches the report (house review culture).
- Output feeds the **existing** machinery: report in `docs/audit/` format,
  candidate findings shaped for `.review/findings/`, queue proposal for
  STATE.md that the owner ratifies. No parallel tracking system.
- Multi-agent workflow execution; whole-program cost is on the order of a few
  million tokens. Each phase is a separate run with an owner checkpoint
  between phases (AGENTS.md §9: the owner declares pass/continue).
- 83K lines of Rust across 6 crates: blit-core 26.7K, blit-tui 28.1K,
  blit-daemon 12.5K, blit-cli 10.8K, blit-app 4K, blit-prometheus-bridge 0.9K.
- Owner decisions (interview 2026-06-11):
  - **blit-tui gets a light pass**: stratum inventory + boundary/duplication
    checks only; no deep per-dimension review of TUI internals (Phase 6 rework
    will absorb that).
  - **Ratification is Phase A only.** Phases B and C each require a fresh
    owner go/no-go at the preceding checkpoint; this plan going Active
    authorizes Phase A alone.
  - **Owner ratifies each Phase C finding individually** before it becomes a
    `REVIEW.md` row; nothing auto-enters the queue.
  - **Wire-breaking recommendations are in scope**: pre-1.0, CLI and daemon
    ship together; `proto/blit.proto` is not compatibility-frozen.

## Acceptance criteria

- [ ] Phase A: a concept-ownership map exists in `docs/audit/` covering, at
  minimum: channel/connection construction, timeouts & liveness policy, chunk
  sizing, retry classification, error types & failure text, progress
  reporting, config constants vs runtime tuning, platform-specific code
  (win_fs parity), cancellation. For each concept: who owns it, every
  implementation site, and whether duplicates agree.
- [ ] Phase B: each review dimension (boundaries, duplication, error-handling
  consistency, hardcoded constants vs no-tuning, async hygiene, dead code &
  abandoned migrations, test quality, docs-vs-code drift) produces findings
  with file:line evidence, each independently verified before inclusion.
- [ ] Phase C: one ranked report (`docs/audit/AUDIT_REPORT_<date>_DESIGN.md`)
  with severity, evidence, and proposed slice boundaries; candidate finding
  docs ready for `.review/findings/`; a proposed STATE.md queue update. Owner
  ratifies what enters the queue.
- [ ] The plan-doc graveyard (`PIPELINE_UNIFICATION.md`,
  `UNIFIED_RECEIVE_PIPELINE.md`, `WORKFLOW_V2.md`, etc.) is cross-checked:
  every half-finished migration named in a plan doc is either confirmed
  complete, confirmed abandoned-and-documented, or filed as a finding.

## Design

Three phases, each a separate workflow run, each ending at an owner
checkpoint. Phase A's output is input to Phase B's reviewer prompts; Phase B's
verified findings are Phase C's raw material.

**Phase A — concept-ownership map.** Parallel readers sweep the crates, one
per concept (list in Acceptance criteria) plus one per crate for a stratum
inventory (which model-era idioms live where; the plan-doc graveyard tells
readers which migrations to trace). Output: a single map doc. This phase is
cheap relative to what it de-risks and goes first because its map determines
where Phase B digs.

**Phase B — dimension reviews, adversarially verified.** Fan out reviewers by
dimension (not by file), seeded with the Phase A map. Every candidate finding
is checked by independent verifier agents prompted to refute it; only
findings that survive reach Phase C. Severity scale mirrors the 2026-06-04
audit (H/M/L) so the two audit lines stay comparable.

**Phase C — synthesis.** Dedup, rank, write the report, shape candidate
finding docs, propose the queue. Owner ratifies.

**Execution venue:** planning and Phase A run in the current session (context
already warm with seed evidence); later phases may run in fresh sessions —
each phase's output doc is the only required cross-session carrier.

**Risks:** (1) Finding volume overwhelms the review loop — mitigated by
ranking and by the owner gating what enters the queue. (2) Verifier agents
rubber-stamp — mitigated by refutation-framed prompts and majority voting.
(3) The TUI's size dominates token spend for code that Phase 6 may rework —
mitigated per the owner's answer to the depth question below. (4) Findings
overlap the existing audit — mitigated by giving reviewers the 2026-06-04
index and requiring cross-references instead of duplicates.

## Slices

One phase per slice; each ends at an owner checkpoint (§9 — owner declares
pass/continue, approvals are single-use).

1. **Phase A** — concept-ownership map + per-crate stratum inventory →
   `docs/audit/` map doc. Owner reviews the map before Phase B launches.
2. **Phase B** — dimension reviews with adversarial verification → verified
   findings set. Owner reviews scope/volume before synthesis.
3. **Phase C** — synthesis: ranked report + candidate `.review/findings/`
   docs + proposed queue update. Owner ratifies queue entries.

## Open questions

None. The four interview questions (TUI depth, ratification scope, finding
disposition, proto compatibility) were answered by the owner 2026-06-11 and
are recorded under Constraints → Owner decisions.
