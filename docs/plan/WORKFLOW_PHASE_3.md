# Phase 3: Remote Operations (Hybrid Transport)

**Goal**: Implement network file transfer operations with chosen transport architecture
**Duration**: 7-10 days
**Prerequisites**: Phase 2 complete, Phase 2.5 passed
**Status**: Not started
**Critical Path**: Transport architecture implementation

## Overview

Phase 3 implements remote operations, enabling the client to push files to and pull files from a remote daemon. This phase builds on the local orchestrator from Phase 2 and adds network communication via the gRPC service definitions.

### Architectural Decision Required

Before starting Phase 3, confirm the transport architecture choice:

#### Option A: gRPC-Only Transport (Current Proto)
- **Use existing** `proto/blit.proto` with `FileData` in gRPC messages
- Simpler implementation, faster to market
- May have performance overhead for large transfers

#### Option B: Hybrid Transport (Plan v4 Specification)
- **Refactor** `proto/blit.proto` to use control/data plane separation
- Maximum performance via raw TCP + zero-copy
- More complex but matches original architectural vision

**Decision Status** (per greenfield_plan_v5.md):
- ✅ **DECIDED**: Hybrid Transport (Option B)
- This workflow implements the authoritative v5 architecture
- Option A (gRPC-only) is NOT pursued per v5 plan

> **Security & Resilience Requirements (per greenfield_plan_v5.md)**
> - ✅ **REQUIRED**: Data-plane token must be cryptographically strong (e.g., signed JWT with nonce + expiry)
> - ✅ **REQUIRED**: Server must bind accepted socket to token before zero-copy writes (prevents replay attacks)
> - ✅ **REQUIRED**: Automatic fallback to gRPC-streamed data if TCP port unreachable (firewall/NAT)
> - ✅ **REQUIRED**: Emit warning when falling back to gRPC data plane
> - ✅ **REQUIRED**: Support advanced override `--force-grpc-data` for locked-down environments

### Success Criteria

- ✅ `blit push <local> <remote>` works over network
- ✅ `blit pull <remote> <local>` works over network
- ✅ `blit list <remote>` shows remote directory contents
- ✅ `blit purge <remote>` deletes files on server
- ✅ Hybrid transport uses zero-copy for data plane (if Option B)
- ✅ Error handling robust for network failures
- ✅ Progress reporting functional
- ✅ Integration tests pass

## Day 1: Protocol & Service Foundations (6-8 hours)

### Task 3.1.1: Finalize Transport Architecture Decision
**Priority**: 🔴 Critical
**Effort**: 1 hour (discussion/planning)
**Output**: Clear architectural direction

**Decision Criteria**:
| Factor | Option A (gRPC-Only) | Option B (Hybrid) |
|--------|---------------------|-------------------|
| Phase 2.5 Performance | ≥95% | <95% or marginal |
| Implementation Time | 7-8 days | 9-10 days |
| Complexity | Lower | Higher |
| Maximum Performance | Good | Excellent |
| v1 Parity | Likely | Guaranteed |

**Action**: Document decision in `DEVLOG.md` and proceed with chosen path

### Task 3.1.2A: Refactor Proto for Hybrid Transport (Option B)
**Priority**: 🔴 Critical (if Option B chosen)
**Effort**: 2-3 hours
**Skip**: If Option A chosen

**Action**: Update `proto/blit.proto` to remove `FileData` from `ClientPushRequest` and add `DataTransferNegotiation`

```protobuf
// proto/blit.proto - Hybrid Transport Version

syntax = "proto3";
package blit.v2;

service Blit {
  // Control plane: Bidirectional stream for negotiation and metadata
  rpc Push(stream ClientPushRequest) returns (stream ServerPushResponse);

  // Other services remain the same
  rpc Pull(PullRequest) returns (stream PullChunk);
  rpc List(ListRequest) returns (ListResponse);
  rpc Purge(PurgeRequest) returns (PurgeResponse);
}

// --- Push Operation (Hybrid Transport) ---

message ClientPushRequest {
  oneof payload {
    PushHeader header = 1;
    FileHeader file_manifest = 2;
    ManifestComplete manifest_complete = 3;
    // FileData removed - sent over data plane instead
  }
}

message ServerPushResponse {
  oneof payload {
    Ack ack = 1;
    DataTransferNegotiation transfer_negotiation = 2;  // NEW
    FileList files_to_upload = 3;
    PushSummary summary = 4;
  }
}

// NEW: Data plane negotiation message
message DataTransferNegotiation {
  uint32 data_port = 1;         // TCP port for data connection
  bytes one_time_token = 2;     // Security token for data connection
  repeated string files_to_upload = 3;  // Files client should send
}

message PushHeader {
  string module = 1;
  bool mirror_mode = 2;
}

message FileHeader {
  string relative_path = 1;
  uint64 size = 2;
  int64 mtime_seconds = 3;
  uint32 permissions = 4;
}

message ManifestComplete {}
message Ack {}
message FileList { repeated string relative_paths = 1; }
message PushSummary {
    uint64 files_transferred = 1;
    uint64 bytes_transferred = 2;
}

// Pull, List, Purge remain unchanged
// ... (same as before)
```

**Rebuild**:
```bash
cargo build -p blit-core  # Regenerates Rust code from proto
```

### Task 3.1.2B: Verify Existing Proto (Option A)
**Priority**: 🔴 Critical (if Option A chosen)
**Effort**: 15 minutes
**Skip**: If Option B chosen

**Action**: Verify current `proto/blit.proto` is complete for gRPC-only approach

**Checklist**:
- [ ] `FileData` present in `ClientPushRequest`
- [ ] All message types defined
- [ ] Proto compiles successfully

### Task 3.1.3: Implement Daemon Service Skeleton
**Priority**: 🔴 Critical
**Effort**: 2-3 hours
**Dependencies**: Proto finalized

**Action**: Flesh out `crates/blit-daemon/src/main.rs` with service implementations

```rust
// crates/blit-daemon/src/main.rs

use blit_core::generated::blit_server::{Blit, BlitServer};
use blit_core::generated::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status};

#[derive(Debug)]
pub struct BlitService {
    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    // For hybrid transport: track active data transfers
    active_transfers: Arc<Mutex<HashMap<String, TransferSession>>>,
}

#[derive(Debug, Clone)]
struct ModuleConfig {
    name: String,
    path: PathBuf,
    read_only: bool,
}

#[derive(Debug)]
struct TransferSession {
    token: Vec<u8>,
    data_port: u16,
    files_to_receive: Vec<String>,
}

impl BlitService {
    pub fn new() -> Self {
        // TODO: Load modules from config file
        let modules = HashMap::new();

        Self {
            modules: Arc::new(Mutex::new(modules)),
            active_transfers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[tonic::async_trait]
impl Blit for BlitService {
    type PushStream = /* ... */;

    async fn push(
        &self,
        request: Request<tonic::Streaming<ClientPushRequest>>,
    ) -> Result<Response<Self::PushStream>, Status> {
        todo!("Implement in Task 3.2.1")
    }

    type PullStream = /* ... */;

    async fn pull(
        &self,
        request: Request<PullRequest>,
    ) -> Result<Response<Self::PullStream>, Status> {
        todo!("Implement in Task 3.3.1")
    }

    async fn list(
        &self,
        request: Request<ListRequest>,
    ) -> Result<Response<ListResponse>, Status> {
        todo!("Implement in Task 3.3.2")
    }

    async fn purge(
        &self,
        request: Request<PurgeRequest>,
    ) -> Result<Response<PurgeResponse>, Status> {
        todo!("Implement in Task 3.3.3")
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:50051".parse()?;
    let service = BlitService::new();

    println!("Blit daemon listening on {}", addr);

    Server::builder()
        .add_service(BlitServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
```

**Verification**:
```bash
# Start daemon
cargo run -p blit-daemon

# Should print: "Blit daemon listening on 127.0.0.1:50051"
```

## Days 2-4: Push Operation Implementation (12-16 hours)

### Task 3.2.1: Implement Control Plane Negotiation (Hybrid)
**Priority**: 🔴 Critical (Option B)
**Effort**: 4-5 hours
**Skip sections**: If Option A, implement direct file streaming

**Status 2025-10-19**: ✅ Handshake + data plane online. `blit-daemon` accepts `Push` streams, validates module headers, emits need lists (path traversal guarded + mtime/size comparison), and either (a) spins up a token-authenticated TCP listener for the hybrid data plane or (b) falls back to the control plane when TCP binding fails. `blit-cli` streams manifests via `RemotePushClient`, connects to the data port when advertised, and otherwise resends file data over gRPC before consuming the summary.

**Server-side implementation** (`blit-daemon`):

```rust
#[tonic::async_trait]
impl Blit for BlitService {
    type PushStream = /* Stream type */;

    async fn push(
        &self,
        request: Request<tonic::Streaming<ClientPushRequest>>,
    ) -> Result<Response<Self::PushStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            let mut received_files = Vec::new();
            let mut module_name = String::new();
            let mut mirror_mode = false;

            // Phase 1: Receive manifest
            while let Some(req) = stream.message().await? {
                match req.payload {
                    Some(Payload::Header(header)) => {
                        module_name = header.module;
                        mirror_mode = header.mirror_mode;
                        tx.send(Ok(ServerPushResponse {
                            payload: Some(server_push_response::Payload::Ack(Ack {})),
                        })).await?;
                    }
                    Some(Payload::FileManifest(file_header)) => {
                        received_files.push(file_header);
                        // Could send acks or buffer
                    }
                    Some(Payload::ManifestComplete(_)) => {
                        break;  // Manifest phase done
                    }
                    _ => {}
                }
            }

            // Phase 2: Compute what files are needed
            let need_list = compute_need_list(&module_name, &received_files).await?;

            // Phase 3: Set up data plane (HYBRID SPECIFIC)
            let data_port = allocate_data_port().await?;
            let token = generate_token();

            // Store transfer session
            let session = TransferSession {
                token: token.clone(),
                data_port,
                files_to_receive: need_list.clone(),
            };
            active_transfers.lock().await.insert(token_hex(&token), session);

            // Send negotiation response
            tx.send(Ok(ServerPushResponse {
                payload: Some(server_push_response::Payload::TransferNegotiation(
                    DataTransferNegotiation {
                        data_port: data_port as u32,
                        one_time_token: token.clone(),
                        files_to_upload: need_list,
                    }
                )),
            })).await?;

            // Phase 4: Accept data plane connection (separate task)
            tokio::spawn(accept_data_connection(data_port, token));

            // Phase 5: Wait for completion and send summary
            // (would be signaled by data plane handler)
            // ...

            Ok::<_, Status>(())
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

// Helper functions
async fn compute_need_list(
    module: &str,
    manifest: &[FileHeader],
) -> Result<Vec<String>, Status> {
    // Use mirror_planner logic to determine which files to request
    // Similar to local mirror but comparing network manifest to local state
    todo!()
}

async fn allocate_data_port() -> Result<u16, Status> {
    // Bind ephemeral TCP port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await
        .map_err(|e| Status::internal(format!("Port allocation failed: {}", e)))?;
    let port = listener.local_addr()?.port();
    // Store listener for later acceptance
    Ok(port)
}

fn generate_token() -> Vec<u8> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32).map(|_| rng.gen()).collect()
}
```

**Client-side implementation** (`blit-cli`):

```rust
// In blit-cli - remote push command
async fn execute_remote_push(
    source: PathBuf,
    remote_url: String,  // e.g., "blit://hostname:port/module"
) -> Result<()> {
    // Parse URL
    let (host, port, module) = parse_blit_url(&remote_url)?;

    // Connect to control plane
    let mut client = BlitClient::connect(format!("http://{}:{}", host, port)).await?;

    // Phase 1: Send manifest
    let orchestrator = TransferOrchestrator::new();
    let manifest = orchestrator.enumerate_for_manifest(&source)?;

    let (tx, rx) = mpsc::channel(32);
    let mut response_stream = client.push(ReceiverStream::new(rx)).await?.into_inner();

    // Send header
    tx.send(ClientPushRequest {
        payload: Some(Payload::Header(PushHeader {
            module,
            mirror_mode: false,
        })),
    }).await?;

    // Send file manifests
    for file in manifest {
        tx.send(ClientPushRequest {
            payload: Some(Payload::FileManifest(FileHeader {
                relative_path: file.relative_path,
                size: file.size,
                mtime_seconds: file.mtime_seconds,
                permissions: file.permissions,
            })),
        }).await?;
    }

    // Signal manifest complete
    tx.send(ClientPushRequest {
        payload: Some(Payload::ManifestComplete(ManifestComplete {})),
    }).await?;

    // Phase 2: Wait for data transfer negotiation
    while let Some(response) = response_stream.message().await? {
        match response.payload {
            Some(server_push_response::Payload::TransferNegotiation(neg)) => {
                // Phase 3: Connect to data plane and transfer files
                transfer_files_via_data_plane(
                    &source,
                    &host,
                    neg.data_port,
                    &neg.one_time_token,
                    &neg.files_to_upload,
                ).await?;
                break;
            }
            _ => {}
        }
    }

    // Phase 4: Wait for summary
    while let Some(response) = response_stream.message().await? {
        match response.payload {
            Some(server_push_response::Payload::Summary(summary)) => {
                println!("Transfer complete: {} files, {} bytes",
                    summary.files_transferred,
                    summary.bytes_transferred);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
```

### Task 3.2.2: Implement Data Plane Transfer (Hybrid)
**Priority**: 🔴 Critical (Option B)
**Effort**: 6-8 hours
**Skip**: If Option A, use gRPC streaming

**Status 2025-10-19**: ✅ Data-plane scaffold implemented. Daemon allocates an ephemeral TCP port, issues a 32-byte token, accepts the connection, verifies the token, and streams requested files to disk with traversal safeguards. CLI connects automatically post-negotiation and streams file contents; fallback path still pending for environments where TCP is blocked.

**Server-side data receiver**:

```rust
async fn accept_data_connection(port: u16, expected_token: Vec<u8>) -> Result<()> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

    // Accept single connection
    let (mut socket, _addr) = listener.accept().await?;

    // Phase 1: Verify token
    let mut token_buf = vec![0u8; 32];
    socket.read_exact(&mut token_buf).await?;

    if token_buf != expected_token {
        return Err(eyre!("Invalid token"));
    }

    // Phase 2: Receive files
    loop {
        // Read file header (path length + path + size)
        let path_len = socket.read_u32().await? as usize;
        if path_len == 0 {
            break;  // End of transfer
        }

        let mut path_buf = vec![0u8; path_len];
        socket.read_exact(&mut path_buf).await?;
        let relative_path = String::from_utf8(path_buf)?;

        let file_size = socket.read_u64().await?;

        // Receive file data
        let dest_path = get_module_path().join(&relative_path);
        std::fs::create_dir_all(dest_path.parent().unwrap())?;

        // Use zero-copy receive if available
        let mut file = tokio::fs::File::create(&dest_path).await?;

        #[cfg(target_os = "linux")]
        {
            use crate::zero_copy::splice_from_socket;
            splice_from_socket(&mut socket, &mut file, file_size).await?;
        }

        #[cfg(not(target_os = "linux"))]
        {
            tokio::io::copy(&mut socket.take(file_size), &mut file).await?;
        }
    }

    Ok(())
}
```

**Client-side data sender**:

```rust
async fn transfer_files_via_data_plane(
    source: &Path,
    host: &str,
    port: u32,
    token: &[u8],
    files_to_upload: &[String],
) -> Result<()> {
    // Connect to data port
    let mut socket = TcpStream::connect(format!("{}:{}", host, port)).await?;

    // Send token
    socket.write_all(token).await?;

    // Send files
    for relative_path in files_to_upload {
        let src_path = source.join(relative_path);
        let metadata = tokio::fs::metadata(&src_path).await?;

        // Send file header
        socket.write_u32(relative_path.len() as u32).await?;
        socket.write_all(relative_path.as_bytes()).await?;
        socket.write_u64(metadata.len()).await?;

        // Send file data with zero-copy
        let mut file = tokio::fs::File::open(&src_path).await?;

        #[cfg(target_os = "linux")]
        {
            use crate::zero_copy::sendfile_to_socket;
            sendfile_to_socket(&mut file, &mut socket, metadata.len()).await?;
        }

        #[cfg(not(target_os = "linux"))]
        {
            tokio::io::copy(&mut file, &mut socket).await?;
        }
    }

    // Send end-of-transfer marker
    socket.write_u32(0).await?;

    Ok(())
}
```

### Task 3.2.3: Add Progress Reporting
**Priority**: 🟡 Important
**Effort**: 2-3 hours

**Implementation**: Add progress callback to orchestrator and wire to UI

```rust
pub struct TransferProgress {
    pub files_completed: u64,
    pub files_total: u64,
    pub bytes_completed: u64,
    pub bytes_total: u64,
}

pub trait ProgressReporter: Send + Sync {
    fn report(&self, progress: &TransferProgress);
}

// In orchestrator
pub fn execute_remote_push<P: ProgressReporter>(
    &self,
    source: &Path,
    remote: &str,
    progress: Option<Arc<P>>,
) -> Result<TransferStats> {
    // During transfer:
    if let Some(reporter) = &progress {
        reporter.report(&TransferProgress {
            files_completed,
            files_total,
            bytes_completed,
            bytes_total,
        });
    }
}
```

## Days 5-6: Pull and Other Operations (8-10 hours)

### Task 3.3.1: Implement Pull Operation
**Priority**: 🔴 Critical
**Effort**: 4-5 hours

**Status 2025-10-19**: ✅ Initial implementation in place: daemon streams individual files or directory trees over `PullChunk`, and the CLI writes them to a destination directory via `RemotePullClient`.

**Server implementation**:
```rust
async fn pull(
    &self,
    request: Request<PullRequest>,
) -> Result<Response<Self::PullStream>, Status> {
    let req = request.into_inner();
    let (tx, rx) = mpsc::channel(32);

    tokio::spawn(async move {
        let module_path = get_module_path(&req.module)?;
        let full_path = module_path.join(&req.path);

        if full_path.is_file() {
            // Single file
            stream_file(&full_path, tx).await?;
        } else if full_path.is_dir() {
            // Directory - stream all files
            let enumerator = FileEnumerator::new(&full_path);
            let files = enumerator.enumerate()?;

            for file_info in files {
                let file_path = full_path.join(&file_info.relative_path);
                stream_file(&file_path, tx).await?;
            }
        }

        Ok::<_, Status>(())
    });

    Ok(Response::new(ReceiverStream::new(rx)))
}

async fn stream_file(
    path: &Path,
    tx: mpsc::Sender<Result<PullChunk, Status>>,
) -> Result<()> {
    // Send file header
    let metadata = tokio::fs::metadata(path).await?;
    tx.send(Ok(PullChunk {
        payload: Some(pull_chunk::Payload::FileHeader(FileHeader {
            relative_path: path.to_string_lossy().to_string(),
            size: metadata.len(),
            // ...
        })),
    })).await?;

    // Stream file data in chunks
    let mut file = tokio::fs::File::open(path).await?;
    let mut buffer = vec![0u8; 64 * 1024];

    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 { break; }

        tx.send(Ok(PullChunk {
            payload: Some(pull_chunk::Payload::FileData(FileData {
                content: buffer[..n].to_vec(),
            })),
        })).await?;
    }

    Ok(())
}
```

### Task 3.3.2: Implement List Operation
**Priority**: 🟡 Important
**Effort**: 2 hours

```rust
async fn list(
    &self,
    request: Request<ListRequest>,
) -> Result<Response<ListResponse>, Status> {
    let req = request.into_inner();
    let module_path = get_module_path(&req.module)?;
    let full_path = module_path.join(&req.path);

    let mut entries = Vec::new();

    for entry in std::fs::read_dir(&full_path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        entries.push(FileInfo {
            name: entry.file_name().to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            mtime_seconds: /* extract from metadata */,
        });
    }

    Ok(Response::new(ListResponse { entries }))
}
```

### Task 3.3.3: Implement Purge Operation
**Priority**: 🟡 Important
**Effort**: 2 hours

```rust
async fn purge(
    &self,
    request: Request<PurgeRequest>,
) -> Result<Response<PurgeResponse>, Status> {
    let req = request.into_inner();
    let module_path = get_module_path(&req.module)?;

    let mut files_deleted = 0u64;

    for relative_path in &req.paths_to_delete {
        let full_path = module_path.join(relative_path);

        if full_path.is_file() {
            std::fs::remove_file(&full_path)?;
            files_deleted += 1;
        } else if full_path.is_dir() {
            let count = count_files_in_dir(&full_path)?;
            std::fs::remove_dir_all(&full_path)?;
            files_deleted += count;
        }
    }

    Ok(Response::new(PurgeResponse { files_deleted }))
}
```

## Day 6: Telemetry & Predictor Integration (5-7 hours)

### Task 3.4.1: Remote Performance History Store
**Priority**: 🔴 Critical  
**Effort**: 2 hours

- Implement JSONL performance history writer for remote runs (parallel to local Phase 2 logic).
- Keep data local-only, capped (~1 MiB); no runtime prompts.
- Record handshake latency, manifest size, transfer duration, transport selection, retry counts.
- Honour `BLIT_DISABLE_PERF_HISTORY` (shared with local) and ensure failures never abort transfers.

### Task 3.4.2: Daemon ⇄ CLI Performance History Exchange
**Priority**: 🟡 Important  
**Effort**: 2 hours

- CLI uploads its run summary to the shared history handler after completion.
- CLI fetches daemon performance snapshot at connection time; no interactive prompts.
- Daemon maintains its own JSONL (same schema) and merges into predictor inputs.

### Task 3.4.3: Idle Interval Profiling (Daemon)
**Priority**: 🟡 Important  
**Effort**: 1-2 hours

- Daemon performs lightweight self-check every 24 h when idle (handshake + small disk probe).
- Writes results to the performance history store; respects disable flag; zero prompts.
- Skip if constrained (high load, disk full) to avoid user disruption.

### Task 3.4.4: Remote Predictor Hook
**Priority**: 🔴 Critical  
**Effort**: 1-2 hours

- Extend predictor to consume remote performance history (both CLI + daemon) and select transport/stream counts.
- First-run defaults remain conservative; performance history refines choices on subsequent runs.
- Document how history age affects routing (stale > N days triggers warning, not prompt).

**Note:** Shipping default (automatic vs. opt-in performance history) depends on benchmark data; once the behavior is chosen it remains consistent across releases.

## Day 7: Integration Testing (6-8 hours)

### Task 3.5.1: Integration Test Suite
**Priority**: 🔴 Critical
**Effort**: 4-5 hours

**Create** `tests/remote_operations_test.rs`:

```rust
use std::process::Command;
use tempfile::TempDir;

#[tokio::test]
async fn test_push_pull_roundtrip() {
    // Start daemon
    let daemon = start_test_daemon().await;

    // Create test data
    let src = TempDir::new().unwrap();
    std::fs::write(src.path().join("test.txt"), b"content").unwrap();

    // Push to daemon
    let push_result = Command::new("cargo")
        .args(&["run", "-p", "blit-cli", "--", "push"])
        .arg(src.path())
        .arg("blit://127.0.0.1:50051/test_module")
        .output()
        .expect("Push failed");

    assert!(push_result.status.success());

    // Pull from daemon
    let dst = TempDir::new().unwrap();
    let pull_result = Command::new("cargo")
        .args(&["run", "-p", "blit-cli", "--", "pull"])
        .arg("blit://127.0.0.1:50051/test_module/test.txt")
        .arg(dst.path())
        .output()
        .expect("Pull failed");

    assert!(pull_result.status.success());

    // Verify content
    let pulled_content = std::fs::read(dst.path().join("test.txt")).unwrap();
    assert_eq!(pulled_content, b"content");

    daemon.kill().unwrap();
}
```

### Task 3.5.2: Manual End-to-End Testing
**Priority**: 🟡 Important
**Effort**: 2-3 hours

**Test scenarios**:
1. Push small file to daemon
2. Push large file (1GB+) to daemon
3. Push directory tree to daemon
4. Pull file from daemon
5. Pull directory from daemon
6. List remote directory
7. Purge files from daemon
8. Error cases (network disconnect, permission errors, etc.)

## Day 8: Polish & Quality Gate (4-6 hours)

### Task 3.5.1: Error Handling Improvements
**Priority**: 🟡 Important
**Effort**: 2-3 hours

- Graceful network error handling
- Retry logic for transient failures
- Clear error messages for user
- Proper cleanup on failure

### Task 3.5.2: Performance Validation
**Priority**: 🔴 Critical
**Effort**: 2-3 hours

**Remote performance benchmark**:
```bash
# Similar to Phase 2.5 but over network
./benchmarks/bench_remote.sh
```

**Verify**:
- Remote transfers approach local transfer speeds (accounting for network)
- Zero-copy working on data plane (if hybrid)
- No excessive memory usage
- CPU utilization reasonable

## Quality Gate Checklist

Before proceeding to Phase 4:

- [ ] All remote operations (push, pull, list, purge) functional
- [ ] Integration tests pass
- [ ] Manual test scenarios pass
- [ ] Error handling robust
- [ ] Progress reporting works
- [ ] Performance acceptable (network-adjusted)
- [ ] Memory usage reasonable
- [ ] No resource leaks
- [ ] DEVLOG.md updated

## Parallelization Opportunities

- Task 3.3.1 (Pull) || Task 3.3.2 (List) || Task 3.3.3 (Purge) - All independent
- Integration test development || Manual testing
- Documentation || Error handling improvements

## Risks & Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Hybrid transport complexity | Medium | High | Prototype early, fallback to gRPC-only |
| Network error scenarios | High | Medium | Comprehensive error handling, retry logic |
| Performance not meeting goals | Low | High | Early benchmarking, profiling tools ready |
| Security of data plane | Medium | High | Token-based auth, consider TLS for data plane |

## Definition of Done

Phase 3 is complete when:

1. ✅ All remote operations implemented and tested
2. ✅ Hybrid transport working (if chosen) with zero-copy
3. ✅ Integration tests passing
4. ✅ Manual testing scenarios complete
5. ✅ Error handling robust
6. ✅ Performance validated
7. ✅ Code documented
8. ✅ DEVLOG.md updated

## Next Phase

Upon completion, proceed to:
- **[WORKFLOW_PHASE_4.md](./WORKFLOW_PHASE_4.md)** - Production Hardening & Packaging
