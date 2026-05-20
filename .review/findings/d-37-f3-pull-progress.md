# d-37-f3-pull-progress: live byte progress on F3 pull

**Severity**: Feature (polish — closes d-35 known gap #1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

d-35 shipped the F3 pull with a static `pulling →
<dest>...` footer — no progress while a large transfer
ran. d-37 wires a live byte/file counter through the
same `RemoteTransferProgress` channel the CLI uses, so
the footer updates as data flows:

```
pulling → /backups/photos...                       ← before first byte
pulling → /backups/photos... (37 file(s) · 412 MiB) ← live (d-37)
pulled 128 file(s) · 1.20 GiB → /backups/photos     ← Done
```

## Approach

### Progress plumbing

`spawn_f3_pull` now builds a `RemoteTransferProgress`
(an `UnboundedSender<ProgressEvent>`) and passes
`Some(&progress)` to `run_pull_sync` (was `None`). A
forwarder task drains the `ProgressEvent` stream,
accumulates cumulative `(files, bytes)` — the events are
per-chunk deltas, exactly as the CLI's monitor sums them
— and ships `F3PullProgress { request_id, files, bytes }`
snapshots to the event loop.

```text
run_pull_sync ──ProgressEvent──► forwarder ──F3PullProgress──► event loop
   (delta events)                (accumulate)   (try_send)        (apply_progress)
```

Snapshots use `try_send` on a small (8) bounded channel:
a full channel drops an intermediate update rather than
backpressuring the forwarder. Progress is approximate by
nature; the **authoritative** final `(files, bytes)`
rides the terminal `F3PullReply` (from the daemon's
report), so a dropped snapshot only means a momentarily
stale counter.

Lifecycle: after `run_pull_sync` returns, `drop(progress)`
closes the event channel → the forwarder drains and
exits → the terminal reply is sent. Ordering guarantees
no progress snapshot races past the Done/Error reply.

### State

`F3PullStatus::Running` gains `files` / `bytes` (0 until
the first event). New `F3PullState::apply_progress`
updates them in place, generation-guarded so a snapshot
from a superseded run is dropped. The event loop's new
`f3_pull_progress_rx` select arm calls it.

### Render

`F3PullDisplay::Running` carries `files` / `bytes`; the
footer appends `(N file(s) · X)` once either is non-zero
(before that, just `pulling → <dest>...` to avoid a
distracting `(0 file(s) · 0 B)`).

## Files changed

- `crates/blit-tui/src/f3pull.rs`:
  - `Running` gains `files` / `bytes`.
  - `apply_progress` (generation-guarded in-place update).
  - `begin_run` inits counters to 0.
  - 4 new tests.
- `crates/blit-tui/src/main.rs`:
  - `F3PullProgress` struct.
  - `spawn_f3_pull` builds the progress monitor +
    forwarder; takes a `progress_tx`.
  - AppState `f3_pull_progress_tx` + channel + select arm.
  - bridge `f3_pull_to_display` carries the counters.
  - AppState test fixtures updated.
- `crates/blit-tui/src/screens/f3.rs`:
  - `F3PullDisplay::Running` carries `files` / `bytes`;
    footer renders the live count; module-doc updated.

## Tests

+4 tests (386 → 390), all in `f3pull::tests`:

- `begin_run_starts_with_zero_progress`.
- `apply_progress_updates_running_counters`.
- `apply_progress_drops_stale_request` — a snapshot for
  a different request id doesn't apply.
- `apply_progress_noop_when_not_running` — harmless on
  Idle.

The forwarder + channel plumbing is exercised manually
(it needs a live daemon to emit real `ProgressEvent`s);
the pure state transition `apply_progress` is fully
unit-tested.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No rate (MiB/s) or ETA.** The footer shows
   cumulative count, not throughput. The CLI's monitor
   computes avg/current MiB/s; the TUI could too, but
   the footer is a single line and the count is the
   primary "is it moving" signal.

2. **No live-tick coupling.** Progress updates arrive as
   channel events that wake the select loop directly, so
   the footer refreshes on each snapshot without needing
   `needs_live_tick`. (If the daemon went silent
   mid-transfer the counter would freeze until the next
   event — but that's also when there's genuinely no
   progress to show.)

3. **Done/Error fragment still has no TTL.** Deferred to
   the next slice (d-35 known gap #2) — the outcome
   persists until the next pull or pane action.

## Out of scope

- Throughput / ETA in the footer.
- Auto-hide TTL on Done/Error (next slice).

## Reviewer comments

(empty — pending grade)
