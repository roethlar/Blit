Reviewed sha: `7a5e7a3c5ee0ac47fc1e19a302c4f7cb80e53997`

# Reopened: m2f-5-f2-fanout

## Findings

1. `crates/blit-tui/src/main.rs:1449` — F2 `r` does not re-fan while a stream is live.

   The finding doc says F2 refresh is one of the re-fan triggers and should pick up daemons discovered since the last setup. The live-stream branch still runs `refresh_via_get_state` only for `parsed_remote`, so a normal launch that starts with only `parsed_remote` subscribed will not subscribe to later-discovered daemons when the operator presses `r`. This leaves the multi-daemon view single-daemon until some other path tears down the stream.

   Expected: `r` should restart the merged setup against `f2_watched_endpoints(app)` when the watch set may have changed, with an accompanying unit test that proves a live receiver plus newly discovered daemon schedules a new setup instead of only querying `parsed_remote`.

2. `crates/blit-tui/src/main.rs:1127` — one stream error drops the whole merged receiver.

   In the fan-out topology, `EventOrError::Error` represents one daemon forwarder ending. The select arm sets `transfers_event_rx = None`, which drops the merged receiver and causes healthy forwarders for other daemons to stop sending too. That contradicts the slice contract that other subscribed daemons still show and that the merged receiver closes only when every watched stream ends.

   Expected: mark the status degraded for that daemon/error, but keep the merged receiver alive so remaining daemon streams continue to feed F2. Let the `None` case handle the all-senders-closed condition. Add a focused test for an error followed by an event from another daemon.

## Gates

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (584 passed)
