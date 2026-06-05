# Blit v2 Planning & Workflow Documentation

This directory contains all planning, workflow, and architectural
documentation for the Blit v2 implementation.

## Current Status

**0.1.0 shipped** (tag `v0.1.0`, 2026-05-31).
[`RELEASE_PLAN_v2_2026-05-04.md`](./RELEASE_PLAN_v2_2026-05-04.md) is
the frozen reference for that release and is **no longer the active
source of truth**.

**Active plan: Phase 6 TUI rework.**
[`TUI_REWORK.md`](./TUI_REWORK.md) is the live source of truth — it
defines the dual-pane Pick-not-Type model (M1–M6) that supersedes the
F1–F4 shipped baseline.

**Open-finding source: 2026-06-04 audit chain.**
[`../audit/AUDIT_REPORT_2026-06-04_INDEX.md`](../audit/AUDIT_REPORT_2026-06-04_INDEX.md)
names the current audit state (R2 inventory + R3 delta). Round 1
(data-loss / DoS class) is the active implementation queue.

[`greenfield_plan_v6.md`](./greenfield_plan_v6.md) §1.1 (streaming
planner + 1 s heartbeat + 10 s stall detector) is **canonical but not
yet built** — owner-ratified per the 2026-06-04 audit; multi-slice
implementation queued after Round 1 hardening closes.

## Document Index

### Live (active source of truth)

- **[TUI_REWORK.md](./TUI_REWORK.md)** — Phase 6 dual-pane TUI plan (active)
- **[greenfield_plan_v6.md](./greenfield_plan_v6.md)** — Active architectural plan (§1.1 streaming planner ratified, not yet built)
- **[MASTER_WORKFLOW.md](./MASTER_WORKFLOW.md)** — Phase coordination and quality gates
- **[../audit/AUDIT_REPORT_2026-06-04_INDEX.md](../audit/AUDIT_REPORT_2026-06-04_INDEX.md)** — Current audit state pointer

### Reference (shipped / frozen)

- **[RELEASE_PLAN_v2_2026-05-04.md](./RELEASE_PLAN_v2_2026-05-04.md)** — 0.1.0 release plan (frozen reference; release shipped 2026-05-31)
- **[TUI_DESIGN.md](./TUI_DESIGN.md)** — Original Phase 5 TUI F1–F4 plan (SUPERSEDED by TUI_REWORK.md; retained for historical context)
- **[PROJECT_STATE_ASSESSMENT.md](./PROJECT_STATE_ASSESSMENT.md)** — 2026-04-07 phase-4 snapshot (historical)
- **[REMOTE_TRANSFER_PARITY.md](./REMOTE_TRANSFER_PARITY.md)** — Remote push/pull parity refactor (shipped)
- **[LOCAL_TRANSFER_HEURISTICS.md](./LOCAL_TRANSFER_HEURISTICS.md)** — Local transfer optimization decisions
- **[BLIT_UTILS_PLAN.md](./BLIT_UTILS_PLAN.md)** — Admin command matrix (utilities ship as `blit` subcommands; doc retained for rationale)

### Phase Workflows (Historical)

- **[WORKFLOW_PHASE_2.md](./WORKFLOW_PHASE_2.md)** — Orchestrator & Local Operations (complete)
- **[WORKFLOW_PHASE_2.5.md](./WORKFLOW_PHASE_2.5.md)** — Performance & Validation Checkpoint (complete)
- **[WORKFLOW_PHASE_3.md](./WORKFLOW_PHASE_3.md)** — Remote Operations / Hybrid Transport (complete)
- **[WORKFLOW_PHASE_4.md](./WORKFLOW_PHASE_4.md)** — Production Hardening & Packaging (complete)

### Code Reviews (Historical)

- `review/code_review_phase2.md` — Phase 2 code review findings

## Quick Start

### If you're new to the project:
1. Read **TUI_REWORK.md** for current direction and active milestones.
2. Read **greenfield_plan_v6.md** for architectural vision.
3. Skim **../audit/AUDIT_REPORT_2026-06-04_INDEX.md** for the open-finding queue.
4. Check `TODO.md` in project root.

### If you're ready to work:
1. `cargo test --workspace` to verify baseline.
2. Pick the highest-priority audit finding from R3 Round 1 (or current TUI rework milestone).
3. Follow the per-slice contract in `.review/README.md`.

---

**Last Updated**: 2026-06-04 (M28 source-of-truth sweep)
**Project**: Blit v2
