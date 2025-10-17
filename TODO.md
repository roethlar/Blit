# Blit v2 TODO

This is the master checklist. Execute the first unchecked item. After completion, check the box and add an entry to `DEVLOG.md`.

## Phase 0: Workspace & Core Logic Foundation

- [x] Initialize Cargo workspace with `blit-core`, `blit-cli`, `blit-daemon`, `blit-utils`.
- [x] Port `checksum.rs` to `blit-core`.
- [x] Port `fs_enum.rs` and `enumeration.rs` to `blit-core`.
- [x] Port `mirror_planner.rs` to `blit-core`.
- [x] Port `buffer.rs` to `blit-core`.
- [x] Extract zero-copy primitives into `blit-core/src/zero_copy.rs`.
- [x] Port unit tests for ported modules.

## Phase 1: gRPC API & Service Scaffolding

- [x] Create `proto/blit.proto` with the full API definition.
- [x] Create `build.rs` in `blit-core` to compile the protocol.
- [x] Add `tonic-build` dependencies to `blit-core/Cargo.toml`.
- [x] Create `generated` module structure in `blit-core`.
- [x] Implement skeleton `blitd` server binary in `blit-daemon`.
- [x] Implement skeleton `blit` CLI binary in `blit-cli`.
- [x] Add a minimal integration test to verify client-server connection.

## Phase 2: Orchestrator & Local Operations

- [x] Create `orchestrator.rs` in `blit-core`.
- [x] Implement the `TransferOrchestrator` struct and `new` method.
- [x] Implement `execute_local_mirror` method on the orchestrator.
- [x] Port consolidated path modules (`copy`, `tar_stream`, `transfer_*`, `local_worker`, `logger`, `delete`, `win_fs`) from v1 into `blit-core`.
- [x] Wire the `blit-cli` `mirror` and `copy` commands to the orchestrator.
- [x] Refactor `TransferFacade` and planner into streaming producer with heartbeat flushes.
- [x] Implement 10 s stall detection and progress messaging in orchestrator.
- [x] Implement fast-path routing for tiny/huge manifests in orchestrator.
- [ ] Add adaptive predictor + local telemetry store with `blit diagnostics perf`.
- [ ] Remove `--ludicrous-speed` behaviour (make no-op) and add CLI progress UI.
- [ ] Update unit/integration tests to cover fast-path routing and predictor logic.

## Phase 2.5: Performance & Validation Checkpoint

- [x] Create benchmark script for local mirror performance.
- [x] Run and compare against v1. (2025-10-16: v2 ~1.93× slower; optimization needed before GO)

## Phase 3: Remote Operations (Hybrid Transport)

- [ ] Implement gRPC handshake for the `Push` service.
- [ ] Implement the raw TCP data plane for `Push`.
- [ ] Implement `Pull`, `List`, and `Purge` services.
- [ ] Add CLI/daemon progress propagation for remote operations.
- [ ] Record remote benchmark metrics in telemetry log + DEVLOG.
- [ ] Generate cryptographically strong one-time tokens (signed, nonce-based) and bind them to accepted sockets.
- [ ] Implement automatic gRPC data-plane fallback with warnings + advanced override (`--force-grpc-data` / env var).

## Phase 4: Production Hardening & Packaging

- [ ] Benchmark remote transfer performance.
- [ ] Add TLS security to the control plane.
- [ ] Create packaging scripts for major platforms.
- [ ] Write comprehensive integration tests.
- [ ] Document advanced options (`--max-threads`, `--force-grpc-data`) in help/man pages; mark as niche.

## Phase 3.5: RDMA Enablement (post Phase 3)

- [ ] Negotiate RDMA capability in control plane and extend data-plane abstraction.
- [ ] Implement RoCEv2 transport option (client + server).
- [ ] Benchmark RDMA path on 25/100 GbE hardware and log results.
- [ ] Update docs and diagnostics to reflect RDMA status.
