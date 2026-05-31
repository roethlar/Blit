# d-39-f3-pull-throughput: MiB/s rate on F3 pull progress

**Severity**: Feature (polish — closes d-37 known gap #1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `43258ab`

## What

d-37 shipped the live pull footer with cumulative file/byte
counts but no throughput (its own known gap #1). d-39 adds an
average MiB/s rate, so the operator can see how fast a large
pull is actually moving:

```
pulling → /backups/photos... (37 file(s) · 412 MiB)          ← d-37
pulling → /backups/photos... (37 file(s) · 412 MiB · 48 MiB/s) ← d-39
```

The rate is appended only once it settles — see the warm-up
note below — so the footer never flashes a bogus startup
figure.

## Approach

### Rate computation (pure, tested)

The throughput is a simple cumulative average,
`bytes / elapsed_secs`, extracted into a pure
`pull_throughput(bytes, elapsed_secs) -> u64` helper rather
than left inline in the forwarder task. This is the direct
lesson from the d-37 round-2 reopen: progress math that lives
inline in an async task is untestable, so it gets a pure
helper + unit tests.

**1-second warm-up.** Below 1s elapsed the helper returns 0.
`bytes / 0.01s` at transfer start produces a meaningless
multi-GiB/s spike; suppressing the rate until the window is
wide enough reads better than a wrong number. After warm-up
it's the honest cumulative average — matching the footer's
"is it moving, roughly how fast" intent, not an instantaneous
rate (the footer is a single line; an EMA/current-rate would
be over-engineering for a glance signal).

### Plumbing

`F3PullProgress` gains `bytes_per_sec`. The `spawn_f3_pull`
forwarder records a `started` Instant when it spawns and, on
each accumulated snapshot, computes
`pull_throughput(bytes, started.elapsed().as_secs_f64())` and
ships it alongside the existing file/byte counts. No new
channel — it rides the d-37 progress snapshot.

`F3PullStatus::Running` gains `bytes_per_sec`; `apply_progress`
takes it as a fourth arg and updates it in place (still
generation-guarded — a stale run's snapshot is dropped). The
bridge `f3_pull_to_display` and `F3PullDisplay::Running` carry
it through to the renderer.

### Render

`render_footer` appends ` · {rate}/s` to the live fragment
only when `bytes_per_sec > 0` (i.e. past warm-up), so the
first ~1s shows `(N file(s) · X)` exactly as d-37 did.

## Files changed

- `crates/blit-tui/src/f3pull.rs`:
  - `Running` gains `bytes_per_sec`.
  - `begin_run` inits it to 0; `apply_progress` takes +
    updates it.
  - 3 existing `apply_progress` tests updated to assert the
    new field.
- `crates/blit-tui/src/main.rs`:
  - `F3PullProgress` gains `bytes_per_sec`.
  - New pure `pull_throughput` helper.
  - Forwarder records `started` + computes the rate.
  - Progress reply arm + `f3_pull_to_display` bridge carry it.
  - 3 new `pull_throughput` tests.
- `crates/blit-tui/src/screens/f3.rs`:
  - `F3PullDisplay::Running` gains `bytes_per_sec`; footer
    appends `· X/s` past warm-up; module doc updated.

## Tests

+3 tests (399 → 402):

- `pull_throughput_suppressed_in_first_second` — 0.0 / 0.5 /
  0.999s all return 0 (no startup spike).
- `pull_throughput_is_cumulative_average_after_warmup` — the
  1.0s boundary and beyond return `bytes / elapsed`.
- `pull_throughput_zero_bytes_is_zero`.

The 3 d-37 `apply_progress` tests now also assert
`bytes_per_sec` propagates / is dropped for stale requests.

The forwarder's Instant-based wiring is exercised manually
(needs a live daemon emitting real `ProgressEvent`s); the pure
rate math and the state transition are fully unit-tested.

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace`
all green.

## Known gaps

1. **Cumulative average, not current/EMA rate.** The displayed
   MiB/s is total-bytes-over-total-time, so it lags a sudden
   speed-up or slow-down. The CLI monitor tracks a current
   rate too; the TUI footer deliberately keeps the simpler
   average as a single-glance signal. A windowed/EMA rate
   would be a future polish.

2. **No ETA.** Rate is shown, remaining-time is not — the pull
   has no upfront total-size estimate to divide against
   without a manifest pre-scan.

## Out of scope

- Current-rate / EMA throughput.
- ETA / remaining-time.

This closes the d-37 known-gaps list (gap #1); combined with
d-38 (auto-hide TTL), the F3 pull live-progress feature is now
feature-complete: counts, throughput, and self-cleaning
outcomes.

## Reviewer comments

(empty — pending grade)
