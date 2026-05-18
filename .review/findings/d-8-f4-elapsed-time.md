# d-8-f4-elapsed-time: surface duration on Done banners

**Severity**: Feature (polish — closes the
"no time info" gap mentioned in d-7's known gaps)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Both F4 banners (Verify + Local transfer) now show
elapsed time. While running, the line ticks live:

```
copy running... (12.3s)
running compare_trees... (432ms)
```

When the run lands on Done, the duration freezes at the
final total:

```
copy done · 1234 planned · 1234 copied · 5678 bytes · 12.3s
matches: 100 · differ: 3 · missing-on-src: 0 · missing-on-dst: 5 · errors: 0 · 432ms
```

The CLI shows transfer duration on its summary; the TUI
was the only surface missing it. With this slice the
operator can tell "this copy took 14s" or "compare took
4 minutes" from the F4 pane alone, without timing it
themselves.

## Approach

### State enrichment

`TransferStatus::Running` and `VerifyStatus::Running` now
carry `started_at: Instant`. The `Done` variant carries
both `started_at` and `finished_at`. `begin()` /
`begin_run()` capture `Instant::now()`; `apply_done` /
`apply_result` thread the timestamp from the Running
variant into Done with a defensive fallback to
`Instant::now()` if state somehow wasn't Running.

### Render

`render_verify` and `render_transfer` gain a `now:
Instant` parameter (already available — the router
computes it once per frame and threads it down to the
profile footer; reusing it avoids one extra clock call).

For Running: `now.saturating_duration_since(started_at)`
ticks live as the operator looks (each frame computes
fresh).

For Done: `finished_at.saturating_duration_since(started_at)`
freezes at completion.

### `format_elapsed`

New helper in `screens/f4.rs` produces a compact human
string across four magnitudes:

| Duration         | Output    |
|------------------|-----------|
| < 1s             | `432ms`   |
| 1s – 59.9s       | `12.3s`   |
| 1m – 59m 59s     | `2m 5s`   |
| 1h+              | `1h 30m`  |

Per the design's "operator should be able to read this
at a glance" principle — millisecond precision matters
for short verify runs but seconds is plenty for
multi-minute transfers.

## Files changed

- `crates/blit-tui/src/transfer.rs`:
  - `TransferStatus::Running` carries `started_at`.
  - `TransferStatus::Done` carries both timestamps.
  - `begin()` captures `Instant::now()`.
  - `apply_done()` threads `started_at` through.
- `crates/blit-tui/src/verify.rs`:
  - `VerifyStatus::Running` carries `started_at`.
  - `VerifyStatus::Done` carries both timestamps.
  - `begin_run()` captures `Instant::now()`.
  - `apply_result()` threads `started_at` through.
- `crates/blit-tui/src/screens/f4.rs`:
  - `format_elapsed(Duration) -> String` helper.
  - `render_verify` + `render_transfer` take `now`.
  - Running arms format live elapsed.
  - Done arms append the frozen duration.
  - `render_into` plumbs `now` to both.

## Tests

+6 unit tests (162 → 168):

In `screens::elapsed_tests` (new submodule):
- `format_elapsed_milliseconds`
- `format_elapsed_seconds_with_tenths`
- `format_elapsed_minutes_seconds`
- `format_elapsed_hours_minutes`

In `transfer::tests`:
- `apply_done_preserves_started_at_from_running` —
  asserts the timestamp from the Running variant flows
  into the Done variant unchanged.

In `verify::tests`:
- `apply_result_preserves_started_at_from_running` —
  twin assertion for the Verify path.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No bytes/sec rate.** The Done banner shows total
   bytes and duration but doesn't compute a transfer
   rate. A future slice could add `(N MB/s)` for
   transfers larger than some threshold.

2. **No countdown / ETA.** Live tick is "elapsed",
   not "remaining". The predictor on F4 has the data to
   estimate ETA but it's not wired to the running
   transfer.

3. **Running tick updates only when the render loop
   wakes.** The event loop currently redraws on each
   input event or stream message — not on a steady
   ticker. So the live "(12.3s)" text only refreshes
   when something else triggers a draw. A future polish
   slice (e-2 prep, frame rate) could add a 1Hz tick
   while a transfer is active.

## Out of scope (next slices)

- **e-3 themes / config** (`~/.config/blit/tui.toml`).
- **Steady-tick render loop** for live updates.
- **Per-file progress** during local transfers.

## Reviewer comments

(empty — pending grade)
