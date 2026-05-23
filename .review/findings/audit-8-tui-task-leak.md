# audit-8-tui-task-leak: TUI subscribe forwarder task leak on reconnect

**Severity**: Robustness
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `2d7b6f7`

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

## Round 2 (commit `2d7b6f7`)

**Reopen finding:** the forwarder leak and Subscribe-open were fixed, but
`spawn_f2_setup_task` still awaited `jobs::query(endpoint, 0)` unbounded
right after a successful subscribe. `jobs::query` bounds its connect
(`connect_with_timeout`) but not the `GetState` RPC, so a daemon that
opened Subscribe then stalled `GetState` would hang the setup task —
`transfers_setup_pending` stuck true, later refans blocked, and the
not-yet-delivered `merged_rx` (plus its freshly spawned forwarder) kept
alive. Same setup-task/connection leak, moved to the snapshot fetch.

**Fix:** extracted `fetch_snapshot_within(daemon, timeout, fetch)` which
wraps the fetch in an OUTER `tokio::time::timeout`
(`SNAPSHOT_FETCH_TIMEOUT = 30s`) and degrades a stall to an `Err`
snapshot. The `snapshots` Vec already carries
`Result<DaemonState, String>`, so a stalled daemon renders as an errored
snapshot in the F2 view rather than hanging setup. The timeout is a
parameter so the degrade path is unit-testable without a 30 s wait;
generic over the fetch error type (`E: Display`) so tests use
`std::future` stand-ins while production passes `jobs::query`
(`eyre::Report`).

**Tests (blit-tui, +2):**
`fetch_snapshot_within_times_out_to_degraded_err` (pending fetch + 10 ms
bound → `Err` containing "timed out"),
`fetch_snapshot_within_passes_through_a_ready_ok`. Full workspace gate
green.

## Reviewer comments

(empty — pending round-2 grade)
