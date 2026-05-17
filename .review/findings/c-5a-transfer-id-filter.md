# c-5a-transfer-id-filter: SubscribeRequest.transfer_id_filter

**Severity**: Feature (Subscribe filter — unblocks c-6 jobs-watch streaming upgrade)
**Status**: In progress / pending review
**Branch**: `phase5/c`
**Commit**: filled by the sentinel commit

## What

Adds the `transfer_id_filter` field to `SubscribeRequest` (tag
3) and wires daemon-side filtering into the Subscribe handler.
Subscribers tracking a single transfer no longer pay the
wire/CPU cost of every other transfer's events.

This is the c-5 m-jobs deferral piece that c-2 reserved tag 3
for. The other half of c-5 — per-job event ring for
replay-on-connect — defers to a follow-up (c-5b-event-ring).

## Why daemon-side filtering

Client-side filtering would work for a small number of
parallel transfers, but at high event rates (10 Hz × N active
rows) a `jobs watch <id>` consumer would have to discard
N×10 events/sec just to surface its one transfer. Daemon-side
filtering reduces that to ~10 events/sec for the one transfer
the client cares about, with zero broadcast traffic to other
events. The TUI's F2 pane will also benefit when watching a
specific transfer (Ctrl-click "follow this transfer").

## Approach

### Proto

```protobuf
message SubscribeRequest {
  uint32 event_mask = 1;       // reserved for future bitfield
  reserved 2;                  // replay_recent (c-5b)
  string transfer_id_filter = 3;
}
```

Empty filter retains today's "every event" behavior.

### Daemon-side filter

```rust
pub(crate) fn event_matches_filter(event: &DaemonEvent, filter: &str) -> bool {
    if filter.is_empty() { return true; }
    match event.payload.as_ref() {
        Some(daemon_event::Payload::TransferStarted(e)) => e.transfer_id == filter,
        Some(daemon_event::Payload::TransferProgress(e)) => e.transfer_id == filter,
        Some(daemon_event::Payload::TransferComplete(e)) => e.transfer_id == filter,
        Some(daemon_event::Payload::TransferError(e)) => e.transfer_id == filter,
        None => false,
    }
}
```

Exhaustive over current variants — adding a future non-
transfer-scoped variant (ModuleListChanged, DaemonHeartbeat)
forces an explicit decision about how the filter treats it.
Today the design doc says daemon-wide events bypass per-job
filters, but locking that in is a future-slice choice rather
than a hidden default.

### Subscribe handler

```rust
let transfer_id_filter = req.transfer_id_filter;  // owned, moved into closure
let stream = BroadcastStream::new(rx).filter_map(move |item| match item {
    Ok(event) if event_matches_filter(&event, &transfer_id_filter) => Some(Ok(event)),
    Ok(_) => None,  // filtered out
    Err(Lagged(n)) => Some(Err(Status::aborted(...))),
});
```

`filter_map` replaces `map` so filtered events don't yield a
frame at all (no empty-frame protocol cost). Lagged still
surfaces as `Status::aborted` so subscribers handle the slow-
consumer case identically.

## Files changed

- `proto/blit.proto`:
  - `SubscribeRequest.transfer_id_filter = 3` (was reserved).
  - `reserved 2;` stays for future `replay_recent`.
- `crates/blit-daemon/src/service/core.rs`:
  - `+event_matches_filter(event, filter)` free function.
  - Subscribe handler now captures the filter into a
    `filter_map` over the BroadcastStream.
  - Existing test sites populate the new field
    (`SubscribeRequest { event_mask: 0, transfer_id_filter: String::new() }`).

## Tests added

3 new in `service::core::tests`:

- `event_matches_filter_empty_filter_accepts_everything` —
  empty filter accepts any TransferStarted.
- `event_matches_filter_matches_only_target_transfer` —
  two transfer ids; assert filter acceptance is symmetric
  across the (event, filter) pairs; covers TransferStarted
  and TransferProgress variants.
- `subscribe_with_transfer_id_filter_drops_other_transfer_events`
  — end-to-end via the real subscribe handler. Subscribe
  with filter=id_a, fire events for id_a and id_b, assert
  only id_a reaches the stream and a timeout fires waiting
  for the (filtered) id_b frame.

Workspace: 569 passing serially (was 566; +3).

## Out of scope (next slices)

- **c-5b-event-ring**: per-job event ring for replay-on-
  connect. `SubscribeRequest.replay_recent = 2` (still
  reserved). Lets a subscriber that joins mid-transfer pick
  up the bytes-completed history of an in-flight job without
  missing the early progress.
- **c-6-jobs-watch-stream**: upgrade `blit jobs watch` from
  GetState polling to Subscribe streaming. Uses the filter
  added here.
- **c-7-module-and-heartbeat**: ModuleListChanged,
  DaemonHeartbeat event variants. Will trigger the explicit
  "non-transfer-scoped events bypass the filter" decision in
  `event_matches_filter`.

## Known gaps

1. **No per-job event ring (replay).** A subscriber that
   connects after the first events for transfer X fired
   doesn't see them — even with the filter set. Mid-transfer
   joiners must combine Subscribe (forward-looking) with
   GetState (current state). c-5b's event ring solves this.

2. **Filter is exact string match.** No glob / regex / prefix
   matching. Matches the design-doc spec; future expansion
   would need a separate wire field.

3. **`SubscribeRequest.event_mask` still parsed-and-ignored.**
   Same posture as c-2 — locked tag, no producer yet.

## Round 2 (sha `7587b46`)

Reviewer caught a real bug: round 1 wrapped the broadcast
Receiver in `BroadcastStream::filter_map`, which is lazy.
The Receiver's cursor still advanced through every daemon
event — so a `jobs watch <id>` consumer could be aborted
with `Status::aborted("subscriber lagged ...")` when
unrelated transfers' events overflowed the 256-slot global
broadcast ring, even though the filter rejected those events
anyway. The feature was nominally implemented but didn't
deliver its load-bearing semantic.

Fix: per-subscriber forwarder task.

- Spawned inside the `subscribe` handler before the response
  returns.
- Eagerly `recv()`s on the broadcast Receiver — cursor stays
  caught up independent of client read pace.
- Applies `event_matches_filter` to each event.
- Forwards only matching events into a bounded
  `mpsc::channel<Result<DaemonEvent, Status>>` of capacity
  `SUBSCRIBE_MPSC_CAPACITY = 64`.
- The mpsc receiver is what tonic streams to the client.

Lagged semantics now correctly distinguish two cases:

- **Forwarder can't keep up with daemon-side event rate**
  (`broadcast::error::RecvError::Lagged`) — emits
  `Status::aborted` and the stream ends. Daemon-side CPU
  issue; rare in practice.
- **Client too slow on filtered subset** — backs up the mpsc
  first. Forwarder's `send().await` blocks; while it's
  blocked, the broadcast cursor stalls; eventually broadcast
  over-capacity fires Lagged through the normal path. The
  "really too slow" signal still surfaces.

+1 regression test that reproduces the reviewer's scenario:

- `filtered_subscriber_survives_overflow_of_other_transfer_events`
  emits `SUBSCRIBE_BROADCAST_CAPACITY + 50` events for `id_b`
  while yielding periodically (so the forwarder gets
  airtime), then emits one for `id_a`. The filtered
  subscriber receives `id_a`, not `Status::aborted`.
  Uses `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`
  so the forwarder runs in parallel with the emit loop;
  under the default `current_thread` runtime a tight sync
  emit loop would starve the forwarder regardless of design.

Workspace: 570 passing serially (was 569; +1).

## Reviewer comments

(empty — pending grade)
