# Blit Architecture

This document describes the high-level architecture of Blit, a high-performance file transfer tool.

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         User Layer                               │
├───────────────────────────────────────┬─────────────────────────┤
│  blit (CLI) │ blit-tui (TUI) │         │      blit-daemon         │
│  blit-prometheus-bridge (exporter)     │      (gRPC server)       │
│  copy/mirror/move/scan/list/           │                          │
│  list-modules/ls/find/du/df/           │                          │
│  rm/completions/profile/diagnostics    │                          │
├───────────────────────────────────────┴─────────────────────────┤
│      blit-app  (shared orchestration: endpoints, dispatch,       │
│                client, admin verbs, diagnostics)                 │
├──────────────────────────────────────────────────────────────────┤
│                       blit-core                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │   Unified Transfer Pipeline (execute_sink_pipeline*)        │ │
│  │   TransferSource → plan_transfer_payloads → TransferSink    │ │
│  └────────────────────────────────────────────────────────────┘ │
│  ┌──────────────┬──────────────┬──────────────┬───────────────┐ │
│  │ Session      │ MirrorPlan   │ Remote/gRPC  │ Dial          │ │
│  ├──────────────┼──────────────┼──────────────┼───────────────┤ │
│  │ Enumeration  │ Checksum     │ TarStream    │ Copy          │ │
│  ├──────────────┼──────────────┼──────────────┼───────────────┤ │
│  │ ZeroCopy     │ PerfPredict  │ Config       │ PerfHistory   │ │
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
| `remote::transfer::source` | `TransferSource` trait (read side) + `FsTransferSource` implementation |
| `remote::transfer::sink` | `TransferSink` trait (write side) + `FsTransferSink`, `DataPlaneSink`, `NullSink` implementations |
| `remote::transfer::payload` | `plan_transfer_payloads` — classifies files into tar shards / raw bundles / large-file payloads |
| `transfer_session` | The ONE transfer choreography (`TransferSession`, both roles); the per-direction driver modules were deleted at cutover (otp-10c-2, D-2026-07-05-1) |
| `mirror_planner` | Computes file differences for sync operations |
| `enumeration` | Directory traversal and file discovery |
| `copy` | Platform-optimized file copying (zero-copy cascade: copy_file_range, sendfile, clonefile, block clone) |
| `checksum` | File integrity verification |
| `remote` | gRPC control plane + TCP data plane |
| `tar_stream` | Batched small-file transfers |
| `perf_predictor` | Performance optimization heuristics |
| `perf_history` | Versioned JSONL performance record storage |
| `fs_capability` | Per-filesystem capability detection and caching |

### blit-cli (produces the `blit` binary)

Command-line interface providing user access to transfer operations and
all admin verbs. The Cargo package is `blit-cli`; the produced binary
is named `blit` (`[[bin]] name = "blit"`). Admin verbs originally
scoped as a separate `blit-utils` artifact were merged into this binary
during Phase 3.

**Structure:**
```
blit-cli/
├── main.rs           # Entry point and CLI argument parsing
├── cli.rs            # Clap argument definitions
├── transfers/        # Transfer command implementations
│   ├── mod.rs        # Common transfer logic
│   ├── local.rs      # Local-to-local transfers
│   └── remote.rs     # Remote transfer handling
├── scan.rs           # mDNS daemon discovery
├── list_modules.rs   # ListModules RPC wrapper
├── ls.rs             # Remote/local directory listing
│                     #  (smart-dispatches bare hosts to list_modules)
├── find.rs           # Recursive remote file search
├── du.rs             # Remote disk usage summary
├── df.rs             # Remote filesystem statistics
├── rm.rs             # Remote file/directory deletion
├── completions.rs    # Shell completion script generation +
│                     #  CompletePath-backed remote completions
├── profile.rs        # Local performance history viewer
├── diagnostics.rs    # Diagnostic dump
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

### blit-app

Shared application/orchestration library sitting between the binaries
(`blit-cli`, `blit-tui`, and `blit-prometheus-bridge`) and `blit-core`.
Holds the logic the front-ends need so it isn't duplicated or trapped in
the CLI. Public modules (`crates/blit-app/src/lib.rs`):

| Module | Responsibility |
|--------|----------------|
| `endpoints` | Endpoint parsing/classification (local vs remote module/root) |
| `transfers` | Transfer dispatch + destination resolution + filter assembly (`dispatch`, `resolution`, `filter`, `local`, `remote`, `remote_remote_direct`) |
| `client` | Control-plane gRPC client with a DNS-aware (outer-timeout) connect |
| `admin` | Admin-verb implementations (`ls`, `find`, `du`, `df`, `rm`, `jobs`, `list_modules`) — `jobs` is what the TUI and the Prometheus bridge call for `GetState`/`Subscribe`/`CancelJob`/`ClearRecent` |
| `check` | Local-tree compare core |
| `scan` | mDNS daemon discovery |
| `diagnostics` | Diagnostics-dump emitter |
| `profile` | Performance-history / predictor reporting |
| `display` | Shared human-readable formatting helpers |

The CLI and TUI are thin shells over these helpers; the Prometheus
bridge consumes `admin::jobs` for its scrape (`GetState`).

### blit-tui

Terminal UI (ratatui + crossterm) producing the `blit-tui` binary. The
active model is the **Phase 6 dual-pane Pick-not-Type design**
(`docs/plan/TUI_REWORK.md`, M1–M6): active pane = source, inactive
pane = destination, visible action bar (Copy / Mirror / Move / Delete
/ Verify), editable path bars, `/` search, and a fan-out batch table
for multi-daemon transfers. The v0.1.0 release shipped the original
F1–F4 model (trigger / transfers / browse / profile-verify-diagnostics);
`TUI_DESIGN.md` describes that baseline and is superseded by the
rework plan.

The TUI is a read-mostly control surface over the daemon: it
`Subscribe`s to each discovered daemon's `DaemonEvent` stream and
renders live transfer state from `GetState`, can launch transfers /
`CancelJob` / `ClearRecent`, and supports configurable keybindings
and theming via `[keys]` / `[theme]` config. Daemon discovery is mDNS;
multi-daemon transfer monitoring merges per-daemon Subscribe streams
into one event channel.

### blit-prometheus-bridge

Standalone Prometheus exporter producing the `blit-prometheus-bridge`
binary. A minimal hand-rolled HTTP server serves `GET /metrics`; each
scrape triggers a fresh `GetState` against the configured daemon (pull
model, no background poll) and formats the result as Prometheus text. A
failed/timed-out scrape still returns `200` with `blit_daemon_up 0` so
the target registers as up-but-down rather than a scrape error.

### Admin verbs

The admin verbs (`scan`, `list-modules`, `ls`, `find`, `du`, `df`,
`rm`, `completions`, `profile`) live inside `crates/blit-cli`
alongside the transfer verbs — see the `blit-cli` structure above.
There is no separate `blit-utils` crate or binary; the
[`docs/plan/BLIT_UTILS_PLAN.md`](./plan/BLIT_UTILS_PLAN.md)
document captures the original command-matrix design but is marked
superseded for the artifact-shape question.

All remote commands connect via gRPC to a running daemon. Output
defaults to human-readable tables; `--json` emits machine-parsable
JSON for scripting.

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
  (client side) and for remote pull (daemon side). The only source
  implementation: every transfer reads from the filesystem of whichever end
  holds the data. (`RemoteTransferSource`, the CLI-relay read half, was
  deleted with `--relay-via-cli` at otp-10c-1, D-2026-07-11-1.)

### Sink implementations

- **`FsTransferSink`** — writes files to a local path using the zero-copy
  cascade (`copy_file_range`, `sendfile`, `clonefile`, block clone); used for
  local→local (client side).
- **`DataPlaneSink`** — wraps a single TCP `DataPlaneSession`; used for push
  (client→daemon) and pull (daemon→client). Multi-stream transfers create one
  sink per TCP connection.
- **`NullSink`** — discards all writes, used for benchmarking source read
  throughput in isolation.

(With `--force-grpc`, or when TCP is unavailable, the session's
in-stream byte carrier sends payloads as `TransferFrame`s on the
control stream — a carrier option inside the one choreography, not a
separate sink; the old wire-specific gRPC fallback sinks died with
the Push/PullSync RPCs at otp-10c-2.)

### Per-direction wiring

| Direction | Source | Sink |
|---|---|---|
| local → local | `FsTransferSource` | `FsTransferSink` |
| local → remote (push, TCP) | `FsTransferSource` | N × `DataPlaneSink` |
| local → remote (push, in-stream carrier) | `FsTransferSource` | session control-stream frames |
| remote → local (pull, TCP) | daemon's `FsTransferSource` | N × `DataPlaneSink` (on daemon) |
| remote → remote (delegated) | source daemon's `FsTransferSource` | destination daemon's receive path (the CLI only triggers and relays progress) |

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

Defined in `proto/blit.proto` — a single `Blit` service:

```protobuf
service Blit {
  // Transfer — the ONE byte-moving RPC (role-tagged session; Push and
  // PullSync were deleted whole at cutover, otp-10c-2 / D-2026-07-05-1;
  // the server-streaming Pull RPC went earlier at ue-r2-1h)
  rpc Transfer(stream TransferFrame) returns (stream TransferFrame);
  rpc DelegatedPull(DelegatedPullRequest) returns (stream DelegatedPullProgress);

  // Admin / query
  rpc List(ListRequest) returns (ListResponse);
  rpc Purge(PurgeRequest) returns (PurgeResponse);
  rpc CompletePath(CompletionRequest) returns (CompletionResponse);
  rpc ListModules(ListModulesRequest) returns (ListModulesResponse);
  rpc Find(FindRequest) returns (stream FindEntry);
  rpc DiskUsage(DiskUsageRequest) returns (stream DiskUsageEntry);
  rpc FilesystemStats(FilesystemStatsRequest) returns (FilesystemStatsResponse);

  // Daemon state / observability (consumed by the TUI + Prometheus bridge)
  rpc GetState(GetStateRequest) returns (DaemonState);
  rpc Subscribe(SubscribeRequest) returns (stream DaemonEvent);
  rpc CancelJob(CancelJobRequest) returns (CancelJobResponse);
  rpc ClearRecent(ClearRecentRequest) returns (ClearRecentResponse);
}
```

`DelegatedPull` lets a daemon pull from another daemon on a client's
behalf (remote→remote). `GetState` / `Subscribe` expose live transfer
state; `CancelJob` cancels an in-flight transfer (authorized to the
originating peer); `ClearRecent` wipes the recent-transfers ring.

### Hybrid Data Plane

For large transfers, Blit uses a hybrid approach:

1. **Control Plane (gRPC)**: The `Transfer` session's frames —
   manifest, need batches, resume hashes, summary, errors
2. **Data Plane (TCP)**: Bulk file content transfer with zero-copy

The session coordinates the handoff itself: the responder issues a
`DataPlaneGrant` frame (port + one-time token + epoch-0 sub-token),
the connection-initiating end dials, and mid-transfer resize rides
`DataPlaneResize`/`DataPlaneResizeAck` frames. With no grant, payload
bytes ride the in-stream carrier on the control stream. (The old
`DataTransferNegotiation` message died with the Push/PullSync RPCs at
otp-10c-2.)

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
2. **Incremental Sync**: journal-assisted no-op detection as a negotiated session capability (sound replay; designed at otp-11, docs/plan/OTP11_LOCAL_SESSION.md D3)
3. **Compression**: Optional transfer compression
4. **Encryption**: End-to-end encryption for data plane
5. **Clustering**: Multi-daemon coordination
