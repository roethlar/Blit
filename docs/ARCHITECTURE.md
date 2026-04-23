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
│  ┌────────────────────────────────────────────────────────────┐ │
│  │   Unified Transfer Pipeline (execute_sink_pipeline*)        │ │
│  │   TransferSource → plan_transfer_payloads → TransferSink    │ │
│  └────────────────────────────────────────────────────────────┘ │
│  ┌──────────────┬──────────────┬──────────────┬───────────────┐ │
│  │ Orchestrator │ MirrorPlan   │ Remote/gRPC  │ ChangeJournal │ │
│  ├──────────────┼──────────────┼──────────────┼───────────────┤ │
│  │ Enumeration  │ Checksum     │ TarStream    │ Copy Engine   │ │
│  ├──────────────┼──────────────┼──────────────┼───────────────┤ │
│  │ ZeroCopy     │ PerfPredict  │ Config       │ AutoTune      │ │
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
| `remote::transfer::pipeline` | `execute_sink_pipeline` + `execute_sink_pipeline_streaming` — the single entry point for every src→dst combination |
| `remote::transfer::source` | `TransferSource` trait (read side) + `FsTransferSource`, `RemoteTransferSource` implementations |
| `remote::transfer::sink` | `TransferSink` trait (write side) + `FsTransferSink`, `DataPlaneSink`, `GrpcFallbackSink`, `NullSink` implementations |
| `remote::transfer::payload` | `plan_transfer_payloads` — classifies files into tar shards / raw bundles / large-file payloads |
| `orchestrator` | Local transfer entry: journal fast-path, mirror deletions, perf history; delegates execution to `execute_sink_pipeline` |
| `remote::push::client` | Push-side negotiation; feeds need-list payloads into `execute_sink_pipeline_streaming` over N `DataPlaneSink`s |
| `mirror_planner` | Computes file differences for sync operations |
| `enumeration` | Directory traversal and file discovery |
| `copy` | Platform-optimized file copying (zero-copy cascade: copy_file_range, sendfile, clonefile, block clone) |
| `checksum` | File integrity verification |
| `change_journal` | OS-specific change detection (USN on Windows, FSEvents on macOS, metadata snapshot on Linux) |
| `remote` | gRPC control plane + TCP data plane |
| `tar_stream` | Batched small-file transfers |
| `auto_tune` | Dynamic tuning of chunk sizes and stream counts based on manifest size |
| `perf_predictor` | Performance optimization heuristics |
| `perf_history` | Versioned JSONL performance record storage |
| `fs_capability` | Per-filesystem capability detection and caching |

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

**Structure:**
```
blit-utils/
├── main.rs           # Entry point, dispatches to subcommands
├── cli.rs            # Clap argument definitions
├── util.rs           # Shared helpers (endpoint parsing, byte formatting)
├── scan.rs           # mDNS daemon discovery
├── list_modules.rs   # ListModules RPC wrapper
├── ls.rs             # Remote/local directory listing
├── find.rs           # Recursive remote file search
├── du.rs             # Remote disk usage summary
├── df.rs             # Remote filesystem statistics
├── rm.rs             # Remote file/directory deletion
├── completions.rs    # Shell path completion via CompletePath RPC
└── profile.rs        # Local performance history viewer
```

All remote commands connect via gRPC to a running daemon. Output defaults to
human-readable tables; `--json` emits machine-parsable JSON for scripting.

## Data Flow: Unified Transfer Pipeline

Every transfer — local→local, local→remote push, remote→local pull, and
remote→remote — routes through the same pipeline. Only the concrete
`TransferSource` and `TransferSink` implementations differ per direction.

```
    TransferSource             plan_transfer_payloads          TransferSink(s)
    ──────────────             ──────────────────────          ────────────────
    ┌──────────────┐           ┌────────────────────┐          ┌──────────────┐
    │ .scan()      │──headers─▶│ classify:          │─payloads▶│ .write_      │
    │              │           │  tar shards /      │          │   payload()  │
    │ .prepare_    │──prepared─│  raw bundles /     │          │              │
    │   payload()  │  payloads │  large files       │          │ .finish()    │
    │              │           │                    │          │              │
    │ .open_file() │           │ PlanOptions tunes  │          │ .root()      │
    └──────────────┘           │  chunk_bytes       │          └──────────────┘
                               └────────────────────┘
                                         │
                                         ▼
                     execute_sink_pipeline[_streaming]
                     • round-robin across N sinks
                     • per-sink preparation prefetch
                     • aggregated SinkOutcome
```

### Source implementations

- **`FsTransferSource`** — reads files from a local path; used for local→local
  (client side) and for remote pull (daemon side).
- **`RemoteTransferSource`** — reads files from another daemon via a
  `RemotePullClient`; used for remote→remote transfers.

### Sink implementations

- **`FsTransferSink`** — writes files to a local path using the zero-copy
  cascade (`copy_file_range`, `sendfile`, `clonefile`, block clone); used for
  local→local (client side).
- **`DataPlaneSink`** — wraps a single TCP `DataPlaneSession`; used for push
  (client→daemon) and pull (daemon→client). Multi-stream transfers create one
  sink per TCP connection.
- **`GrpcFallbackSink`** — sends payloads over the gRPC control plane; used
  when `--force-grpc` is set or TCP is unavailable.
- **`NullSink`** — discards all writes, used for benchmarking source read
  throughput in isolation.

### Per-direction wiring

| Direction | Source | Sink |
|---|---|---|
| local → local | `FsTransferSource` | `FsTransferSink` |
| local → remote (push, TCP) | `FsTransferSource` | N × `DataPlaneSink` |
| local → remote (push, gRPC fallback) | `FsTransferSource` | `GrpcFallbackSink` |
| remote → local (pull, TCP) | daemon's `FsTransferSource` | N × `DataPlaneSink` (on daemon) |
| remote → remote | `RemoteTransferSource` | N × `DataPlaneSink` |

### Destination resolution

Before routing hits the pipeline, `resolve_destination` in
`crates/blit-cli/src/transfers/mod.rs` applies rsync-style trailing-slash
semantics uniformly across all directions:

- Source ends with `/`, `/.`, or is exactly `.` → merge contents into dest
- Dest has trailing slash or is an existing local directory → nest under dest
- Otherwise → use dest as the exact target (rename-style)

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

| Platform | Primary | Fallback |
|----------|---------|----------|
| macOS | `clonefile()` (APFS CoW) | `fcopyfile()` |
| Linux | `copy_file_range()` (4.5+) | `sendfile()` |
| Windows | `CopyFileExW` | Block Cloning (ReFS) |

### Filesystem Capability Probing

Blit detects the filesystem type at runtime and maps it to accurate capability flags.

| Platform | Detection Method |
|----------|-----------------|
| macOS | `statfs` → `f_fstypename` |
| Linux | `statfs` → `f_type` magic number mapping |
| Windows | `GetVolumeInformationW` → filesystem name |

Detected capabilities (reflink, sparse files, xattrs, sendfile, copy_file_range,
block cloning) are cached per device ID and used by the planner and copy engine
to select the optimal transfer strategy.

Supported filesystem types: APFS, HFS+, btrfs, XFS, ext2/3/4, ZFS, tmpfs,
NFS/CIFS/SMB, NTFS, ReFS.

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
| `remote_transfer_edges` | Edge cases: nested dirs, empty dirs, many small files |
| `admin_verbs` | Administrative commands (list-modules, completions, find, rm) |
| `mirror_planner` | Sync algorithm correctness |
| `perf_predictor` | Adaptive predictor convergence and stability |
| `perf_history` | Schema versioning and migration |
| `fs_capability` | Filesystem detection and capability probing |

## Future Directions

1. **RDMA Support**: Reserved fields in protocol for RDMA data plane
2. **Incremental Sync**: Enhanced change journal integration
3. **Compression**: Optional transfer compression
4. **Encryption**: End-to-end encryption for data plane
5. **Clustering**: Multi-daemon coordination
