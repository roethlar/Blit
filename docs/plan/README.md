# Blit v2 Planning & Workflow Documentation

This directory contains all planning, workflow, and architectural documentation for the Blit v2 greenfield implementation.

## 📊 Current Status

For up-to-date progress, blockers, and percentages, see [PROJECT_STATE_ASSESSMENT.md](./PROJECT_STATE_ASSESSMENT.md). This README now focuses on the document map and workflow references.

## 📋 Document Index

### Executive & Planning

- **[PROJECT_STATE_ASSESSMENT.md](./PROJECT_STATE_ASSESSMENT.md)** - Current state analysis, blockers, and completion status
- **[MASTER_WORKFLOW.md](./MASTER_WORKFLOW.md)** - Overall project coordination and phase management

### Architecture & Design

- **[greenfield_plan_v6.md](./greenfield_plan_v6.md)** - Active architectural plan (feature completeness + hybrid transport)
- **[greenfield_plan_v5.md](./greenfield_plan_v5.md)** - Archived for historical context
- **[greenfield_plan_v4.md](./greenfield_plan_v4.md)** - Older revision (legacy reference only)
- **[WORKFLOW_V2.md](./WORKFLOW_V2.md)** - High-level phase descriptions (legacy, superseded by phase-specific workflows)

### Feature Plans

- **[RCLONE_FEATURE_INTEGRATION.md](./RCLONE_FEATURE_INTEGRATION.md)** - Phase 5: Operational features adopted from rclone (filtering, bandwidth limiting, retries, check, metrics, dedupe, ncdu, bisync, VFS)
- **[REMOTE_TRANSFER_PARITY.md](./REMOTE_TRANSFER_PARITY.md)** - Push/pull parity refactor (complete)
- **[LOCAL_TRANSFER_HEURISTICS.md](./LOCAL_TRANSFER_HEURISTICS.md)** - Local copy path optimization

### Phase-Specific Workflows

- **[WORKFLOW_PHASE_2.md](./WORKFLOW_PHASE_2.md)** - Orchestrator & Local Operations (3-4 days)
- **[WORKFLOW_PHASE_2.5.md](./WORKFLOW_PHASE_2.5.md)** - Performance & Validation Checkpoint (1-2 days) **[CRITICAL GATE]**
- **[WORKFLOW_PHASE_3.md](./WORKFLOW_PHASE_3.md)** - Remote Operations / Hybrid Transport (7-10 days)
- **[WORKFLOW_PHASE_4.md](./WORKFLOW_PHASE_4.md)** - Production Hardening & Packaging (5-7 days)

## 🚀 Quick Start

### If you're new to the project:
1. Read **PROJECT_STATE_ASSESSMENT.md** to understand current state
2. Read **MASTER_WORKFLOW.md** for overall strategy
3. Review **greenfield_plan_v6.md** for architectural vision (v5 retained for historical context)

### If you're ready to work:
1. Check **PROJECT_STATE_ASSESSMENT.md** for current blockers
2. Follow the current phase workflow (currently **WORKFLOW_PHASE_2.md**)
3. Update **TODO.md** and **DEVLOG.md** in project root as you progress

## 📈 Phase Progression

Phase sequencing and quality gates are defined in [MASTER_WORKFLOW.md](./MASTER_WORKFLOW.md). Refer there for the latest status snapshots.

## 📝 Document Conventions

- **Status Markers**: ✅ Complete | ⚠️ In Progress | ⏳ Not Started | 🔴 Blocked
- **Priority**: 🔴 Critical | 🟡 Important | 🟢 Nice to have
- **Effort**: Estimated in hours or days
- **Quality Gates**: Mandatory checkpoints before phase progression

## 🔄 Keeping Documents Updated

When working on the project:

1. **Before starting work**: Read current phase workflow
2. **During work**: Update TODO.md checkboxes
3. **After completing tasks**: Add entry to DEVLOG.md
4. **At phase boundaries**: Review quality gate checklist
5. **When making architectural decisions**: Document in DEVLOG.md

## 📖 Recommended Reading Order

1. **greenfield_plan_v6.md** – Architectural vision (active)
2. **PROJECT_STATE_ASSESSMENT.md** – Current status, blockers, metrics
3. **MASTER_WORKFLOW.md** – Phase coordination and quality gates
4. **Current phase workflow** (e.g., `WORKFLOW_PHASE_2.md`)
5. **Next phase workflow** for upcoming work (e.g., `WORKFLOW_PHASE_3.md`, `WORKFLOW_PHASE_4.md`)

---

**Last Updated**: 2025-10-20
**Project**: Blit v2 Greenfield Implementation
**Status**: See PROJECT_STATE_ASSESSMENT.md for live phase details
