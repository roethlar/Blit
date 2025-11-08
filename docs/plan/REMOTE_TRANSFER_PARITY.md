# Remote Transfer Parity Refactor

## Goal

Achieve “absolute parity” across every transfer direction (local→remote, remote→local, local→local) so that:

1. **Feature parity:** Hybrid transport (token negotiation, TCP data plane, tar shard batching) and progress reporting are identical no matter which side initiates the transfer.
2. **Performance parity:** Push and pull achieve comparable throughput (target 10 GbE line rate on commodity hardware) with consistent first-payload latency.
3. **Maintainability:** Shared modules keep the codebase AI‑manageable; no duplicated implementations that can drift.

## Baseline Problems

- Remote pull still uses the diagnostic gRPC stream (`crates/blit-core/src/remote/pull.rs`, `crates/blit-daemon/src/service/pull.rs`), sending 64 KiB chunks per file with no batching or TCP data plane.
- CLI pull path does not wire `-p` / `-v` progress reporting.
- Push refactors (payload planning, data-plane negotiation, tar sharding) live entirely under `remote::push`, so future work keeps drifting toward the push direction only.

## Current Status (2025-11-07)

- ✅ Push path already uses hybrid transport + tar shards; lacks shared module but functions at 9–10 Gbps when tuned manually.
- ⚠️ Pull path is still gRPC-only with 64 KiB buffers and no progress output, so throughput stalls (~400 Mbps observed).
- ⚠️ Auto-tuning module exists but is unused by the CLI/transfer engine; stream counts/chunk sizes stay at conservative defaults.
- ⚠️ No integration tests cover remote pull parity, so regressions slipped in unnoticed.

## Refactor Overview

| Step | Description | Deliverables / Files |
|------|-------------|----------------------|
| 1. Shared Transfer Modules + Auto-Tune | Move payload planning, tar shard builder, control-plane streaming, and progress types into `crates/blit-core/src/remote/transfer/{payload,progress,data_plane}.rs`. Export via `remote::transfer`. While here, wire `auto_tune::determine_tuning` back into both push and pull schedulers so stream counts/chunk sizes react to warmup probes (mirroring v1). | New module files + `remote::push`/`transfer_engine` updates |
| 2. Protocol Updates | Extend `PullChunk` to include `DataTransferNegotiation` + `PullSummary` so pull can negotiate the TCP data plane just like push. Update rust proto bindings. | `proto/blit.proto`, generated bindings |
| 3. Daemon Pull Pipeline | Rebuild `crates/blit-daemon/src/service/pull.rs` so it enumerates manifests once, plans payloads via shared module, sends negotiation (unless `force_grpc`), and streams files/tar shards over TCP using the existing push data-plane listener. Keep gRPC fallback behind `force_grpc`, but increase data-plane buffers / enable zero-copy just as v1 does so each record can saturate 10 GbE. | `service/pull.rs`, shared listener wiring |
| 4. CLI/Client Pull Rewrite | Update `RemotePullClient` to consume negotiation messages, connect to the TCP data plane, and stream tar shards locally (mirroring push). Wire `run_remote_pull_transfer` to the shared `RemoteTransferProgress` channel so `-p`/`-v` behave identically, and ensure auto-tuned stream counts/chunk sizes flow through `SchedulerOptions`. | `crates/blit-core/src/remote/pull.rs`, `crates/blit-cli/src/transfers/remote.rs` |
| 5. Tests & Tooling | Add integration tests covering push/pull (TCP + forced gRPC) to verify throughput and progress parity. Add benchmarks or scripted runs for 10 GbE validation. Update docs (TODO, workflow) and remove the “diagnostic-only” note from the pull path once parity is proven. | `tests/`, `TODO.md`, `docs/plan/WORKFLOW_PHASE_3.md`, `DEVLOG.md` |

## Detailed Tasks

1. **Module Extraction**
   - Create `remote/transfer/payload.rs`: defines `TransferPayload`, tar shard builder, `transfer_payloads_via_control_plane`.
   - Create `remote/transfer/progress.rs`: defines shared `ProgressEvent` + `RemoteTransferProgress`.
   - Create `remote/transfer/data_plane.rs`: share record tags/helpers, buffer utilities.
   - Update push modules to re-export/use these definitions.

2. **Protocol & Types**
   - Modify `PullChunk` to include `DataTransferNegotiation` and `PullSummary`.
   - Regenerate tonic bindings (`cargo build` triggers build.rs).
   - Add a `RemotePullProgress` type alias to the shared progress struct.

3. **Daemon Pull Refactor**
   - Replace current `stream_pull` with:
     1. Manifest enumeration using `FileEnumerator`.
     2. Payload planning via shared module.
     3. Negotiation message (control plane) to client with port/token.
     4. Data-plane listener (reuse push’s `bind_data_plane_listener`).
     5. Streaming payloads (files/tar shards) over TCP.
     6. Summary (files/bytes/zero-copy/fallback) sent after completion.
   - Preserve `--force-grpc` path by toggling negotiation.

4. **Client Pull Refactor**
   - Update CLI to create a progress reporter for pull like push.
   - Update `RemotePullClient::pull`:
     * Handle negotiation message.
     * Connect to TCP using token/port, stream records, write files locally (respecting metadata).
     * When fallback is forced, reuse old gRPC path.
     * Emit `ProgressEvent`s for manifest/payload stats.
     * Capture summary metrics in `RemotePullReport`.

5. **Testing & Validation**
   - Integration tests (Rust or shell) covering:
     * push (TCP + forced gRPC),
     * pull (TCP + forced gRPC),
     * mirror operations across local↔remote.
   - Performance harness measuring first-payload latency and throughput.
   - Documentation updates (TOD0, workflow, devlog) with parity guarantees.

## Risks & Mitigations

- **Large refactor touching multiple crates:** Mitigate via staged commits (shared module first, then proto update, then daemon/CLI rewrites).
- **Compatibility with existing deployments:** Keep gRPC fallback fully functional; document any required upgrades (daemon + CLI must be updated together once parity lands).
- **AI file size limits:** Keep each new module under ~400 LOC; split helpers (`data_plane.rs`, `progress.rs`, etc.) instead of growing monoliths.
- **Performance regressions:** Add regression tests/benchmarks to CI and document required metrics in `TODO.md`.

## Success Criteria

- `blit-cli mirror -p -v local remote` and the reverse both display progress and achieve comparable throughput on 10 GbE.
- Force-grpc runs remain available and behave identically in both directions.
- Shared modules eliminate duplicate implementations, keeping future changes centralized.

## Why Previous Workflows Missed Parity

1. **Transport split:** V1’s net_async “throughput stability” plan covered both push and pull, but V2 only ported the push half; no plan item tracked the pull rewrite, so the CLI/daemon kept the diagnostic gRPC path.
2. **Auto-tune not wired:** The warmup probes and auto-tune module landed early in V2 but were never plumbed into `SchedulerOptions`, so stream counts/chunk sizes stayed static and we failed to notice the missing wiring.
3. **No pull benchmarks/tests:** Existing workflows validated only push throughput; without a regression harness for pull, parity regressions slipped through.
4. **Docs/TODO gaps:** Previous TODO/workflow entries celebrated push streaming but never captured “remote pull must match push,” so it fell through the cracks. This document and the TODO P0 item now make that requirement explicit.
