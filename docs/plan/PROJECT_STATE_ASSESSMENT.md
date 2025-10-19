# Blit v2 Project State Assessment

**Date**: 2025-10-16
**Status**: Phase 2 In Progress
**Overall Completion**: ~40%

## Executive Summary

Blit v2 is a greenfield rewrite using a Cargo workspace architecture. Phase 0 (Foundation) and Phase 1 (gRPC Scaffolding) are complete. Phase 2 (Orchestrator & Local Operations) has been initiated but is blocked by:

1. **Critical**: Build failure due to dependency version mismatch (`globset ^1.1` doesn't exist)
2. **Critical**: Empty `orchestrator.rs` file - core component not yet implemented
3. **Architectural Decision Needed**: Proto implementation doesn't match hybrid transport plan

## Phase Completion Status

### ✅ Phase 0: Workspace & Core Logic Foundation (COMPLETE)

**Completion**: 100%
**Quality**: High - all core modules ported successfully

**Deliverables**:
- ✅ Cargo workspace with 4 crates: `blit-core`, `blit-cli`, `blit-daemon`, `blit-utils`
- ✅ Core modules ported from v1:
  - `checksum.rs` - File checksumming logic
  - `fs_enum.rs` - Filesystem enumeration primitives
  - `enumeration.rs` - High-level directory traversal
  - `mirror_planner.rs` - Mirror operation planning (including Windows logic)
  - `buffer.rs` - Buffer management
  - `zero_copy.rs` - Zero-copy primitives (sendfile, splice)
- ✅ Module structure defined in `blit-core/src/lib.rs`
- ✅ Tests ported (assumed, need verification)

**Files Created**:
```
crates/blit-core/src/checksum.rs
crates/blit-core/src/fs_enum.rs
crates/blit-core/src/enumeration.rs
crates/blit-core/src/mirror_planner.rs
crates/blit-core/src/buffer.rs
crates/blit-core/src/zero_copy.rs
crates/blit-core/src/lib.rs
```

### ✅ Phase 1: gRPC API & Service Scaffolding (COMPLETE)

**Completion**: 100%
**Quality**: Medium - functional but architecture mismatch with plan

**Deliverables**:
- ✅ `proto/blit.proto` created with full service definition
- ✅ Generated code module: `blit-core/src/generated/mod.rs`
- ✅ Skeleton daemon: `blit-daemon/src/main.rs` with `BlitService` struct
- ✅ Skeleton CLI: `blit-cli/src/main.rs` with `Commands` enum
- ✅ gRPC infrastructure integrated via Tonic

**Files Created**:
```
proto/blit.proto
crates/blit-core/src/generated/mod.rs
crates/blit-core/build.rs (assumed)
crates/blit-daemon/src/main.rs
crates/blit-cli/src/main.rs
```

**Status Notes**:
- Hybrid transport design retained per v5 plan; proto updates pending Phase 3 implementation.
- Streaming orchestrator + fast-path routing complete; performance history/predictor work still pending.
- macOS test harness (`scripts/macos/run-blit-tests.sh`) mirrors the Windows runner for parity logging.

### ⚠️ Phase 2: Orchestrator & Local Operations (IN PROGRESS)

**Completion**: ~70%
**Status**: Streaming orchestrator and fast-path routing implemented; performance history/predictor + UX pending; Windows local performance now competitive for ≤512 MiB workloads (CopyFileExW)

**Completed (this session)**:
- ✅ `TransferFacade::stream_local_plan` emitting streaming batches
- ✅ `TaskStreamSender`/`create_task_stream`/`execute_streaming_plan`
- ✅ `TransferOrchestrator` with heartbeat, stall detection, mirror deletions
- ✅ Fast-path routing for tiny manifests and single huge files
- ✅ Local performance history writer wired into orchestrator (JSONL, capped, env toggle)
- ✅ Predictor state scaffold (`perf_predictor.rs`) loading/saving coefficients; tiny fast path now uses predictions once observations exist
- ✅ Unit tests covering fast-path predictor gating (`orchestrator::tests`)
- ✅ Windows + Linux unit tests for `transfer_engine` streaming path
- ✅ Local benchmark harnesses (macOS/Linux + Windows) run v2-only binaries with perf-history disabled and include rsync/robocopy baselines
- ✅ Windows local-copy path now uses CopyFileExW fast path; 512 MiB benchmark beats robocopy (~7% faster)

**Remaining Work**:
- [ ] Predictor-driven routing refinements beyond tiny fast path (e.g., streaming heuristics)
- [ ] Verify new Windows large-file caching heuristics via 1–4 GiB benchmarks; iterate on worker fan-out/cache hints if parity still lags
- [x] Split `copy.rs` and `orchestrator.rs` into smaller modules to keep AI-driven edits safe before expanding Phase 3 scope
    - Result: `copy/` now hosts `mod.rs` + `windows.rs`; `orchestrator/` contains `mod.rs`, `fast_path.rs`, `planner.rs`, and `history.rs`, shrinking primary files to manageable size for future work.
- [ ] CLI progress UX + flag cleanup
- [x] Broader integration coverage for fast-path and predictor heuristics
- [ ] Integration / benchmark coverage for streaming + fast-path scenarios
- [ ] Benchmark performance history warm-up impact (run 1 vs. 10 vs. 100) and capture results in docs

### ⚙️ Phase 2.5: Performance & Validation Checkpoint (IN PROGRESS)

**Completion**: ~40%

### ⏳ Phase 3: Remote Operations (NOT STARTED)

**Completion**: 0%

### ⏳ Phase 4: Production Hardening (NOT STARTED)

**Completion**: 0%

## Critical Blockers

None currently. Previous dependency/orchestrator blockers resolved; next decisions tracked under Phase 3 planning.

## Technical Debt

1. **Test Coverage**: Streaming path covered; need fast-path + predictor unit/integration tests
2. **Documentation**: No inline documentation for ported modules
3. **Error Handling**: Unknown error handling strategy
4. **Logging**: No logging infrastructure visible

## Next Steps (Priority Order)

1. Implement adaptive performance history + predictor stack (Phase 2.2) and persist coefficients.
2. Deliver CLI progress UX / flag cleanup (Phase 2.3) with streaming hooks.
3. Add unit/integration coverage for fast-path routing & upcoming predictor (Phase 2.4).
4. Expand benchmarks and performance history capture to validate fast-path vs streaming mix (Phase 2.4/2.5).
5. Continue updating docs/DEVLOG after each milestone to maintain handoff fidelity.

## File Structure Overview

```
blit_v2/
├── Cargo.toml                    # Workspace definition
├── proto/
│   └── blit.proto               # gRPC service definition
├── crates/
│   ├── blit-core/               # Core library
│   │   ├── src/
│   │   │   ├── lib.rs          # Module exports
│   │   │   ├── checksum.rs     # ✅ Ported
│   │   │   ├── fs_enum.rs      # ✅ Ported
│   │   │   ├── enumeration.rs  # ✅ Ported
│   │   │   ├── mirror_planner.rs # ✅ Ported
│   │   │   ├── buffer.rs       # ✅ Ported
│   │   │   ├── zero_copy.rs    # ✅ Ported
│   │   │   ├── orchestrator.rs # ✅ Streaming + fast-path orchestrator
│   │   │   └── generated/
│   │   │       └── mod.rs      # ✅ gRPC generated code
│   │   └── build.rs            # Proto compilation
│   ├── blit-cli/                # CLI binary
│   │   └── src/
│   │       └── main.rs         # ✅ Skeleton with Commands enum
│   ├── blit-daemon/             # Server daemon
│   │   └── src/
│   │       └── main.rs         # ✅ Skeleton with BlitService
│   └── blit-utils/              # Utilities
│       └── src/
│           └── main.rs
├── tests/                       # Integration tests
├── TODO.md                      # Task checklist
├── DEVLOG.md                    # Development log
├── WORKFLOW_V2.md               # High-level workflow
└── greenfield_plan_v5.md        # Architecture plan (current)
```

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Hybrid transport complexity | Medium | High | Start with gRPC-only, upgrade later if needed |
| Performance below v1 | Medium | High | Early benchmarking (Phase 2.5) |
| Scope creep | Low | Medium | Strict adherence to MVP scope |
| Platform compatibility | Low | Medium | Reuse proven v1 Windows logic |

## Recommendations

1. **Immediate**: Fix globset dependency to unblock development
2. **Short-term**: Make architectural decision on transport model
3. **Short-term**: Implement orchestrator with test-driven approach
4. **Medium-term**: Establish CI/CD with performance regression testing
5. **Long-term**: Document migration path from v1 to v2
