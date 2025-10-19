# Blit v2 Master Workflow

**Project**: Blit v2 - Hybrid Transport File Transfer System
**Strategy**: Phased greenfield development with continuous validation
**Current Phase**: Phase 2 (Orchestrator & Local Operations)
**Target Completion**: Open-ended; progress advances only when quality gates pass

## Overview

This master workflow coordinates all phases of the Blit v2 development effort. The project follows a **validation-driven approach** where each phase must meet quality gates before proceeding to the next.

### Core Principles (v5 Inviolable Standards)

1. **FAST**: Start copying immediately, minimize perceived latency (≤1s)
2. **SIMPLE**: No user tunables for speed; planner chooses best path automatically
3. **RELIABLE**: Mirror deletions, checksums, and correctness outweigh speed
4. **PRIVATE**: No external performance reporting; user data never leaves the machine

### Development Principles

1. **Build Working, Then Build Complete**: Each phase delivers functional capability
2. **Performance Parity**: Must match or exceed v1 performance benchmarks (≥95%)
3. **Test-First**: Integration tests guide implementation
4. **Incremental Complexity**: Start local, add network, add production features
5. **Documentation Survival**: Update docs + DEVLOG to survive context resets

### Phase Dependencies

```
Phase 0 (Foundation) ✅
    ↓
Phase 1 (gRPC Scaffolding) ✅
    ↓
Phase 2 (Local Operations) ← CURRENT
    ↓
Phase 2.5 (Validation Gate) ← CRITICAL CHECKPOINT
    ↓
Phase 3 (Remote Operations)
    ↓
Phase 4 (Production Hardening)
```

## Phase Summary

| Phase | Status | Duration (guidance) | Completion | Critical Path |
|-------|--------|----------------------|------------|---------------|
| 0: Foundation | ✅ Complete | n/a | 100% | Core logic porting |
| 1: gRPC Scaffolding | ✅ Complete | n/a | 100% | Proto definition |
| 2: Orchestrator | ⚠️ In Progress | Open | ~70% | Streaming orchestrator + local heuristics |
| 2.5: Validation | ⚠️ In Progress | Open | ~40% | Performance benchmarks & parity tuning |
| 3: Remote Ops | ⏳ Not Started | Open | 0% | Hybrid transport |
| 4: Production | ⏳ Not Started | Open | 0% | TLS & packaging |
| **Total** | **In Progress** | **Open** | **~60%** | |

## Current State

**Last Completed**: Phase 1 (gRPC Scaffolding)
**Current Focus**: Phase 2 streaming heuristics + Windows large-file parity (Phase 2.5 gate)
**Next Milestone**: Validate Windows benchmarks (1–4 GiB) and close Phase 2.5

See [PROJECT_STATE_ASSESSMENT.md](./PROJECT_STATE_ASSESSMENT.md) for detailed status.

## Quality Gates

Each phase has **mandatory quality gates** that must pass before proceeding:

### Phase 2 Gate: Local Operations Functional
- ✅ `cargo build` succeeds for all crates
- ✅ `cargo test` passes for blit-core
- ✅ `blit mirror <src> <dst>` works locally
- ✅ `blit copy <src> <dst>` works locally
- ✅ Zero-copy logic executes correctly
- ✅ Integration tests pass

### Phase 2.5 Gate: Performance Validated
- ✅ Single large file (4GiB) within 5% of v1 speed
- ✅ Many small files (100k) within 5% of v1 speed
- ✅ Memory usage within acceptable bounds
- ✅ CPU utilization reasonable

**🚨 HARD GATE**: Do not proceed to Phase 3 if Phase 2.5 fails. Fix performance issues first.

### Phase 3 Gate: Remote Operations Functional
- ✅ Push operation works over network
- ✅ Pull operation works over network
- ✅ List and Purge operations work
- ✅ Network error handling robust
- ✅ Progress reporting functional

### Phase 4 Gate: Production Ready
- ✅ TLS working for control plane
- ✅ Authentication functional
- ✅ Platform-specific packages built
- ✅ End-to-end integration tests pass
- ✅ Documentation complete

## Architectural Decisions

### Decision 1: Transport Model

**Status**: ✅ **DECIDED** (per greenfield_plan_v5.md)
**Selected**: **Hybrid Transport** (gRPC control plane + TCP data plane)

**Architecture**:
- **Control Plane**: gRPC for manifests, negotiations, progress, list/purge operations
- **Data Plane**: Raw TCP negotiated via one-time cryptographically strong token
- **Zero-Copy**: Linux sendfile/splice/copy_file_range on data plane
- **Fallback**: Automatic gRPC-streamed data when TCP port unreachable (firewall/NAT)
- **Override**: Advanced `--force-grpc-data` / `BLIT_FORCE_GRPC_DATA=1` for locked-down environments

**Security Requirements**:
- Data-plane token must be cryptographically strong (e.g., signed JWT with nonce + expiry)
- Server must bind accepted socket to token before zero-copy writes
- Prevents replay attacks

**Rationale**:
- Matches v1 data-path performance
- Full control over zero-copy optimizations
- Prepares for RDMA extension (Phase 3.5)
- Maintains v5 plan FAST/SIMPLE/RELIABLE/PRIVATE principles

### Decision 2: Error Handling Strategy

**Status**: ✅ **DECIDED**
**Selected**: **`eyre`** (with `color-eyre` for CLI)

**Rationale**:
- Blit is a user-facing CLI tool - UX matters
- Beautiful error formatting with color-coded output
- Clear error chains show full context for debugging
- Drop-in replacement compatible with existing `anyhow` call sites (same API surface)
- Modern CLI tools use similar approaches (ripgrep, fd, bat)
- File transfer errors benefit from clear path/permission/network diagnostics

**Implementation**:
- `blit-cli`: Use `eyre::Result<()>` with `color_eyre::install()`
- `blit-daemon`: Use `eyre::Result<()>`
- `blit-core`: Use `eyre::Result<T>` for public APIs
- Add dependencies: `eyre`, `color-eyre`

### Decision 3: Async Runtime

**Status**: ✅ **DECIDED**
**Selected**: **Tokio** (required by Tonic gRPC framework)
**Implication**: Use Tokio throughout codebase for consistency

### Decision 4: Progress Reporting Architecture

**Status**: ✅ **DECIDED**
**Selected**: Quiet CLI with progress hooks for future GUIs/daemons

**Details**:
- CLI keeps transfer output minimal to avoid any performance impact.
- Orchestrator exposes structured progress events so GUI layers can render rich status.
- Debug/verbose modes may surface extra telemetry; default runs stay silent.
- Implementation guidance lives in WORKFLOW_PHASE_2.md and LOCAL_TRANSFER_HEURISTICS.md.

## Phase Workflows

Detailed workflows for each phase are in separate documents:

- **[WORKFLOW_PHASE_2.md](./WORKFLOW_PHASE_2.md)** - Streaming Orchestrator & Local Operations (v5 design)
- **[WORKFLOW_PHASE_2.5.md](./WORKFLOW_PHASE_2.5.md)** - Performance & Validation Checkpoint
- **[WORKFLOW_PHASE_3.md](./WORKFLOW_PHASE_3.md)** - Hybrid Remote Operations (gRPC + TCP)
- **[WORKFLOW_PHASE_4.md](./WORKFLOW_PHASE_4.md)** - Production Hardening & Packaging

**Note**: Phase 3.5 (RDMA Enablement) documented in [greenfield_plan_v5.md](./greenfield_plan_v5.md) is post-v2.0 work

## Phase 2: Streaming Orchestrator & Local Operations (CURRENT)

**Goal**: Deliver v5 streaming local pipeline - streaming planner, adaptive predictor, performance history, stall detection
**Duration**: 7-10 days (per v5 plan)
**Status**: In progress (Phase 2.5 benchmarks showing parity)

### Immediate Blockers (Day 1, Hour 1)

1. **Fix globset dependency** (5 minutes)
   ```bash
   # In crates/blit-core/Cargo.toml
   # Change: globset = "^1.1"
   # To: globset = "0.4"
   ```

2. **Verify build** (2 minutes)
   ```bash
   cargo build
   cargo test -p blit-core
   ```

### Core Tasks (Days 1-10)

See [WORKFLOW_PHASE_2.md](./WORKFLOW_PHASE_2.md) for detailed breakdown.

**Summary** (per v5 plan):
- Streaming planner with heartbeat scheduler (1s default, 500ms when workers starved)
- 10s stall detector (planner + workers idle) with error messaging
- Fast-path routing (tiny manifests → direct copy, huge files → large-file worker)
- Adaptive predictor fed by local performance history (perceived latency ≤ 1s)
- ✅ Planner auto-tunes buffer sizing and worker counts (no manual speed flags; debug limiters hidden behind `--workers`)
- CLI progress indicator (spinner + throughput + ETA)
- `blit diagnostics perf` command
- Local performance history in capped JSONL (~/.config/blit/perf_local.jsonl)
- Comprehensive unit/integration tests for streaming behavior
- TODO: Generate CLI/manpage documentation covering debug options, diagnostics, and upcoming transport flags

## Phase 2.5: Performance Validation (CRITICAL GATE)

**Goal**: Verify v2 local performance ≥95% of v1
**Duration**: 1-2 days
**Status**: Not started

### Benchmark Scenarios

1. **Large File**: 4GiB single file
2. **Many Small Files**: 100k files (1-10KB each)
3. **Mixed Workload**: 1k files (varying sizes)
4. **Mirror Update**: Incremental changes to large directory tree

### Success Criteria

| Metric | v1 Baseline | v2 Target | Status |
|--------|-------------|-----------|--------|
| Large file (4GiB) | X GB/s | ≥95% of v1 | ⏳ |
| 100k small files | X sec | ≥95% of v1 | ⏳ |
| Mixed workload | X sec | ≥95% of v1 | ⏳ |
| Memory usage | X MB | ≤110% of v1 | ⏳ |

**Gate Decision**:
- ✅ Pass → Proceed to Phase 3
- ❌ Fail → Profile, optimize, or implement hybrid transport

## Phase 3: Hybrid Remote Operations

**Goal**: Implement gRPC control plane + TCP data plane with zero-copy
**Duration**: 8-12 days (per v5 plan)
**Status**: Not started

**Prerequisites**:
- Phase 2 complete
- Phase 2.5 passed (≥95% v1 parity validated)

**Key Features** (per v5 plan):
- Control plane handshake with needlist exchange
- One-time cryptographically strong token for data plane
- TCP data plane with zero-copy (sendfile/splice/copy_file_range on Linux)
- Automatic fallback to gRPC-streamed data when TCP port unreachable
- Advanced override: `--force-grpc-data` / `BLIT_FORCE_GRPC_DATA=1`
- Network tuning: disable Nagle, large send/recv buffers, optional BBR
- Progress signals from remote operations to CLI
- Remote performance history: local-only JSONL store (no runtime prompts), optional manual `blit-utils profile`, daemon idle self-profiling if benchmarks prove value

See [WORKFLOW_PHASE_3.md](./WORKFLOW_PHASE_3.md) for details.

## Phase 4: Production Hardening

**Goal**: Deployable, secure, documented system
**Duration**: 5-7 days
**Status**: Not started

**Prerequisites**:
- Phase 3 complete
- All core features working

See [WORKFLOW_PHASE_4.md](./WORKFLOW_PHASE_4.md) for details.

## Parallelization Opportunities

### Within Phases

**Phase 2**:
- ⚡ Orchestrator implementation || CLI wiring (after orchestrator API defined)
- ⚡ Unit tests || Integration tests (can develop in parallel)

**Phase 3**:
- ⚡ Push implementation || Pull implementation (independent services)
- ⚡ List service || Purge service (independent)

**Phase 4**:
- ⚡ TLS implementation || Package building
- ⚡ Documentation || Integration tests

### Across Phases (Advanced)

**Speculative Development**:
- While in Phase 2, can prototype Phase 3 hybrid transport negotiation
- While in Phase 3, can prepare Phase 4 packaging scripts

**Caution**: Don't parallelize core features until current phase validated

## Risk Management

### High-Risk Items

| Risk | Phase | Mitigation |
|------|-------|------------|
| Performance regression | 2.5 | Early benchmarking, hybrid transport option |
| Hybrid transport complexity | 3 | Start with gRPC-only, upgrade if needed |
| Platform compatibility | 4 | Reuse v1 Windows logic, early testing |
| Scope creep | All | Strict MVP adherence, defer nice-to-haves |

### Rollback Points

**After Phase 2.5 Failure**:
- Option 1: Optimize gRPC path (profile-guided)
- Option 2: Implement hybrid transport
- Option 3: Investigate zero-copy improvements

**After Phase 3**:
- If network issues arise, can simplify to basic streaming
- Can defer TLS to later iteration

## Success Metrics

### Phase-Level Metrics

- **Phase 2**: `blit mirror <src> <dst>` works locally, tests pass
- **Phase 2.5**: Performance ≥95% of v1
- **Phase 3**: Remote operations work reliably
- **Phase 4**: Deployable packages, TLS working, docs complete

### Project-Level Metrics

- **Functional**: All v1 operations supported
- **Performance**: ≥95% of v1 speed across all scenarios
- **Quality**: >80% test coverage, no critical bugs
- **Usability**: Drop-in replacement for v1 CLI

## Communication & Tracking

### Daily Status Updates (during active development)

Update `DEVLOG.md` with:
- Tasks completed
- Blockers encountered
- Decisions made
- Next steps

### Checkpoint Reviews

- **End of Phase 2**: Local operations demo
- **End of Phase 2.5**: Performance results review
- **End of Phase 3**: Remote operations demo
- **End of Phase 4**: Release candidate review

## Next Actions

### Immediate (Today)
1. Fix globset dependency in `blit-core/Cargo.toml`
2. Verify build succeeds
3. Make transport architecture decision
4. Begin orchestrator implementation

### This Week (Phase 2)
1. Implement `TransferOrchestrator`
2. Wire CLI to orchestrator
3. Add integration tests
4. Complete Phase 2 quality gate

### Next Week (Phase 2.5)
1. Create benchmark harness
2. Run v1 baseline benchmarks
3. Run v2 benchmarks
4. Analyze results and make go/no-go decision

### Following Weeks (Phase 3 & 4)
See phase-specific workflows

## Appendix: Command Reference

### Development Commands

```bash
# Build all crates
cargo build

# Build specific crate
cargo build -p blit-core

# Test all
cargo test

# Test specific crate
cargo test -p blit-core

# Run daemon
cargo run -p blit-daemon

# Run CLI
cargo run -p blit-cli -- mirror <src> <dst>
```

### Workflow Commands

```bash
# Check current status
cat TODO.md
cat DEVLOG.md

# Update task status
# Edit TODO.md and check off completed items

# Log progress
echo "**2025-10-16 HH:MM:00Z** - **ACTION**: <description>" >> DEVLOG.md
```

## Document History

| Date | Author | Change |
|------|--------|--------|
| 2025-10-16 | Claude | Initial master workflow creation |
| 2025-10-17 | Claude | Updated to align with greenfield_plan_v5.md: transport decision, phase durations, v5 principles |
