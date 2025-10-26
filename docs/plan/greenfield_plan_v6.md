# **Blit v2: Final Implementation Plan (v4 - Hybrid Transport)**

**Version**: 4.0 (Final)
**Status**: Proposed
**Strategy**: A greenfield Cargo Workspace using a hybrid transport model: gRPC for control and a raw TCP data plane for maximum performance.

## 1. Architecture: A Hybrid Transport Model

This plan's core architectural decision is to use **two channels** for communication:

1.  **Control Plane (gRPC):** All negotiation, metadata exchange, and commands (manifests, file lists, purge requests, progress) will be handled over a standard gRPC connection. This gives us a robust, modern, and AI-friendly API.
2.  **Data Plane (Raw TCP):** For the actual bulk transfer of large files, the control plane will negotiate a separate, short-lived, raw TCP connection. This allows us to reuse the hyper-optimized, zero-copy `sendfile` and `splice` logic from v1 directly on a raw socket, bypassing any potential gRPC overhead and guaranteeing maximum performance.

This hybrid model gives us the best of both worlds: the safety and structure of gRPC for control, and the raw, un-abstracted speed of a bespoke TCP stream for data.

## 2. Refined Protocol Definition (`proto/blit.proto`)

The protocol is updated to reflect the hybrid model. Note that `FileData` is no longer part of the main `Push` RPC.

```protobuf
syntax = "proto3";
package blit.v2;

service Blit {
  // Push uses a bidirectional stream for the control plane.
  // The actual file data is sent over a separate, negotiated TCP connection.
  rpc Push(stream ClientPushRequest) returns (stream ServerPushResponse);

  // Other services remain as before...
  rpc Pull(PullRequest) returns (stream PullChunk);
  rpc List(ListRequest) returns (ListResponse);
  rpc Purge(PurgeRequest) returns (PurgeResponse);
}

// --- Push Operation ---

message ClientPushRequest {
  oneof payload {
    PushHeader header = 1;
    FileHeader file_manifest = 2;
    ManifestComplete manifest_complete = 3;
    // FileData is no longer sent here.
  }
}

message ServerPushResponse {
  oneof payload {
    // Server tells the client which files to upload and how.
    DataTransferNegotiation transfer_negotiation = 1;
    PushSummary summary = 2;
  }
}

// The server provides the client with a port and a one-time token
// to establish the high-performance data plane connection.
message DataTransferNegotiation {
  uint32 data_port = 1;
  bytes one_time_token = 2;
  repeated string files_to_upload = 3; // The "NeedList"
}

// ... other message definitions (FileHeader, PushSummary, etc.) remain the same.
message PushHeader { /* ... */ }
message FileHeader { /* ... */ }
message ManifestComplete {}
message PushSummary { /* ... */ }
message PullRequest { /* ... */ }
message PullChunk { /* ... */ }
message ListRequest { /* ... */ }
message ListResponse { /* ... */ }
message PurgeRequest { /* ... */ }
message PurgeResponse { /* ... */ }

```

## 3. Final Phase-by-Phase Implementation Plan

### Phase 0: Workspace & Core Logic Foundation (4-5 days)

**Goal:** Set up the workspace and port all proven v1 logic into the `blit-core` library.

1.  **Initialize Workspace:** Create the full Cargo Workspace structure (`blit-core`, `blit-cli`, `blit-daemon`, `blit-utils`).
2.  **Define Public API:** In `blit-core`, sketch out the main traits and public functions for the `TransferOrchestrator`.
3.  **Port Core Logic:** Copy and adapt the following from v1 into `crates/blit-core/src/`: `enumeration.rs`, `mirror_planner.rs` (including Windows-specific logic), `checksum.rs`, and `zero_copy.rs`.
4.  **Port Tests:** Move all corresponding unit and integration tests for the above modules into `blit-core`.
5.  **Document v1->v2 Migration:** Create a document outlining the v2 URL syntax and a strategy for migrating any existing v1 automation.
6.  **Verification:** `cargo test -p blit-core` must pass cleanly.

### Phase 1: gRPC API & Service Scaffolding (2-3 days)

**Goal:** Generate all gRPC code and create skeleton binaries that can communicate.

1.  **Define Protocol:** Create `proto/blit.proto` with the full hybrid-transport API.
2.  **Generate Code:** Set up `build.rs` in `blit-core` to generate the Rust gRPC stubs.
3.  **Implement Skeletons:** Create the `blitd` server and `blit` client skeletons.
4.  **Minimal Integration Test:** Add a simple test that starts `blitd`, connects a client, and exercises a simple RPC (e.g., a new `Ping` RPC) to prove the gRPC stack is working.

### Phase 2: Orchestrator & Local Operations (3-4 days)

**Goal:** Implement the central orchestrator and make local-only file transfers fully functional.

1.  **Build Orchestrator:** Implement the `TransferOrchestrator` in `blit-core`.
2.  **Implement Local Path:** Implement an `execute_local_mirror` method on the orchestrator.
3.  **Connect CLI:** Wire the `blit-cli` `mirror` and `copy` commands to the orchestrator.

### Phase 2.5: Performance & Validation Checkpoint (1-2 days)

**Goal:** Objectively verify that the new architecture's local performance is acceptable before building network features.

1.  **Define Benchmarks:** Specify exact benchmark scenarios (e.g., single 4GiB file, 100k small files).
2.  **Benchmark:** Compare `blit-v2` local mirror vs. `blit-v1` local mirror.
3.  **Decision Gate:** Performance must be within a 5% margin of v1. Do not proceed until this is met.

### Phase 3: Remote Operations (Hybrid Transport) (7-10 days)

**Goal:** Implement the hybrid gRPC/TCP transport model.

1.  **Implement Handshake:** Implement the control plane logic where the server receives a manifest and responds with the data port and token.
2.  **Implement Data Plane:** Implement the logic for the client to connect to the raw TCP port and for both client/server to use the ported `zero_copy` code to transfer the file data.
3.  **Implement Other Services:** Fill out the `Pull`, `Purge`, and other RPCs, using the same hybrid model.

### Phase 4: Production Hardening & Packaging (5-7 days)

**Goal:** Finalize features and prepare for distribution.

1.  **Final Performance Validation:** Benchmark remote transfer speeds against v1.
2.  **Add TLS:** Secure the gRPC control plane. The data plane can also be wrapped in TLS.
3.  **Packaging & Distribution:** Create installers, service files, and set up code signing.
4.  **Integration Testing:** Write a comprehensive suite of end-to-end tests.


# **Blit v2: Final Implementation Plan (v5 - Streaming Orchestrator + Hybrid Transport)**

**Version**: 5.0 (Supersedes v4)
**Status**: Active
**Strategy**: A greenfield Cargo workspace that pairs a streaming, self-adapting local orchestrator with a hybrid remote transport stack (gRPC control + zero-copy TCP data plane) and prepares for RDMA acceleration.

## 1. Architecture Overview

1. **Streaming Orchestrator** — Local transfers use the unified `TransferOrchestrator` with:
   - Incremental planner that emits work every heartbeat (1 s default, 500 ms when workers are starved).
   - 10 s stall detector (planner *and* workers idle) with precise error reporting.
   - Automatic fast-paths for trivial workloads and huge single files.
   - Adaptive predictor fed by local telemetry to keep perceived latency ≤ 1 s.
   - No user speed flags (`--ludicrous-speed` is deprecated); buffers/workers auto-tuned.

2. **Hybrid Remote Transport** — Remote push/pull mirror the v1 data-path performance by keeping:
   - gRPC control plane for manifests, negotiations, progress, and purge/list operations.
   - Raw TCP data plane negotiated via one-time, cryptographically strong token for bulk transfers (zero-copy on Linux via `sendfile`, `copy_file_range`, `splice`).
   - Automatic fallback to gRPC-streamed data when the negotiated TCP port cannot be reached (firewall/NAT); surface as a warning and continue, with an advanced `--force-grpc-data`/`BLIT_FORCE_GRPC_DATA=1` override for locked-down environments.
   - Planned RDMA/RoCEv2 extension point in Phase 3.5 for 25/100 GbE environments.

3. **Telemetry & Diagnostics** — All metrics stay on-device:
   - Capped JSONL log (`~/.config/blit/perf_local.jsonl`) storing workload signature, planner/copy durations, stall events.
   - `blit diagnostics perf` surfaces recent runs for troubleshooting.
   - `BLIT_DISABLE_LOCAL_TELEMETRY=1` opt-out for debugging.

4. **Inviolable Principles** — Every code change must respect:
   - **FAST**: Start copying immediately, minimise perceived latency.
   - **SIMPLE**: No user tunables for speed; planner chooses the best path automatically.
   - **RELIABLE**: Mirror deletions, checksums, and correctness outweigh speed.
   - **PRIVATE**: No external telemetry; user data never leaves the machine.

5. **Future-Proofing** — The plan explicitly reserves:
   - RDMA support (RoCEv2 / InfiniBand) after the hybrid TCP path lands.
   - Progress UI (spinner + throughput) exposed consistently in CLI.
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

### Phase 2 — Streaming Orchestrator & Local Operations (7–10 days)

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
   - Remove `--ludicrous-speed`; accept as no-op for compatibility until v2 CLI release.
   - Introduce unified progress indicator (spinner + throughput + ETA) for copy/mirror.

4. **Testing & Benchmarks**
   - Unit tests for fast-path selection, predictor updates, stall detection.
   - Integration tests for: 1 file, 8 files, 100k files, sparse directories, checksum mirror.
   - Initial benchmark harness (`scripts/bench_local_mirror.sh`) covering same scenarios.

### Phase 2.5 — Performance Gate (2–3 days)

**Goal:** Validate parity with v1 before touching remote code.

- Run benchmark suite (small, medium, large, mixed) comparing v1 vs. v2.
- Record metrics in DEVLOG and `docs/plan/WORKFLOW_PHASE_2.5.md`.
- Gate: all scenarios ≥ 95 % of v1 throughput, planner overhead perceptions ≤ 1 s.
- If gate fails, iterate within Phase 2 until satisfied.

### Phase 3 — Hybrid Remote Operations (8–12 days)

**Goal:** Implement and stabilise the gRPC + TCP data path.

1. Control plane handshake, needlist exchange, one-time token issuance.
   - Token must be cryptographically strong (e.g., signed JWT with nonce/expiry) and bound to the accepted socket to prevent replay.
2. Data plane client/server flows with zero-copy fallback to buffered copy as needed.
   - Implement automatic gRPC-stream fallback when TCP negotiation fails; emit warning and respect advanced `force-grpc` override.
3. Pull/List/Purge services mirrored on the hybrid transport.
4. Network tuning: disable Nagle, set large send/recv buffers, optional BBR hints.
5. Progress signals piped back to CLI from remote operations.

### Phase 3.5 — RDMA Enablement (5–7 days)

**Goal:** Prepare for 25/100 GbE deployments.

- Introduce optional RDMA transport (RoCEv2, later InfiniBand) based on negotiated capability.
- Abstract transport layer so zero-copy operations select TCP vs. RDMA blindly.
- Add benchmarks on RDMA-capable hardware (pending availability).

### Phase 4 — Production Hardening & Packaging (5–7 days)

**Goal:** Final polish prior to general availability.

- TLS for control plane (and optionally data plane via STARTTLS-style negotiation).
- AuthN/authZ (token-based or mTLS) once core performance is validated.
- Packaging: installers, systemd units, cross-compilation targets.
- End-to-end integration test matrix (Linux, macOS, Windows).
- Documentation updates and CLI help refresh.
  - Ensure advanced options (`--max-threads`, `--force-grpc-data`) are documented in “Advanced / Niche” sections of help/man pages.

### Phase 5 — Future Optimisations (Post-launch)

- Change journal integrations (USN, FSEvents) for faster incremental planning.
- GPU-accelerated hashing for checksum mode.
- Optional remote telemetry opt-in (if ever justified, with explicit user consent).
- Advanced storage tuning (stripe-aware writes, preallocation heuristics).

---

## 4. Tooling & Logging Expectations

- Every major milestone must be logged in `DEVLOG.md` with timestamp + action.
- `TODO.md` remains the canonical task list; mark items off only when code + docs land.
- `agent_comms/codex_resume.md` (or equivalent) must capture session state so any LLM can resume after context reset.
- All scripts/configs default to no network access; explicit callouts required otherwise.

---

## 5. Non-Negotiables (for any contributor or AI agent)

1. Respect the FAST/SIMPLE/RELIABLE/PRIVATE principles at all times.
2. Never reintroduce deprecated flags (`--mir`, `--ludicrous-speed`) as behaviour toggles.
3. Do not add user-facing performance tunables unless explicitly approved.
4. Telemetry stays local; no remote logging without signed-off design change.
5. Every change must update relevant docs + DEVLOG to survive context resets.

---

## 6. Open Questions (Tracking)

| Topic | Status | Notes |
|-------|--------|-------|
| Windows RDMA viability | TBD | Evaluate once TCP hybrid stabilises. |
| Progress UI granularity | Planned | Must include throughput + ETA; evaluate `indicatif`. |
| RDMA hardware procurement | Pending | Coordinate when Phase 3.5 starts. |
| TLS for data plane | Deferred | Evaluate cost once TCP path proven. |

---

**This v5 plan replaces v4.** All workflow documents, TODO items, and onboarding material must reference v5 as the canonical architecture.

# Blit v2 – Implementation Plan (v6 “Feature Completeness & Transport”)

**Status**: Active (supersedes v5)  
**Purpose**: Capture the functionality v2 still needs—CLI verbs, remote semantics, daemon configuration, discovery, admin utilities—and integrate it with the streaming/hybrid transport roadmap. The focus is shipping the required features; backward compatibility with v1 is not a goal.

---

## 1. Guiding Principles

1. **Deliver the Needed Features** – Ensure the CLI, daemon, and utilities expose the commands and behaviours the project relies on (copy/mirror/move, remote discovery, module management, admin tooling). These are functional requirements, not a promise of backward compatibility.
2. **FAST / SIMPLE / RELIABLE / PRIVATE** – Same non-negotiables as v5: planner auto-tunes, no user speed knobs, correctness outweighs raw throughput, and metrics never leave the machine.
3. **Transport Evolution** – Hybrid TCP, automatic gRPC fallback, and future RDMA remain core differentiators, layered once the required feature set is present.
4. **Clarity Over Legacy** – Document what v2 provides; references to v1 exist only for historical context.

---

## 2. Feature Gaps To Close

### CLI & Remote Semantics
- Replace the current `push`/`pull` model with the required command set: `copy`, `mirror`, `move`, `scan`, `list`, plus diagnostics.
- Support local ↔ remote transfers in any direction for `copy`, `mirror`, and `move`.
- Adopt canonical remote syntax:
  - `server:/module/` → root of a named module (must end with `/`).
  - `server:/module/path` → path under the module root.
  - `server://path` → default export. If `--root` is supplied (or config defines a default root), that path is used; otherwise the daemon’s current working directory is exposed.
  - Bare `server` (optionally `:port`) → discovery (list modules).
  - `server:/module` without a trailing slash is invalid (ambiguous) and should error.
- Default remote port is 9031; allow overrides via `server:port/...` and CLI flags.
- `move` performs a mirror followed by source removal (local or remote).

### Module Configuration & Daemon Behaviour
- Load module definitions from a TOML config (`/etc/blit/config.toml` by default) with fields: `name`, `path`, `comment`, `read_only`, `use_chroot`, and daemon-level settings `bind`, `port`, `motd`, `no_mdns`, `mdns_name`.
- Expose flags such as `--config`, `--bind`, `--port`, `--root`, `--no-mdns`, `--mdns-name`.
- Behaviour when no modules are defined:
  - If `--root` is provided (or the config defines a default root), expose it via `server://`.
  - Otherwise `server://` resolves to the daemon’s working directory, matching historical behaviour. Log a warning so operators know they are running with an implicit root export.
- Enforce read-only modules and chroot semantics for every remote operation.

### Discovery & Admin Utilities (`blit-utils`)
- Implement subcommands: `scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, and a `profile` command for local performance capture.
- Utilities must share the URL parser and RPC clients with the CLI.
- Destructive operations (`rm` and any future destructive verbs) require confirmation unless `--yes` is supplied.
- `scan` consumes mDNS advertisements; `find`, `du`, `df` rely on new daemon RPCs.

### Remote Services
- Extend the daemon API to support utility verbs:
  - Directory listing (for `list`/`ls`), recursive enumeration (`find`), space usage (`du`, `df`), and remote remove (`rm`).
- Keep transfer verbs (`copy`, `mirror`, `move`) on the hybrid transport path (TCP + gRPC fallback).
- Administrative RPCs can remain gRPC-only but must honour module boundaries and read-only flags.
- Automatic gRPC fallback for data transfers is mandatory; CLI prints a warning but continues.

### Discovery & Advertisements
- Advertise `_blit._tcp.local.` via mDNS by default; provide opt-out (`--no-mdns`) and custom instance name (`--mdns-name`).
- Confirm behaviour on Linux, macOS, and Windows.

### Documentation & Tests
- Update CLI help/man pages to reflect the command set and remote syntax. No migration guide—documentation describes v2 only.
- Extend integration tests to cover:
  - All transfer permutations (local↔remote, remote↔local, remote↔remote).
  - Utility workflows (`scan`, `list`, `ls`, `find`, `du`, `df`, `rm`).
  - Daemon startup combinations (modules present, modules absent with `--root`, mDNS toggles).

---

## 3. Revised Phase Breakdown

### Phase 0 – Required Feature Surface
1. **CLI Command Set**
   - Replace command matrix with `copy`, `mirror`, `move`, `scan`, `list`, diagnostics.
   - Implement canonical remote URL parsing in `blit-core::remote::endpoint`.
   - Add tests for all transfer permutations and error cases (e.g., `server:/module` without `/`).
2. **blit-utils Tooling**
   - Implement admin verbs (`scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile` command).
   - Share networking code with CLI.
3. **Daemon Config & Flags**
   - Load modules/root from TOML; respect overrides.
   - Enforce read-only/chroot semantics.
   - Define behaviour when no modules exist (require `--root` or emit clear errors).
4. **mDNS Advertising**
   - Advertise `_blit._tcp.local.` by default; verify discovery via CLI/util tests.

### Phase 1 – gRPC API & Service Skeleton (Carry-over from v5)
Unchanged scope, but proto definitions must include listing/usage RPCs needed by utilities.

### Phase 2 – Streaming Orchestrator & Local Operations
Same as v5 (streaming planner, predictor, quiet CLI). Benchmarks/tests now use the new command set.

### Phase 2.5 – Performance Gate
Benchmarks include remote scenarios using the canonical syntax (TCP + gRPC fallback).

### Phase 3 – Hybrid Remote Operations
Augment v5 tasks with:
- Module-aware authorisation (read_only, chroot).
- RPCs powering `list`, `find`, `du`, `df`, `rm`.
- Ensure transfer transport defaults to port 9031 (configurable).

### Phase 3.5 – RDMA Enablement
Future work (defer until after core TCP/gRPC paths and required features are complete).

### Phase 4 – Production Hardening & Packaging
- Package mDNS dependencies, config directories, and blit-utils alongside CLI/daemon.
- Document command surface and admin workflows.

---

## 4. Deliverables Checklist

- [ ] CLI (`copy`, `mirror`, `move`, `scan`, `list`, diagnostics) operational with canonical remote syntax.
- [ ] `blit-core::remote::endpoint` parses `server:/module/...`, `server://...`, discovery forms, and rejects ambiguous inputs.
- [ ] `blit-daemon` loads modules/root from config, supports flags, advertises via mDNS, enforces read_only/chroot semantics, and handles “no exports configured” cleanly.
- [ ] RPC surface supports `list`, `find`, `du`, `df`, `rm`.
- [ ] `blit-utils` implements `scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile` command.
- [ ] Test suite covers transfer permutations, admin workflows, and daemon startup scenarios.
- [ ] Documentation (CLI, blit-daemon, blit-utils) reflects the updated feature set.
- [ ] Benchmarks include remote scenarios over TCP and gRPC fallback using the new commands.

---

## 5. Open Questions & Decisions Needed

| Topic | Decision Needed |
|-------|-----------------|
| Config search order | Confirm precedence (CLI flag → config). No environment variables. |
| Windows service specifics | Identify any mDNS/config nuances when running as a Windows service (e.g., service account working directory, firewall rules). |
| Future admin verbs | Additional utilities beyond `scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile` will be added as requirements are confirmed. |

---

## 6. Next Steps

1. Archive v5 as historical reference (pointer only).
2. Break Phase 0 items into actionable tasks (CLI changes, utils implementation, config loader, mDNS).
3. Begin execution, ensuring DEVLOG/TODO/workflows record progress for seamless hand-offs.
