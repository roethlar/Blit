# c-5b-event-ring: per-job event ring + `SubscribeRequest.replay_recent`

**Severity**: Feature (last m-jobs deferral — late-joining subscribers see in-flight history)
**Status**: In progress / pending review
**Branch**: `phase5/c`
**Commit**: filled by the sentinel commit

## What

Adds a bounded per-row event ring to each `ActiveJob` and
unreserves `SubscribeRequest.replay_recent` (tag 2). A
subscriber that joins after a transfer has started — with
`replay_recent: true, transfer_id_filter: id` — receives the
row's recent events (Started, Progress, …) before live
broadcast events start flowing.

This closes the m-jobs-4 + m-jobs-5 deferrals tracked through
all of milestone C. With c-5b landed, the Subscribe wire
surface is complete: filter, replay, and live forwarding all
work end-to-end.

## Why this is the right place to stop

The original m-jobs design listed the per-job event ring +
`replay_recent` as a single load-bearing piece. Splitting it
into c-5a (filter) and c-5b (ring) let the reviewer scrutinize
each contract in isolation:

- c-5a: the filter semantic — daemon-side, doesn't lag on
  unrelated events, doesn't leak forwarders on disconnect.
- c-5b: the replay semantic — bounded ring, exactly-once
  delivery (no replay+broadcast duplicate, no missed-event
  gap), terminal events explicitly NOT replayed.

c-5b can land standalone because its only consumer (TUI
joining mid-transfer) isn't built yet. Future TUI work in A.1
will exercise both flags together.

## Approach

### Proto

```protobuf
message SubscribeRequest {
  uint32 event_mask = 1;
  bool replay_recent = 2;       // c-5b
  string transfer_id_filter = 3;
}
```

No-op when `replay_recent` is set on its own (no specific row
to drain). No-op when the targeted row doesn't exist.

### Per-row ring

`TableEntry` gains:

```rust
events_ring: VecDeque<DaemonEvent>  // bounded by JOB_EVENT_RING_CAP (64)
```

Sized to comfortably hold one TransferStarted plus several
seconds of 10 Hz TransferProgress events — enough for a TUI
joining mid-transfer to render a coherent progress bar.

Terminal events (TransferComplete/TransferError)
**intentionally bypass the ring**. They're broadcast AFTER
row drain (c-3 round 2), at which point the row's ring is
gone with the row. Subscribers that join after row drain
fall back to `GetState.recent[]` (c-6's reconcile path).

### Lock-held emit + lock-held subscribe

The load-bearing ordering invariant: any event either appears
in the row's ring OR on the broadcast — never both, never
neither. Achieved by serializing all three operations
(emit-side: ring push + broadcast send; subscribe-side: ring
snapshot + Receiver registration) under the same table lock.

```rust
impl ActiveJobs {
    pub fn emit_event(
        &self,
        events_tx: &broadcast::Sender<DaemonEvent>,
        transfer_id: &str,
        event: DaemonEvent,
    ) {
        let mut table = self.inner.table.lock();
        if let Some(entry) = table.get_mut(transfer_id) {
            // bounded push
            if entry.events_ring.len() >= JOB_EVENT_RING_CAP {
                entry.events_ring.pop_front();
            }
            entry.events_ring.push_back(event.clone());
        }
        // Broadcast inside the lock — see below.
        let _ = events_tx.send(event);
    }

    pub fn subscribe_with_ring(
        &self,
        events_tx: &broadcast::Sender<DaemonEvent>,
        transfer_id_filter: &str,
        replay: bool,
    ) -> (broadcast::Receiver<DaemonEvent>, Vec<DaemonEvent>) {
        let table = self.inner.table.lock();
        let rx = events_tx.subscribe();
        let events = if replay && !transfer_id_filter.is_empty() {
            table.get(transfer_id_filter)
                .map(|e| e.events_ring.iter().cloned().collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        (rx, events)
    }
}
```

**Why broadcast inside the lock**: ordering. If subscribe
acquires the lock first (snapshot taken, Receiver
registered, lock released), subsequent emits add to the ring
AND broadcast — subscriber sees them once via broadcast. If
emit acquires the lock first (ring updated, broadcast sent,
lock released), the broadcast has already fired before
subscribe registers its Receiver — subscriber misses it via
broadcast but a later snapshot has it. Both cases:
exactly-once delivery.

`broadcast::Sender::send` is synchronous (pushes to an
in-memory ring; never awaits). Holding the std::sync::Mutex
across the send keeps the critical section short — bounded
by the active-row count.

### tick_progress emits through the ring too

`ActiveJobs::tick_progress_emit(events_tx, build_event)` —
sibling of `for_each_progress_sample` that also pushes each
built event into its row's ring + broadcasts, all under the
same lock. `tick_progress_once` in `service/core.rs` calls
this instead of the old closure-form.

### Subscribe handler

```rust
let (mut broadcast_rx, replay) = active_jobs.subscribe_with_ring(
    &self.events_tx,
    &transfer_id_filter,
    req.replay_recent,
);
// ...
tokio::spawn(async move {
    // Drain replay events first.
    for event in replay {
        if tx.send(Ok(event)).await.is_err() { return; }
    }
    // Then transition to live broadcast forwarding.
    loop {
        tokio::select! {
            biased;
            () = tx.closed() => break,
            recv = broadcast_rx.recv() => { /* ... */ }
        }
    }
});
```

Replay events are de-facto deduped against the broadcast
Receiver because the table lock serialized their landing.
The forwarder simply forwards both: replay vec first, then
broadcast — no overlap.

## Files changed

- `proto/blit.proto`:
  - `SubscribeRequest.replay_recent = 2` (was reserved).
- `crates/blit-app/src/admin/jobs.rs`:
  - Existing `subscribe()` helper extended to populate
    `replay_recent: false` in its built request (matches the
    current CLI default; c-6 doesn't set it yet).
- `crates/blit-daemon/src/active_jobs.rs`:
  - `+const JOB_EVENT_RING_CAP = 64`.
  - `+TableEntry.events_ring: VecDeque<DaemonEvent>`.
  - `+ActiveJobs::emit_event(events_tx, transfer_id, event)`.
  - `+ActiveJobs::subscribe_with_ring(events_tx, filter, replay)`.
  - `+ActiveJobs::tick_progress_emit(events_tx, build_event)`.
  - `for_each_progress_sample` gated on `#[cfg(test)]` (only
    test consumers remain after `tick_progress_once` moved
    to `tick_progress_emit`).
- `crates/blit-daemon/src/service/core.rs`:
  - `emit_transfer_started` routes through `emit_event`.
  - `tick_progress_once` routes through `tick_progress_emit`.
  - Subscribe handler uses `subscribe_with_ring` + drains
    `replay` to the mpsc before the broadcast loop.
- Existing `SubscribeRequest { ... }` constructors across
  the test surface (~10 sites) populate the new
  `replay_recent: false` field.

## Tests added

2 new in `service::core::tests`:

- `subscribe_replay_recent_replays_per_row_ring_to_late_joiner`
  — emit Started + 2 Progress events BEFORE subscribing,
  then subscribe with `replay_recent: true`, assert all
  three arrive in order over the stream.
- `subscribe_without_replay_recent_skips_ring` — confirms
  default `replay_recent: false` behavior (no replay; a
  100ms timeout fires with no frame received).

Both use `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`
so the per-subscriber forwarder gets real parallelism.

Workspace: 575 passing serially (was 573; +2).

## Known gaps

1. **Terminal events not replayed.** TransferComplete /
   TransferError fire after the row drains; the ring is
   gone by then. A subscriber that joins after row drain
   sees no events from the ring; their c-6 reconcile path
   queries `GetState.recent[]` to learn the terminal state.
   This is by design — replay is a "joined mid-flight"
   feature, not a post-mortem history.

2. **No across-restart persistence.** Restarting the daemon
   loses all rings (in-memory only). Same posture as the
   recent-runs ring and the ActiveJobs table — durability is
   a 0.2.0 design question (`TUI_DESIGN.md` §10).

3. **`SubscribeRequest.replay_recent` is no-op without
   `transfer_id_filter`.** A future variant could replay a
   global recent-event ring (across all rows), but no such
   ring exists. Documented in the proto comment.

4. **No CLI consumer of replay_recent.** c-6's
   `blit jobs watch` doesn't set `replay_recent: true`
   today — it does its own initial GetState snapshot for
   the "already finished" check. A TUI in A.1 will likely
   set replay_recent to get the in-flight history rendered
   immediately on the F2 Transfers pane. Worth wiring in
   `blit jobs watch` too as a follow-up, but the slot is
   live now.

## Out of scope (next slices)

- **c-7-module-and-heartbeat**: ModuleListChanged,
  DaemonHeartbeat event variants.
- **bytes_total / files counters** wired from manifest stage.
- **Throughput EWMA** smoothing.

## Reviewer comments

(empty — pending grade)
