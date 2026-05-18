# d-23-cancel-status-auto-clear: TTL on cancel fragment

**Severity**: Feature (polish — closes a d-22 known gap)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

The d-22 cancel fragment (`cancelled abc-123`,
`cancel: id … not found`, etc.) stayed in the F2
footer indefinitely until the operator triggered
another `K`. d-23 adds a 5-second TTL — the fragment
auto-hides after the terminal state has been on
screen long enough for the operator to read it,
returning the footer to its clean default state.

`Sending` has no TTL — it stays until the RPC reply
lands. Only Done / Error variants expire.

## Approach

### State

`F2CancelStatus::Done` and `::Error` gain a
`finished_at: Instant` field. The reply arm stamps
`Instant::now()` when the cancel reply lands.

```rust
F2CancelStatus::Done { outcome, finished_at },
F2CancelStatus::Error { transfer_id, message, finished_at },
```

### TTL constant

```rust
const CANCEL_STATUS_TTL: Duration = Duration::from_secs(5);
```

Long enough to read a 3-word message, short enough not
to clutter the footer indefinitely.

### Conversion

`cancel_status_to_display` now takes `now: Instant`. For
Done/Error variants:

```rust
if now.saturating_duration_since(*finished_at) >= CANCEL_STATUS_TTL {
    return F2CancelDisplay::Hidden;
}
```

The state itself isn't mutated — the renderer just stops
showing it. A future `K` press will overwrite the state
with a fresh Sending → Done cycle. The renderer-side
expiry keeps state pure and avoids a "transition timer"
on the event loop.

### Live tick interaction

The F2 footer is already in the d-9/d-13 live-tick gate
via `last_event_at`. A cancel implies prior Subscribe
activity (you can only cancel a transfer the operator
saw start), so `last_event_at` is `Some` and the F2
footer ticks every `live_tick.interval_ms`. The TTL
expiry boundary lands within one tick of the actual
5-second deadline.

## Files changed

- `crates/blit-tui/src/main.rs`:
  - `F2CancelStatus::Done` + `::Error` gain
    `finished_at: Instant`.
  - `CANCEL_STATUS_TTL` constant (5s).
  - Reply arm stamps `finished_at = Instant::now()`.
  - `cancel_status_to_display` gains `now: Instant`
    parameter; Done/Error past TTL return Hidden.
  - Draw call site passes `now`.
- `crates/blit-tui/src/screens/f2.rs`:
  - Module-doc layout sketch mentions d-23 in the
    "polish" list + adds a sentence about TTL.

## Tests

+6 unit tests (243 → 249):

In `main::tests`:
- `cancel_status_idle_renders_hidden`
- `cancel_status_sending_renders_sending_regardless_of_time`
  — Sending has no TTL.
- `cancel_status_done_within_ttl_renders_terminal_variant`
- `cancel_status_done_past_ttl_renders_hidden`
- `cancel_status_error_past_ttl_renders_hidden`
- `cancel_status_done_exactly_at_ttl_renders_hidden` —
  boundary is `>=`, picks "hidden" on the dot for
  less clutter.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No config-tunable TTL.** Hardcoded 5s. A future
   polish could add `[transfer] cancel_status_ttl_ms`
   for operators who want more / less time to read.

2. **TTL is per-render, not per-event.** The state
   keeps the Done variant after the TTL elapses — only
   the display hides. A second action that introspects
   `app.cancel_status` would still see the Done state.
   In practice nothing does, but worth noting.

## Out of scope (next slices)

- **Config-tunable TTL.**
- **Cancel confirmation prompt.**
- **Batch cancel (Shift-K).**

## Reviewer comments

(empty — pending grade)
