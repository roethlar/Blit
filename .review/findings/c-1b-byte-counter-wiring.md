# c-1b-byte-counter-wiring: data plane reports bytes against `ActiveJobs`

**Severity**: Feature (foundational slice of milestone C — final byte-counter wiring)
**Status**: In progress / pending review
**Branch**: `phase5/c`
**Commit**: filled by the sentinel commit

## What

Second atomic slice of milestone C. Wires the c-1a
`ByteProgressSink` through the daemon's data-plane receive
chain so a live delegated_pull transfer reports bytes as the
data plane writes them, surfacing in
`GetState.active[].bytes_completed` and (at completion)
`GetState.recent[].bytes`.

## Why ship this on top of c-1a

c-1a landed the registry-side machinery (per-row atomic +
`ByteProgressSink` type in `blit-core`). With no callers, the
infrastructure was dead and the wire fields stayed at zero.
c-1b is the call-site work: thread the sink from the dispatch
site through the pull chain to the data plane's chunk loop.

After c-1b a delegated_pull transfer over many GB shows
non-zero `bytes_completed` while in flight, and the post-Drop
ring entry's `bytes` field reflects the final byte count.
That's the first user-visible deliverable of milestone C.

## Approach

### Plumbing chain

```
service/core.rs::delegated_pull dispatch
  ├ captures job.bytes_counter()  ← ByteProgressSink
  └ moves it into the spawn closure
    │
    handle_delegated_pull(req, ..., transfer_id, byte_progress)
      │
      run_delegated_pull(..., byte_progress: &ByteProgressSink)
        │
        pull_client.pull_sync_with_spec(
            dest_root, manifest, spec, track_paths,
            progress, Some(byte_progress))
          │
          spawn_data_plane_receiver(neg, dest_root, …, progress,
                                    byte_progress: Option<&ByteProgressSink>)
            │
            tokio::spawn → receive_data_plane_streams_owned(…, Option<ByteProgressSink>)
              │
              ├ stream_count == 1:
              │   receive_data_plane_stream_inner(…, byte_progress: Option<&ByteProgressSink>)
              │
              └ stream_count > 1:
                  N parallel workers, each receives a clone()
                  of the same Arc-backed sink (Arc::clone, not
                  a new counter)
                │
                receive_data_plane_stream_inner(…)
                  │
                  ├ if Some(bp) → FsTransferSink::with_byte_progress(bp.clone())
                  └ if None     → no field set; behavior unchanged
                    │
                    execute_receive_pipeline(socket, sink, progress)
                      │
                      sink.write_file_stream(header, &mut reader)
                        │
                        FsTransferSink::write_file_stream:
                          receive_stream_double_buffered(
                              reader, &mut file, size, RECEIVE_CHUNK_SIZE,
                              self.byte_progress.as_ref())
```

`receive_stream_double_buffered` calls `byte_progress.report(delta)`
after each successful `write_all` — including the final-tail
write — so a snapshot taken mid-transfer reflects what's
actually on disk, never more.

### Why N parallel workers share one counter

For multi-stream pulls (data-plane stream_count > 1), each
worker gets a `clone()` of the sink. Clone is an Arc bump —
all clones point at the same atomic. Reported numbers add up
naturally: the wire-level `bytes_completed` is the sum across
all worker streams, which is what an operator wants to see.

### CLI-only paths pass None

The CLI side has no `ActiveJobs` row to report against. Three
callsites pass `None`:

1. `RemotePullClient::pull` (CLI's simple pull entry) — its
   internal `spawn_data_plane_receiver` call gets `None`.
2. `RemotePullClient::pull_sync` (CLI wrapper around
   `pull_sync_with_spec`) — forwards `None`.
3. `FsTransferSink::write_file_stream`'s dry-run branch —
   bytes never reach disk; reporting them would lie.
4. `NullSink::write_file_stream` — benchmark-only path that
   discards bytes; same reasoning as dry-run.

### Tests added

In `crates/blit-core/src/remote/transfer/data_plane.rs`:

- `copies_without_progress_when_sink_omitted` — `None`
  parameter matches pre-c-1b behavior; bytes copied, no
  sink calls.
- `cumulative_reports_match_bytes_copied` — final atomic
  value equals `n` returned from the copy.
- `reports_fire_incrementally_under_load` — at least one
  intermediate report fires before the final-tail one
  (prove the hook is inside the loop, not bolted on at the
  end).

The existing wire-roundtrip test
(`pull_sync_with_spec_wire.rs`) was updated to pass the new
`None` parameter for both calls. Test surface stays at the
same coverage; new positive coverage comes from the data-plane
tests above.

End-to-end behavior (a real delegated_pull transferring N MB
and seeing GetState.recent[].bytes == N) is not unit-tested
here — the existing daemon integration test
(`remote_remote.rs`) exercises the path and would catch a
regression in the wiring. A dedicated integration test for
the live-snapshot behavior is deferred to a future C slice
(needs a way to pause mid-transfer, which the c-3 throughput
EWMA slice will naturally provide via its time-based hooks).

## Files changed

- `crates/blit-core/src/remote/transfer/data_plane.rs`:
  - `receive_stream_double_buffered` gains
    `byte_progress: Option<&ByteProgressSink>` parameter.
  - In-loop `report(bytes_a as u64)` after each `write_all`.
  - Final-tail `report` after the closing write.
  - `+#[cfg(test)] mod byte_progress_tests` with 3 unit tests.
- `crates/blit-core/src/remote/transfer/sink.rs`:
  - `FsTransferSink` gains `byte_progress: Option<ByteProgressSink>`
    field (default None).
  - `FsTransferSink::with_byte_progress(sink)` builder.
  - `write_file_stream` passes `self.byte_progress.as_ref()`
    to the data-plane fn for the real-write branch; passes
    `None` for the dry-run drain branch.
  - `NullSink::write_file_stream` passes `None`.
- `crates/blit-core/src/remote/pull.rs`:
  - `spawn_data_plane_receiver` gains
    `byte_progress: Option<&ByteProgressSink>` parameter
    (cloned into the spawn closure).
  - `receive_data_plane_streams_owned` gains
    `byte_progress: Option<ByteProgressSink>` parameter
    (cloned for each parallel worker).
  - `receive_data_plane_stream_inner` gains
    `byte_progress: Option<&ByteProgressSink>` parameter;
    applies via `FsTransferSink::with_byte_progress` when
    Some.
  - `pull_sync_with_spec` gains
    `byte_progress: Option<&ByteProgressSink>` parameter.
  - `pull` and `pull_sync` (CLI paths) pass `None` to their
    `spawn_data_plane_receiver` call.
- `crates/blit-core/tests/pull_sync_with_spec_wire.rs`:
  - Two existing `pull_sync_with_spec` callsites updated to
    pass the new `None` parameter.
- `crates/blit-daemon/src/service/core.rs`:
  - Delegated pull dispatch captures `job.bytes_counter()`
    before the spawn closure and threads it into
    `handle_delegated_pull`.
- `crates/blit-daemon/src/service/delegated_pull.rs`:
  - `handle_delegated_pull` gains
    `byte_progress: ByteProgressSink` parameter.
  - `run_delegated_pull` gains
    `byte_progress: &ByteProgressSink` parameter.
  - The internal `pull_client.pull_sync_with_spec(...)` call
    now passes `Some(byte_progress)`.
  - Unit test `handle_delegated_pull_returns_false_on_handler_failure`
    updated to pass a fresh sink via `ByteProgressSink::new()`.

## Out of scope (next slices)

- **c-2-bytes-total**: wire `bytes_total` from the manifest
  stage so progress renderers can compute percentages.
- **c-3-throughput**: 1-second throughput EWMA in the
  ActiveJob row.
- **c-4-files-counter**: `files_completed` / `files` analogue
  of the byte counter, fed from `report_file_complete`.
- **c-5-event-ring**: per-job event ring (m-jobs-4 deferred to C).
- **c-6-subscribe**: Subscribe RPC + DaemonEvent family +
  broadcast + `transfer_id_filter` (the remaining bulk of C).
- **c-7-jobs-watch-stream**: upgrade `blit jobs watch` from
  polling to streaming.
- **c-1c-push-wiring**: extend the byte-counter plumbing to
  the push dst-side reception path
  (`service/push/data_plane.rs`). Lower priority since push
  has the CLI in the byte path — operators can read CLI
  progress directly; daemon-side reporting matters more for
  remote→remote.

## Known gaps

1. **Push path not yet instrumented.** The dst-side receive
   for `push` runs through `execute_receive_pipeline` like
   delegated_pull, but the daemon currently constructs its
   `FsTransferSink` there without a byte sink. Deferred to
   c-1c. Until then, `push` transfers always report
   `bytes_completed = 0` in `GetState` while in flight (the
   final ring `bytes` also stays at zero on push). The metrics
   ring already has the data via `metrics.inc_push()`, so this
   is purely a wire-shape gap.

2. **Pull_sync (CLI consumer) not instrumented.** Same
   architectural reason as push — `pull_sync` is a streaming
   RPC with the CLI in the byte path. Daemon-side ActiveJobs
   row exists, but no counter feeds into it for this RPC
   shape. The fix in c-1c would similarly extend the data-plane
   sender side (the daemon is the SOURCE for pull_sync), which
   is a different code path.

3. **No integration test of live-snapshot mid-transfer.** A
   delegated_pull transfer of N MB is exercised by
   `remote_remote.rs`, but it doesn't currently assert
   `GetState.active[].bytes_completed > 0` mid-transfer. That
   requires either a long-enough transfer to race against
   polling, or a pause hook in the data plane. Deferred —
   c-3's throughput EWMA work needs the same hook and will
   add it naturally.

## Reviewer comments

(empty — pending grade)
