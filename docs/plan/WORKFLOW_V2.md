# Blit v2 Development Workflow

This document outlines the high-level, phased plan for completing `blit-v2`. It serves as the strategic map for the project.

## Phase 2: Streaming Orchestrator & Local Operations

**Goal:** Realise the v5 local design—streaming planner, adaptive predictor, and performance-history-backed heuristics.

- ✅ Streaming planner + heartbeat/stall guard wired into `TransferOrchestrator`.
- 🔄 Next: performance history/predictor, progress UI, Windows/Linux parity tests.
- Ensure tests/benchmarks cover new behaviour; keep docs/logs current.

## Phase 2.5: Performance & Validation Gate

**Goal:** Prove v2 meets or exceeds v1 for all benchmark scenarios before touching remote code.

1. Run `scripts/bench_local_mirror.sh` across single-file, small-file, mixed, and checksum workloads.
2. Record results in docs + DEVLOG; ensure perceived latency ≤ 1 s.
3. Do not proceed until parity ≥ 95 % for every scenario.

## Phase 3: Hybrid Remote Operations

**Goal:** Implement the gRPC control + zero-copy TCP data plane with full progress propagation.

1. Control-plane handshake (NeedList, one-time token, transport negotiation).
2. Data-plane implementation with zero-copy + buffered fallback, plus pull/list/purge.
3. Network tuning (socket buffers, congestion control hints) and remote progress messages.

## Phase 3.5: RDMA Enablement

**Goal:** Add optional RoCEv2/InfiniBand transport for 25/100 GbE deployments.

1. Extend negotiation protocol for RDMA capabilities.
2. Implement RDMA transport abstraction + benchmarks once hardware available.

## Phase 4: Production Hardening & Packaging

**Goal:** Security, packaging, and integration polish.

1. TLS for control plane (optional data-plane TLS after perf validation).
2. Auth, installers, service units, cross-platform builds.
3. Full integration/regression test matrix and documentation refresh.
