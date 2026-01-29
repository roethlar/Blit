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
- [x] Support remote-to-local mirrors (pull + local purge) so `blit mirror skippy://module dest/` downloads files and removes stray local state. *(2025-11-06: CLI routes remote mirrors through pull path, `RemotePullClient` tracks downloaded paths, and extraneous local entries are purged.)*
- [x] Investigate edge-case filesystem mirror gaps (ReFS clone path delivered but still ~35% slower than robocopy; follow-up benchmark `logs/windows/bench_local_windows_4gb_clone_20251026T020337Z.log`; ZFS baseline logged at `logs/truenas/bench_local_zfs_20251026T004021Z.log`). *(2025-10-26: Captured ReFS ETW profile `logs/windows/refs_clone_profile_20251026T022401Z.etl` plus bench log `bench_local_windows_4gb_clone_profile_20251026T022401Z.log`; manual `blit mirror --workers 1` runs (~0.28 s) show remaining ~1.7× gap vs robocopy 0.17 s is not purely task fan-out.)*
- [x] Update benchmark harnesses to honour `BLIT_BENCH_ROOT` (or similar) so Windows runs stay on the intended filesystem; document TEMP/TMP requirement in workflow docs. *(2025-10-26: scripts respect BLIT_BENCH_ROOT; workflow tip added.)*
- [x] Prototype clone-only metadata fast path on Windows (skip redundant attribute sync + reduce IOCTL overhead) and compare against robocopy using ETW traces (`logs/windows/refs_clone_profile_20251026T022401Z.etl`). *(2025-11-10: `copy_file` now returns `FileCopyOutcome`, skips metadata + timestamp preservation when block clone succeeds, and instructs operators to validate with the existing ETW trace + bench harness.)*
- [x] Implement streaming manifest/need-list for remote push so arbitrarily large file sets do not exhaust RAM (see memory `manifest_streaming_plan`). *(2025-10-26: CLI streams manifests over mpsc channel with back-pressure; daemon batches need list incrementally; control-plane fallback reads files in 1 MiB chunks.)*
- [x] Bundle remote transfers (TCP data plane + gRPC fallback) into tar shards so small files aren't shipped one-by-one. *(2025-10-27: Extended proto with `TarShard{Header,Chunk,Complete}`; CLI plans shard batches, TCP data plane streams them with new record types, daemon unpacks incrementally; `remote_tcp_fallback` test now time-bounded.)*
- [x] Parallelize daemon tar-shard unpacking so the data plane doesn't stall on decode. *(2025-10-28: `TarShardExecutor` spawns up to 4 blocking workers via `JoinSet`, updating stats as shards complete; ready for throughput benchmarking on `skippy`/`mycroft`.)*
- [x] Add large-manifest stress test (≥1 M entries) to validate streaming push memory footprint, <1 s transfer start, and throughput; capture logs/metrics with CLI/daemon traces. *(2025-11-07: Added ignored `drain_pending_headers_handles_one_million_entries` test; run manually with `cargo test -p blit-core drain_pending_headers_handles_one_million_entries -- --ignored` to collect RSS data.)*
- [ ] Benchmark remote fallback + data-plane streaming on Linux/macOS/Windows to confirm sub-second first-byte timings and document results in workflows.
- [x] Ensure destructive operations prompt unless `--yes` is supplied. *(2025-01-21: `blit mirror` prompts before deleting extraneous destination files; `blit move` prompts before deleting source after transfer; `--yes`/`-y` bypasses; `blit rm` already had prompts.)*
- [x] Document that remote transfers rely on operator-provided secure networks or SSH tunnels (no built-in TLS); update CLI/daemon help text and plan docs accordingly. *(2025-01-21: Added SECURITY sections to both `blit.1.md` and `blit-daemon.1.md` documenting TLS absence and recommended secure deployment patterns: SSH tunnel, VPN, trusted network, reverse proxy.)*
- [ ] **P0** Remote transfer parity refactor (see `docs/plan/REMOTE_TRANSFER_PARITY.md`):
    - [x] Extract shared modules `remote::transfer::{payload, progress, data_plane}` and migrate push to use them, wiring the common planner through remote push. *(2025-11-10: Added `remote::transfer` with shared payload/progress/data-plane logic and refactored push to consume it; pull wiring + auto-tune hookup still pending.)*
    - [x] Extend `PullChunk` proto with negotiation + summary messages; regenerate bindings. *(2025-11-10: Added negotiation/summary variants; regenerated prost bindings.)*
    - [x] Rebuild daemon pull pipeline to reuse hybrid transport + TCP data plane (with `--force-grpc` fallback) and enlarge data-plane buffers / zero-copy paths to match v1’s 10 GbE throughput. *(2025-11-10: `service/pull.rs` now enumerates manifests, plans payloads via `remote::transfer`, and streams via TCP listener + negotiation, falling back to gRPC when forced.)*
    - [x] Rewrite CLI / `RemotePullClient` to use the shared transport, emit progress (`-p/-v`), and connect to the data plane with the auto-tuned scheduler. *(2025-11-10: `RemotePullClient` connects to the negotiated TCP stream, writes files/tar shards to the destination, records summary info, and the CLI now reuses the shared progress monitor for push/pull.)*
    - [x] Feed `auto_tune::determine_tuning` outputs into remote push/pull schedulers so stream counts and chunk sizes adapt automatically. *(2025-11-10: `RemotePushClient` now applies the tuned chunk size across both TCP and gRPC fallback paths, drives data-plane payload prefetching via the tuned stream counts, and the daemon pull path reuses those parameters when sharding + streaming payloads.)*
    - [x] Ensure manifest need-lists flush immediately so first payloads start within seconds even on multi-hundred-thousand file manifests, and surface unreadable files inline while keeping the TCP data plane active. *(2025-11-10: daemon `FileListBatcher` gained an early flush path; CLI push now logs permission/not-found entries in red and filters them before planning.)*
    - [x] Implement multiplexed TCP data-plane streams (client + daemon) driven by auto-tuned worker counts so push/pull saturate 10 GbE. *(2025-11-10: push/pull negotiations carry `stream_count`; daemon and client spawn parallel TCP workers with auto-tuned stream counts.)*
    - [x] Implement multiplexed TCP data-plane streams (client + daemon) driven by auto-tuned worker counts so push/pull saturate 10 GbE. *(2025-11-10: push/pull negotiations carry `stream_count`; daemon and client spawn parallel TCP workers with auto-tuned stream counts.)*
    - [x] Balance TCP data-plane payload scheduling across streams so negotiated workers actually receive work; `MultiStreamSender` now slices plans into 32–512 MiB batches per stream instead of routing entire manifests through a single connection. *(2025-11-15: eliminates the ~450 Mbps cap observed on push tests and unlocks true multi-stream throughput.)*
    - [x] Revert hardcoded performance optimizations and implement orchestrator-controlled configuration for TCP settings (Nagle's algorithm, buffer sizes), chunk sizes, stream counts, and payload prefetching. *(2025-11-20: Refactored `TuningParams` to include `tcp_buffer_size` and `prefetch_count`, populated by heuristics, and passed down to data plane.)*
    - [ ] Add integration/perf tests proving push/pull parity (TCP + forced gRPC) and document the results.
- [ ] **P0** 25GbE performance improvements:
    - [x] Implement `BufferPool` with reusable allocations, semaphore memory control, and RAII guards (no hardcoded defaults; accepts orchestrator params). *(2025-01-26: Added to `buffer.rs` with tests.)*
    - [x] Integrate `BufferPool` with `DataPlaneSession` using `TuningParams` (pass pool via orchestrator, replace `vec![0u8; buffer_len]` allocation). *(2025-01-26: Added `SessionBuffer` enum, `connect_with_pool`, `MultiStreamSender` creates shared pool from `chunk_bytes`/`stream_count`.)*
    - [x] Implement async read-ahead pipeline in `DataPlaneSession::send_file` (overlap disk reads with network writes using double-buffering from pool). *(2025-01-26: Added `send_file_double_buffered` using `tokio::join!` with two pool buffers, falls back to single-buffer when no pool.)*
    - [x] Parallel payload dispatch across TCP streams (concurrent stream workers). *(2025-01-26: Already implemented via `MultiStreamSender` - spawns N workers as concurrent tokio tasks, round-robin batch distribution 32-512 MiB. Work-stealing queue is potential future optimization.)*
    - [ ] Benchmark TCP data plane throughput targeting 10+ Gbps per stream.
    - [x] Add remote↔remote transfers (CLI + daemon support for server-to-server sync initiated from a third host) so every src/dst combination is covered. *(2025-01-21: Implementation via `RemoteTransferSource` abstraction complete; integration test `remote_remote.rs` covers dual-daemon server-to-server copy; CLI supports `blit copy/mirror server1:/mod/ server2:/mod/` syntax.)*
- [x] Diagnose TCP data-plane resets during remote push (upload_tx channel closes while streaming tar shards; see `crates/blit-daemon/src/service/push/data_plane.rs`). Reproduce with `blit-cli mirror -v -p ~/ skippy://elphaba/home`, capture daemon `[data-plane]` logs, and fix underlying disk/write/mismatch issue so the connection no longer drops mid-transfer. *(2025-11-06: Hardened client TCP sender + restored streamed-file metadata; added CLI trace flag and confirmed `source/venvs/superclaude` run completes without resets.)*
- [ ] Refactor oversized sources into AI-manageable modules:
    - [x] Split `crates/blit-daemon/src/main.rs` (service wiring, data plane handlers, admin RPCs). *(2025-10-27: introduced `runtime.rs` for config/args and `service.rs` for gRPC/data plane; main now only boots the server.)*
    - [x] Break down `crates/blit-cli/src/main.rs` (argument parsing vs command execution). *(2025-10-27: extracted `cli.rs`, `context.rs`, `diagnostics.rs`, `scan.rs`, `list.rs`, and `transfers.rs`; main now wires modules only.)*
    - [x] Decompose `crates/blit-core/src/copy/` (move platform-specific helpers into submodules). *(2025-10-27: split into `compare.rs`, `file_copy.rs`, `parallel.rs`, `stats.rs`, keeping platform helpers isolated; `mod.rs` now re-exports public API.)*
    - [x] Split `crates/blit-utils/src/main.rs` (verb dispatch vs helpers). *(2025-10-27: introduced `cli.rs`, `util.rs`, and verb modules `scan/list_modules/ls/find/du/df/completions/rm/profile`; main now dispatches only.)*
    - [ ] Extract helpers from `crates/blit-core/src/change_journal.rs`, `transfer_facade.rs`, and `remote/push/client.rs` below 500 lines.
    - [x] `change_journal` split into `types/snapshot/tracker/util` (2025-10-28).
    - [x] `transfer_facade` modularised into `types/aggregator/planner` (2025-10-28).
    - [x] `remote/push/client.rs` reorganised into `client/{mod,types,helpers}` with spawn helpers (2025-10-28).
    - [x] Break `crates/blit-cli/src/transfers.rs` into module directory (`mod.rs` + `endpoints`, `remote`, `local`, `mmap`) to keep files under 400 LOC (2025-10-28).
    - [x] Split `crates/blit-core/src/orchestrator/mod.rs` into `options.rs`, `summary.rs`, and `orchestrator.rs` alongside existing helpers (2025-10-28).
    - [x] Restructure `crates/blit-core/src/copy/file_copy.rs` into submodules (`clone`, `metadata`, `mmap`, `chunked`) so each stays <300 LOC (2025-10-28).
- [x] Wire remote `copy`/`mirror`/`move` to hybrid transport with automatic gRPC fallback. *(2025-10-25: integration test `remote_tcp_fallback` forces `--force-grpc-data` and verifies CLI output + successful transfer.)*
- [ ] Add integration tests covering remote transfer + admin verbs across Linux/macOS/Windows.
- [ ] Capture remote benchmark runs (TCP vs forced gRPC fallback) and log results.
- [ ] Design adaptive predictor regression test suite (parsing, coefficient updates, accuracy, runtime overhead); automate as part of CI.
- [ ] Implement performance history schema versioning/migration to handle future format changes without data loss.

## Phase 4: Production Hardening & Packaging

- [ ] **P1** Integrate resumable file copy into transfer flow:
    - [x] Implement `resume_copy_file` with block-level comparison (`copy/file_copy/resume.rs`). *(2025-01-28: Added with 5 unit tests.)*
    - [ ] Add `resume: bool` field to `CopyConfig`.
    - [ ] Modify `local_worker.rs` to use `resume_copy_file` when `config.resume` is true.
    - [ ] Add `--resume` flag to CLI for copy/mirror commands.
    - [ ] Extend resume logic for remote transfers (checksum exchange over network).
- [ ] **P1** Implement filesystem capability probes and caching (daemon idle probes + CLI profile hook) so per-mount features like reflink/sparse/xattr are detected automatically and exposed to the planner.
- [ ] Explore optional AI-powered telemetry analysis (anomaly detection, tuning recommendations, diagnostics) using local performance history data; document scope and guardrails.
- [ ] Produce packaging artifacts for supported platforms (Linux, macOS, Windows).
- [ ] Document installation/configuration (config.toml, `--root`, mDNS, service setup).
- [ ] Build end-to-end integration/regression suite and integrate with CI.
- [ ] Review logging/error output for production readiness.
- [ ] Prepare release notes/changelog with benchmark data and support matrix.

## Phase 3.5: RDMA Enablement (post-release)

- [ ] Track deferred RDMA/RoCE work (control-plane negotiation, transport abstraction, benchmarking) for future planning.

- [ ] Investigate SeManageVolumePrivilege requirement for ReFS block clone on dev machine; backups showing CopyFileEx fallback (~0.6 s). Need elevated shell or alternative clone mechanism to validate fast path.
