# Blit API Reference

This document provides API reference for developers integrating with or extending Blit.

## gRPC API

The Blit daemon exposes a gRPC service defined in `proto/blit.proto`.

### Service Definition

```protobuf
service Blit {
  // The ONE byte-moving RPC: a role-tagged bidirectional transfer
  // session (contract: docs/TRANSFER_SESSION.md). Push and PullSync
  // were deleted whole at cutover (otp-10c-2, D-2026-07-05-1).
  rpc Transfer(stream TransferFrame) returns (stream TransferFrame);

  // Daemon-to-daemon remote→remote trigger + progress relay (no
  // payload bytes ever cross this RPC).
  rpc DelegatedPull(DelegatedPullRequest) returns (stream DelegatedPullProgress);

  // Admin / query
  rpc List(ListRequest) returns (ListResponse);
  rpc Purge(PurgeRequest) returns (PurgeResponse);
  rpc CompletePath(CompletionRequest) returns (CompletionResponse);
  rpc ListModules(ListModulesRequest) returns (ListModulesResponse);
  rpc Find(FindRequest) returns (stream FindEntry);
  rpc DiskUsage(DiskUsageRequest) returns (stream DiskUsageEntry);
  rpc FilesystemStats(FilesystemStatsRequest) returns (FilesystemStatsResponse);

  // Daemon state / observability
  rpc GetState(GetStateRequest) returns (DaemonState);
  rpc Subscribe(SubscribeRequest) returns (stream DaemonEvent);
  rpc CancelJob(CancelJobRequest) returns (CancelJobResponse);
  rpc ClearRecent(ClearRecentRequest) returns (ClearRecentResponse);
}
```

`proto/blit.proto` is the source of truth; this page summarizes.

---

## Transfer Session

Every remote transfer — push-shaped or pull-shaped, either verb —
rides the single bidirectional `Transfer` RPC. The initiator declares
which end is SOURCE and which is DESTINATION in `SessionOpen`; the
frames each end may send are determined by that ROLE, never by which
end is the gRPC client. The full state machine (same-build handshake,
streaming manifest, need batches, resume block phase, mirror pass,
carriers, error/cancel semantics) is specified in
`docs/TRANSFER_SESSION.md` — the authoritative contract.

Highlights:

- **Same-build only** (D-2026-07-05-2): the `SessionHello` exchange
  refuses any peer not built from identical sources. There is no
  version negotiation.
- **Carriers**: payload bytes ride the TCP data plane by default
  (parallel streams, one-time tokens, mid-transfer resize) or the
  in-stream carrier (`TransferFrame` payload frames on the control
  stream) when requested (`--force-grpc`) or when the responder
  cannot bind sockets.
- **One summary shape**: `TransferSummary` — files/bytes transferred,
  files resumed, entries deleted, carrier used — computed by the
  DESTINATION (the end that wrote the bytes).

Remote→remote transfers are delegated: the CLI calls `DelegatedPull`
on the destination daemon, which validates the request through its
delegation gate and initiates a `Transfer` session against the source
daemon itself. The `DelegatedPullProgress` stream back to the CLI
carries only progress counters and diagnostics — never file content.

---

## List Operation

Unary RPC for directory listing.

### Messages

#### ListRequest

```protobuf
message ListRequest {
  string module = 1;  // Module name
  string path = 2;    // Path within module
}
```

#### ListResponse

```protobuf
message ListResponse {
  repeated FileInfo entries = 1;
}

message FileInfo {
  string name = 1;
  bool is_dir = 2;
  uint64 size = 3;
  int64 mtime_seconds = 4;
}
```

### Example Usage (Rust)

```rust
let request = ListRequest {
    module: "backup".to_string(),
    path: "/documents".to_string(),
};

let response = client.list(request).await?;
for entry in response.entries {
    println!("{} {} bytes", entry.name, entry.size);
}
```

---

## Purge Operation

Delete files/directories on the server (used for mirror operations).

### Messages

#### PurgeRequest

```protobuf
message PurgeRequest {
  string module = 1;
  repeated string paths_to_delete = 2;
}
```

#### PurgeResponse

```protobuf
message PurgeResponse {
  uint64 files_deleted = 1;
}
```

---

## Find Operation

Server-side streaming RPC for recursive file search.

### Messages

#### FindRequest

```protobuf
message FindRequest {
  string module = 1;
  string start_path = 2;
  string pattern = 3;              // Glob pattern
  bool case_sensitive = 4;
  bool include_files = 5;
  bool include_directories = 6;
  uint32 max_results = 7;
}
```

#### FindEntry

```protobuf
message FindEntry {
  string relative_path = 1;
  bool is_dir = 2;
  uint64 size = 3;
  int64 mtime_seconds = 4;
}
```

---

## DiskUsage Operation

Server-side streaming RPC for disk usage analysis.

### Messages

#### DiskUsageRequest

```protobuf
message DiskUsageRequest {
  string module = 1;
  string start_path = 2;
  uint32 max_depth = 3;
}
```

#### DiskUsageEntry

```protobuf
message DiskUsageEntry {
  string relative_path = 1;
  uint64 byte_total = 2;
  uint64 file_count = 3;
  uint64 dir_count = 4;
}
```

---

## FilesystemStats Operation

Unary RPC for filesystem capacity information.

### Messages

#### FilesystemStatsRequest

```protobuf
message FilesystemStatsRequest {
  string module = 1;
}
```

#### FilesystemStatsResponse

```protobuf
message FilesystemStatsResponse {
  string module = 1;
  uint64 total_bytes = 2;
  uint64 used_bytes = 3;
  uint64 free_bytes = 4;
}
```

---

## Data Plane Negotiation

The DESTINATION advertises its receive capacity (`CapacityProfile`)
in `SessionOpen`/`SessionAccept`; the responder grants TCP data-plane
access with a `DataPlaneGrant` (port + one-time token + epoch-0
sub-token). The connection-initiating end dials the sockets; byte
direction within a socket is set by role, not by who dialed.
Mid-transfer stream resize rides `DataPlaneResize`/`DataPlaneResizeAck`
frames. When no grant is issued, payloads ride the in-stream carrier
on the control stream.

---

## Rust Client Example

```rust
use blit_core::remote::transfer::session_client::{run_push_session, PushSessionOptions};
use blit_core::remote::transfer::source::FsTransferSource;
use blit_core::remote::RemoteEndpoint;
use std::sync::Arc;

async fn push_files() -> eyre::Result<()> {
    let endpoint = RemoteEndpoint::parse("192.168.1.100:50051:/backup/docs")?;

    let outcome = run_push_session(
        &endpoint,
        Arc::new(FsTransferSource::new("/local/documents".into())),
        PushSessionOptions::default(),
    )
    .await?;

    println!(
        "Transferred {} files, {} bytes",
        outcome.summary.files_transferred, outcome.summary.bytes_transferred
    );

    Ok(())
}
```

---

## Error Handling

gRPC errors follow standard status codes:

| Code | Meaning |
|------|---------|
| `OK` | Success |
| `NOT_FOUND` | Module or path doesn't exist |
| `PERMISSION_DENIED` | Read-only module or access denied |
| `INVALID_ARGUMENT` | Bad request parameters |
| `RESOURCE_EXHAUSTED` | Disk full or quota exceeded |
| `INTERNAL` | Server error |
| `UNAVAILABLE` | Server not ready |

### Error Response

Errors include descriptive messages:

```rust
match result {
    Err(status) if status.code() == Code::NotFound => {
        eprintln!("Path not found: {}", status.message());
    }
    Err(status) => {
        eprintln!("Error {}: {}", status.code(), status.message());
    }
    Ok(_) => {}
}
```

---

## Module Configuration

Modules provide named access points to filesystem paths.

### ListModules RPC

```protobuf
message ListModulesRequest {}

message ListModulesResponse {
  repeated ModuleInfo modules = 1;
}

message ModuleInfo {
  string name = 1;
  string path = 2;
  bool read_only = 3;
}
```

### Access Patterns

```
# Module access
server://host:port/module-name/relative/path

# Root access (no modules configured)
server://host:port/relative/path
```

---

## Versioning

There is none, by decision (D-2026-07-05-2): client and daemon
interoperate only when built from the same sources, and the session
handshake refuses a mismatched peer at open. The proto package name
(`blit.v2`) is a namespace, not a compatibility promise.
