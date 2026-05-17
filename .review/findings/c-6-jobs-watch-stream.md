# c-6-jobs-watch-stream: `blit jobs watch` uses Subscribe streaming

**Severity**: Feature (CLI upgrade — first real consumer of the Subscribe wire surface)
**Status**: In progress / pending review
**Branch**: `phase5/c`
**Commit**: filled by the sentinel commit

## What

Replaces the `blit jobs watch <remote> <transfer_id>` GetState
polling loop (m-jobs-6) with a c-2 `Subscribe` stream filtered
by c-5a's `transfer_id_filter`. The CLI now sees live byte-
level progress and the terminal verdict as they fire, with
zero polling overhead.

This is the first end-to-end consumer of the Subscribe wire
surface — proves the c-2 / c-3 / c-4 / c-5a chain works as a
coherent observability story.

## Why this is the right next slice

The polling implementation (m-jobs-6) was explicitly built as
a stopgap. Subscribe was always the load-bearing wire shape
for live progress; with c-5a's filter the CLI can subscribe to
exactly one transfer without paying the daemon-wide event
broadcast cost. This slice ships the consumer side and
validates the daemon's emit/filter contract end-to-end.

## Approach

### CLI flow

```
1. GetState snapshot (one query, no loop).
   - Already in recent[] → emit terminal line, return.
   - In active[]        → emit initial line, open Subscribe.
   - NotFound           → emit not-found line, return 2.

2. Subscribe stream scoped to transfer_id.
   loop {
       match stream.message().await {
           Ok(Some(TransferProgress)) → update progress.
           Ok(Some(TransferComplete)) → emit done, return 0.
           Ok(Some(TransferError))    → emit failed, return 1.
           Ok(Some(TransferStarted))  → ignored.
           Ok(None) | Err(_)          → reconcile via GetState.
       }
   }
```

The initial GetState handles three cases the Subscribe-only
flow can't on its own:

- **Already terminal**: no further events will fire. Without
  the snapshot the CLI would hang.
- **Never existed**: same — Subscribe would just sit silent.
- **In flight at subscribe time**: the operator gets immediate
  feedback ("seen, watching now") before the next progress
  tick.

Stream errors (typically `Status::aborted` from the c-2/c-5a
Lagged path) and clean stream-end (`Ok(None)`, daemon
shutdown) fall back to a final GetState query so the operator
always gets a coherent exit code rather than a stream failure.

### Timeout handling

`--timeout-secs` is preserved. The Subscribe receive is
wrapped in `tokio::time::timeout(remaining, stream.message())`;
on elapsing, the CLI emits the same timeout line + JSON shape
m-jobs-6 already documented (exit code 3).

### `--interval-ms` is now a no-op

The CLI flag is preserved for backward compatibility (scripts
that pass `--interval-ms 500` shouldn't error), but the
streaming model has no polling cadence. Documented in the
rustdoc and finding doc. A future slice could remove it from
the clap surface; that's a breaking change deferred.

### Output

Human (stderr):

```
Watching transfer t1779-42 on host-b (streaming)...
[active] delegated_pull mod/path peer=10.0.0.5:443 age=2.4s
[progress] t1779-42 bytes=1048576 throughput=10.49 MB/s
[progress] t1779-42 bytes=2097152 throughput=10.50 MB/s
[done] transfer t1779-42 bytes=10485760 duration=1.00s ok
```

JSON (stdout, JSON-Lines):

```
{"state":"active","transfer_id":"t1779-42",...}
{"state":"progress","transfer_id":"t1779-42","bytes_completed":1048576,"throughput_bps":10485760,...}
{"state":"progress","transfer_id":"t1779-42",...}
{"state":"finished","transfer_id":"t1779-42","ok":true,...}
```

Existing `active` / `finished` / `not_found` / `timeout` JSON
states are preserved verbatim — a new `progress` state slots
in between `active` and `finished`. Consumers that didn't
recognize `progress` would only need to update their parsing
once they want byte-level updates; their existing terminal
recognition still works.

## Files changed

- `crates/blit-app/src/admin/jobs.rs`:
  - `+subscribe(remote, transfer_id_filter)` opens a Subscribe
    stream and returns the raw `tonic::Streaming<DaemonEvent>`.
- `crates/blit-cli/src/jobs.rs`:
  - `run_jobs_watch` rewritten end-to-end (~200 LOC).
  - `+reconcile_via_get_state` helper for stream-end / Lagged
    fallback.
  - `+emit_human_active`, `emit_human_finished`,
    `emit_human_progress`, `emit_human_complete` helpers split
    out for clarity.
  - `+format_bps` formatter for throughput (B / KB / MB / GB).
  - `+print_watch_progress_json`, `+print_watch_complete_json`,
    `+print_watch_error_json` JSON emitters.

## Tests added

No new unit tests in this slice — the watch flow needs a live
tonic server to exercise the Subscribe stream end-to-end.
The components are individually well-covered:

- `jobs::query` + `watch_snapshot` (m-jobs-6).
- `subscribe_with_transfer_id_filter_drops_other_transfer_events`
  + `filtered_subscriber_survives_overflow_*` +
  `filtered_subscriber_forwarder_exits_on_client_disconnect`
  (c-5a).
- `subscribe_delivers_transfer_started_event_to_subscriber`
  + `build_transfer_finished_event_*` +
  `tick_progress_emits_transfer_progress_per_active_row`
  (c-2 / c-3 / c-4).

A future integration test under `crates/blit-cli/tests/`
could stand up a tonic server + actually drive a transfer +
spawn the CLI subprocess + verify stream output. That's an
end-to-end test scope beyond this slice; the building blocks
above each test their own contract.

Workspace: 571 passing serially (unchanged from c-5a round 3).

## Known gaps

1. **No replay-on-connect.** A subscriber that opens the
   Subscribe stream after the first TransferProgress events
   for the transfer have already fired doesn't see them. The
   initial GetState snapshot bridges the gap by reporting
   current `bytes_completed`, but per-tick progress between
   that snapshot and the next live tick is invisible. c-5b
   (per-job event ring + replay_recent) would close this.

2. **`--interval-ms` is now a no-op.** Existing scripts that
   set it won't fail, but the value is ignored. A `clap`
   deprecation warning would be ideal; deferred.

3. **No end-to-end integration test.** See "Tests added"
   above.

4. **Stream-error fallback is exit 3 if still active.**
   `reconcile_via_get_state` maps "stream lost + transfer
   still active" to exit code 3 (timeout-equivalent). The
   operator can re-run `blit jobs watch` to resume; the
   choice of code matches the timeout semantic ("we gave up
   watching").

## Out of scope (next slices)

- **c-5b-event-ring**: per-job replay buffer + `replay_recent`.
- **c-7-module-and-heartbeat**: ModuleListChanged,
  DaemonHeartbeat.
- **`bytes_total` / files counters** wired from manifest stage
  (lower priority; the CLI rendering tolerates zero).

## Reviewer comments

(empty — pending grade)
