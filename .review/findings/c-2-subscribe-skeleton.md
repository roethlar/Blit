# c-2-subscribe-skeleton: Subscribe RPC + DaemonEvent + TransferStarted

**Severity**: Feature (headline slice of milestone C — first cut of the streaming wire surface)
**Status**: In progress / pending review
**Branch**: `phase5/c`
**Commit**: filled by the sentinel commit

## What

First atomic slice of milestone C's `Subscribe` deliverable
(`docs/plan/TUI_DESIGN.md` §6.2). Lands the wire surface and the
daemon-side broadcast plumbing with a single event variant
(`TransferStarted`). Subsequent C sub-slices fan more variants
into the oneof and add the request-side filter fields.

## Why ship just `TransferStarted` first

The design doc enumerates six event variants
(`TransferStarted`, `TransferProgress`, `TransferComplete`,
`TransferError`, `ModuleListChanged`, `DaemonHeartbeat`). Each
has its own producer plumbing and semantics. Slicing them lets
the reviewer audit:

1. The wire-shape choices (oneof field-number stability,
   reserved tags) in isolation.
2. The broadcast channel sizing and the slow-consumer policy
   (Lagged → `Status::aborted`).
3. The dispatch-site emit ordering at four call sites.

without conflating with the question of "is TransferProgress's
EWMA cadence right" or "do we send DaemonHeartbeat on a timer
or a transfer-rate gate."

`TransferStarted` is the first variant because it has the
simplest semantics (fire once per transfer at the same site
that registers the `ActiveJob`) and it's also the variant that
provides the most immediate operator value — the TUI's F2
Transfers pane can now see new transfers arriving live, without
polling GetState.

## Approach

### Proto wire shape

```protobuf
service Blit {
  // ...
  rpc Subscribe(SubscribeRequest) returns (stream DaemonEvent);
}

message SubscribeRequest {
  uint32 event_mask = 1;   // reserved for future filtering
  reserved 2, 3;           // transfer_id_filter, replay_recent
}

message DaemonEvent {
  oneof payload {
    TransferStarted transfer_started = 1;
  }
  reserved 2 to 6;         // remaining event variants
}

message TransferStarted {
  string transfer_id = 1;
  TransferKind kind = 2;   // reuses the top-level enum
  string peer = 3;
  string module = 4;
  string path = 5;
  uint64 start_unix_ms = 6;
}
```

`TransferKind` is reused from `GetState.active[].kind` so the
wire enum stays single-sourced.

Reserved field numbers in `DaemonEvent.payload` and
`SubscribeRequest` lock the tags now so future slices don't
accidentally pick a number that a peer of a different release
expected to mean something else.

### Daemon-side broadcast

```rust
struct BlitService {
    // ...
    events_tx: tokio::sync::broadcast::Sender<DaemonEvent>,
}
```

Capacity is `SUBSCRIBE_BROADCAST_CAPACITY = 256` — enough that
a momentary subscriber stall (1-2s of activity at a few events
per transfer) doesn't immediately drop events. Subscribers that
fall further behind that get a `tonic::Status::aborted` and
re-subscribe.

### Subscribe handler

```rust
async fn subscribe(...) -> Result<Response<Self::SubscribeStream>, Status> {
    let rx = self.events_tx.subscribe();   // BEFORE returning
    let stream = BroadcastStream::new(rx).map(|item| match item {
        Ok(event) => Ok(event),
        Err(BroadcastStreamRecvError::Lagged(n)) => Err(Status::aborted(
            format!("subscriber lagged {n} events; re-subscribe and refresh via GetState")
        )),
    });
    Ok(Response::new(Box::pin(stream)))
}
```

The `events_tx.subscribe()` call runs BEFORE the response is
returned so any event emitted between this call and the
client's first `.next()` lands in the broadcast queue rather
than being dropped.

### Emit at dispatch sites

`BlitService::emit_transfer_started(guard, kind, peer, module, path)`
helper centralizes the event-building. Called from each of the
four dispatch sites in `service/core.rs`:

| RPC          | module/path source            | empty at register time? |
|--------------|-------------------------------|--------------------------|
| `push`       | first stream frame (header)   | yes — emits empty        |
| `pull`       | `PullRequest` (synchronous)   | no                       |
| `pull_sync`  | first stream frame (spec)     | yes — emits empty        |
| `delegated_pull` | `DelegatedPullRequest` (sync) | no                       |

Streaming RPCs fire `TransferStarted` with empty module/path —
this matches `GetState.active[]`'s view of the same row at
registration time (it stays empty until the handler calls
`ActiveJobGuard::set_endpoint`). A subscriber that wants the
populated endpoint queries `GetState` once or waits for a
future "endpoint resolved" event family member.

### `ActiveJobGuard.start_unix_ms` accessor

Subscribers need the same `start_unix_ms` that
`GetState.active[].start_unix_ms` will surface. The value is
stamped once in `ActiveJobs::register` and stored inside the
locked table; reading it via a snapshot requires a table lock
on every event emit. We instead store a copy on the guard so
`emit_transfer_started` can read it lock-free.

### `tokio-stream` feature

`tokio-stream`'s `sync` feature added to `blit-daemon/Cargo.toml`
to enable `wrappers::BroadcastStream`. The dep was already
present; only the feature flag changed.

## Files changed

- `proto/blit.proto`:
  - `+rpc Subscribe`.
  - `+message SubscribeRequest` (event_mask field + reserved tags).
  - `+message DaemonEvent` (oneof + reserved tags).
  - `+message TransferStarted`.
- `crates/blit-daemon/Cargo.toml`:
  - Add `sync` feature to `tokio-stream`.
- `crates/blit-daemon/src/active_jobs.rs`:
  - `+ActiveJobGuard.start_unix_ms` field (populated at
    register time).
  - `+ActiveJobGuard::start_unix_ms()` accessor.
- `crates/blit-daemon/src/service/core.rs`:
  - `+const SUBSCRIBE_BROADCAST_CAPACITY = 256`.
  - `+BlitService.events_tx: broadcast::Sender<DaemonEvent>`.
  - `+BlitService::emit_transfer_started(guard, kind, peer, module, path)`.
  - `+type SubscribeStream` + `async fn subscribe` on the
    `Blit` trait impl.
  - Four dispatch sites updated to call `emit_transfer_started`
    after `register`.
- `crates/blit-cli/tests/remote_remote.rs`:
  - Two `Blit` test stub impls grew `type SubscribeStream` +
    `fn subscribe` returning `Status::unimplemented`.
- `crates/blit-core/tests/pull_sync_with_spec_wire.rs`:
  - Same addition for the `SpyServer` stub.

## Tests added

3 new in `service::core::tests`:

- `subscribe_delivers_transfer_started_event_to_subscriber` —
  subscribe → fire one TransferStarted → assert the frame
  contents (transfer_id, kind, peer, module, path,
  start_unix_ms>0) on the stream.
- `subscribe_delivers_to_multiple_subscribers` — two parallel
  subscribers see the same event with the same transfer_id.
- `subscribe_drops_event_silently_when_no_subscribers` —
  firing with no subscribers attached doesn't panic; the
  active-row drain semantics are unaffected.

Workspace: 559 passing serially (was 556; +3).

## Out of scope (next slices)

- **c-3-transfer-progress-event**: periodic `TransferProgress`
  events fired off a tokio interval timer per active transfer,
  carrying `bytes_completed` + throughput EWMA (the latter is
  itself a sub-slice).
- **c-4-transfer-complete-event**: fire `TransferComplete` at
  the same site that calls `record_outcome` so subscribers see
  a terminal frame per transfer.
- **c-5-transfer-error-event**: fire on the error branch
  (parallel to TransferComplete).
- **c-6-event-ring**: per-job event ring (m-jobs-4 deferral) +
  `SubscribeRequest.transfer_id_filter` (m-jobs-5 deferral).
  Lets a Subscribe(transfer_id=X) replay the last N events for
  X on connect.
- **c-7-jobs-watch-stream**: upgrade `blit jobs watch` from
  GetState polling to Subscribe stream.
- **c-8-module-and-heartbeat**: `ModuleListChanged`,
  `DaemonHeartbeat` event variants (lower priority — TUI's F1
  pane refreshes from GetState).
- **bytes_total / files_completed / files** on
  `GetState.active[]` and `TransferRecord` — wired from the
  manifest stage in a separate slice.

## Known gaps

1. **Streaming RPCs fire `TransferStarted` with empty
   module/path.** That matches the corresponding
   `GetState.active[]` row at registration time (the field is
   populated by `set_endpoint` once the first stream frame
   parses). Subscribers that need the populated endpoint can
   query GetState; a future event variant ("endpoint resolved")
   could fire from `set_endpoint` itself, but the operator
   value is small.

2. **No replay-on-connect.** A subscriber that joins after a
   transfer started doesn't see its `TransferStarted` event.
   `SubscribeRequest.replay_recent` (reserved field 3) gates a
   future replay-buffer mechanism; not in this slice.

3. **No backpressure on slow producers.** The broadcast channel
   has capacity 256 events. If transfers fire faster than
   subscribers consume, the oldest events drop with a Lagged
   notification to the slow subscriber; the producer
   (`emit_transfer_started`) never blocks. This is the
   intended design — TUI re-fetches via GetState on Lagged.

4. **`SubscribeRequest.event_mask` is parsed and ignored.** No
   event filtering yet (only one variant exists). The field
   tag is locked so future filtering can use it without a
   wire-shape change.

## Reviewer comments

(empty — pending grade)
