# audit-8-tui-task-leak: TUI subscribe forwarder task leak on reconnect

**Severity**: Robustness
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `c003bb3`

## What

Ground-up audit found a task leak in the TUI's Subscribe stream management.

**`crates/blit-tui/src/main.rs:6120`** — `open_subscribe_stream` spawns a
`forward_subscribe_stream` task for each watched daemon. The forwarder loops on
`stream.message().await` and forwards events into `merged_tx`.

**`crates/blit-tui/src/main.rs:2216`** — On re-fan (manual `r` refresh or
mDNS-driven discovery change), `refan_f2_setup` drops the old merged receiver
(`*transfers_event_rx = None`). The comment says "per-daemon forwarders exit when
the receiver is gone," but this is only true for forwarders that are actively
sending. A forwarder blocked on `stream.message().await` never polls the channel
again, so it never discovers the receiver was dropped. The ghost task hangs
indefinitely.

**`crates/blit-tui/src/main.rs:6105`** — `jobs::subscribe(endpoint, ...).await`
has no timeout on `BlitClient::connect` or the `subscribe` RPC. If a daemon is
unreachable, the entire `spawn_f2_setup_task` can hang for OS TCP timeout
duration (60-180s). Old setup tasks are never explicitly aborted.

Over a long TUI session with frequent reconnects, leaked forwarder tasks
accumulate. Each leaked task holds an open HTTP/2 connection, a broadcast
Receiver, and an mpsc slot.

## Approach

1. Store `AbortHandle`s for each spawned forwarder and the setup task itself.
   On re-fan, explicitly abort the old tasks before dropping the receiver.

2. Add `tokio::time::timeout` to `jobs::subscribe` (or a wrapper), covering both
   `connect()` and the initial `subscribe()` RPC.

3. Alternatively, add an idle timeout inside `forward_subscribe_stream` so a
   forwarder that hasn't seen an event in N seconds wakes up and checks whether
   the receiver is still alive.

## Files changed

TBD by coder. Primarily `crates/blit-tui/src/main.rs` and optionally
`crates/blit-app/src/admin/jobs.rs` for the subscribe timeout wrapper.

## Tests

- Simulate a re-fan while a forwarder is mid-await on a slow stream; verify
  the old task exits promptly (via abort handle or idle timeout).
- Simulate an unreachable daemon during setup; verify the setup task times out
  instead of hanging indefinitely.

## Resolution (commit `c003bb3`)

Chose the receiver-close-race approach (the finding's option 3) over
AbortHandle bookkeeping (option 1). AbortHandles would have to be
threaded `open_subscribe_stream` → `spawn_f2_setup_task` → app state →
`refan_f2_setup` across an 11K-line file; the close race is localized to
two adjacent functions and is strictly more robust (the forwarder
self-terminates the instant its receiver vanishes, by any cause).

**1 — Forwarder leak (the headline).** Extracted `forward_step(next_msg,
tx)` which `tokio::select!`s (biased) the next-message future against
`tx.closed()`; `forward_subscribe_stream` loops over it. When an F2
re-fan drops the merged receiver, `tx.closed()` resolves immediately, so
a forwarder watching a silent daemon exits at once instead of parking on
`stream.message().await` forever. `forward_step` is generic over the
message future so the invariant is unit-testable (a real
`tonic::Streaming` can't be built off the wire; `futures` is not a
blit-tui dep, so `std::future` stand-ins are used).

**2 — Unbounded Subscribe open.** `jobs::subscribe`'s connect is already
bounded by `connect_with_timeout` (audit-2a), but the Subscribe RPC was
not. Wrapped the whole `jobs::subscribe(...)` call in
`open_subscribe_stream` in an OUTER `tokio::time::timeout`
(`SUBSCRIBE_OPEN_TIMEOUT = 30s`) — the `feedback-server-await-timeouts`
lesson that an inner connect_timeout alone doesn't bound the RPC (or slow
DNS). Did NOT modify `jobs.rs` (the call-site wrap is sufficient and
keeps the change localized).

## Files changed

- `crates/blit-tui/src/main.rs`: `ForwardStep` enum + `forward_step`
  helper; `forward_subscribe_stream` loops over it; `SUBSCRIBE_OPEN_TIMEOUT`
  + outer timeout in `open_subscribe_stream`; 2 tests.

## Tests added

`blit-tui` +2: `forward_step_exits_when_receiver_dropped_even_if_message_pending`
(dropped receiver + pending message → `Closed`),
`forward_step_yields_a_ready_message_while_receiver_live`. The
subscribe-open timeout is integration-shaped (needs a hung daemon) — not
unit-tested; the wrap is a standard `tokio::time::timeout`. Full
workspace gate green.

## Reviewer comments

(empty — pending review)
