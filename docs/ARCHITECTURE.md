# Blit Architecture

This document describes the high-level architecture of Blit, a high-performance file transfer tool.

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         User Layer                               │
├─────────────┬─────────────────────────┬─────────────────────────┤
│  blit-cli   │      blit-daemon        │      blit-utils         │
│  (CLI app)  │    (gRPC server)        │   (admin tools)         │
├─────────────┴─────────────────────────┴─────────────────────────┤
│                       blit-core                                  │
│  ┌──────────────┬──────────────┬──────────────┬───────────────┐ │
│  │ Orchestrator │ TransferEng  │ MirrorPlan   │ Remote/gRPC   │ │
│  ├──────────────┼──────────────┼──────────────┼───────────────┤ │
│  │ ChangeJournal│ Enumeration  │ Checksum     │ TarStream     │ │
│  ├──────────────┼──────────────┼──────────────┼───────────────┤ │
│  │ Copy Engine  │ ZeroCopy     │ PerfPredict  │ Config        │ │
│  └──────────────┴──────────────┴──────────────┴───────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                    Platform Abstraction                          │
│           Windows APIs │ macOS APIs │ POSIX/Linux               │
└─────────────────────────────────────────────────────────────────┘
```

## Crate Structure

### blit-core

The core library containing all transfer logic, protocols, and platform abstractions.

**Key Modules:**

| Module | Responsibility |
|--------|----------------|
| `orchestrator` | Coordinates parallel transfer operations |
| `transfer_engine` | Manages end-to-end transfer lifecycle |
| `transfer_facade` | Unified interface for local/remote transfers |
| `mirror_planner` | Computes file differences for sync operations |
| `enumeration` | Directory traversal and file discovery |
| `copy` | Platform-optimized file copying |
| `checksum` | File integrity verification |
| `change_journal` | OS-specific change detection |
| `remote` | gRPC client implementation |
| `tar_stream` | Batched small-file transfers |
| `perf_predictor` | Performance optimization heuristics |

### blit-cli

Command-line interface providing user access to all transfer operations.

**Structure:**
```
blit-cli/
├── main.rs           # Entry point and CLI argument parsing
├── admin.rs          # Administrative commands (du, df, rm, find)
├── list.rs           # Remote listing operations
├── transfers/        # Transfer command implementations
│   ├── mod.rs        # Common transfer logic
│   ├── local.rs      # Local-to-local transfers
│   └── remote.rs     # Remote transfer handling
└── tests/            # Integration tests
```

### blit-daemon

gRPC server for remote file transfer operations.

**Structure:**
```
blit-daemon/
├── main.rs           # Entry point
├── config.rs         # Configuration parsing
├── runtime.rs        # Tokio runtime management
└── service/          # gRPC service implementations
    ├── core.rs       # Main service logic
    ├── admin.rs      # Administrative RPCs
    ├── pull.rs       # Pull (download) operations
    ├── push/         # Push (upload) operations
    │   ├── control.rs    # Control plane (gRPC)
    │   └── data_plane.rs # Data plane (TCP)
    └── util.rs       # Shared utilities
```

### blit-utils

Standalone utilities for daemon administration and diagnostics.

## Data Flow

### Local Transfer

```
Source Path                          Destination Path
     │                                      ▲
     ▼                                      │
┌─────────────┐                      ┌──────────────┐
│ Enumeration │──────────────────────│  Copy Engine │
└─────────────┘                      └──────────────┘
     │                                      ▲
     ▼                                      │
┌─────────────┐    ┌─────────────┐   ┌──────────────┐
│ChangeJournal│───▶│MirrorPlanner│───│ Orchestrator │
│ (optional)  │    │             │   │ (parallel)   │
└─────────────┘    └─────────────┘   └──────────────┘
```

### Remote Push (Client → Server)

```
Client                                    Server
┌──────────────┐                    ┌──────────────┐
│  Enumerate   │                    │ blit-daemon  │
│  Source Dir  │                    │              │
└──────┬───────┘                    └──────────────┘
       │                                   ▲
       ▼                                   │
┌──────────────┐    gRPC Stream     ┌──────────────┐
│ Send Manifest│───────────────────▶│Parse Manifest│
│ (FileHeaders)│                    │ Build NeedList│
└──────────────┘                    └──────┬───────┘
       ▲                                   │
       │         FileList (need)           │
       ◀───────────────────────────────────┘
       │
       ▼
┌──────────────┐    TCP Data Plane   ┌──────────────┐
│ Send Payloads│────────────────────▶│ Write Files  │
│ (parallel)   │                     │              │
└──────────────┘                     └──────────────┘
```

### Remote Pull (Server → Client)

```
Client                                    Server
┌──────────────┐                    ┌──────────────┐
│ PullRequest  │────────────────────│ Enumerate    │
│ (path)       │                    │ Server Path  │
└──────────────┘                    └──────┬───────┘
       │                                   │
       ▼                                   ▼
┌──────────────┐    gRPC Stream     ┌──────────────┐
│ Receive      │◀───────────────────│ Stream Files │
│ File Headers │                    │ (headers+data│
└──────────────┘                    └──────────────┘
       │                                   │
       ▼                            TCP Data Plane
┌──────────────┐                           │
│ Write Local  │◀──────────────────────────┘
│ Files        │
└──────────────┘
```

## Key Abstractions

### TransferPlan

Represents a planned file transfer operation with source/destination pairs and metadata.

```rust
pub struct TransferPlan {
    pub entries: Vec<TransferEntry>,
    pub total_bytes: u64,
    pub file_count: u64,
}

pub struct TransferEntry {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub size: u64,
    pub mtime: SystemTime,
}
```

### MirrorPlan

Extends TransferPlan with deletion information for mirror operations.

```rust
pub struct MirrorPlan {
    pub to_copy: Vec<TransferEntry>,
    pub to_delete: Vec<PathBuf>,
    pub unchanged: u64,
}
```

### RemoteEndpoint

Abstracts remote server connections with unified interface.

```rust
pub struct RemoteEndpoint {
    pub host: String,
    pub port: u16,
    pub module: Option<String>,
    pub path: PathBuf,
}
```

## Platform Abstractions

### Change Journal

Platform-specific change detection for incremental sync optimization.

| Platform | Implementation |
|----------|----------------|
| Windows | USN Change Journal via `DeviceIoControl` |
| macOS | FSEvents API |
| Linux | Fallback to mtime comparison |

### Copy Operations

Platform-optimized file copying strategies.

| Platform | Optimization |
|----------|--------------|
| Windows | `CopyFileExW`, Block Cloning (ReFS) |
| macOS | `clonefile()`, `fcopyfile()` |
| Linux | `copy_file_range()`, `sendfile()` |

### Zero-Copy

Kernel-bypass data transfer when available.

```rust
pub trait ZeroCopy {
    fn supports_zero_copy(&self) -> bool;
    fn zero_copy_transfer(&self, src: &Path, dst: &Path) -> Result<u64>;
}
```

## Protocol Design

### gRPC Services

Defined in `proto/blit.proto`:

```protobuf
service Blit {
  rpc Push(stream ClientPushRequest) returns (stream ServerPushResponse);
  rpc Pull(PullRequest) returns (stream PullChunk);
  rpc List(ListRequest) returns (ListResponse);
  rpc Purge(PurgeRequest) returns (PurgeResponse);
  rpc Find(FindRequest) returns (stream FindEntry);
  rpc DiskUsage(DiskUsageRequest) returns (stream DiskUsageEntry);
  rpc FilesystemStats(FilesystemStatsRequest) returns (FilesystemStatsResponse);
}
```

### Hybrid Data Plane

For large transfers, Blit uses a hybrid approach:

1. **Control Plane (gRPC)**: Manifest exchange, coordination, status
2. **Data Plane (TCP)**: Bulk file content transfer with zero-copy

The `DataTransferNegotiation` message coordinates the handoff:

```protobuf
message DataTransferNegotiation {
  uint32 tcp_port = 1;
  string one_time_token = 2;
  bool tcp_fallback = 3;
  uint32 stream_count = 4;
}
```

## Performance Optimizations

### Parallel Execution

```
┌─────────────────────────────────────────────────────────────┐
│                      Orchestrator                            │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐        │
│  │Worker 1 │  │Worker 2 │  │Worker 3 │  │Worker N │        │
│  │ (copy)  │  │ (copy)  │  │ (copy)  │  │ (copy)  │        │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘        │
│       │            │            │            │              │
│       ▼            ▼            ▼            ▼              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Shared Progress Tracker                │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Small File Batching

Files under a size threshold are batched into tar archives:

```
┌────────────┐  ┌────────────┐  ┌────────────┐
│ file1.txt  │  │ file2.txt  │  │ file3.txt  │
│ (1KB)      │  │ (2KB)      │  │ (500B)     │
└─────┬──────┘  └─────┬──────┘  └─────┬──────┘
      │               │               │
      └───────────────┼───────────────┘
                      ▼
              ┌───────────────┐
              │  TAR Shard    │ ─────▶ Single Transfer
              │  (3.5KB)      │
              └───────────────┘
```

### Performance History

Historical transfer data is used for optimization:

```rust
pub struct PerformanceRecord {
    pub timestamp: SystemTime,
    pub bytes_transferred: u64,
    pub duration_ms: u64,
    pub file_count: u64,
    pub strategy_used: TransferStrategy,
}
```

## Security Considerations

### Current State

- **Transport**: gRPC with optional TLS (not enforced)
- **Authentication**: Token-based (placeholder in proto)
- **Authorization**: Module-level read/write permissions

### Planned Enhancements

- TLS certificate validation
- Per-module access control lists
- Audit logging

## Error Handling

Blit uses `eyre` for error handling with rich context:

```rust
use eyre::{Context, Result};

fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    std::fs::copy(src, dst)
        .context(format!("failed to copy {} to {}", src.display(), dst.display()))?;
    Ok(())
}
```

Error types propagate through the stack with full context for debugging.

## Testing Strategy

### Unit Tests

Located in each module's source file or adjacent `tests/` directory.

### Integration Tests

- `tests/` directory in workspace root
- `crates/blit-cli/tests/` for CLI integration tests

### Test Categories

| Category | Purpose |
|----------|---------|
| `local_transfers` | Local copy/mirror operations |
| `remote_*` | Remote push/pull with daemon |
| `admin_verbs` | Administrative commands |
| `mirror_planner` | Sync algorithm correctness |

## Future Directions

1. **RDMA Support**: Reserved fields in protocol for RDMA data plane
2. **Incremental Sync**: Enhanced change journal integration
3. **Compression**: Optional transfer compression
4. **Encryption**: End-to-end encryption for data plane
5. **Clustering**: Multi-daemon coordination
