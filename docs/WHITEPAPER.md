# Blit v2 — Technical Whitepaper

**Audience:** an LLM agent (or human reviewer) being asked to assess code
quality, architectural soundness, and likely bug surfaces.

**Repo:** `~/dev/Blit` (Rust workspace, ~26 KLOC excluding generated proto).
Three crates: `blit-core` (library), `blit-cli` (user binary), `blit-daemon`
(server binary). Workspace `Cargo.toml` lists them; nothing else.

**Stated philosophy:** "fastest, most reliable, most stable file transfer
in any scenario." Adaptive tuning over hardcoded constants. Identical data
path for every src↔dst combination.

---

## 1. What blit does

Three transfer verbs (`copy`, `mirror`, `move`) over four src/dst
combinations:

| Combination | Implementation |
|---|---|
| local → local | `crates/blit-core/src/orchestrator/orchestrator.rs` (zero-copy cascade: `copy_file_range`, `clonefile`, ReFS block clone, `sendfile`, fallback to read/write) |
| local → remote | `RemotePushClient` in `crates/blit-core/src/remote/push/client/mod.rs` |
| remote → local | `RemotePullClient` in `crates/blit-core/src/remote/pull.rs` (`pull` for raw, `pull_sync` for compare-and-mirror) |
| remote → remote | `RemoteTransferSource` (pull-from-A) wrapped into a push-to-B (relay) |

Wire transport: dual-channel hybrid.

- **Control plane:** gRPC bidirectional streams (`tonic 0.14`). Carries
  manifests, need-lists, summaries. Resumable / restartable.
- **Data plane:** custom binary protocol over plain TCP (parallel
  streams, optional `--force-grpc` fallback that pushes file bytes via
  the gRPC channel). The data plane is the throughput path.

Proto definitions: `proto/blit.proto`. The data plane's binary format is
documented inline in `crates/blit-core/src/remote/transfer/data_plane.rs`.

---

## 2. The unified pipeline (the load-bearing abstraction)

Two traits + two executors. Same shape for every transfer.

```rust
// crates/blit-core/src/remote/transfer/source.rs
#[async_trait]
pub trait TransferSource: Send + Sync {
    fn scan(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (mpsc::Receiver<FileHeader>, JoinHandle<Result<u64>>);

    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload>;
    async fn check_availability(&self, headers: Vec<FileHeader>, unreadable: Arc<Mutex<Vec<String>>>) -> Result<Vec<FileHeader>>;
    async fn open_file(&self, header: &FileHeader) -> Result<Box<dyn AsyncRead + Unpin + Send>>;
    fn root(&self) -> &Path;
}

// crates/blit-core/src/remote/transfer/sink.rs
#[async_trait]
pub trait TransferSink: Send + Sync {
    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome>;
    async fn write_file_stream(
        &self,
        header: &FileHeader,
        _reader: &mut (dyn AsyncRead + Unpin + Send),
    ) -> Result<SinkOutcome> { /* default: bail */ }
    async fn finish(&self) -> Result<()> { Ok(()) }
    fn root(&self) -> &Path;
}
```

Concrete implementations:

- **Sources:** `FsTransferSource` (local filesystem walk),
  `RemoteTransferSource` (pull from a remote daemon — used in the relay
  case).
- **Sinks:** `FsTransferSink` (local writes; uses zero-copy cascade for
  whole-file payloads, manual stream for receive payloads),
  `DataPlaneSink` (writes to a TCP stream wrapped in `DataPlaneSession`),
  `NullSink` (drains + counts; for benchmarking),
  `GrpcFallbackSink` (control-plane fallback; outbound only).

### 2.1 The outbound executor

`pipeline.rs::execute_sink_pipeline_streaming(source, sinks, payload_rx, prefetch, progress)`
is the workhorse. It:

1. Per-sink `mpsc::channel<TransferPayload>` of capacity `prefetch`.
2. Spawns one async task per sink: receive payload → `source.prepare_payload` →
   `sink.write_payload` → progress callback → loop. Terminates on channel
   close, then calls `sink.finish()`.
3. Spawns a **dispatcher** task that pulls from `payload_rx` and
   round-robins to per-sink channels.
4. Joins all sink tasks, propagates the first error, returns merged
   `SinkOutcome { files_written, bytes_written }`.

```rust
// crates/blit-core/src/remote/transfer/pipeline.rs (~line 70)
pub async fn execute_sink_pipeline_streaming(
    source: Arc<dyn TransferSource>,
    sinks: Vec<Arc<dyn TransferSink>>,
    mut payload_rx: mpsc::Receiver<TransferPayload>,
    prefetch: usize,
    progress: Option<&RemoteTransferProgress>,
) -> Result<SinkOutcome> {
    // ... per-sink workers spawn ...
    let dispatcher = tokio::spawn(async move {
        let mut next = 0usize;
        while let Some(payload) = payload_rx.recv().await {
            let idx = next % sink_count;
            next = next.wrapping_add(1);
            if sink_senders[idx].send(payload).await.is_err() { return; }
        }
        drop(sink_senders);  // signal EOF to workers
    });
    // ... await joins ...
}
```

Round-robin is deliberately simple. Adaptive load-balancing is left to
the lower layers (multi-stream payload sharding, BDP-tuned buffers).

### 2.2 The inbound executor — symmetric counterpart

The receive side was unified in commits `1baa981` / `a232dbd` / `b64bfd8`.
Where the outbound executor fans one source into N parallel sinks, the
inbound executor drives one sink from one wire (parallelism comes from
N concurrent invocations, one per inbound TCP stream).

```rust
// crates/blit-core/src/remote/transfer/pipeline.rs (~line 175)
pub async fn execute_receive_pipeline(
    socket: &mut TcpStream,
    sink: Arc<dyn TransferSink>,
    progress: Option<&RemoteTransferProgress>,
) -> Result<SinkOutcome> {
    let mut total = SinkOutcome::default();
    loop {
        let mut tag = [0u8; 1];
        socket.read_exact(&mut tag).await.context("reading data-plane record tag")?;
        match tag[0] {
            DATA_PLANE_RECORD_END => break,
            DATA_PLANE_RECORD_FILE => {
                let mut header = read_file_header(socket).await?;
                let file_size = read_u64(socket).await?;
                let mtime = read_i64(socket).await?;
                let perms = read_u32(socket).await?;
                header.size = file_size;
                header.mtime_seconds = mtime;
                header.permissions = perms;
                use tokio::io::AsyncReadExt;
                let mut reader = (&mut *socket).take(file_size);
                let outcome = sink
                    .write_file_stream(&header, &mut reader)
                    .await
                    .with_context(|| format!("receiving {}", header.relative_path))?;
                if let Some(p) = progress {
                    p.report_payload(0, outcome.bytes_written);
                    p.report_file_complete(header.relative_path.clone(), outcome.bytes_written);
                }
                total.merge(&outcome);
            }
            DATA_PLANE_RECORD_TAR_SHARD => { /* parse + write_payload(TarShard) */ }
            DATA_PLANE_RECORD_BLOCK | DATA_PLANE_RECORD_BLOCK_COMPLETE => { /* resume */ }
            other => bail!("unknown data-plane record tag: 0x{:02X}", other),
        }
    }
    sink.finish().await.context("finalising sink")?;
    Ok(total)
}
```

Both the daemon's push-receive task and the client's pull-receive task
call this — same code, ~30 LOC each on the call sites versus the ~300
LOC bespoke dispatch loops they replaced. Wire format extension
(commit `b64bfd8`) put `mtime` and `permissions` inline in `FILE`
records so the receive-side sink applies metadata without an
out-of-band manifest cache.

---

## 3. Data plane wire format

The TCP data plane is a tagged stream of records. All multi-byte ints
are big-endian. Authoritative encoders/decoders:

- Encoder: `crates/blit-core/src/remote/transfer/data_plane.rs::DataPlaneSession::send_*`
- Decoder: `crates/blit-core/src/remote/transfer/pipeline.rs::execute_receive_pipeline`

```text
record stream := token (32 B) (record)* END_TAG

FILE  := 0x00 path_len:u32 path:bytes size:u64 mtime:i64 perms:u32 bytes:size
TAR_SHARD := 0x01 count:u32 [path_len:u32 path:bytes size:u64 mtime:i64 perms:u32]xN
             tar_size:u64 tar_bytes:tar_size
BLOCK := 0x02 path_len:u32 path:bytes offset:u64 len:u32 bytes:len
BLOCK_COMPLETE := 0x03 path_len:u32 path:bytes total_size:u64
END   := 0xFF
```

Tar shards bundle small files for amortization (the planner targets
~8–64 MiB shard size). Block records implement resume — server
requests block hashes via gRPC, sends only differing blocks via the
data plane.

### 3.1 Symmetric byte copy

The send and receive byte-copy loops are intentionally similar
(double-buffered, 1 MiB chunks; mirror each other so push and pull
hit the same throughput):

```rust
// data_plane.rs (~line 213, send side)
async fn send_file_double_buffered(
    &mut self,
    file: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
    header: &FileHeader,
    rel: &str,
) -> Result<()> {
    let mut remaining = header.size;
    if remaining == 0 { return Ok(()); }
    let mut buf_a = self.pool.acquire().await;
    let mut buf_b = self.pool.acquire().await;
    let mut bytes_a = file.read(buf_a.as_mut_slice()).await?;
    if bytes_a == 0 { bail!("EOF early"); }
    remaining -= bytes_a as u64;
    while remaining > 0 {
        // Overlap: write buf_a to network while filling buf_b from disk.
        let (write_result, read_result) = tokio::join!(
            self.stream.write_all(&buf_a.as_slice()[..bytes_a]),
            file.read(buf_b.as_mut_slice())
        );
        write_result?;
        let bytes_b = read_result?;
        if bytes_b == 0 && remaining > 0 { bail!("EOF early"); }
        remaining -= bytes_b as u64;
        std::mem::swap(&mut buf_a, &mut buf_b);
        bytes_a = bytes_b;
    }
    if bytes_a > 0 { self.stream.write_all(&buf_a.as_slice()[..bytes_a]).await?; }
    Ok(())
}

// data_plane.rs (~line 470, receive side — symmetric)
pub async fn receive_stream_double_buffered<R, W>(
    src: &mut R,
    dst: &mut W,
    expected: u64,
    buffer_size: usize,
) -> Result<u64>
where R: AsyncRead + Unpin + ?Sized, W: AsyncWrite + Unpin + ?Sized {
    /* same shape, opposite direction */
}
```

`pool: Arc<BufferPool>` is a buffer pool with `Send`-able buffers; max
two outstanding per session. Without these (e.g. with `tokio::io::copy`'s
default 8 KiB buffer), measured throughput dropped from 9.3 → 1 Gbps
on push to ZFS. See commit `1baa981` for the diagnosis.

---

## 4. Outbound transfer planning

`crates/blit-core/src/transfer_plan.rs::build_plan` decides how to bundle
files. Rough categorization:

```rust
// transfer_plan.rs (~line 75)
for e in entries {
    if e.size < 64 * 1024            { small.push(rel); }    // tar shard candidate
    else if e.size < 1_048_576       { small.push(rel); }
    else if e.size < 256 * 1_048_576 { medium.push(rel, sz); } // raw bundle
    else                              { large_files.push(...); } // single TransferTask::Large
}

// then:
let use_tar = if options.force_tar { small_count >= 1 }
              else if small_count < 2 { false }
              else { small_count >= 32 || avg_small_size <= 128 * 1024 };

// shard target adapts to total: 4 / 32 / 64 MiB depending on workload size
// count target: 256 / 1024 / 2048 entries depending on file count
```

Tasks are interleaved across the three categories so streams stay
busy — small/medium tasks fill the gaps between large-file work. The
final `chunk_bytes` (network framing) is also adaptive (16 vs 32 MiB).

---

## 5. Adaptive tuning surface

`crates/blit-core/src/auto_tune.rs` produces `TuningParams` from an
observed size hint and persisted history.

```rust
pub struct TuningParams {
    pub initial_streams: u32,         // TCP data-plane parallelism at start
    pub max_streams: usize,           // ceiling for stream count
    pub chunk_bytes: usize,           // wire chunk / buffer pool size
    pub prefetch_count: Option<usize>,// payload pipeline depth
    pub tcp_buffer_size: Option<usize>, // SO_SNDBUF / SO_RCVBUF
}
```

Inputs: total bytes of the manifest, number of files, prior `perf_history`
records keyed by transfer profile. Output values are smoothed across
runs (`crates/blit-core/src/perf_predictor.rs`).

Currently `auto_tune` does NOT cover manifest-batching parameters or
receive-side parallelism; those are hardcoded in places. **This is the
top architectural gap** — see § 8.

---

## 6. Resume / mirror comparison protocol (`pull_sync`)

`pull_sync` is mirror's main path. The flow:

1. Client opens bidi gRPC stream.
2. Client sends `PullSyncHeader` then a `LocalFile` per local entry,
   then `ManifestDone`.
3. Daemon enumerates source, builds server manifest.
4. `manifest::compare_manifests(source, target, opts)` produces a
   `ManifestDiff { files_to_transfer, files_to_delete, ... }`.
5. If diff is empty: daemon sends `Summary`, both sides finish.
6. Otherwise: daemon sends `FilesToDownload` need-list, opens TCP data
   plane, streams via outbound pipeline (`execute_sink_pipeline`).

The comparison is in:

```rust
// crates/blit-core/src/manifest.rs (~line 83)
pub fn compare_manifests(
    source: &[FileHeader],
    target: &[FileHeader],
    options: &CompareOptions,
) -> ManifestDiff {
    let target_map: HashMap<&str, (u64, i64, &[u8])> = target.iter()
        .map(|h| (h.relative_path.as_str(), (h.size, h.mtime_seconds, h.checksum.as_slice())))
        .collect();
    for src in source {
        let status = match target_map.get(src.relative_path.as_str()) {
            None => FileStatus::New,
            Some(&(t_size, t_mtime, t_checksum)) => {
                if options.ignore_existing { FileStatus::SkippedExisting }
                else { compare_file(src, t_size, t_mtime, t_checksum, options.mode) }
            }
        };
        if status == FileStatus::New || status == FileStatus::Modified {
            diff.bytes_to_transfer += src.size;
            diff.files_to_transfer.push(FileComparison { /* ... */ });
        }
    }
    if options.include_deletions { /* extras to delete for mirror_mode */ }
    diff
}
```

`compare_file` switches on `CompareMode` (Default = size+mtime,
SizeOnly, IgnoreTimes, Force, Checksum).

For files marked `Modified` (size matched but mtime differs, or
explicit `--resume`), the daemon can request block hashes via gRPC and
send only differing blocks via the data plane (see
`stream_via_data_plane_resume` in `crates/blit-daemon/src/service/pull_sync.rs`).
This is a Blake3-block-hash-based delta protocol, not rsync's rolling
checksum.

---

## 7. Recent fixes worth scrutiny

These are the changes the last few weeks touched. A reviewer should
focus here for correctness & corner cases.

### 7.1 Receive-pipeline unification (`1baa981`, `a232dbd`, `b64bfd8`)

Pre-unification: daemon's push-receive used `tokio::io::copy` with an
8 KiB buffer (capped throughput at ~1 Gbps); client's pull-receive
used a hand-rolled 64 KiB-buffer loop. Two different code paths for
the same conceptual operation.

Post-unification: both call `execute_receive_pipeline(socket, FsTransferSink, progress)`.
Wire format extended so `FILE` records carry `mtime + perms` inline
(eliminating the daemon's manifest cache requirement). ~525 LOC
deleted from the daemon's bespoke dispatch loop.

**Watch for:** path-tracking semantics (mirror needs to know which
files survived the receive to compute its purge list — done via
`FsTransferSink::with_path_tracker` decorator pattern).

### 7.2 Tar shard parallel extraction (`0bd8bde`)

Two-phase to handle that tar is sequential-read-only:

```rust
// crates/blit-core/src/remote/transfer/sink.rs (~line 330)
fn write_tar_shard_payload(...) -> Result<SinkOutcome> {
    // Phase 1: walk the in-memory tar serially, buffer (path, contents)
    let mut pending: Vec<Pending> = Vec::new();
    for entry_result in archive.entries()? {
        let mut entry = entry_result?;
        if entry.header().entry_type().is_dir() { continue; }
        let rel = /* normalize */;
        let mut contents = Vec::with_capacity(entry.size() as usize);
        std::io::copy(&mut entry, &mut contents)?;
        pending.push(Pending { rel, contents, mtime: /* lookup */ });
    }
    // Phase 2: write files to disk in parallel via rayon
    let results: Vec<Result<u64>> = pending.into_par_iter().map(|p| {
        let dest_path = dst_root.join(&p.rel);
        if let Some(parent) = dest_path.parent() { std::fs::create_dir_all(parent)?; }
        std::fs::write(&dest_path, &p.contents)?;
        if let Some(ft) = p.mtime { let _ = filetime::set_file_mtime(&dest_path, ft); }
        Ok(p.contents.len() as u64)
    }).collect();
    /* aggregate results */
}
```

**Watch for:** memory blowup — the in-memory `pending` Vec holds the
full uncompressed shard contents. Bounded by shard target (~64 MiB
worst case) but per-stream multiplied. Rayon's default thread count
is `num_cpus`.

### 7.3 mtime preservation race (`946bd77`)

When `set_file_mtime` was called while the tokio File handle was
still open (with deferred writes in flight from tokio's blocking-thread
pool), 5/8 files lost mtime. Fix: drop the handle before the syscall.

```rust
// sink.rs::FsTransferSink::write_file_stream
{
    use tokio::io::AsyncWriteExt as _;
    let mut file = tokio::fs::File::create(&dst).await?;
    receive_stream_double_buffered(reader, &mut file, header.size, RECEIVE_CHUNK_SIZE).await?;
    let _ = file.flush().await;
}   // <-- file dropped here, kernel close() complete

if self.config.preserve_times && header.mtime_seconds > 0 {
    let ft = FileTime::from_unix_time(header.mtime_seconds, 0);
    let _ = filetime::set_file_mtime(&dst, ft);
}
```

**Watch for:** the `let _ = ...` swallows errors from `set_file_mtime`.
That's deliberate (cross-fs / permission cases) but masks bugs.

### 7.4 pull_sync deadlock (`946bd77`)

The bug: `pull_sync` was pushing all manifest messages into a 32-deep
mpsc BEFORE opening the gRPC bidi stream. For >30 entries, the 33rd
`tx.send().await` blocked forever — no consumer. Cold mirror worked
accidentally (empty local manifest = 2 messages); noop on a populated
dest hung silently.

```rust
// pull.rs::pull_sync (current, post-fix)
let (tx, rx) = mpsc::channel::<ClientPullMessage>(32);
// Open the gRPC stream FIRST so the daemon starts consuming.
let request_stream = ReceiverStream::new(rx);
let mut response_stream = self.client.pull_sync(request_stream).await?.into_inner();

// Send header on the open channel
tx.send(ClientPullMessage { /* Header */ }).await?;

// Spawn manifest send into its own task — runs concurrently with
// response loop so daemon's responses don't block manifest send.
let manifest_send_task = tokio::spawn(async move {
    for header in &local_manifest_clone {
        tx_for_manifest.send(/* LocalFile */).await?;
    }
    tx_for_manifest.send(/* ManifestDone */).await?;
    Ok(())
});

// ... response loop ...
manifest_send_task.await??;
drop(tx);
```

**Watch for:** the spawned manifest task holds a clone of `tx`. The
original `tx` is dropped explicitly after the response loop. Daemon
sees end-of-stream only when *all* clones drop. Verify the sequence
makes the daemon see EOF reliably (it does in current testing, but
the ordering is subtle).

### 7.5 push small-file completion race (`60d152f`, `5bb78d9`)

The push client's early `data_plane_sender.finish()` could fire while
the daemon was still sending need-list batches (because
`files_requested` only reflected need-lists *already received*). Closing
the data plane mid-stream caused the daemon's `upload_tx` receiver to
drop, and subsequent manifest entries failed to enqueue.

Fix: daemon now always emits an empty `FilesToUpload` as a "no more
need-lists" terminator; client sets `need_lists_done = true` on
receipt and gates the early finish on that flag. 9% completion → 100%
completion verified.

```rust
// daemon side: FileListBatcher::finish always flushes terminator
async fn finish(mut self) -> Result<(), Status> {
    if !self.batch.is_empty() { self.flush().await?; }
    // Always emit empty terminator — discrete "no more need_lists" signal.
    send_control_message(&self.tx,
        server_push_response::Payload::FilesToUpload(FileList { relative_paths: Vec::new() })
    ).await?;
    Ok(())
}

// client side: gate early finish
if matches!(transfer_mode, TransferMode::DataPlane)
    && !need_list_fresh
    && need_lists_done   // <-- new
    && pending_queue.is_empty()
    && manifest_done
    && data_plane_outstanding == 0
    && data_plane_files_sent >= files_requested.len()
{
    if let Some(sender) = data_plane_sender.take() { sender.finish().await?; }
}
```

**Watch for:** old daemon × new client interactions. The daemon now
emits a trailing empty FilesToUpload that an old client would
interpret as a 0-entry need-list (harmless — it advances the loop
once more). New daemon × old client never receives the terminator and
falls back to the post-loop `finish()` after `Summary` arrives.

### 7.6 Daemon push-receive backpressure (`b64bfd8`)

The control plane's gRPC loop pushes manifest entries into an mpsc
that the data plane previously consumed. After the receive
unification (which gets metadata off the wire instead of from the
cache), the consumer is gone — but the producer would still block on
`upload_tx.send().await` if no one drained.

Fix: spawn a drain task in the per-stream handler that consumes the
channel's contents and discards them (since the data plane no longer
needs them).

```rust
// daemon push/data_plane.rs
let drain_handle = {
    let files = Arc::clone(&files);
    tokio::spawn(async move {
        let mut guard = files.lock().await;
        while guard.recv().await.is_some() {}
    })
};
let _ = cache; // cache no longer needed — wire carries full headers

// run the unified receive pipeline
let outcome = execute_receive_pipeline(&mut socket, sink, None).await?;

drain_handle.abort();  // post-receive cleanup
```

**Watch for:** the drain task holds the AsyncMutex across an await on
recv() — fine for tokio's async mutex but not for std::sync. Multiple
parallel data-plane streams share the same Arc<AsyncMutex<Receiver>>;
only one drain task can recv at a time, but with a constant stream of
manifest entries it doesn't matter. The aborted task's lock release
happens during its drop — verify this.

---

## 8. Known issues / open questions

### 8.1 Per-file gRPC overhead during push manifest

Identified in commit-message and code review of `crates/blit-core/src/remote/push/client/mod.rs:615`:

```rust
// One ClientPushRequest::FileManifest per file, one .await per send.
send_payload(&tx, ClientPayload::FileManifest(header.clone())).await?;
```

For 10 000 × 4 KiB files this is ~1 s of pure protocol overhead on top
of network + disk. The bench shows blit at ~10k files/sec while the
underlying ZFS does 110k files/sec for the same set.

The proposed fix (in `docs/plan/UNIFIED_RECEIVE_PIPELINE.md` and a
research synthesis) is an **adaptive batched manifest** — `FileManifestBatch`
proto variant + Kafka-style opportunistic coalescing driven by
in-flight backpressure (no `linger_ms`). Batch caps live in
`TuningParams`, set by `auto_tune::determine_remote_tuning` using
RTT × file count.

A more radical alternative ("skip the manifest phase entirely for
fresh copies and stream tar shards directly") is mentioned in the same
plan but not yet implemented.

### 8.2 Mirror cannot detect `mtime touched, content unchanged`

Comparative mirror bench against rsync:

| Scenario | blit | rsync | rclone |
|---|---|---|---|
| large 4 GiB cold | 4.43 s | 6.91 s | 4.00 s |
| large 4 GiB no-op | **0.01 s** | 0.21 s | 0.16 s |
| large 4 GiB incr (mtime touch only) | **3.96 s** | 2.23 s | 4.15 s |

blit wins the no-op (filesystem journal fast-path: `crates/blit-core/src/change_journal/`)
but loses the incremental: rsync's rolling-checksum diff sends 0 bytes
when content matches despite mtime change; blit re-transfers the
whole file.

The block-hash resume path exists (`stream_via_data_plane_resume`) but
isn't triggered for plain `mirror` — only when `--resume` is set or
the file is newly `Modified` per the size+mtime test. Worth examining:
when, if ever, should mirror auto-promote a size-match-mtime-mismatch
file to block-hash comparison?

### 8.3 PULL gRPC fallback >4 GiB body limit

Pre-existing. The gRPC fallback path has an undocumented 4 GB body
size cap that errors immediately on any single file ≥ 4 GiB. The TCP
data plane has no such limit; the fallback is for restrictive
networks. Easy fix (chunk the gRPC frames) but unscheduled.

### 8.4 Hardcoded constants that should be in TuningParams

A grep for `1024 * 1024` / `MAX_*` in the transfer subsystem turns up:

- `RECEIVE_CHUNK_SIZE = 1024 * 1024` in `data_plane.rs` (good default,
  but should auto-tune based on RTT/disk).
- `MAX_PARALLEL_TAR_TASKS = 4` in daemon's old TarShardExecutor
  (now used only by gRPC fallback).
- Tar shard count thresholds (32, 1024, 2048) in `transfer_plan.rs`.
- mpsc channel capacities (32 in pull_sync, 32 in push manifest exchange).

Each of these is a reasonable static default; none is adaptive. The
project's stated philosophy says they should be.

---

## 9. Test coverage

The workspace has 173 tests passing on Linux; daemon-on-Windows runs
add 20-some platform-specific. Categories:

- Unit tests in each module (`#[cfg(test)] mod tests`).
- Integration tests in `crates/blit-cli/tests/`:
  - `remote_parity` — push TCP / pull TCP / gRPC fallback parity
  - `remote_pull_subpath` — single-file pulls + rsync-style basename rules
  - `remote_push_single_file` — single-file push regression (recent)
  - `remote_resume`, `remote_remote`, `remote_pull_mirror`,
    `remote_transfer_edges`, `remote_move`
  - `single_file_copy` — the local rsync semantics
  - `diagnostics_dump` — bug-report tooling
  - `blit_utils` — admin verbs (scan/ls/find/du/df/rm/completions/profile/list-modules/perf)
- A live-bench harness in `testing/` (gitignored) that orchestrates
  daemon + client + iperf3 baseline.

Notable gaps:
- No fuzz tests for the data plane wire format
- No integration test for the `pull_sync` deadlock scenario fixed in
  `946bd77` — it would be a 30-line test (mirror to a populated dir)
- No tests verify mtime preservation end-to-end (the bug in `946bd77`
  passed all existing tests because none checked mtimes)

---

## 10. Where a reviewer should focus

In rough order of likely yield:

1. **`crates/blit-core/src/remote/pull.rs::pull_sync`** — the longest
   function, most concurrent state, recent fix that may have
   introduced subtleties. ~250 LOC.
2. **`crates/blit-core/src/remote/push/client/mod.rs::push`** —
   even longer, lots of nested `match` arms across the bidi response
   loop, recently modified for the `need_lists_done` gate.
3. **The receive pipeline unification** — sink trait, `write_file_stream`,
   `execute_receive_pipeline`, `WireReader → take`. New code, exposed
   to all push and pull receive paths.
4. **`auto_tune.rs` + `perf_predictor.rs`** — the adaptive tuning
   surface. What it covers (chunk_bytes, streams) and what it doesn't
   (manifest batching, anything receive-side).
5. **The data plane wire format encoders/decoders** —
   `data_plane.rs::send_*` and `pipeline.rs::execute_receive_pipeline`.
   Field order changes (e.g. recent mtime+perms inline addition) need
   to stay in sync between sender and receiver.
6. **Resume protocol (`stream_via_data_plane_resume`)** — least
   exercised, most state.
7. **`change_journal/` subsystem** — three platform-specific
   implementations (Linux, macOS FSEvents, Windows USN). Drives the
   no-op fast path.

Specific questions a reviewer could answer:

- Are there any `await` points in either pull or push hot loops where
  we hold a lock or open file handle that we shouldn't?
- The bidi gRPC stream's tx is shared between a sender task and the
  main loop in both `push` and `pull_sync` — is the drop ordering
  correct, or could the daemon see a premature EOF?
- The wire format has `path_len: u32` on three different records. Are
  the bounds checks consistent? (Spot check: `helpers.rs` and
  `data_plane.rs::send_file_from_reader` both check; what about
  receive-side?)
- TarShardExecutor in `crates/blit-daemon/src/service/push/data_plane.rs`
  is now used only by the gRPC fallback. Worth deleting outright?
- Is there a sensible test that would have caught the
  `pull_sync` channel deadlock (3rd-party reviewer was supposed to
  look for `await` on a bounded channel before its consumer is
  attached)?

---

## Appendix: build / run

```sh
# Build everything
cargo build --release --workspace

# Run daemon
./target/release/blit-daemon --root /some/path

# Run a transfer
./target/release/blit-cli copy /local/path server:/path/
./target/release/blit-cli mirror server:/path/ /local/dest/

# Test
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Static x86_64-musl + arm64-musl builds for portable deployment are
documented in scripts/build-release.sh and supported via
`rustup target add aarch64-unknown-linux-musl`.
