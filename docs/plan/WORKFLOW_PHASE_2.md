# Phase 2: Streaming Orchestrator & Local Operations (historical)

**Goal**: Historical proposal for the local transfer pipeline defined in plan v6 (streaming planner, adaptive predictor, local performance history, and progress UX) while keeping FAST/SIMPLE/RELIABLE/PRIVATE principles intact.
**Prerequisites**: Phase 0 & 1 complete (workspace, ported modules, gRPC scaffolding).
**Status**: Historical
**Critical Path**: None; this document is not an active implementation or release checklist.

## Reality correction (2026-07-22)

This plan was previously and incorrectly marked Shipped. The promised
`TransferFacade::stream_local_plan`, `PlannerEvent`, `drive_planner_events`,
planner heartbeat, and 10-second local-planner stall detector never shipped.
The adaptive predictor was recorded but never consumed for routing. The old
orchestrator fast paths existed for a time, then were deleted at otp-11b after
an unsound journal skip was found.

Current local, push, pull, and remote-to-remote transfers use the unified
`transfer_session`. It streams manifest entries, applies destination diffs in
bounded chunks, and has transfer/data-plane stall protection; those are not
the local planner state machine promised here. Current behavior and release
status live in `docs/TRANSFER_SESSION.md`, `docs/STATE.md`, and
`docs/RELEASE_READINESS.md`. The remaining text is retained as historical
design intent, not proof that a feature shipped.

## Success Criteria

- `blit copy` / `blit mirror` start emitting data within ≤ 1 s of command invocation for qualifying workloads.
- Planner flushes batches incrementally; stall detector aborts with clear messaging after 10 s of inactivity.
- CLI remains quiet by default; progress mode exposes manifest + throughput events for GUIs/verbose listeners.
- Telemetry log and `blit diagnostics perf` work; predictor adjusts routing automatically.
- Remote transfers expose first-payload latency via `--progress`/`--verbose` so benchmarks can confirm < 1 s start.
- All unit/integration tests (including new fast-path scenarios) pass; benchmarks meet Phase 2.5 targets.

## Guiding Principles

1. **No user tunables** – Planner owns performance decisions. The sole debug limiter (`--workers`) must be clearly labelled, pause “FAST” guarantees when active, and remain hidden from normal help output (documented in `docs/cli/blit.1.md`).
2. **Telemetry stays local** – JSONL log under config dir, capped to ~1 MiB. Opt-out should be driven by CLI/config settings (no environment variables once work completes).
3. **Documentation-first** – Update plan/docs/DEVLOG as tasks complete to survive context resets.

## Work Breakdown

### 2.1 Streaming Planner Infrastructure

| Task | Description | Deliverable |
|------|-------------|-------------|
| 2.1.1 | Refactor `TransferFacade::build_local_plan` into an async stream producing batches. | Not shipped as specified; named API never existed in the completed tree. |
| 2.1.2 | Implement heartbeat scheduler (1 s default, adaptive 0.5 s when workers are idle). | Not shipped. |
| 2.1.3 | Add 10 s stall detector (planner + workers idle) with error messaging. | Not shipped; current transfer stall guards are a different mechanism. |
| 2.1.4 | Wire fast-path routing: tiny manifests → direct copy; single huge file → large-file worker. | Shipped historically in the old orchestrator, then retired with that architecture at otp-11b. |

### 2.2 Adaptive Predictor & Telemetry

| Task | Description | Deliverable |
|------|-------------|-------------|
| 2.2.1 | Implement local performance history writer (capped JSONL). | `perf_history.rs` with rotate-on-size logic. |
| 2.2.2 | Build EMA-based predictor segmented by filesystem profile. | Predictor struct + serde (for persistence). |
| 2.2.3 | Integrate predictor into orchestrator routing decisions. | Not shipped; predictor output was never consumed for routing. |
| 2.2.4 | Add `blit diagnostics perf` CLI command. | ✅ Command prints recent runs + stats. |
| 2.2.5 | Add CLI/config toggle for telemetry (`profile` command remains visible). Replace environment variable usage. | Diagnostics toggles (`blit diagnostics perf --enable/--disable`) + settings file. |

**Note:** Final release toggle (enabled by default vs. opt-in) will be decided from benchmark evidence; once committed, the setting remains stable across releases. Implementation must avoid environment-variable configuration.

### 2.3 CLI UX & Flag Cleanup

| Task | Description | Deliverable |
|------|-------------|-------------|
| 2.3.1 | Keep planner fully automatic; manual speed flags are not exposed (only debug limiters). | CLI parsing update + release note. |
| 2.3.2 | Keep CLI quiet while exposing progress hooks (events/verbose logging) for GUI surfaces. | Progress event plumbing + docs. |
| 2.3.3 | When debug limiters are used, make it obvious (CLI banner/log) that FAST mode is capped. | Logging hooks + tests. |

### 2.4 Testing & Benchmarks

| Task | Description | Deliverable |
|------|-------------|-------------|
| 2.4.1 | Extend unit tests for planner streaming, predictor, stall detector. | Not shipped; the claimed planner/stall machinery and tests do not exist. |
| 2.4.2 | Add integration tests covering 1-file, 8-file, 100k-file, checksum mirror scenarios. | `tests/integration/local_transfers.rs` exercises tiny vs streaming manifests; large/100k cases still pending. |
| 2.4.3 | Keep macOS/Linux + Windows benchmarks v2-only (synthetic payload, perf-history disabled by default) and capture rsync/robocopy baselines. | `scripts/bench_local_mirror.sh` (vs `rsync`) / `scripts/windows/bench-local-mirror.ps1` (vs `robocopy`) emit summary timings + log paths. |
| 2.4.4 | Quantify performance history warm-up impact (first vs. 10th vs. 100th run) across representative workloads. | Benchmark report captured in docs with hard numbers and log references. |

### 2.5 Documentation & Logging

| Task | Description | Deliverable |
|------|-------------|-------------|
| 2.5.1 | Keep `DEVLOG.md` updated per milestone. | Timestamped entries. |
| 2.5.2 | Update knowledge docs (`LOCAL_TRANSFER_HEURISTICS.md`, plan summaries) as behaviour changes. | Docs remain authoritative. |
| 2.5.3 | Capture handoff context via DEVLOG/TODO/workflow updates; reserve `agentcomms/` for live coordination. | Docs remain authoritative between sessions. |

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

## Historical exit checklist (never completed)

- [ ] Streaming planner + heartbeat + stall detector merged. **Not shipped.**
- [ ] Performance history/predictor integrated; diagnostics command works. **History/diagnostics shipped, but predictor-driven routing did not.**
- [x] CLI progress indicator + flag cleanup in place.
- [ ] New unit/integration tests pass.
    - Local run: `cargo test --all-targets` (2025-10-18)
    - Local run: `cargo test -p blit-core`; `cargo test -p blit-cli` (2025-10-27)
    - Remote CI pending (runs once repo is hosted)
- [ ] Benchmarks executed and recorded in Phase 2.5 doc.
- [ ] Remote push streaming validated (<1 s first-byte) across Linux/macOS/Windows with logs captured.
- [ ] DEVLOG and docs updated with outcomes.

This unchecked historical list is not a current release gate; use
`docs/RELEASE_READINESS.md` for the release boundary.
