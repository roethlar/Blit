# Blit Performance Roadmap: Path to 25GbE Saturation

## Executive Summary

**Current State**: ~1.2 Gbps observed throughput (based on memory notes)
**Target**: 25 Gbps (20x improvement needed)
**Gap Analysis**: Architectural bottlenecks in I/O pipeline, buffering, and parallelization

This document outlines critical improvements derived from rclone architecture analysis and enterprise file transfer best practices.

---

## Part 1: Critical Performance Bottlenecks

### 1.1 Single-Stream Data Plane (CRITICAL)

**Current Implementation** (`data_plane.rs:99-125`):
```rust
// Sequential payload sending - one at a time
while let Some(prepared) = stream.next().await {
    match prepared? {
        PreparedPayload::File(header) => {
            self.send_file(source.clone(), &header).await?;
        }
        // ...
    }
}
```

**Problem**: Even with multiple TCP streams (16 max), the client sends payloads sequentially within each stream. No pipelining or async prefetching.

**rclone Solution** (`multipart.go:73-116`):
- Parallel chunk upload with `errgroup`
- Token-based concurrency control
- Read-ahead buffering independent of write

### 1.2 Small Buffer Size

**Current** (`data_plane.rs:46`):
```rust
let buffer_len = chunk_bytes.max(64 * 1024);  // 64KB default
```

**Problem**: 64KB buffers are too small for 25GbE. At 25 Gbps, a 64KB buffer fills in 20 microseconds - not enough to hide network latency.

**rclone Solution** (`pool/pool.go:18-23`):
```go
BufferSize = 1024 * 1024  // 1MB per buffer
BufferCacheSize = 64       // Pool of 64 buffers
```

### 1.3 No Memory Pool

**Current**: Allocates new `Vec<u8>` per transfer
**Problem**: Allocation overhead, GC pressure, memory fragmentation

**rclone Solution** (`pool/pool.go`):
- Reusable buffer pool with mmap support
- Semaphore-controlled total memory budget
- Automatic pool flushing for unused buffers

### 1.4 Synchronous Read-Then-Write Pattern

**Current** (`data_plane.rs:178-195`):
```rust
while remaining > 0 {
    let chunk = file.read(&mut self.buffer).await?;  // Wait for read
    self.stream.write_all(&self.buffer[..chunk]).await?;  // Then write
}
```

**Problem**: Network sits idle during disk reads; disk sits idle during network writes.

**rclone Solution** (`asyncreader/asyncreader.go`):
- Async read-ahead into multiple buffers
- Producer-consumer pattern with channels
- Soft-start (4KB → 1MB) to avoid over-reading small files

### 1.5 No Parallel File Processing

**Current**: Files processed sequentially per stream
**rclone Solution** (`sync/sync.go:70-72`):
```go
toBeChecked  *pipe  // Checker queue
toBeUploaded *pipe  // Upload queue (parallel)
```
- Separate checker and transfer pipelines
- Configurable parallelism: `--checkers` and `--transfers`

---

## Part 2: Recommended Improvements

### Priority 1: Async I/O Pipeline (HIGH IMPACT)

**Goal**: Overlap disk reads with network writes

**Implementation**:
```rust
// New: AsyncPayloadReader with prefetch buffer
pub struct AsyncPayloadReader {
    prefetch_queue: VecDeque<PreparedPayload>,
    prefetch_task: JoinHandle<()>,
    buffer_pool: Arc<BufferPool>,
    max_prefetch: usize,  // 8-16 payloads ahead
}

impl AsyncPayloadReader {
    pub async fn next(&mut self) -> Option<PreparedPayload> {
        // Returns immediately if prefetched
        // Prefetch task runs independently
    }
}
```

**Expected Impact**: 2-3x throughput improvement

### Priority 2: Buffer Pool with mmap

**Goal**: Eliminate allocation overhead, control memory usage

**Implementation**:
```rust
pub struct BufferPool {
    buffers: Mutex<Vec<Box<[u8]>>>,
    buffer_size: usize,          // 1MB
    max_buffers: usize,          // 64
    total_memory: Semaphore,     // Global memory budget
    use_mmap: bool,              // Use mmap for large buffers
}

impl BufferPool {
    pub async fn acquire(&self) -> PoolBuffer {
        self.total_memory.acquire().await;
        // Return from cache or allocate
    }

    pub fn release(&self, buf: PoolBuffer) {
        // Return to pool for reuse
    }
}
```

**Expected Impact**: 20-30% latency reduction, stable memory usage

### Priority 3: Multi-Stream Parallel Uploads

**Goal**: Fully utilize all TCP streams concurrently

**Current**: 16 streams but sequential payload dispatch
**Proposed**: True parallel dispatch with work-stealing

```rust
pub struct ParallelDataPlane {
    streams: Vec<DataPlaneSession>,
    work_queue: Arc<ArrayQueue<PreparedPayload>>,
    workers: Vec<JoinHandle<Result<TransferStats>>>,
}

impl ParallelDataPlane {
    pub async fn send_all(&mut self, payloads: Vec<PreparedPayload>) -> Result<Stats> {
        // Distribute payloads across streams
        // Each stream worker pulls from shared queue
        // True parallel I/O across all connections
    }
}
```

**Expected Impact**: Near-linear scaling with stream count

### Priority 4: Chunked Large File Transfers

**Goal**: Parallelize single large file transfers

**rclone Pattern** (`multithread.go:67-119`):
- Split large files into chunks (64MB default)
- Upload chunks in parallel across streams
- Reassemble on server

**Implementation**:
```rust
const CHUNK_SIZE: u64 = 64 * 1024 * 1024;  // 64MB
const MIN_SIZE_FOR_CHUNKING: u64 = 256 * 1024 * 1024;  // 256MB

pub async fn send_large_file_chunked(
    &mut self,
    file: &FileHeader,
    chunk_size: u64,
) -> Result<()> {
    let num_chunks = (file.size + chunk_size - 1) / chunk_size;

    // Send chunk headers
    // Parallel chunk upload with position tracking
    // Server reassembles
}
```

**Expected Impact**: Large file transfers scale with parallelism

### Priority 5: TCP Tuning

**Goal**: Optimize TCP stack for high-bandwidth

**Current** (`data_plane.rs:76-81`):
```rust
socket.set_tcp_nodelay(true)?;
if let Some(size) = tcp_buffer_size {
    socket.set_send_buffer_size(size);
    socket.set_recv_buffer_size(size);
}
```

**Recommended Additions**:
```rust
// For 25GbE, need large buffers
const TCP_BUFFER_SIZE: usize = 16 * 1024 * 1024;  // 16MB

// Enable TCP_CORK for batching small writes (Linux)
#[cfg(target_os = "linux")]
socket.set_tcp_cork(true)?;

// Consider SO_BUSY_POLL for low-latency
#[cfg(target_os = "linux")]
socket.set_option(libc::SOL_SOCKET, libc::SO_BUSY_POLL, 50)?;  // 50µs
```

**Expected Impact**: Better utilization of bandwidth-delay product

---

## Part 3: Enterprise Features from rclone

### 3.1 Resumable Transfers

**rclone Feature**: `--partial-suffix` for incomplete uploads

**Implementation**:
```rust
pub struct ResumableTransfer {
    state_file: PathBuf,  // .blit-resume
    completed_files: HashSet<String>,
    partial_file: Option<(String, u64)>,  // (path, bytes_sent)
}
```

### 3.2 Bandwidth Limiting

**rclone Feature**: `--bwlimit 10M` for rate limiting

**Implementation**:
```rust
pub struct BandwidthLimiter {
    bucket: TokenBucket,
    bytes_per_second: u64,
}

impl AsyncWrite for ThrottledWriter {
    fn poll_write(...) -> Poll<Result<usize>> {
        // Acquire tokens before writing
    }
}
```

### 3.3 Transfer Retries with Backoff

**rclone Feature**: `--retries 3 --retries-sleep 10s`

**Implementation**:
```rust
pub async fn transfer_with_retry<F, T>(
    operation: F,
    max_retries: u32,
    backoff: ExponentialBackoff,
) -> Result<T>
where
    F: Fn() -> Future<Output = Result<T>>,
{
    for attempt in 0..max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if is_retriable(&e) => {
                tokio::time::sleep(backoff.next()).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### 3.4 Server-Side Copy

**rclone Feature**: `--server-side-copy` for zero-network transfers

**Implementation** (daemon-to-daemon):
```protobuf
rpc ServerSideCopy(ServerSideCopyRequest) returns (ServerSideCopyResponse);

message ServerSideCopyRequest {
  string source_module = 1;
  string source_path = 2;
  string dest_module = 3;
  string dest_path = 4;
}
```

### 3.5 Checksum Verification

**rclone Feature**: `--checksum` with multiple hash algorithms

**Current**: Blake3, XXHash, MD5 available
**Enhancement**: Stream checksums during transfer (no second pass)

```rust
pub struct ChecksummedWriter<W: AsyncWrite> {
    inner: W,
    hasher: blake3::Hasher,
}

impl AsyncWrite for ChecksummedWriter<W> {
    fn poll_write(...) -> Poll<Result<usize>> {
        self.hasher.update(buf);
        self.inner.poll_write(...)
    }
}
```

### 3.6 Progress Callbacks / Webhooks

**rclone Feature**: `--progress` with detailed stats, `--rc` for remote control

**Implementation**:
```rust
pub trait TransferProgress: Send + Sync {
    fn on_file_start(&self, path: &str, size: u64);
    fn on_bytes_transferred(&self, bytes: u64);
    fn on_file_complete(&self, path: &str);
    fn on_error(&self, path: &str, error: &str);
}

// Webhook implementation
pub struct WebhookProgress {
    endpoint: Url,
    client: reqwest::Client,
    batch_interval: Duration,
}
```

---

## Part 4: Implementation Roadmap

### Phase 1: Core Performance (Weeks 1-3)

| Task | Impact | Effort | Priority |
|------|--------|--------|----------|
| Buffer pool implementation | High | Medium | P0 |
| Async read-ahead pipeline | Very High | Medium | P0 |
| Increase default buffer sizes | Medium | Low | P0 |
| TCP tuning (16MB buffers, CORK) | Medium | Low | P0 |

**Expected Result**: 4-5 Gbps

### Phase 2: Parallel Scaling (Weeks 4-6)

| Task | Impact | Effort | Priority |
|------|--------|--------|----------|
| True parallel stream dispatch | Very High | High | P1 |
| Large file chunking | High | Medium | P1 |
| Work-stealing queue | Medium | Medium | P1 |

**Expected Result**: 10-15 Gbps

### Phase 3: Enterprise Features (Weeks 7-10)

| Task | Impact | Effort | Priority |
|------|--------|--------|----------|
| Resumable transfers | Medium | Medium | P2 |
| Bandwidth limiting | Low | Low | P2 |
| Transfer retries | Medium | Low | P2 |
| Streaming checksums | Medium | Medium | P2 |
| Progress webhooks | Low | Low | P2 |

**Expected Result**: 15-20 Gbps + enterprise readiness

### Phase 4: Advanced Optimizations (Weeks 11+)

| Task | Impact | Effort | Priority |
|------|--------|--------|----------|
| io_uring integration (Linux) | High | High | P3 |
| Zero-copy sendfile | Medium | Medium | P3 |
| RDMA support | Very High | Very High | P3 |
| Compression (zstd streaming) | Variable | Medium | P3 |

**Expected Result**: 20-25 Gbps

---

## Part 5: Benchmarking Strategy

### Test Scenarios

```bash
# Small files (1KB-1MB) - tests metadata overhead
blit-bench small-files --count 100000 --size-range 1K-1M

# Large files (1GB-10GB) - tests raw throughput
blit-bench large-files --count 10 --size-range 1G-10G

# Mixed workload (realistic)
blit-bench mixed --small-ratio 0.8 --large-ratio 0.2

# Network saturation test
blit-bench sustained --duration 60s --target-gbps 25
```

### Metrics to Track

```rust
pub struct BenchmarkMetrics {
    pub throughput_gbps: f64,
    pub files_per_second: f64,
    pub latency_p50_ms: f64,
    pub latency_p99_ms: f64,
    pub cpu_utilization: f64,
    pub memory_peak_mb: u64,
    pub tcp_retransmits: u64,
}
```

### Profiling Tools

- **CPU**: `perf record` / `flamegraph`
- **Memory**: `heaptrack` / `valgrind --tool=massif`
- **I/O**: `iostat`, `blktrace`
- **Network**: `iperf3`, `tcpdump`, `ss -i`

---

## Part 6: Code Changes Summary

### Files to Modify

1. **`crates/blit-core/src/buffer_pool.rs`** (NEW)
   - Implement memory pool with mmap support

2. **`crates/blit-core/src/remote/transfer/data_plane.rs`**
   - Async read-ahead pipeline
   - Parallel payload dispatch
   - Larger default buffers

3. **`crates/blit-core/src/remote/transfer/chunked.rs`** (NEW)
   - Large file chunking logic

4. **`crates/blit-daemon/src/service/push/data_plane.rs`**
   - Parallel chunk reception
   - Streaming checksum verification

5. **`crates/blit-core/src/remote/push/client/mod.rs`**
   - Integration of new pipeline
   - Retry logic

6. **`proto/blit.proto`**
   - Add chunked transfer messages
   - Add resumable transfer state

### New Dependencies

```toml
[dependencies]
# Memory mapping
memmap2 = "0.9"

# Better async primitives
crossbeam = "0.8"
async-channel = "2.0"

# Rate limiting
governor = "0.6"

# Streaming compression (optional)
zstd = "0.13"

# io_uring (Linux, optional)
tokio-uring = { version = "0.4", optional = true }
```

---

## Appendix: rclone Configuration Reference

Key rclone flags for high-performance transfers:

```bash
rclone copy source: dest: \
  --transfers 32 \           # Parallel file transfers
  --checkers 16 \            # Parallel file checkers
  --buffer-size 64M \        # Per-file buffer
  --multi-thread-streams 8 \ # Streams per large file
  --multi-thread-cutoff 256M \ # Threshold for multi-stream
  --multi-thread-chunk-size 64M \ # Chunk size
  --no-traverse \            # Skip dest enumeration
  --fast-list \              # Use fewer API calls
  --use-mmap                 # Memory-mapped I/O
```

These translate to blit as:
- `--workers 32` (file parallelism)
- `--streams 8` (TCP parallelism)
- `--buffer-size 64M`
- `--chunk-size 64M`
- `--chunk-threshold 256M`
