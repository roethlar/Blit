and# Phase 2: Streaming Orchestrator & Local Operations

**Goal**: Deliver the v5 local transfer pipeline (streaming planner, adaptive predictor, performance history, and progress UX) while keeping FAST/SIMPLE/RELIABLE/PRIVATE principles intact.
**Prerequisites**: Phase 0 & 1 complete (workspace, ported modules, gRPC scaffolding).
**Status**: In progress (streaming planner + fast-path routing in place)
**Critical Path**: Adaptive predictor/performance history, CLI progress UX.

## Success Criteria

- `blit copy` / `blit mirror` start emitting data within ≤ 1 s of command invocation for qualifying workloads.
- Planner flushes batches incrementally; stall detector aborts with clear messaging after 10 s of inactivity.
- CLI shows throughput + ETA progress for local operations.
- Telemetry log and `blit diagnostics perf` work; predictor adjusts routing automatically.
- All unit/integration tests (including new fast-path scenarios) pass; benchmarks meet Phase 2.5 targets.

## Guiding Principles

1. **No user tunables** – `--ludicrous-speed` becomes a no-op; planner owns performance decisions.
2. **Telemetry stays local** – JSONL log under config dir, capped to ~1 MiB, with optional opt-out.
3. **Documentation-first** – Update plan/docs/DEVLOG as tasks complete to survive context resets.

## Work Breakdown

### 2.1 Streaming Planner Infrastructure

| Task | Description | Deliverable |
|------|-------------|-------------|
| 2.1.1 | Refactor `TransferFacade::build_local_plan` into an async stream producing batches. | ✅ `TransferFacade::stream_local_plan` emitting `PlannerEvent` |
| 2.1.2 | Implement heartbeat scheduler (1 s default, adaptive 0.5 s when workers are idle). | ✅ Heartbeat loop in `drive_planner_events` |
| 2.1.3 | Add 10 s stall detector (planner + workers idle) with error messaging. | ✅ Stall guard in `drive_planner_events`; Windows+Linux verified |
| 2.1.4 | Wire fast-path routing: tiny manifests → direct copy; single huge file → large-file worker. | ✅ Fast-path routing implemented in orchestrator; dedicated tests follow under 2.4.x. |

### 2.2 Adaptive Predictor & Telemetry

| Task | Description | Deliverable |
|------|-------------|-------------|
| 2.2.1 | Implement local performance history writer (capped JSONL). | `perf_history.rs` with rotate-on-size logic. |
| 2.2.2 | Build EMA-based predictor segmented by filesystem profile. | Predictor struct + serde (for persistence). |
| 2.2.3 | Integrate predictor into orchestrator routing decisions. | Orchestrator chooses streaming vs. fast-path based on prediction. |
| 2.2.4 | Add `blit diagnostics perf` CLI command. | Command prints recent runs + stats. |

**Note:** Final release toggle (enabled by default vs. opt-in) will be decided from benchmark evidence; once committed, the setting remains stable across releases.

### 2.3 CLI UX & Flag Cleanup

| Task | Description | Deliverable |
|------|-------------|-------------|
| 2.3.1 | Remove `--ludicrous-speed` behaviour; leave as no-op with warning slated for removal. | CLI parsing update + release note. |
| 2.3.2 | Add progress indicator (spinner + throughput + ETA) using `indicatif`. | Shared progress module for copy/mirror. |
| 2.3.3 | Ensure verbose mode prints heartbeat stats; default remains quiet until stall. | Logging hooks + tests. |

### 2.4 Testing & Benchmarks

| Task | Description | Deliverable |
|------|-------------|-------------|
| 2.4.1 | Extend unit tests for planner streaming, predictor, stall detector. | `transfer_engine` streaming tests passing on Windows/Linux |
| 2.4.2 | Add integration tests covering 1-file, 8-file, 100k-file, checksum mirror scenarios. | TODO |
| 2.4.3 | Update `scripts/bench_local_mirror.sh` to record performance history snapshots and run new scenarios. | Script outputs ratio + writes to log. |
| 2.4.4 | Quantify performance history warm-up impact (first vs. 10th vs. 100th run) across representative workloads. | Benchmark report captured in docs with hard numbers and log references. |

### 2.5 Documentation & Logging

| Task | Description | Deliverable |
|------|-------------|-------------|
| 2.5.1 | Keep `DEVLOG.md` updated per milestone. | Timestamped entries. |
| 2.5.2 | Update knowledge docs (`LOCAL_TRANSFER_HEURISTICS.md`, plan summaries) as behaviour changes. | Docs remain authoritative. |
| 2.5.3 | Sync `agent_comms/codex_resume.md` (or equivalent) after major steps. | Resume file stays current. |

## Execution Order

1. **Planner stream & heartbeat (2.1.1–2.1.3)** – foundation for all other tasks.
2. **Fast-path routing (2.1.4)** – once streaming works, ensure trivial cases bypass overhead.
3. **Telemetry + predictor (2.2.x)** – requires streaming output to sample real timings.
4. **CLI/UX updates (2.3.x)** – dependent on orchestrator progress callbacks.
5. **Testing + benchmarks (2.4.x)** – continuous as features land.
6. **Docs/logging (2.5.x)** – ongoing; do not defer to the end.

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Streaming refactor introduces deadlocks | Extensive unit tests; use async-aware mutexes | 
| Predictor destabilises routing | Start with conservative defaults; log mispredictions; allow performance history opt-out |
| Progress UI regresses non-interactive usage | Provide quiet mode, ensure logs respect TTY detection |
| Telemetry log growth | Size cap + rotation; disable when env set |

## Exit Checklist (Phase 2 Complete)

- [ ] Streaming planner + heartbeat + stall detector merged.
- [ ] Telemetry/predictor integrated; diagnostics command works.
- [ ] CLI progress indicator + flag cleanup in place.
- [ ] New unit/integration tests pass; CI green.
- [ ] Benchmarks executed and recorded in Phase 2.5 doc.
- [ ] DEVLOG and docs updated with outcomes.
