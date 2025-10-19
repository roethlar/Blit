# **Blit v2: Final Implementation Plan (v5 - Streaming Orchestrator + Hybrid Transport)**

**Version**: 5.0 (Supersedes v4)
**Status**: Active
**Strategy**: A greenfield Cargo workspace that pairs a streaming, self-adapting local orchestrator with a hybrid remote transport stack (gRPC control + zero-copy TCP data plane) and prepares for RDMA acceleration.

## 1. Architecture Overview

1. **Streaming Orchestrator** — Local transfers use the unified `TransferOrchestrator` with:
   - Incremental planner that emits work every heartbeat (1 s default, 500 ms when workers are starved).
   - 10 s stall detector (planner *and* workers idle) with precise error reporting.
   - Automatic fast-paths for trivial workloads and huge single files.
   - Adaptive predictor fed by local performance history to keep perceived latency ≤ 1 s.
   - No user speed flags; planner auto-tunes buffers/workers based on workload.

2. **Hybrid Remote Transport** — Remote push/pull mirror the v1 data-path performance by keeping:
   - gRPC control plane for manifests, negotiations, progress, and purge/list operations.
   - Raw TCP data plane negotiated via one-time, cryptographically strong token for bulk transfers (zero-copy on Linux via `sendfile`, `copy_file_range`, `splice`).
   - Automatic fallback to gRPC-streamed data when the negotiated TCP port cannot be reached (firewall/NAT); surface as a warning and continue, with an advanced `--force-grpc-data`/`BLIT_FORCE_GRPC_DATA=1` override for locked-down environments.
   - Planned RDMA/RoCEv2 extension point in Phase 3.5 for 25/100 GbE environments.

3. **Telemetry & Diagnostics** — All metrics stay on-device:
   - Capped JSONL log (`~/.config/blit/perf_local.jsonl`) storing workload signature, planner/copy durations, stall events.
   - `blit diagnostics perf` surfaces recent runs for troubleshooting.
   - `BLIT_DISABLE_PERF_HISTORY=1` opt-out for debugging.

4. **Inviolable Principles** — Every code change must respect:
   - **FAST**: Start copying immediately, minimise perceived latency.
   - **SIMPLE**: No user tunables for speed; planner chooses the best path automatically.
   - **RELIABLE**: Mirror deletions, checksums, and correctness outweigh speed.
   - **PRIVATE**: No external performance reporting; user data never leaves the machine.

5. **Future-Proofing** — The plan explicitly reserves:
   - RDMA support (RoCEv2 / InfiniBand) after the hybrid TCP path lands.
   - Structured progress events ready for GUI/daemon surfaces while CLI stays quiet by default.
   - Optional OS-specific change journals (USN/FSEvents) once baseline is stable.

## 2. Protocol Definition Updates (`proto/blit.proto`)

- `DataTransferNegotiation` remains the core handshake (port + token).
- Add reserved fields for RDMA capability negotiation (`bool supports_rdma`, `uint32 rdma_qp`, etc.). These are no-ops until Phase 3.5.
- Ensure `PushSummary` carries transport stats (bytes/sec, zero-copy usage) for diagnostics.

## 3. Phased Execution Plan

### Phase 0 — Workspace & Core Foundation (4–5 days)

**Goal:** Port v1’s proven core into `blit-core` with updated concurrency primitives.

- Create workspace layout (`blit-core`, `blit-cli`, `blit-daemon`, `blit-utils`).
- Port `enumeration`, `mirror_planner`, `checksum`, `zero_copy`, `transfer_plan`, `transfer_engine`, `local_worker` with streaming-friendly APIs.
- Update imports to eliminate legacy env-var flags (no `BLIT_PERF`, no `--mir`).
- Ensure unit tests run (`cargo test -p blit-core`).

### Phase 1 — gRPC API & Service Skeleton (2–3 days)

**Goal:** Stand up the control plane and binaries.

- Finalise `proto/blit.proto` with v5 fields.
- Configure `build.rs` for tonic/prost generation (vendored `protoc`).
- Implement skeleton `blit` CLI (`copy`, `mirror`, `push`, `pull`) and `blitd` server.
- Add integration test to prove Ping/Pong RPC and negotiation message wiring.

### Phase 2 — Streaming Orchestrator & Local Operations (open schedule)

**Goal:** Deliver the end-to-end local pipeline exactly as designed.

1. **Streaming Planner**
   - Refactor `TransferFacade` to emit batches via async stream.
   - Implement heartbeat scheduler + queue depth monitoring.
   - Bake in fast-path routing and direct copy fallback.

2. **Adaptive Predictor & Telemetry**
   - Persist metrics locally; implement EMA-based predictor segmented by FS profile.
   - Route orchestrator decisions through predictor.
   - Add `blit diagnostics perf` command.

3. **CLI Experience**
   - Keep planner fully automatic; expose only debug limiters when absolutely necessary.
   - Keep CLI quiet to avoid overhead, but emit structured progress events/verbose hooks for GUI layers.

4. **Testing & Benchmarks**
   - Unit tests for fast-path selection, predictor updates, stall detection.
   - Integration tests for: 1 file, 8 files, 100k files, sparse directories, checksum mirror.
   - Initial benchmark harness (`scripts/bench_local_mirror.sh`) covering same scenarios.

### Phase 2.5 — Performance Gate (open schedule)

**Goal:** Validate parity with v1 before touching remote code.

- Run benchmark suite (small, medium, large, mixed) comparing v1 vs. v2.
- Record metrics in DEVLOG and `docs/plan/WORKFLOW_PHASE_2.5.md`.
- Gate: all scenarios ≥ 95 % of v1 throughput, planner overhead perceptions ≤ 1 s.
- If gate fails, iterate within Phase 2 until satisfied.

### Phase 3 — Hybrid Remote Operations (open schedule)

**Goal:** Implement and stabilise the gRPC + TCP data path.

1. Control plane handshake, needlist exchange, one-time token issuance.
   - Token must be cryptographically strong (e.g., signed JWT with nonce/expiry) and bound to the accepted socket to prevent replay.
2. Data plane client/server flows with zero-copy fallback to buffered copy as needed.
   - Implement automatic gRPC-stream fallback when TCP negotiation fails; emit warning and respect advanced `force-grpc` override.
3. Pull/List/Purge services mirrored on the hybrid transport.
4. Network tuning: disable Nagle, set large send/recv buffers, optional BBR hints.
5. Progress signals piped back for GUI consumption; CLI remains quiet unless verbose/debug output is requested.

### Phase 3.5 — RDMA Enablement (open schedule)

**Goal:** Prepare for 25/100 GbE deployments.

- Introduce optional RDMA transport (RoCEv2, later InfiniBand) based on negotiated capability.
- Abstract transport layer so zero-copy operations select TCP vs. RDMA blindly.
- Add benchmarks on RDMA-capable hardware (pending availability).

### Phase 4 — Production Hardening & Packaging (open schedule)

**Goal:** Final polish prior to general availability.

- AuthN/authZ (token-based) once core performance is validated.
- Packaging: installers, systemd units, cross-compilation targets.
- End-to-end integration test matrix (Linux, macOS, Windows).
- Documentation updates and CLI help refresh.
  - Ensure advanced options (`--max-threads`, `--force-grpc-data`) are documented in "Advanced / Niche" sections of help/man pages.

### Phase 5 — Future Optimisations (Post-launch)

- Change journal integrations (USN, FSEvents) for faster incremental planning.
- GPU-accelerated hashing for checksum mode.
- Optional remote performance history opt-in (if ever justified, with explicit user consent).
- Advanced storage tuning (stripe-aware writes, preallocation heuristics).

---

## 4. Tooling & Logging Expectations

- Every major milestone must be logged in `DEVLOG.md` with timestamp + action.
- `TODO.md` remains the canonical task list; mark items off only when code + docs land.
- DEVLOG/TODO/workflow updates must capture session state so any LLM can resume after context reset; `agentcomms/` is reserved for real-time coordination between agents.
- All scripts/configs default to no network access; explicit callouts required otherwise.

---

## 5. Non-Negotiables (for any contributor or AI agent)

1. Respect the FAST/SIMPLE/RELIABLE/PRIVATE principles at all times.
2. Never reintroduce removed speed flags (`--mir`, etc.) as behaviour toggles.
3. Do not add user-facing performance tunables unless explicitly approved.
4. Telemetry stays local; no remote logging without signed-off design change.
5. Every change must update relevant docs + DEVLOG to survive context resets.

---

## 6. Open Questions (Tracking)

| Topic | Status | Notes |
|-------|--------|-------|
| Windows RDMA viability | TBD | Evaluate once TCP hybrid stabilises. |
| Progress event granularity | Planned | Define lightweight event schema for GUI layers; CLI stays minimal. |
| RDMA hardware procurement | Pending | Coordinate when Phase 3.5 starts. |

---

**This v5 plan replaces v4.** All workflow documents, TODO items, and onboarding material must reference v5 as the canonical architecture.
