# Blit v2 Master Workflow

**Project**: Blit v2 – feature-complete hybrid transport implementation  
**Plan Reference**: [greenfield_plan_v6.md](./greenfield_plan_v6.md)  
**Live Status**: See [PROJECT_STATE_ASSESSMENT.md](./PROJECT_STATE_ASSESSMENT.md) for current percentages, blockers, and ownership

---

## 1. Operating Principles

### Core Delivery Principles
1. **FAST** – Transfers begin immediately; planner keeps perceived latency ≤ 1 s.
2. **SIMPLE** – No user-facing speed knobs. Planner, orchestrator, and heuristics own performance.
3. **RELIABLE** – Correctness beats speed. Mirror deletions, checksums, and failure handling remain uncompromised.
4. **PRIVATE** – Metrics stay local; no external telemetry.

### Feature-Completeness Goals
- CLI verbs: `copy`, `mirror`, `move`, `scan`, `list`, diagnostics.
- Remote syntax: `server:/module/...`, `server://...`, discovery on bare host.
- Admin tooling (`blit-utils`): `scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile`.
- Daemon configuration via TOML modules + optional `--root`; mDNS advertised unless disabled.
- Hybrid transport (gRPC control plane + TCP data plane with secure tokens and gRPC fallback).

---

## 2. Phase Map & Quality Gates

| Phase | Purpose | Gate Summary |
|-------|---------|--------------|
| **Phase 0 – Foundation** | Workspace, core library port, baseline CLI/daemon scaffolding | ✅ Complete |
| **Phase 1 – gRPC Scaffolding** | Proto definitions, tonic wiring, build plumbing | ✅ Complete |
| **Phase 2 – Streaming Orchestrator & Local Ops** | Streaming planner, predictor, local CLI UX | Gate: local `copy`/`mirror`/`move` fast & reliable; tests and benchmarks pass |
| **Phase 2.5 – Validation Checkpoint** | Benchmark parity against v1 | Gate: local workloads ≥ 95 % v1 throughput; resource usage within budgets |
| **Phase 3 – Remote Operations** | Hybrid transport, remote CLI verbs, admin RPCs, mDNS discovery | Gate: remote `copy`/`mirror`/`move`, `scan`, `list`/`ls`, `find`, `du`, `df`, `rm`, `profile` all succeed with fallback + error handling |
| **Phase 4 – Production Hardening** | Packaging, documentation, service polish | Gate: packages & installers, integration suite, docs complete |
| **Phase 3.5 – RDMA (Deferred)** | RDMA/RoCE integration | Deferred until after core release (recorded in plan for future work) |

### Detailed Gates
- **Phase 2 Gate**:  
  - `blit copy` / `blit mirror` / `blit move` (local) deliver within FAST target.  
  - Streaming planner heartbeat + stall detector operational.  
  - Predictor/history toggles documented; CLI remains quiet by default with verbose hooks.  
  - Unit/integration tests green.

- **Phase 2.5 Gate**:  
  - Benchmarks: large-file, many-small-files, mixed workloads, incremental mirrors.  
  - Throughput ≥ 95 % of v1; memory/cpu within budgets.  
  - Bench results logged (DEVLOG + Phase 2.5 doc).

- **Phase 3 Gate**:  
  - Remote transfer verbs operate across the network with hybrid transport and gRPC fallback.  
  - `blit scan` discovers daemons via mDNS.  
  - `blit list` / `blit ls` enumerate modules and paths; forbid traversal outside exports.  
  - `blit-utils` verbs (`scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile`) succeed against daemon with read-only/chroot enforcement.  
  - Structured progress events exist for future GUIs; CLI remains quiet unless verbose.  
  - Integration tests cover remote transfer + admin flows.

- **Phase 4 Gate**:  
  - Packages built for supported platforms (Linux, macOS, Windows).  
  - Installation/configuration docs ready (daemon config, mDNS, service guidance).  
  - End-to-end test suite covers local and remote operations.  
  - Release checklist complete.

---

## 3. Decision Log

| Decision | Status | Notes |
|----------|--------|-------|
| Transport model | ✅ Hybrid (gRPC control + TCP data plane with secure tokens, auto fallback) |
| Error handling | ✅ `eyre`/`color-eyre` for CLI + daemon, consistent context-rich errors |
| Async runtime | ✅ Tokio across crates |
| Progress UX | ✅ CLI quiet by default; structured events exposed for GUIs/debug |
| Telemetry | ✅ Local JSONL history (optional opt-out); `blit-utils profile` surfaces data |
| Environment variables | ✅ Not used for configuration; precedence is CLI flag → config file |

Future architectural decisions must be recorded here and in DEVLOG before implementation.

---

## 4. Workflow Documents

Use these documents for day-to-day execution:

- **[WORKFLOW_PHASE_2.md](./WORKFLOW_PHASE_2.md)** – Streaming orchestrator, predictor, local CLI UX.
- **[WORKFLOW_PHASE_2.5.md](./WORKFLOW_PHASE_2.5.md)** – Benchmark/validation checklist.
- **[WORKFLOW_PHASE_3.md](./WORKFLOW_PHASE_3.md)** – Remote transport, CLI verbs, admin RPCs, mDNS.
- **[WORKFLOW_PHASE_4.md](./WORKFLOW_PHASE_4.md)** – Packaging, docs, integration tests.

Phase 3.5 (RDMA) is intentionally left out of the active workflow docs until the core release is complete.

---

## 5. Coordination & Reporting

- **Status Tracking** – Update `PROJECT_STATE_ASSESSMENT.md` as phases advance or blockers arise.  
- **Task Tracking** – Check off `TODO.md` items and add new ones as the plan expands.  
- **Daily Logging** – Append significant work, benchmarks, and decisions to `DEVLOG.md` (UTC timestamps).  
- **Agent Hand-offs** – Summaries belong in DEVLOG/TODO/workflow docs; `agentcomms/` is for real-time coordination only.

---

## 6. Recommended Execution Rhythm

1. **Before starting work** – Read the relevant phase workflow, PLAN v6, and latest DEVLOG entries.
2. **During work** – Keep TODO and workflow docs in sync with progress; capture notable findings immediately.
3. **After completing a task** – Log in DEVLOG, update TODO, and ensure tests/benchmarks are recorded.
4. **At phase gates** – Run the appropriate checklists; halt if criteria are not met.
5. **When making decisions** – Update this master workflow and DEVLOG so future contributors inherit the full context.

---

## 7. Contact Points & Next Steps

- **Questions about plan scope** → greenfield_plan_v6.md  
- **Live status / percent complete** → PROJECT_STATE_ASSESSMENT.md  
- **Implementation steps** → Current phase workflow document  
- **Upcoming work** → Next phase workflow document

Keep this master workflow lean: record new decisions, adjust gate definitions when requirements change, and ensure downstream documents stay aligned with Plan v6.
