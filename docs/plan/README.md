# Blit v2 Planning & Workflow Documentation

This directory contains all planning, workflow, and architectural
documentation for the Blit v2 implementation.

## Current Status

**Release-readiness work for 0.1.0 is tracked in
[`RELEASE_PLAN_v2_2026-05-04.md`](./RELEASE_PLAN_v2_2026-05-04.md).**
That document is the live source of truth for what's left before
shipping; everything else in this directory is either still-applicable
design (greenfield_plan_v6, REMOTE_TRANSFER_PARITY,
LOCAL_TRANSFER_HEURISTICS) or historical phase-workflow / state
artifacts.

PROJECT_STATE_ASSESSMENT.md is a 2026-04-07 snapshot — superseded by
the release plan above for current readiness, but retained as a phase-4
checkpoint reference.

## Document Index

### Status & Planning

- **[RELEASE_PLAN_v2_2026-05-04.md](./RELEASE_PLAN_v2_2026-05-04.md)** — Live 0.1.0 release plan (P0/P1/P2 + decisions)
- **[PROJECT_STATE_ASSESSMENT.md](./PROJECT_STATE_ASSESSMENT.md)** — 2026-04-07 phase-4 snapshot (superseded for current state by the release plan)
- **[MASTER_WORKFLOW.md](./MASTER_WORKFLOW.md)** — Phase coordination and quality gates

### Architecture & Design

- **[greenfield_plan_v6.md](./greenfield_plan_v6.md)** — Active architectural plan
- **[BLIT_UTILS_PLAN.md](./BLIT_UTILS_PLAN.md)** — Admin command matrix and UX principles (utilities now ship as `blit` subcommands; doc retained for the design rationale)
- **[REMOTE_TRANSFER_PARITY.md](./REMOTE_TRANSFER_PARITY.md)** — Remote push/pull parity refactor (completed)
- **[LOCAL_TRANSFER_HEURISTICS.md](./LOCAL_TRANSFER_HEURISTICS.md)** — Local transfer optimization decisions

### Phase Workflows (Historical)

- **[WORKFLOW_PHASE_2.md](./WORKFLOW_PHASE_2.md)** — Orchestrator & Local Operations (complete)
- **[WORKFLOW_PHASE_2.5.md](./WORKFLOW_PHASE_2.5.md)** — Performance & Validation Checkpoint (complete)
- **[WORKFLOW_PHASE_3.md](./WORKFLOW_PHASE_3.md)** — Remote Operations / Hybrid Transport (complete)
- **[WORKFLOW_PHASE_4.md](./WORKFLOW_PHASE_4.md)** — Production Hardening & Packaging (complete)

### Code Reviews (Historical)

- `review/code_review_phase2.md` — Phase 2 code review findings

## Quick Start

### If you're new to the project:
1. Read **PROJECT_STATE_ASSESSMENT.md** for current state and architecture overview
2. Read **greenfield_plan_v6.md** for architectural vision
3. Check `TODO.md` in project root for remaining items

### If you're ready to work:
1. `cargo test --workspace` to verify baseline
2. Check `TODO.md` for unchecked items
3. Update `TODO.md` and `DEVLOG.md` as you progress

---

**Last Updated**: 2026-04-07
**Project**: Blit v2
