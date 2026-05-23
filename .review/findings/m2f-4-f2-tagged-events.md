# m2f-4-f2-tagged-events: carry the source daemon per F2 stream event

**Severity**: Feature (multi-daemon F2 event-loop foundation)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `8979ff2`

## What

Fourth sub-slice of multi-daemon F2. The Subscribe forwarder now
stamps every signal with the daemon it came from
(`F2Event { daemon, kind }`), so the F2 event loop tags rows from
the **event's** daemon rather than the single global
`f2_source_label`. This is the per-stream identity m2f-5's fan-out
(one forwarder per discovered daemon, all feeding one channel)
needs — each forwarder stamps its own daemon, and the merged
consumer routes by it.

## Why behavior-preserving (single daemon)

Today there's one stream (`parsed_remote`), so
`event.daemon == endpoint.host_port_display() == f2_source_label` —
identical state. The change is purely *where* the identity comes
from (per-event from the stream vs. computed at the apply site).

## Approach

- `F2Event { daemon: String, kind: EventOrError }`.
- `open_subscribe_stream` tags with `endpoint.host_port_display()`
  (matches the `row_key` daemon component, the reset label, and the
  F2 header); `forward_subscribe_stream` takes the daemon and stamps
  each `message()`.
- The channel, `F2SetupPayload::Ready.event_rx`, and
  `transfers_event_rx` all carry `F2Event`.
- The `select!` arm + `drain_startup_events` apply with
  `event.daemon` (drain drops its now-redundant `source_daemon`
  param).
- The per-setup `Option<Receiver<F2Event>>` topology is unchanged —
  the shared merged channel + N forwarders land in m2f-5.

## Files changed

- `crates/blit-tui/src/main.rs`: `F2Event`; tagging in
  `open_subscribe_stream` / `forward_subscribe_stream`;
  `F2SetupPayload` + `transfers_event_rx` type; `select!` arm +
  `drain_startup_events` use `event.daemon`; test-channel updates;
  1 test.

## Tests

583 total (+1): `drain_startup_events_tags_row_with_event_daemon` —
an Event tagged `skippy:9001` produces a row with
`source_daemon == "skippy:9001"`, proving the per-stream identity
flows through (independent of `parsed_remote`). Existing F2 setup /
drain / reset tests pass with the `F2Event` channel.

## Multi-daemon F2 sub-slice plan

- m2f-1 ✓ source_daemon · m2f-2 ✓ composite key · m2f-3 ✓ merge_snapshot.
- **m2f-4 (this):** per-event daemon tag.
- **m2f-5:** persistent merged channel + a Subscribe forwarder per
  discovered daemon (each `merge_snapshot` + stream) + render the
  source-daemon column.
- **m2f-6:** dynamic discovery + per-daemon reconnect; multi-daemon
  cancel.

## Reviewer comments

(empty — pending grade)
