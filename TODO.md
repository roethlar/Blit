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
- [x] Realign CLI verbs (`copy`, `mirror`, `move`, `scan`, `list`) and remove legacy `push`/`pull`. *(2025-10-23: CLI now routes remote copy/mirror via RemotePush/RemotePull; docs/manpages updated.)*
- [x] Update canonical remote URL parser to support `server:/module/...` and `server://...` syntax. *(Parser already handled the forms; CLI + remote push now accept module sub-paths.)*
- [x] Implement daemon TOML config loader (modules, root export, mDNS flags) with warnings for implicit working-directory exports. *(2025-10-20: `blit-daemon` loads `/etc/blit/config.toml`/`--config`, supports `--root`, `--bind`, `--port`, `--no-mdns`, `--mdns-name`, and warns on implicit working-directory exports.)*
- [x] Investigate small-file performance (100 k × 4 KiB); target ≥95 % of rsync baseline. *(2025-10-21: blit 2.90 s vs tuned rsync 8.56 s on Linux; macOS 10.53 s vs rsync 11.62 s; Windows 60.63 s vs robocopy 218.48 s.)*
- [x] Investigate mixed workload (512 MiB + 50 k × 2 KiB); target ≥95 % of rsync baseline. *(2025-10-22: Linux blit 2.24 s vs rsync 6.95 s; macOS 6.32 s vs 6.56 s; Windows 31.26 s vs robocopy 110.51 s.)*
- [x] Improve incremental mirror throughput (touch 2 k/delete 1 k/add 1 k); target ≥95 % of rsync baseline. *(2025-10-22: Linux baseline 0.86 s vs rsync 1.32 s, mutation 0.61 s vs 1.23 s; macOS 0.65 s vs 0.69 s; Windows 7.10 s baseline and 6.45 s mutation vs robocopy 20.72 s/6.94 s.)*
- [x] Implement filesystem journal-based change detection on Windows (USN) to avoid full re-enumeration on no-op incremental runs; re-benchmark 0-change mutation once implemented.
- [x] Re-run Windows incremental 0-change benchmark to capture USN fast-path results (<200 ms target) and log findings in `DEVLOG`. *(2025-10-25: wingpt-53.md logged 28 ms zero-change mirror after USN fast-path fix.)*
- [x] Implement filesystem journal-based change detection on macOS (FSEvents) to avoid full re-enumeration on no-op incremental runs. *(2025-10-25: `change_journal` captures FSEvents snapshot; verify via `scripts/macos/run-journal-fastpath.sh` once mac agent available.)*
- [x] Implement filesystem journal-based change detection on Linux (metadata snapshot) to avoid full re-enumeration on no-op incremental runs. *(2025-10-25: `LinuxSnapshot` tracks device/inode/ctime; verified via `scripts/linux/run-journal-fastpath.sh` with 3 ms zero-change run.)*
- [x] Enable mDNS advertising by default with opt-out flag; update `blit scan` to consume results. *(2025-10-23: `blit-daemon` advertises `_blit._tcp.local.` unless `--no-mdns`; `blit scan`/`blit-utils scan` list discovered daemons.)*
- [x] Implement admin RPCs (module list, directory list, recursive find, du/df metrics, remote remove). *(2025-10-24: `Find`, `DiskUsage`, `FilesystemStats`, and enhanced `CompletePath` wired through daemon + proto.)*
- [x] Finish `blit-utils` admin surface: implement `find`, `du`, `df`, and `completions` (scan/list/ls/profile/rm implemented; rm wired to Purge on 2025-10-23). *(2025-10-24: new subcommands stream find/du/df results, completions delegates to daemon.)*
- [x] Wire remote mirror execution to the Purge RPC so remote mirrors delete extraneous files using the daemon. *(2025-10-23: `handle_push_stream` reuses purge helpers to remove remote extras and reports `entries_deleted` in summary.)*
- [ ] Ensure destructive operations prompt unless `--yes` is supplied.
- [ ] Wire remote `copy`/`mirror`/`move` to hybrid transport with automatic gRPC fallback.
- [ ] Add integration tests covering remote transfer + admin verbs across Linux/macOS/Windows.
- [ ] Capture remote benchmark runs (TCP vs forced gRPC fallback) and log results.
- [ ] Design adaptive predictor regression test suite (parsing, coefficient updates, accuracy, runtime overhead); automate as part of CI.
- [ ] Implement performance history schema versioning/migration to handle future format changes without data loss.

## Phase 4: Production Hardening & Packaging

- [ ] **P1** Implement filesystem capability probes and caching (daemon idle probes + CLI profile hook) so per-mount features like reflink/sparse/xattr are detected automatically and exposed to the planner.
- [ ] Explore optional AI-powered telemetry analysis (anomaly detection, tuning recommendations, diagnostics) using local performance history data; document scope and guardrails.
- [ ] Produce packaging artifacts for supported platforms (Linux, macOS, Windows).
- [ ] Document installation/configuration (config.toml, `--root`, mDNS, service setup).
- [ ] Build end-to-end integration/regression suite and integrate with CI.
- [ ] Review logging/error output for production readiness.
- [ ] Prepare release notes/changelog with benchmark data and support matrix.

## Phase 3.5: RDMA Enablement (post-release)

- [ ] Track deferred RDMA/RoCE work (control-plane negotiation, transport abstraction, benchmarking) for future planning.
