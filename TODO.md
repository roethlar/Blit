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
- [x] Add adaptive predictor + local performance history store with `blit diagnostics perf`.
- [x] Remove `--ludicrous-speed` behaviour (make no-op) and add CLI progress UI.
- [x] Update unit/integration tests to cover fast-path routing and predictor logic.

## Phase 2.5: Performance & Validation Checkpoint

- [x] Create benchmark script for local mirror performance.
- [x] Run and compare against v1. (2025-10-16: v2 ~1.93× slower; optimization needed before GO)
- [x] Analyse Windows ETW traces (wingpt-4/5.md findings logged) (`logs/blit_windows_bench.zip`) and propose copy-path optimisations.
- [x] Re-run Windows benchmark after CopyFileExW fix (512 MiB) and update docs.
- [x] Prototype large-file heuristics (>1 GiB) and rerun 1–4 GiB suites.
  - 2025-10-19: Heuristic tuned (≤512 MiB cached, 2 GiB floor). wingpt-10.md confirms 512 MiB regression fixed and 4 GiB now beats robocopy.
- [x] Refactor oversized modules (`crates/blit-core/src/copy/`, `crates/blit-core/src/orchestrator/`) into focused submodules before Phase 3 to keep AI edits manageable.
- [x] Produce CLI/manpage documentation (include debug limiter behaviour, diagnostics commands, and hybrid transport flags once available). *(2025-10-19: `docs/cli/blit.1.md` added; hybrid transport flags pending Phase 3.)*
- [x] Extend proto (`proto/blit.proto`) with DataTransferNegotiation + reserved RDMA fields and transport stats ahead of Phase 3. *(2025-10-19: control-plane negotiation message plus push summary stats in place.)*
- [x] Document CLI debug limiter mode (`--workers`) in help text and plan docs. *(2025-10-19: CLI man page + plan updates.)*

## Phase 3: Remote Operations (Hybrid Transport)

- [x] Implement gRPC handshake for the `Push` service. *(2025-10-19: CLI streams manifest, daemon returns need list + fallback negotiation; data plane transfer pending.)*
- [x] Implement the raw TCP data plane for `Push`. *(2025-10-19: token-authenticated TCP port allocation in daemon + CLI streaming.)*
- [x] Implement the `Pull` service. *(2025-10-19: daemon streams files/directories, CLI writes to destination.)*
- [x] Add remote pull integration tests (directory + single-file, forced gRPC path, invalid traversal/missing path errors). *(2025-10-19: new async tests in `crates/blit-daemon/src/main.rs`.)*
- [ ] Implement the `List` service.
- [ ] Implement the `Purge` service.
- [ ] Add CLI/daemon progress propagation for remote operations.
- [ ] Record remote benchmark metrics in performance history log + DEVLOG.
- [ ] Generate cryptographically strong one-time tokens (signed, nonce-based) and bind them to accepted sockets.
- [x] Implement automatic gRPC data-plane fallback with warnings + advanced override (`--force-grpc-data`). *(2025-10-19: CLI flag exposes fallback path; daemon streams via control plane and logs summary.)*

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
