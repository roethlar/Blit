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
