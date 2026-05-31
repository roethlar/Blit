# c-4-transfer-progress: periodic TransferProgress events

**Severity**: Feature (third event-family member for milestone C's Subscribe wire surface)
**Status**: In progress / pending review
**Branch**: `phase5/c`
**Commit**: filled by the sentinel commit

## What

Adds `TransferProgress` events to the `DaemonEvent.oneof`
family. Fills the gap between c-2's `TransferStarted` and
c-3's `TransferComplete`/`TransferError`: subscribers now see
chunk-granular byte progress for active transfers without
polling `GetState`.

The daemon spawns a background tokio interval task at 10 Hz
that walks the `ActiveJobs` table and broadcasts one event per
active row each tick, with the row's current `bytes_completed`
and an instantaneous `throughput_bps` derived from the delta
since the last tick.

## Approach

### Proto

`DaemonEvent.oneof.transfer_progress = 2` was reserved by c-2;
this slice unreserves it and adds the message:

```protobuf
message TransferProgress {
  string transfer_id = 1;
  uint64 bytes_completed = 2;
  uint64 bytes_total = 3;      // 0 until manifest-stage slice
  uint64 files_completed = 4;  // 0 until files-counter slice
  uint64 files_total = 5;      // 0 until manifest-stage slice
  uint64 throughput_bps = 6;   // instantaneous over tick window
}
```

Fields beyond `bytes_completed` and `throughput_bps` stay at 0
in this slice — the wire shape carries them so future
subscribers don't need a proto roll once their producers land.

### Per-row tracking

```rust
struct TableEntry {
    // ...
    last_progress_bytes: AtomicU64,
    last_progress_unix_ms: AtomicU64,
}
```

Each row tracks "byte count + timestamp at the most recent
sample." Initialized to `(0, start_unix_ms)` at register time
so the first tick sees a real `(now - start)` window.

### Sampling: `ActiveJobs::snapshot_progress_samples()`

Atomic-swap pattern: load the current `bytes_counter`, swap
into `last_progress_bytes` (returning the old value), same
for the unix-ms. Compute throughput as
`(delta_bytes * 1000) / delta_ms`, clamped at 0 for
same-millisecond ticks.

Done under the table lock so the snapshot is internally
consistent (no per-row tearing).

### Per-tick fan-out: `tick_progress_once`

Free function in `service/core.rs`:

```rust
pub(crate) fn tick_progress_once(
    active_jobs: &ActiveJobs,
    events_tx: &broadcast::Sender<DaemonEvent>,
) -> usize {
    let samples = active_jobs.snapshot_progress_samples();
    let n = samples.len();
    for sample in samples {
        let event = DaemonEvent {
            payload: Some(daemon_event::Payload::TransferProgress(TransferProgress {
                transfer_id: sample.transfer_id,
                bytes_completed: sample.bytes_completed,
                bytes_total: 0,
                files_completed: 0,
                files_total: 0,
                throughput_bps: sample.throughput_bps,
            })),
        };
        let _ = events_tx.send(event);
    }
    n
}
```

Free function rather than a method on `BlitService` so tests
can drive it without standing up the broadcast subscriber side
of the world.

### Long-running ticker: `spawn_progress_ticker`

Spawns a tokio task that ticks every `DEFAULT_PROGRESS_TICK_MS`
(100ms, per `TUI_DESIGN.md` §6.2 step 3) and calls
`tick_progress_once`. `MissedTickBehavior::Skip` so a daemon
pause doesn't cause a burst of catch-up ticks afterwards.

Wired into `main.rs` after `BlitService::from_runtime`. The
returned `JoinHandle` is stored in a `_progress_ticker` binding
that lives for the daemon's lifetime; tokio aborts in-flight
tasks on process exit.

Tests don't call `spawn_progress_ticker` — they drive
`tick_progress_once` directly for deterministic ordering.

## Files changed

- `proto/blit.proto`:
  - `DaemonEvent.oneof.transfer_progress = 2` (was reserved).
  - `+message TransferProgress`.
- `crates/blit-daemon/src/active_jobs.rs`:
  - `+ProgressSample` public struct.
  - `+TableEntry.last_progress_bytes`,
    `last_progress_unix_ms` atomic fields.
  - `+ActiveJobs::snapshot_progress_samples()`.
- `crates/blit-daemon/src/service/core.rs`:
  - `+const DEFAULT_PROGRESS_TICK_MS = 100`.
  - `+tick_progress_once(active_jobs, events_tx)` free fn.
  - `+spawn_progress_ticker(svc)` long-running ticker spawn.
- `crates/blit-daemon/src/service/mod.rs`:
  - Re-export `spawn_progress_ticker`.
- `crates/blit-daemon/src/main.rs`:
  - Call `spawn_progress_ticker(&service)` and bind the
    handle to `_progress_ticker`.

## Tests added

3 new in `service::core::tests`:

- `tick_progress_emits_transfer_progress_per_active_row` —
  two active rows, one with reported bytes; assert two
  TransferProgress frames on a subscriber stream with
  matching `(transfer_id, bytes_completed)` pairs.
- `tick_progress_throughput_reflects_delta_between_ticks` —
  baseline tick → sleep 50ms → report 50 KiB → second tick;
  assert `throughput_bps > 0` and within a sane ceiling.
- `tick_progress_emits_zero_events_when_no_active_rows` —
  empty table returns count 0.

Workspace: 565 passing serially (was 562; +3).

## Known gaps

1. **`bytes_total` always 0.** Wired from the manifest stage in
   a follow-up slice — the source-side enumeration knows the
   total bytes once it builds the manifest, but that signal
   doesn't flow back to the dst-daemon's ActiveJobs row today.

2. **`files_completed` / `files_total` always 0.** Same posture
   as bytes_total. Wire shape correct so consumers don't need
   a proto roll once the producers land.

3. **`throughput_bps` is instantaneous, not smoothed.** Future
   slice can wrap an EWMA around the per-row delta if the
   raw values prove jittery in practice. Field name +
   semantics are forward-stable; the EWMA would replace the
   value without a wire change.

4. **Tick cadence is a compile-time constant.** Future slice
   could expose it via `SubscribeRequest` (the design doc
   mentions "configurable via `SubscribeRequest`" as an open
   question, §10 Q3). Default-only for now.

5. **First tick after register reports zero throughput.** The
   per-row tracking initializes to `start_unix_ms` so the
   first sample sees a non-zero window — but the byte counter
   starts at 0, so the delta is whatever was reported in
   that window (usually a small first chunk). The math is
   correct; just noting it's not "zero on first call by
   construction."

6. **No subscriber-side throttling.** Slow subscribers fall
   back to the c-2 Lagged → `Status::aborted` recovery path.
   At 10 Hz with N active transfers, a stalled subscriber
   exhausts the broadcast capacity (256) after ~25/N seconds.
   That's the right behavior for the design — TUI re-fetches
   via GetState on Lagged.

## Out of scope (next slices)

- **c-5-event-ring + transfer_id_filter**: per-job event ring
  (m-jobs-4 deferral) + `SubscribeRequest.transfer_id_filter`
  (m-jobs-5 deferral). Lets `Subscribe(transfer_id=X)` replay
  recent events for X and filter the live stream.
- **c-6-jobs-watch-stream**: upgrade `blit jobs watch` from
  GetState polling to Subscribe streaming.
- **c-7-module-and-heartbeat**: `ModuleListChanged`,
  `DaemonHeartbeat` event variants.
- Wire `bytes_total`, `files_completed`, `files_total`,
  `tcp_fallback_used` (split across several lower-priority
  slices).

## Round 2 (sha `5b88f3a`)

Reviewer caught a real race: round 1's `snapshot_progress_samples`
returned a Vec; the ticker then iterated and broadcast outside
the table lock. A c-3 spawn closure that did
`Drop(guard) + events_tx.send(terminal)` could interleave
between the snapshot and the first progress send, leading a
subscriber to see `TransferProgress` AFTER `TransferComplete`
for the same `transfer_id`.

Fix: `ActiveJobs::for_each_progress_sample(emit)` invokes the
emit closure while still holding the table lock. The ticker's
emit does `broadcast::Sender::send`, which is synchronous (no
await, no I/O — pushes to an in-memory ring), so the lock
window stays bounded by the active-row count.

Drop also acquires this lock to remove its row. The two paths
now serialize:

- **Ticker wins the lock**: progress events for every live row
  fire while Drop is blocked. When the ticker releases, Drop
  runs (row removed); the spawn closure's terminal event
  broadcast follows. Subscriber sees `Progress*, Terminal`.
- **Drop wins the lock**: row gone before ticker iterates, no
  progress event fires for that id. Terminal event has been
  (or will shortly be) broadcast. Subscriber sees `Terminal`
  only.

Either way, no progress-after-terminal for any transfer id.

Kept `snapshot_progress_samples` as a `#[cfg(test)]`
convenience that walks `for_each_progress_sample` into a Vec
— the c-4 ticker doesn't use it.

+1 regression test:

- `progress_event_cannot_arrive_after_terminal_for_same_transfer`
  runs the ticker and a build→drop→broadcast dropper as two
  `tokio::task::spawn_blocking` tasks. Either interleave must
  yield a stream where any `TransferProgress` for the id
  comes before its `TransferComplete`/`TransferError`. Test
  is robust to either lock ordering since both are valid
  outcomes; it just asserts the invariant.

Workspace: 566 passing serially (was 565; +1).

## Reviewer comments

(empty — pending grade)
