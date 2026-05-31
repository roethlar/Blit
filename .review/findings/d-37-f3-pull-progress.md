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

### Round 1 verdict — reopened (`.review/results/d-37-f3-pull-progress.reopened.md`)

One finding:

- **Double-counted bytes on the data-plane path.** The
  forwarder added bytes from BOTH `Payload` and
  `FileComplete`. But the TCP data-plane receive path
  (`pipeline.rs` `execute_receive_pipeline`) emits both
  `Payload { files: 0, bytes: N }` AND
  `FileComplete { bytes: N }` for the same completed
  file. F3 uses `PullSyncOptions::default()`, so that
  path is in scope — the footer could show ~2× the bytes
  mid-transfer, then snap backward when the authoritative
  `F3PullReply` total landed.

### Round 2 fix

Extracted the accumulation into a pure
`accumulate_pull_progress(files, bytes, event)` with
pull-receive semantics:

- **bytes** come from `Payload` only.
- **file count** comes from `FileComplete` only (ignore
  its `bytes`).

This is correct on both data-plane paths:
- **TCP data-plane**: `Payload(0, N)` + `FileComplete(N)`
  → bytes += N (Payload), files += 1 (FileComplete,
  byte field ignored) = N bytes / 1 file. No double.
- **Direct-gRPC**: chunk `Payload`s carry the bytes,
  `FileComplete(0)` carries no bytes → bytes sum from
  chunks, files += 1. Correct.

The forwarder now calls the helper; the logic is no
longer inline-and-untestable.

### Round 2 file changes

- `crates/blit-tui/src/main.rs`:
  - New pure `accumulate_pull_progress` helper.
  - Forwarder calls it instead of the inline match.
  - 4 new accumulator tests.

### Round 2 tests

+4 tests (390 → 394):

- `accumulate_pull_progress_data_plane_pair_no_double_count`
  — the reviewer's exact regression: `Payload{bytes:1024}`
  + `FileComplete{bytes:1024}` → 1024 bytes / 1 file, NOT
  2048.
- `accumulate_pull_progress_grpc_chunks_then_zero_byte_complete`
  — chunk Payloads sum; `FileComplete{bytes:0}` counts
  the file.
- `accumulate_pull_progress_manifest_batch_is_inert`.
- `accumulate_pull_progress_multi_file_data_plane` — the
  pair per file across 3 files totals honestly.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

### Lesson restated

When two event types can describe the same underlying
fact (here: a completed file's bytes reported as both a
`Payload` and a `FileComplete`), summing both
double-counts. Pick one event as the byte authority and
one as the count authority — and unit-test the
accumulator against the actual emitter's event sequence,
not an assumed one. The CLI's monitor sums both too;
the TUI's footer is more visible mid-transfer, so the
discrepancy surfaced here.
