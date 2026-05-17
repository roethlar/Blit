# c-5a-transfer-id-filter reopened

Verdict: Reopened
Reviewed sha: `6330a7d877455bd00eb21d864f1d94dbe8402e61`
Reviewer: `reviewer`
Timestamp: `2026-05-17T14:54:50Z`

Validation:

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: pass

## Findings

1. Medium - `transfer_id_filter` is applied after the subscriber has already lagged on unrelated events.

   The new subscribe path still creates a normal receiver on the global 256-slot broadcast ring (`crates/blit-daemon/src/service/core.rs:352`; capacity at `core.rs:42`) and only applies `event_matches_filter` inside the returned `BroadcastStream::filter_map` (`core.rs:366-380`). That means the receiver cursor still advances through every daemon event. If enough unrelated transfer events arrive before the filtered client is polled, `BroadcastStream` yields `Lagged(n)` at `core.rs:375-378` even if all missed events were for other transfer IDs that the client explicitly asked not to receive.

   This defeats the main reason for the feature as described in the finding doc: a `jobs watch <id>` stream should be scoped to that transfer and should not be aborted by high event volume from unrelated transfers. With the current lazy `filter_map`, the wire frames are filtered, but the subscriber still pays the global broadcast lag semantics and server-side scan cost for irrelevant events.

   Move the filter in front of the client-paced stream. One viable shape is a per-subscriber forwarding task: it eagerly drains the global broadcast receiver, applies `event_matches_filter`, and sends only matching events into a bounded `mpsc` stream returned to tonic. Lagged on the global receiver should only abort if the forwarder itself cannot keep up; unrelated filtered-out events should not fill the client-facing buffer or cause a filtered watcher to abort. Add a regression test that subscribes with `transfer_id_filter = A`, emits more than `SUBSCRIBE_BROADCAST_CAPACITY` events for `B`, then emits an event for `A`; the filtered subscriber should receive `A` rather than `Status::aborted`.
