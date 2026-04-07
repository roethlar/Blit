# Blit v2 Planning & Workflow Documentation

This directory contains all planning, workflow, and architectural documentation for the Blit v2 implementation.

## Current Status

**Feature-complete as of 2026-04-07.** All phases through Phase 4 (Production
Hardening) are done. Remaining work is benchmarking (needs 10+ GbE hardware)
and post-release investigations (RDMA, ReFS privilege).

See [PROJECT_STATE_ASSESSMENT.md](./PROJECT_STATE_ASSESSMENT.md) for details.

## Document Index

### Status & Planning

- **[PROJECT_STATE_ASSESSMENT.md](./PROJECT_STATE_ASSESSMENT.md)** — Current state, what's done, what's left, Windows dev guide
- **[MASTER_WORKFLOW.md](./MASTER_WORKFLOW.md)** — Phase coordination and quality gates
- **[AI_TELEMETRY_ANALYSIS.md](./AI_TELEMETRY_ANALYSIS.md)** — Scoping doc for optional performance analysis features

### Architecture & Design

- **[greenfield_plan_v6.md](./greenfield_plan_v6.md)** — Active architectural plan
- **[BLIT_UTILS_PLAN.md](./BLIT_UTILS_PLAN.md)** — Admin utilities command matrix and UX principles
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
