# c-3-transfer-finished-events: TransferComplete + TransferError

**Severity**: Feature (second event-family pair for milestone C's Subscribe wire surface)
**Status**: In progress / pending review
**Branch**: `phase5/c`
**Commit**: filled by the sentinel commit

## What

Adds `TransferComplete` and `TransferError` to the
`DaemonEvent.oneof` family. Fires from each spawn closure at
the same site that calls `record_outcome` so every transfer
that emitted `TransferStarted` (c-2) also emits exactly one
terminal event — bracketing the transfer's lifetime on the
Subscribe stream.

## Why this is the next slice after c-2

c-2 landed `TransferStarted` so subscribers see new transfers
arriving. Without a matching terminal event, subscribers can't
tell when a transfer ends — they'd need to poll `GetState`
every cycle to detect "still active" vs "moved to recent".

The TransferComplete/Error pair is the second piece of the
load-bearing event lifecycle. Choosing it before
`TransferProgress` because:

- It's simpler: fire-once-at-end semantics, no timer/EWMA
  machinery.
- It composes cleanly with c-2's spawn-closure pattern (same
  call sites, same broadcast channel).
- Operators get the most value from "transfer started" +
  "transfer ended" before they need byte-level progress
  updates between those poles.

Progress events land in c-4.

## Approach

### Proto wire shape

```protobuf
message DaemonEvent {
  oneof payload {
    TransferStarted  transfer_started  = 1;
    // tag 2 reserved for TransferProgress (c-4)
    TransferComplete transfer_complete = 3;
    TransferError    transfer_error    = 4;
    // 5..6 reserved for ModuleListChanged, DaemonHeartbeat
  }
  reserved 2, 5, 6;
}

message TransferComplete {
  string transfer_id = 1;
  uint64 bytes = 2;             // from per-row atomic (c-1a)
  uint64 files = 3;             // 0 until files-counter slice
  uint64 duration_ms = 4;       // unix_ms_now() - start_unix_ms
  bool tcp_fallback_used = 5;   // false today; wired later
}

message TransferError {
  string transfer_id = 1;
  string message = 2;
}
```

Field numbers match the `TransferStarted=1, TransferProgress=2,
TransferComplete=3, TransferError=4, ModuleListChanged=5,
DaemonHeartbeat=6` ordering from
`docs/plan/TUI_DESIGN.md` §6.2. Tag 2 stays reserved.

### Daemon-side: terminal-event builder

```rust
pub(crate) fn build_transfer_finished_event(
    guard: &ActiveJobGuard,
    ok: bool,
    error_message: Option<&str>,
) -> DaemonEvent {
    if ok {
        DaemonEvent { payload: Some(daemon_event::Payload::TransferComplete(...)) }
    } else {
        DaemonEvent { payload: Some(daemon_event::Payload::TransferError(...)) }
    }
}
```

Free function rather than `&self` method because the caller is
a spawn closure that doesn't have `BlitService` in scope —
it holds a cloned `broadcast::Sender` instead. The builder
takes a `&ActiveJobGuard` so the emitted event's `bytes` /
`duration_ms` come from the same row that `TransferRecord`
will freeze on Drop.

### Spawn-closure wiring

Each of the four RPC handlers (push, pull, pull_sync,
delegated_pull) already has a spawn closure that calls
`record_outcome` near the end. The c-3 addition:

```rust
let events_tx = self.events_tx();   // clone at dispatch site
tokio::spawn(async move {
    // ... handler runs ...
    job.record_outcome(ok, err_msg.clone());
    let _ = events_tx.send(build_transfer_finished_event(
        &job, ok, err_msg.as_deref(),
    ));
    drop(job);
    // ...
});
```

The terminal-event emit happens AFTER `record_outcome` (so the
row's outcome is committed) but BEFORE `drop(job)` (so the
guard is still alive to read its byte counter and start time).

### `ActiveJobGuard` accessors

Two new methods on the guard so the builder can read snapshot
data without locking the table:

- `bytes_completed_load()` — Relaxed load of the per-row
  atomic.
- `elapsed_ms()` — `unix_ms_now() - start_unix_ms` saturating
  at zero on backwards-clock jump.

Both follow the same posture as `start_unix_ms()` from c-2:
expose the data the builder needs, no table lock needed.

## Files changed

- `proto/blit.proto`:
  - `+message TransferComplete`.
  - `+message TransferError`.
  - `DaemonEvent.oneof` grew `transfer_complete=3`,
    `transfer_error=4`. `reserved 2, 5, 6;`.
- `crates/blit-daemon/src/active_jobs.rs`:
  - `+ActiveJobGuard::bytes_completed_load()`.
  - `+ActiveJobGuard::elapsed_ms()`.
- `crates/blit-daemon/src/service/core.rs`:
  - `+BlitService::events_tx()` accessor (clone of the
    broadcast Sender).
  - `+build_transfer_finished_event(&guard, ok, error_message)`
    free function.
  - Four spawn closures clone `events_tx` at dispatch and emit
    the terminal event after `record_outcome`.
  - Existing subscribe tests grew non-exhaustive-match guards
    (`other => panic!(...)`) on `daemon_event::Payload` since
    the oneof now has multiple variants.

## Tests added

2 new in `service::core::tests`:

- `build_transfer_finished_event_ok_emits_transfer_complete`
  — register row, report bytes via sink, build event with
  ok=true, assert TransferComplete shape (transfer_id, bytes,
  files=0, duration_ms, tcp_fallback_used=false).
- `build_transfer_finished_event_err_emits_transfer_error`
  — build event with ok=false + an error string, assert
  TransferError shape.

End-to-end "subscribe → fire transfer → see Started then
Complete on stream" would require driving a real RPC through
the in-process tonic server. The existing
`subscribe_delivers_transfer_started_event_to_subscriber`
test exercises the broadcast path; the c-3 builder is unit-
tested in isolation. A follow-up slice that exercises the
full Started→Complete pair through a real spawn closure is
out of scope here.

Workspace: 561 passing serially (was 559; +2).

## Out of scope (next slices)

- **c-4-transfer-progress**: periodic `TransferProgress`
  events fired off a tokio interval per active transfer.
  Needs the throughput EWMA work too.
- **c-5-event-ring + transfer_id_filter**: per-job event ring
  (m-jobs-4 deferral) so a Subscribe(transfer_id=X) can
  replay recent events for X on connect.
- **c-6-jobs-watch-stream**: upgrade `blit jobs watch` from
  GetState polling to Subscribe stream.
- **c-7-module-and-heartbeat**: `ModuleListChanged`,
  `DaemonHeartbeat`. Lower priority.
- **Wire `files_completed` and `tcp_fallback_used`**: today
  TransferComplete carries 0 / false for these. files needs
  the files-counter slice; tcp_fallback_used needs the
  handler's result struct to expose the bit.

## Known gaps

1. **`TransferComplete.files` is always 0.** No files counter
   on the row today. The wire shape is correct so future
   subscribers don't need a proto roll, but the rendered
   value will be unhelpful until a follow-up slice wires it.

2. **`TransferComplete.tcp_fallback_used` is always false.**
   The data plane knows this bit (it's already on
   `DelegatedPullSummary.tcp_fallback_used`), but it doesn't
   flow back through the handler result type the spawn
   closure sees. A follow-up slice plumbs it.

3. **Cancelled-via-CancelJob and client-hangup paths emit
   `TransferError`.** The delegated_pull spawn closure maps
   `outcome = None` to `(ok=false, message=...)`, so a
   cancellation surfaces on the wire as TransferError rather
   than a dedicated TransferCancelled variant. That matches
   the current `TransferRecord.ok=false` shape for the same
   row. A future slice could split it if operator tools want
   to distinguish — out of scope here.

4. **No end-to-end integration test.** The builder is unit-
   tested; the spawn-closure emit isn't covered by a test that
   drives a real RPC through the in-process server. The
   existing daemon integration tests (`remote_remote.rs`)
   exercise the spawn closures and would catch a panic, but
   don't subscribe-and-observe the events. Adding a real
   end-to-end test requires standing up a tonic server +
   subscribing in the same process; deferred.

## Round 2 (sha `7d4fd28`)

Reviewer caught a real ordering bug: round 1 broadcast the
terminal event while the `ActiveJobGuard` and metrics gauges
were still alive (and, for delegated_pull, before
`inc_error()` incremented). A subscriber that received
`TransferComplete` / `TransferError` and immediately
refreshed GetState could observe the transfer still in
`active[]`, missing from `recent[]`, or with stale
`counters.transfer_errors_total` / `counters.active_transfers`.

That contradicts the terminal-event contract — the event is
supposed to signal reconcilable state.

Fix: in each of the four spawn closures, **build → drain →
broadcast**. Concretely:

```rust
// 1. Build the event while the guard is alive (we still
//    need its byte counter + start_unix_ms).
let finished_event = build_transfer_finished_event(&job, ok, err);

// 2. Drain the daemon bookkeeping.
drop(job);              // active row → recent ring
drop(guard);            // active-transfers gauge -1
if matches!(outcome, Some(false)) {
    metrics_for_log.inc_error();   // delegated_pull only
}

// 3. Broadcast. Subscribers can now race GetState and see
//    drained state.
let _ = events_tx.send(finished_event);
```

Applied identically at push (lines ~333-340), pull (~395-403),
pull_sync (~454-463), and delegated_pull (~625-650). The
delegated_pull path also moves `inc_error()` ahead of the
broadcast.

Coverage:

- `+terminal_event_observable_only_after_active_row_drained`
  asserts the ordering invariant. Replays the dispatch-site
  build→drain→broadcast sequence directly (the spawn
  closures themselves are anonymous and can't be unit-tested
  in isolation). Subscriber's `next()` resolves only after
  `events_tx.send(event)` returns, and at that point the
  active-jobs table is empty + recent ring carries the row.

Workspace: 562 passing serially (was 561; +1).

## Reviewer comments

(empty — pending grade)
