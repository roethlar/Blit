

**2025-10-15 18:15:00Z** - **ACTION**: Began Phase 2. Created the `orchestrator.rs` file and module in `blit-core`.
**2025-10-16 04:20:00Z** - **ACTION**: Added `scripts/bench_local_mirror.sh` to benchmark local mirror performance (v1 vs v2) and vendored `protoc` via `protoc-bin-vendored`, enabling `cargo check` without external dependencies. CLI mirror/copy now runs via `spawn_blocking`.

**2025-10-16 04:35:00Z** - **ACTION**: Ran `SIZE_MB=128 scripts/bench_local_mirror.sh`. Observed v1 mirror 0.269 s vs v2 mirror 0.519 s (≈52% slower). Logged NO-GO in Phase 2.5 plan; next step is profiling and optimization before moving to Phase 3.

**2025-10-17 06:05:00Z** - **ACTION**: Eliminated per-task Tokio `spawn_blocking` in local workers and introduced sequential skip-unchanged filtering for small manifests. `SIZE_MB=128 scripts/bench_local_mirror.sh` now reports v1 mirror 0.267 s vs v2 mirror 0.266–0.286 s (≈0–7% slower); large-file workload meets Phase 2.5 parity threshold pending remaining scenario benchmarks.

**2025-10-17 06:25:00Z** - **ACTION**: Fixed `transfer_engine` test failures by guaranteeing unique worker indices during dynamic scaling and mirroring real worker accounting in the mock factory. `cargo test` now passes cleanly.

**2025-10-17 06:40:00Z** - **ACTION**: Restored automatic skip-unchanged heuristics in `TransferFacade`; planner now switches to parallel filtering only for large or checksum-heavy manifests. Sequential remains default, no user-exposed tunables, and `cargo test` confirms behavior.

**2025-10-17 07:10:00Z** - **ACTION**: Published `greenfield_plan_v5.md` and refreshed workflow/TODO docs to capture the streaming planner, performance history, and RDMA roadmap. Updated non-negotiable principles, added new Phase 2 tasks, and ensured all references point to v5 so future sessions retain context after reset.

**2025-10-17 07:25:00Z** - **ACTION**: Incorporated firewall/NAT handling into the spec. Added automatic gRPC fallback, advanced `--force-grpc-data` override, cryptographically strong token binding requirements, and help/man page updates to keep advanced flags clearly documented.

**2025-10-17 21:18:41Z** - **ACTION**: Implemented streaming `TransferFacade::stream_local_plan`, `TaskStreamSender`, and `TransferOrchestrator` with heartbeat + 10 s stall detection. Verified Windows/Linux unit tests (`cargo test -p blit-core`) succeed via `scripts/windows/run-blit-tests.ps1`.
**2025-10-17 21:48:06Z** - **ACTION**: Added orchestrator fast-path routing for tiny manifests and single large files (direct copy + large-file worker). Updated TODO/plan docs and confirmed `cargo test -p blit-core` passes.
**2025-10-17 22:13:55Z** - **ACTION**: Added `scripts/macos/run-blit-tests.sh` to mirror the Windows test harness on macOS; runs fmt/check/unit tests and captures UTF-8 logs under `logs/`.
**2025-10-17 23:17:33Z** - **ACTION**: Implemented local performance history writer (`perf_history.rs`) with JSONL append + rotation, integrated orchestrator logging (fast-path + streaming) and env guard `BLIT_DISABLE_PERF_HISTORY`. `cargo test -p blit-core` remains green.
**2025-10-18 00:11:04Z** - **ACTION**: Wired predictor scaffolding into fast-path routing; tiny-manifest fast path now consults learned planning thresholds (defaulting to heuristics until observations exist). Added dev `tempfile` dependency to satisfy macOS capability tests. `cargo test -p blit-core` clean.
**2025-10-18 00:31:15Z** - **ACTION**: Added `blit diagnostics perf` command and CLI local copy/mirror execution with indicatif spinner + final throughput summary; macOS/Windows test wrappers now run `cargo test -p blit-cli`.
**2025-10-18 01:16:18Z** - **ACTION**: Removed legacy `--ludicrous-speed` flag entirely and added CLI local transfer tests covering copy + dry-run flows. `cargo test -p blit-cli` now exercises these scenarios.
**2025-10-18 01:58:45Z** - **ACTION**: Added predictor/streaming integration tests under `tests/integration/`, surfaced verbose planner estimates from the predictor, verified `cargo test --tests` plus platform suites, and introduced `scripts/windows/bench-local-mirror.ps1` for parity benchmarks.
