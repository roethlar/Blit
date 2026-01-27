# Blit API Reference

This document provides API reference for developers integrating with or extending Blit.

## gRPC API

The Blit daemon exposes a gRPC service defined in `proto/blit.proto`.

### Service Definition

```protobuf
service Blit {
  rpc Push(stream ClientPushRequest) returns (stream ServerPushResponse);
  rpc Pull(PullRequest) returns (stream PullChunk);
  rpc List(ListRequest) returns (ListResponse);
  rpc Purge(PurgeRequest) returns (PurgeResponse);
  rpc CompletePath(CompletionRequest) returns (CompletionResponse);
  rpc ListModules(ListModulesRequest) returns (ListModulesResponse);
  rpc Find(FindRequest) returns (stream FindEntry);
  rpc DiskUsage(DiskUsageRequest) returns (stream DiskUsageEntry);
  rpc FilesystemStats(FilesystemStatsRequest) returns (FilesystemStatsResponse);
}
```

---

## Push Operation

Bidirectional streaming RPC for uploading files from client to server.

### Protocol Flow

```
Client                                    Server
   │                                         │
   ├──── PushHeader ─────────────────────────▶│
   │                                         │
   ├──── FileHeader (file 1) ────────────────▶│
   ├──── FileHeader (file 2) ────────────────▶│
   ├──── FileHeader (file N) ────────────────▶│
   ├──── ManifestComplete ───────────────────▶│
   │                                         │
   │◀─── DataTransferNegotiation ────────────┤  (TCP handoff)
   │◀─── FileList (need list) ───────────────┤
   │                                         │
   ├──── FileData / TarShard ────────────────▶│  (via TCP or gRPC)
   ├──── UploadComplete ─────────────────────▶│
   │                                         │
   │◀─── PushSummary ────────────────────────┤
   │                                         │
```

### Messages

#### ClientPushRequest

```protobuf
message ClientPushRequest {
  oneof payload {
    PushHeader header = 1;
    FileHeader file_manifest = 2;
    ManifestComplete manifest_complete = 3;
    FileData file_data = 4;
    UploadComplete upload_complete = 5;
    TarShardHeader tar_shard_header = 6;
    TarShardChunk tar_shard_chunk = 7;
    TarShardComplete tar_shard_complete = 8;
  }
}
```

#### PushHeader

Initial message specifying transfer parameters.

```protobuf
message PushHeader {
  string module = 1;           // Target module name
  bool mirror_mode = 2;        // Enable mirror (delete extra files)
  string destination_path = 3; // Relative path within module
  bool force_grpc = 4;         // Disable TCP data plane
}
```

#### FileHeader

Metadata for a single file in the manifest.

```protobuf
message FileHeader {
  string relative_path = 1;    // Path relative to destination
  uint64 size = 2;             // File size in bytes
  int64 mtime_seconds = 3;     // Modification time (Unix epoch)
  uint32 permissions = 4;      // POSIX permissions
}
```

#### ServerPushResponse

```protobuf
message ServerPushResponse {
  oneof payload {
    Ack ack = 1;
    FileList files_to_upload = 2;  // Files server needs
    PushSummary summary = 3;        // Final transfer summary
    DataTransferNegotiation negotiation = 4;
  }
}
```

#### PushSummary

```protobuf
message PushSummary {
  uint64 files_transferred = 1;
  uint64 bytes_transferred = 2;
  uint64 bytes_zero_copy = 3;
  bool tcp_fallback_used = 4;
  uint64 entries_deleted = 5;  // Mirror mode deletions
}
```

---

## Pull Operation

Server-side streaming RPC for downloading files from server to client.

### Protocol Flow

```
Client                                    Server
   │                                         │
   ├──── PullRequest ────────────────────────▶│
   │                                         │
   │◀─── DataTransferNegotiation ────────────┤  (optional TCP handoff)
   │◀─── FileHeader (file 1) ────────────────┤
   │◀─── FileData chunks ────────────────────┤
   │◀─── FileHeader (file 2) ────────────────┤
   │◀─── FileData chunks ────────────────────┤
   │◀─── PullSummary ────────────────────────┤
   │                                         │
```

### Messages

#### PullRequest

```protobuf
message PullRequest {
  string module = 1;        // Source module name
  string path = 2;          // Path within module
  bool force_grpc = 3;      // Disable TCP data plane
  bool metadata_only = 4;   // Return headers only, no content
}
```

#### PullChunk

```protobuf
message PullChunk {
  oneof payload {
    FileHeader file_header = 1;
    FileData file_data = 2;
    DataTransferNegotiation negotiation = 3;
    PullSummary summary = 4;
  }
}
```

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

## Data Transfer Negotiation

For high-performance transfers, Blit negotiates a TCP data plane.

```protobuf
message DataTransferNegotiation {
  uint32 tcp_port = 1;           // Port for TCP connections
  string one_time_token = 2;     // Authentication token
  bool tcp_fallback = 3;         // True if TCP unavailable
  uint32 stream_count = 4;       // Number of parallel TCP streams
}
```

### TCP Data Plane Protocol

1. Client receives `DataTransferNegotiation` via gRPC
2. Client opens `stream_count` TCP connections to `tcp_port`
3. Client sends `one_time_token` on each connection
4. File data transferred in parallel across connections
5. Client signals completion via gRPC

---

## Rust Client Example

```rust
use blit_core::remote::{RemoteEndpoint, RemotePushClient};
use std::path::PathBuf;

async fn push_files() -> eyre::Result<()> {
    let endpoint = RemoteEndpoint::parse("server://192.168.1.100:50051/backup/docs")?;

    let client = RemotePushClient::connect(&endpoint).await?;

    let report = client
        .push(
            PathBuf::from("/local/documents"),
            false, // mirror_mode
            false, // dry_run
            false, // force_grpc
        )
        .await?;

    println!("Transferred {} files, {} bytes",
             report.files_transferred,
             report.bytes_transferred);

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

The gRPC API is versioned via the package name:

```protobuf
package blit.v2;
```

Breaking changes will increment the version number. Clients should specify the expected version when connecting.
