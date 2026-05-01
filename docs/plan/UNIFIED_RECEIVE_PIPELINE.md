# Plan: Unified receive pipeline

## Goal

Make the receive side of every transfer go through the same sink
abstraction the send side already uses, so the project's tenet —
"all data goes through the same pipeline regardless of location or
direction" — holds end-to-end.

Today: send side is unified (`TransferSource → execute_sink_pipeline →
TransferSink`); receive side has bespoke wire-parsing loops in
`crates/blit-daemon/src/service/push/data_plane.rs::handle_data_plane_stream`
and `crates/blit-core/src/remote/pull.rs::{handle_file_record,
handle_tar_shard_record,…}` that write to disk directly.

After: `FsTransferSink` is the single write surface for every
destination (local copy, remote push receive, remote pull receive,
remote→remote relay). The wire becomes a streaming source.

## Architecture target

```
                          TransferSink trait
                   ┌───────────┬──────────────┐
                   ▼           ▼              ▼
              FsTransferSink  DataPlaneSink  NullSink/GrpcFallbackSink

  Outbound:  TransferSource  ──► execute_sink_pipeline_streaming ──► sinks…
              (Fs / Remote)                                          (parallel)

  Inbound:   DataPlaneSource ──► execute_receive_pipeline       ──► single sink
              (one per wire)                                        (FsTransferSink)
```

The send executor stays as-is. We add an inbound executor that drives
one sink per wire stream sequentially (the wire is a sequential
producer; parallelism comes from N wire streams, each with its own
executor + sink, exactly mirroring the send side which has N sinks).

## Phases

### Phase 1 — `PreparedPayload::FileStream` variant

**File:** `crates/blit-core/src/remote/transfer/payload.rs`

Add a new variant carrying an owned async reader:

```rust
pub enum PreparedPayload {
    File(FileHeader),                       // existing — bytes on disk at src_root
    FileStream {                             // new — bytes arrive on a stream
        header: FileHeader,
        reader: Box<dyn AsyncRead + Unpin + Send>,
    },
    TarShard { headers: Vec<FileHeader>, data: Vec<u8> },
}
```

Justification for the new variant rather than buffering into
`PreparedPayload::File(header) + bytes_field`: a 4 GiB file cannot be
buffered in memory. The reader hands off ownership of "the next N
bytes of this stream" to the sink.

Touched code: every `match` on `PreparedPayload` (currently in
`sink.rs` and `pipeline.rs`). Each must add the new arm or `_ => …`
fallback. Clippy + cargo check will surface them all.

### Phase 2 — sink support for `FileStream`

**File:** `crates/blit-core/src/remote/transfer/sink.rs`

For each existing sink, add a `FileStream` arm:

- **`FsTransferSink`**: create dest file, call
  `receive_stream_double_buffered(&mut reader, &mut file, header.size,
  RECEIVE_CHUNK_SIZE)`, fsync, apply mtime/permissions if configured,
  return `SinkOutcome { files_written: 1, bytes_written: header.size }`.
  Reuses the helper added in commit XXX (the partial fix).
- **`NullSink`**: drain the reader (`tokio::io::copy(reader,
  &mut tokio::io::sink())`), return outcome with bytes counted but
  zero files written. Lets `--null` work for receive benchmarks.
- **`DataPlaneSink`**: read from `reader` and forward to wire — i.e.,
  the relay case (remote→remote). Implementation: copy from reader
  straight into `self.session.send_file_with_reader(header, reader)`,
  a new method that mirrors `send_file_double_buffered` but takes a
  reader instead of opening from disk.
- **`GrpcFallbackSink`**: similar — chunk the reader into gRPC frames.

### Phase 3 — `DataPlaneSource`

**File:** `crates/blit-core/src/remote/transfer/source.rs`

```rust
pub struct DataPlaneSource {
    socket: Arc<Mutex<TcpStream>>,   // interior mutability — prepare_payload is &self
    dst_root: PathBuf,                // where we'd write (stored, not used here directly)
}

impl TransferSource for DataPlaneSource {
    fn scan(...) {
        // Spawn a task that reads wire records and emits FileHeader
        // for each FILE/TAR_SHARD record. Does NOT read file bytes —
        // those are read lazily in prepare_payload.
        // Halts on DATA_PLANE_RECORD_END.
    }

    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
        match payload {
            File(h) => {
                // Wire layout for File: [size:u64][bytes:size]
                // We've already consumed [tag][path_len][path] in scan.
                // Read size + return a Take<&mut TcpStream>-equivalent.
                Ok(PreparedPayload::FileStream { header: h, reader: ... })
            }
            TarShard { headers } => {
                // Wire: [shard_size:u64][tar_data:shard_size]
                // Read into Vec<u8> (existing behavior — tar shards
                // are small by construction; the tar-shard threshold
                // bounds memory).
                Ok(PreparedPayload::TarShard { headers, data })
            }
        }
    }
}
```

The Mutex synchronizes `scan`'s producer task with `prepare_payload`'s
consumer task — they're alternating reads of the same socket, never
concurrent. In practice we could express this as a single sequential
state machine (next phase) and avoid the Mutex.

### Phase 4 — `execute_receive_pipeline` executor

**File:** `crates/blit-core/src/remote/transfer/pipeline.rs`

```rust
pub async fn execute_receive_pipeline(
    source: DataPlaneSource,
    sink: Arc<dyn TransferSink>,
    progress: Option<&RemoteTransferProgress>,
) -> Result<SinkOutcome> {
    // Sequential per-wire-stream loop:
    //   read record header → match on tag →
    //     File / TarShard: source.prepare_payload(...) →
    //       sink.write_payload(prepared) → progress
    //     Block / BlockComplete: route to sink's resume API (Phase 6)
    //     End: break
    //   accumulate SinkOutcome
    // Call sink.finish() at the end. Return total.
}
```

Why a separate executor: the wire is sequential. A single TCP stream
cannot be fanned out to N parallel sinks (records come ordered).
Parallelism on receive is N TCP streams × one executor each, mirroring
the send side's N sinks.

Architecturally this is the inverse of `execute_sink_pipeline_streaming`.
Together they form a complete pair: outbound = fan-one-source-into-N-sinks;
inbound = fan-N-wires-into-N-(executor+sink)-pairs that share a final
SinkOutcome aggregate.

### Phase 5 — daemon push-receive call site

**File:** `crates/blit-daemon/src/service/push/data_plane.rs`

Replace the body of `handle_data_plane_stream` (after token validation):

```rust
let source = DataPlaneSource::new(socket, module.path.clone());
let sink = Arc::new(FsTransferSink::new(
    /* src_root: doesn't apply for stream — pass dst_root or dummy */,
    module.path.clone(),
    FsSinkConfig { preserve_times: true, .. },
));
let outcome = execute_receive_pipeline(source, sink, None).await?;
stats.files_transferred = outcome.files_written;
stats.bytes_transferred = outcome.bytes_written;
```

Delete the inline File/TarShard/Block parsing. The Block (resume)
branch needs Phase 6.

### Phase 6 — daemon pull-receive call site

**File:** `crates/blit-core/src/remote/pull.rs`

Replace `handle_file_record` / `handle_tar_shard_record` /
`handle_block_record` / `handle_block_complete_record` with one call:

```rust
let source = DataPlaneSource::new(stream, dest_root.to_path_buf());
let sink = Arc::new(FsTransferSink::new(
    /* src_root */ PathBuf::new(),
    dest_root.to_path_buf(),
    FsSinkConfig { /* preserve_times etc. from PullOptions */ },
));
let outcome = execute_receive_pipeline(source, sink, progress).await?;
stats.files_transferred = outcome.files_written;
stats.bytes_transferred = outcome.bytes_written;
```

The `track_paths` use case (recording downloaded paths for mirror
purge) needs to come from the sink — add an optional path-tracker
field on `FsTransferSink` or a wrapping sink decorator.

### Phase 7 — Resume block records

**Open question — needs decision before implementation.**

The data plane carries two kinds of records that don't fit the
"whole file" payload model:
- `DATA_PLANE_RECORD_BLOCK { path, offset, bytes }` — patch a block
  of an existing file at offset.
- `DATA_PLANE_RECORD_BLOCK_COMPLETE { path, total_size }` — truncate
  to total_size after all blocks applied.

Two reasonable directions:

**A. New payload variants.**
```rust
pub enum TransferPayload {
    File(FileHeader),
    TarShard { headers: Vec<FileHeader> },
    FileBlock { path: String, offset: u64, size: u64 },          // new
    FileBlockComplete { path: String, total_size: u64 },         // new
}
```
Then `FsTransferSink::write_payload` adds arms that open the
existing file at `dst_root/path`, seek, write, or truncate.

Pros: receive pipeline stays uniform — all wire records become
payload events.

Cons: inflates the payload enum with concepts that don't apply to
local copies. Block records are receive-only; they'd be `unreachable!()`
in the send pipeline.

**B. Direct sink API.**
Add `TransferSink::apply_block(&self, path, offset, bytes)` and
`TransferSink::truncate(&self, path, total_size)`. The receive
executor calls these directly when it sees block records. Outbound
sinks default-impl them as `unreachable!()` or no-op.

Pros: keeps `TransferPayload` semantically about whole files +
shards.

Cons: adds methods to the sink trait that are receive-only.

**Recommendation:** A. Keeps the executor simpler (one match
statement, all on `TransferPayload`); the trait stays focused.
The "doesn't apply outbound" cost is honest — the variants exist,
but `RemoteTransferSource` and `FsTransferSource` never produce them.

### Phase 8 — Tests + cleanup

- Existing tests:
  - `remote_parity` (push TCP, pull TCP, push gRPC, pull gRPC) — must
    pass unchanged.
  - `remote_resume` — block-level resume; must pass after Phase 7.
  - `remote_remote` — remote→remote relay; exercises the new
    `DataPlaneSink::FileStream` arm.
  - `remote_pull_subpath`, `remote_push_single_file` — single-file
    edge cases.
  - `remote_transfer_edges`, `remote_pull_mirror`, `remote_move`.
- New unit tests:
  - `DataPlaneSource::scan` parsing FILE / TAR_SHARD / END records.
  - `FsTransferSink::write_payload(FileStream)` writes bytes correctly,
    fsyncs, applies metadata.
  - `execute_receive_pipeline` end-to-end with a fake socket pair.
- Delete dead code:
  - `crates/blit-daemon/src/service/push/data_plane.rs`:
    `handle_data_plane_stream`'s File/TarShard branches.
  - `crates/blit-core/src/remote/pull.rs`:
    `handle_file_record`, `handle_tar_shard_record`,
    `handle_block_record`, `handle_block_complete_record`.
- Bench:
  - `testing/bench.sh skippy admin 9031 …` — push large 4 GiB should
    now match pull throughput (target ≥7 Gbps; pull baseline is 9.29
    Gbps).

### Phase 9 — Docs

- `docs/ARCHITECTURE.md` — update the transfer-pipeline diagram.
- `DEVLOG.md` — entry covering the unification + perf restoration.
- `CHANGELOG.md` — note the receive-side rewrite under the next
  unreleased section.

## Risk register

1. **Mutex contention in `DataPlaneSource`.** Phase 3's Mutex around
   the socket is a held lock during 4 GiB file streams. If we ever
   want to allow `scan` to read ahead, this becomes a bottleneck.
   Mitigation: keep scan strictly synchronous-with-prepare for v1
   (alternation, no contention); revisit if needed.

2. **`Box<dyn AsyncRead>` allocation per file.** Heap-allocates per
   payload. At 35k files this is 35k allocations. Negligible perf
   cost, called out for completeness.

3. **`FsTransferSink::FileStream` doesn't get the zero-copy
   cascade.** It writes bytes from a reader; can't use `clonefile` /
   `copy_file_range`. That's correct — the bytes don't exist on disk
   yet — but it means receive throughput is bound by `tokio::fs::File`
   write speed, not block clone speed. (For pure-network-to-disk
   transfers there's no block-clone case anyway.)

4. **Resume API in `TransferSink`.** Block records (Phase 7) muddy
   the trait. If we go with payload variants (option A), no API
   change but the enum grows. Either way it's a small surface change.

## Estimated effort

- Phase 1: 30 min (enum + match arms + trivial sinks)
- Phase 2: 2 h (FsTransferSink streaming write + DataPlaneSink relay
  + tests for each)
- Phase 3: 2 h (DataPlaneSource + scan loop + prepare_payload)
- Phase 4: 1 h (executor + sink.finish + outcome aggregation)
- Phase 5: 30 min (daemon call site swap)
- Phase 6: 30 min (client call site swap + path-tracker integration)
- Phase 7: 1.5 h (resume payload variants + sink arms + tests)
- Phase 8: 1 h (run/fix integration tests, bench)
- Phase 9: 30 min (docs)
- **Total: ~9 hours of focused work**, plus iteration if perf doesn't
  match pull on the first run.

## Success criteria

- `cargo test --workspace` green; clippy clean.
- `testing/bench.sh skippy …` reports push large TCP ≥ 90 % of pull
  large TCP throughput. Both within 10 % of iperf3 baseline.
- `grep -rn "tokio::io::copy\|read_exact.*file_size" crates/blit-{daemon,core}`
  finds no hand-rolled receive loops outside `receive_stream_double_buffered`.
- Hand-rolled record dispatch in `handle_data_plane_stream` and
  `handle_file_record` deleted; their bodies are 5–10 lines that
  call `execute_receive_pipeline`.
