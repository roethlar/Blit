# d-9-live-tick: 500ms wakeup ticks the F4 elapsed counter

**Severity**: Feature (closes a d-8 known gap — the
live elapsed counter only refreshed when something else
woke the event loop)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

The event loop now arms a 500ms wakeup whenever a Verify
run or a local transfer is in flight. Each tick triggers
the next `terminal.draw` call which re-renders the F4
elapsed counter with a fresh `Instant::now()`. So the
"copy running... (12.3s)" line ticks visibly while the
operator watches, instead of freezing on the last byte
that crossed any other channel.

When nothing is running, the tick future is `pending()`
— the loop sleeps indefinitely on real events. No idle
CPU burn, no terminal flicker on idle screens.

## Approach

### Gate predicate

New `needs_live_tick(app: &AppState) -> bool` in
`main.rs`:

```rust
fn needs_live_tick(app: &AppState) -> bool {
    app.transfer.is_running() || app.verify.is_running()
}
```

Verify state gains a symmetric `is_running()` method
(mirrors `TransferState::is_running()`).

**Deliberately false** for `ConfirmingMirror` /
`ConfirmingMove`: those banners are static prompts —
nothing changes over time, so waking up to redraw them
would be pure overhead.

### Loop branch

`run_router`'s `tokio::select!` gains a new arm:

```rust
let live_tick = async {
    if needs_live_tick {
        tokio::time::sleep(Duration::from_millis(500)).await;
    } else {
        std::future::pending::<()>().await;
    }
};
tokio::pin!(live_tick);

tokio::select! {
    _ = &mut live_tick => {}
    key = key_rx.recv() => { ... }
    // ...other arms
}
```

The body is empty — the wakeup just lets the loop loop
back to the top, where the next `terminal.draw` recomputes
`now = Instant::now()` and re-renders.

### Cadence choice

500ms is the chosen interval. The `format_elapsed`
helper (d-8) shows tenths-of-a-second precision for runs
under a minute, so 500ms aligns the visible-update
cadence with the smallest renderable change. Anything
faster (100ms, 250ms) is visible flicker. Anything
slower (1s+) is choppy.

## Files changed

- `crates/blit-tui/src/verify.rs`:
  - New `is_running()` method (symmetric to
    `TransferState::is_running`).
- `crates/blit-tui/src/main.rs`:
  - `needs_live_tick(&AppState) -> bool` helper.
  - `tokio::select!` gains a `live_tick` arm with the
    conditional `pending()` / `sleep` future.

## Tests

+1 unit test (168 → 169):

In `main::tests`:
- `needs_live_tick_only_during_active_runs` — asserts
  Idle / ConfirmingMirror return false, Running returns
  true (both transfer and verify), Done returns false.

(No new tests in `verify.rs` — the new `is_running()`
mirrors a method already covered by the transfer tests
and is exercised by the main test above.)

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No tick during F1/F2 "fetched Xs ago" footers.**
   Those timestamps are also computed from a captured
   `Instant`, so they go stale until an event arrives.
   The current gate only fires for F4 Running states.
   A future polish could extend the gate to F1/F2
   when the operator is looking at those panes (the
   freshness banners would tick once a second).

2. **Tick stops at terminal states.** Done/Error stop
   the tick (correctly — frozen duration doesn't need
   updates). But the transition from Running → Done is
   visible only on the next external event, not exactly
   on the moment the reply arrives. In practice the
   reply IS an event so this is moot, but the failure
   mode worth noting: if the reply channel somehow
   delivered without waking the loop (it does wake it,
   so this is hypothetical), the elapsed would freeze
   at the last 500ms tick.

## Out of scope (next slices)

- **e-3 themes / config** — `~/.config/blit/tui.toml`.
- **Per-file progress** during local transfers.
- **F1/F2 freshness footers tick on the same cadence.**

## Reviewer comments

### Round 1 verdict — reopened (`.review/results/d-9-live-tick.reopened.md`)

One Low-severity finding, addressed in round 2:

- **Stale rustdoc on `needs_live_tick`.** The round-1
  edit inserted the new "d-9: arm the 500ms wakeup..."
  paragraph ABOVE the existing `can_start_transfer`
  paragraph instead of replacing it. As a result both
  paragraphs ended up attached to `needs_live_tick`,
  with the leading one ("true when the operator can
  kick a local transfer") contradicting the function's
  actual contract (true while a transfer IS active),
  and `can_start_transfer` itself ended up
  undocumented.

  Round 2 swaps the two paragraphs back: the
  `can_start_transfer` rustdoc moves back above its
  declaration, leaving only the d-9 paragraph on
  `needs_live_tick`. No behavior change, no test
  change — pure doc hygiene.

### Round 2 file changes

- `crates/blit-tui/src/main.rs`: re-home the
  `can_start_transfer` rustdoc, leave the d-9 paragraph
  on `needs_live_tick`.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test -p blit-tui` (169
tests) all green. No test count change.
