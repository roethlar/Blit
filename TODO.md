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

## Phase 3: Remote Operations & Admin Tooling

- [x] Hybrid transport control/data plane scaffolding (push/pull) – initial implementation complete.
- [x] Remote pull integration tests (directory + single-file, forced gRPC path, traversal errors).
- [ ] Realign CLI verbs (`copy`, `mirror`, `move`, `scan`, `list`) and remove legacy `push`/`pull`.
- [ ] Update canonical remote URL parser to support `server:/module/...` and `server://...` syntax.
- [ ] Implement daemon TOML config loader (modules, root export, mDNS flags) with warnings for implicit working-directory exports.
- [ ] Enable mDNS advertising by default with opt-out flag; update `blit scan` to consume results.
- [ ] Implement admin RPCs (module list, directory list, recursive find, du/df metrics, remote remove).
- [ ] Implement `blit-utils` verbs (`scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile`) using shared clients.
- [ ] Ensure destructive operations prompt unless `--yes` is supplied.
- [ ] Wire remote `copy`/`mirror`/`move` to hybrid transport with automatic gRPC fallback.
- [ ] Add integration tests covering remote transfer + admin verbs across Linux/macOS/Windows.
- [ ] Capture remote benchmark runs (TCP vs forced gRPC fallback) and log results.

## Phase 4: Production Hardening & Packaging

- [ ] Produce packaging artifacts for supported platforms (Linux, macOS, Windows).
- [ ] Document installation/configuration (config.toml, `--root`, mDNS, service setup).
- [ ] Build end-to-end integration/regression suite and integrate with CI.
- [ ] Review logging/error output for production readiness.
- [ ] Prepare release notes/changelog with benchmark data and support matrix.

## Phase 3.5: RDMA Enablement (post-release)

- [ ] Track deferred RDMA/RoCE work (control-plane negotiation, transport abstraction, benchmarking) for future planning.
