# Blit v2 Planning & Workflow Documentation

This directory contains all planning, workflow, and architectural
documentation for the Blit v2 implementation.

## Current Status

The canonical current-state entry point is [`../STATE.md`](../STATE.md).
The active release plan is
[`RELEASE_COMPLETION.md`](./RELEASE_COMPLETION.md), with blockers in
[`../RELEASE_READINESS.md`](../RELEASE_READINESS.md). Older phase, TUI, and
audit plans below are retained as historical records unless `docs/STATE.md`
explicitly names them as active.

## Document Index

### Live (active source of truth)

- **[../STATE.md](../STATE.md)** — canonical current state and plan routing
- **[RELEASE_COMPLETION.md](./RELEASE_COMPLETION.md)** — active release plan
- **[../RELEASE_READINESS.md](../RELEASE_READINESS.md)** — active release ledger and blockers

### Reference (shipped / frozen)

- **[RELEASE_PLAN_v2_2026-05-04.md](./RELEASE_PLAN_v2_2026-05-04.md)** — 0.1.0 release plan (frozen reference; release shipped 2026-05-31)
- **[TUI_DESIGN.md](./TUI_DESIGN.md)** — Original Phase 5 TUI F1–F4 plan (SUPERSEDED by TUI_REWORK.md; retained for historical context)
- **[PROJECT_STATE_ASSESSMENT.md](./PROJECT_STATE_ASSESSMENT.md)** — 2026-04-07 phase-4 snapshot (historical)
- **[REMOTE_TRANSFER_PARITY.md](./REMOTE_TRANSFER_PARITY.md)** — Remote push/pull parity refactor (shipped)
- **[LOCAL_TRANSFER_HEURISTICS.md](./LOCAL_TRANSFER_HEURISTICS.md)** — Local transfer optimization decisions
- **[BLIT_UTILS_PLAN.md](./BLIT_UTILS_PLAN.md)** — Admin command matrix (utilities ship as `blit` subcommands; doc retained for rationale)
- **[MASTER_WORKFLOW.md](./MASTER_WORKFLOW.md)** — Historical phase coordination; superseded by `docs/STATE.md`
- **[TUI_REWORK.md](./TUI_REWORK.md)** — TUI plan retained for its implementation record
- **[greenfield_plan_v6.md](./greenfield_plan_v6.md)** — Historical architectural proposal
- **[../audit/AUDIT_REPORT_2026-06-04_INDEX.md](../audit/AUDIT_REPORT_2026-06-04_INDEX.md)** — Historical audit index

### Phase Workflows (Historical)

- **[WORKFLOW_PHASE_2.md](./WORKFLOW_PHASE_2.md)** — Historical local-orchestrator proposal; never completed as written and later superseded by the unified transfer session
- **[WORKFLOW_PHASE_2.5.md](./WORKFLOW_PHASE_2.5.md)** — Performance & Validation Checkpoint (complete)
- **[WORKFLOW_PHASE_3.md](./WORKFLOW_PHASE_3.md)** — Remote Operations / Hybrid Transport (complete)
- **[WORKFLOW_PHASE_4.md](./WORKFLOW_PHASE_4.md)** — Production Hardening & Packaging (complete)

### Code Reviews (Historical)

- `review/code_review_phase2.md` — Phase 2 code review findings

## Quick Start

### If you're new to the project:
1. Read **../STATE.md** for current direction and active milestones.
2. Follow the active plan and release ledger it names.
3. Use historical plans only for design provenance.

### If you're ready to work:
1. `cargo test --workspace` to verify baseline.
2. Pick the next action from **../STATE.md**.
3. Follow the repository guidance and the active plan's slice contract.

---

**Last Updated**: 2026-06-04 (M28 source-of-truth sweep)
**Project**: Blit v2
