# Blit v2 Project State Assessment

**Last Updated**: 2025-10-20  
**Author**: Nova  
**Plan Reference**: [greenfield_plan_v6.md](./greenfield_plan_v6.md)

---

## 1. Executive Summary

Blit v2 continues under the Feature Completeness & Transport plan (v6). Core-local functionality (Phases 0–2) is largely in place: the streaming orchestrator, fast-path routing, Windows CopyFileEx heuristics, and performance history/predictor scaffolding all exist. Remote work (Phase 3) is underway with hybrid transport scaffolding and initial remote pull support, but the CLI verb surface and admin utilities still need to be realigned with the new plan. Phase 2.5 benchmarks and documentation refreshes remain open items.

**High-Level Status**

| Phase | Status | Notes |
|-------|--------|-------|
| Phase 0 – Foundation | ✅ Complete | Workspace + core modules ported |
| Phase 1 – gRPC Scaffolding | ✅ Complete | Proto + tonic scaffolding live |
| Phase 2 – Local Ops | ⚠️ In Progress | Streaming planner, predictor, CopyFileEx heuristics done; UX/tests/benchmarks outstanding |
| Phase 2.5 – Validation | ⚠️ In Progress | Large + small + mixed workloads GO on Linux; incremental mirror still below target (needs tuning + macOS/Windows reruns) |
| Phase 3 – Remote Ops | ⚠️ In Progress | Hybrid transport + remote pull implemented; CLI verb realignment + admin RPCs pending |
| Phase 4 – Production | ⏳ Not Started | Packaging/docs/tests to follow |
| Phase 3.5 – RDMA | Deferred | Documented for post-release work |

---

## 2. Phase Highlights

### Phase 0 – Foundation (Complete)
- Cargo workspace with `blit-core`, `blit-cli`, `blit-daemon`, `blit-utils`.
- Core modules (`checksum`, `fs_enum`, `enumeration`, `mirror_planner`, `buffer`, `zero_copy`) ported and exporting via `blit-core::lib`.

### Phase 1 – gRPC Scaffolding (Complete)
- `proto/blit.proto` defined; tonic code generated via `build.rs`.
- Daemon and CLI skeletons wired to generated interfaces.

### Phase 2 – Streaming Orchestrator & Local Ops (In Progress)
- **Delivered**:  
  - `TransferFacade::stream_local_plan`, heartbeat scheduler, stall detection.  
  - Fast-path routing (tiny manifests, huge files).  
  - Windows CopyFileEx heuristic updates (sparse copy/NO_BUFFERING logic).  
  - Performance history writer + predictor scaffold (`perf_history.rs`, `perf_predictor.rs`).  
  - CLI diagnostics command (`blit diagnostics perf`).  
  - Module refactors (split `copy/` and `orchestrator/` into manageable submodules).  
- **Outstanding**:  
  - CLI progress UX + debug limiter messaging refresh.  
  - Comprehensive integration tests for fast-path vs streaming routes.  
  - Benchmark warm-up experiments (run 1 vs 10 vs 100).  
  - Documentation refresh (`WORKFLOW_PHASE_2.md`, `LOCAL_TRANSFER_HEURISTICS.md`) to reflect latest behaviour.

### Phase 2.5 – Performance Validation (In Progress)
- Benchmark harnesses (`scripts/bench_local_mirror.sh`, Windows PowerShell equivalent) exist.
- Latest reruns (2025-10-25) captured 1 GiB/4 GiB numbers across Linux/macOS/Windows; ReFS 4 GiB mirror shows blit ~41 % of robocopy throughput and needs follow-up.

### Phase 3 – Remote Operations (In Progress)
- **Delivered**:  
  - Hybrid transport control/data plane scaffolding.  
  - Remote pull end-to-end (daemon streams files; CLI writes to destination) with forced gRPC fallback tests (Windows/macOS).  
  - Proto extensions (`DataTransferNegotiation`, enriched summaries).  
  - Windows CopyFileEx improvements validated via wingpt logs.  
- **Outstanding relative to Plan v6**:  
  - CLI verb realignment (remove `push`/`pull`, restore `copy`/`mirror`/`move` remote paths, `scan`, `list`).  
  - Canonical URL parser updates to `server:/module/...` / `server://...`.  
  - blit-utils tooling (`scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile`).  
  - mDNS wiring for the new daemon config (advertising + CLI `scan`) and follow-up behaviours (chroot, read-only enforcement).
  - mDNS advertisement integration + discovery tests.  
  - Admin RPCs backing the utilities (list modules/paths, recursive enumeration, disk usage, remote delete).  
  - Integration tests covering remote transfer + admin verbs.  

### Phase 4 – Production Hardening (Not Started)
- Tasks to plan: packaging, platform docs, integration suites, service installation guidance.

---

## 3. Risks & Blockers

| Category | Risk | Mitigation |
|----------|------|------------|
| CLI/API Drift | Existing CLI still exposes `push`/`pull` and `blit://` syntax | Prioritize Phase 0 alignment tasks (CLI restructure, URL parser update) before expanding remote features |
| Admin Tooling Gap | `blit-utils` currently stub (`println!("Hello, world!")`) | Implement verbs listed in plan v6; add supporting RPCs |
| Config Behaviour | Daemon still hardcodes “default” module from CWD | Implement TOML loader + `--root` semantics; warn when running with implicit CWD |
| Benchmark Evidence | Phase 2.5 gate lacks updated numbers | Schedule benchmark run after telemetry/predictor integration stabilises |
| Documentation Drift | Many docs reference v5 push/pull workflows | Update phase workflows + CLI/man docs alongside code changes |

No hard blockers at this moment; focus is aligning implementation with plan v6 requirements.

---

## 4. Next Actions

1. **Finalize Plan Adoption**  
   - ✅ (complete with greenfield_plan_v6.md finalised); archive references to v5 in docs/workflows.

2. **Phase 0 Alignment Tasks (Feature Set)**  
   - Convert CLI command set to `copy`, `mirror`, `move`, `scan`, `list`, diagnostics.  
   - Update URL parsing to canonical syntax.  
   - Implement `blit-utils` verbs and supporting RPCs.  
  - Wire mDNS advertising + discovery on top of the new config loader.

3. **Phase 2 Finishing Work**  
 - Complete CLI progress UX clean-up and debugging messages.  
  - Expand integration tests and document planner/predictor behaviour.  
  - Run benchmark warm-up experiments and log results.
  - ✅ Daemon service/data plane modules extracted (`runtime.rs` + `service.rs`) so `main.rs` now just boots config/mdns/server (2025-10-27).  
  - ✅ CLI split into dedicated modules (`cli.rs`, `context.rs`, `diagnostics.rs`, `scan.rs`, `list.rs`, `transfers.rs`) leaving `main.rs` as a thin dispatcher (2025-10-27).  
  - ✅ Utilities binary modularised (`cli.rs`, `util.rs`, and per-verb modules) so `blit-utils/src/main.rs` only dispatches commands (2025-10-27).  
  - ✅ Core copy subsystem refactored (`compare.rs`, `file_copy.rs`, `parallel.rs`, `stats.rs`) with platform helpers isolated; public API re-exported from a slim `mod.rs` (2025-10-27).  
  - ✅ Change journal reorganised into `change_journal/{types,snapshot,tracker,util}.rs`, keeping platform probes and persistence manageable (2025-10-28).  
  - ✅ Transfer facade rewritten under `transfer_facade/{types,aggregator,planner}.rs`, leaving only re-exports in `mod.rs` (2025-10-28).  
  - ✅ Remote push client split across `client/{mod,types,helpers}` plus shared tasks (2025-10-28).  
  - ✅ Remote push streaming now flushes need-list batches immediately; gRPC fallback streams file data mid-manifest to satisfy the <1 s first-byte requirement (2025-10-28).  
  - ✅ Remote push instrumentation reports first-payload latency (visible with `--progress`/`--verbose`) to support benchmark capture (2025-10-28).  
  - ☐ Capture cross-platform benchmarks validating the sub-second start guarantee and document the procedure in Phase 2 workflow docs.

4. **Phase 3 Expansion**  
  - Implement remote transfer support using new CLI verbs (hybrid transport + fallback).  
  - Expose admin RPCs for `find`, `du`, `df`, `rm`, etc.  
  - Add remote integration tests (Linux/Windows/macOS) covering CLI + blit-utils flows.
  - Remote data plane + fallback now batch small files into tar shards (2025-10-27); verify large-manifest stress tests once implemented.
  - Streaming manifest/need-list landed (2025-10-26); follow-up: add large-manifest stress tests + document memory/throughput results.

5. **Documentation Sweep**  
   - Update all workflow docs (`WORKFLOW_PHASE_2/2.5/3/4.md`) to match plan v6 terminology and deliverables.  
   - Refresh CLI/manpage documentation (`docs/cli/blit*.md`) once command set is updated.  
   - Keep DEVLOG/TODO entries current for cross-agent continuity.

---

## 5. File & Directory Reference

```
blit_v2/
├── Cargo.toml
├── proto/blit.proto
├── crates/
│   ├── blit-core/      # Core logic (streaming planner, predictor, hybrid transport scaffolding)
│   ├── blit-cli/       # CLI binary (needs verb realignment)
│   ├── blit-daemon/    # Daemon (config loader in place; admin RPCs + mDNS outstanding)
│   └── blit-utils/     # Admin tooling (verbs to be implemented)
├── scripts/            # Benchmark/test harnesses (macOS, Windows)
├── docs/plan/          # Plan v6, workflows, state assessments (this folder)
├── TODO.md             # Task checklist
└── DEVLOG.md           # Chronological log
```

---

## 6. Communication

- **Status Updates**: Update this document when significant milestones land (phase gate, major feature).  
- **Daily Notes**: Log in `DEVLOG.md` immediately after completing meaningful work.  
- **Task Tracking**: Reflect new or completed tasks in `TODO.md`.  
- **Collaboration**: Use `agentcomms/` for real-time coordination only; long-lived context belongs in docs.

---

## 7. Revision History

| Date | Author | Notes |
|------|--------|-------|
| 2025-10-16 | Claude | Initial v5-oriented assessment |
| 2025-10-20 | Nova   | Reworked for greenfield_plan_v6 (feature completeness + transport) |
