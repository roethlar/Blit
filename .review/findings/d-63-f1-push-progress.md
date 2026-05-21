# d-63-f1-push-progress: live byte/file footer for the F1 push

**Severity**: Feature (closes d-61 known gap #2)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `391dcd3`

## What

d-61 shipped the F1 local→remote push but with no live progress —
`run_remote_push` was called with `progress = None`, so the
footer showed a static `push → <dest>…` until the terminal reply.
d-63 wires live progress so the push footer matches the F3 pull
footer: `push → <dest>... (N file(s) · X · R/s)`.

## Approach

### Progress forwarder in `spawn_f1_push`

Same shape as `spawn_f3_pull` (d-37): an unbounded
`ProgressEvent` channel + a `RemoteTransferProgress` handed to
`run_remote_push(.., Some(&progress))`; a forwarder task
accumulates cumulative `(files, bytes)` and ships
`F1PushProgress` snapshots on a small lossy (`try_send`) channel.
The channel is dropped + the forwarder drained before the
terminal reply (so the authoritative totals from the reply land
last). A new event-loop select arm applies snapshots via
`F1PushState::apply_progress` (generation-guarded by
`request_id`). `F1PushStatus::Running` gains `files` / `bytes` /
`bytes_per_sec`.

### A push-specific accumulator (the correctness crux)

The push **send** path and the pull **receive** path emit
progress differently, so they need different accumulators:

- **Pull (receive)** — `accumulate_pull_progress` takes bytes
  from `Payload` only (the TCP path emits `Payload`+`FileComplete`
  for the same file; counting both would double-count).
- **Push (send)** — `data_plane.rs send_payloads` calls
  `report_file_complete(path, header.size)` and emits **no**
  `Payload` (there is no `report_payload` anywhere under
  `remote/push/`). So `accumulate_push_progress` takes files AND
  bytes from `FileComplete`. Reusing the pull accumulator would
  report **0 bytes** for a push. It ignores `Payload` /
  `ManifestBatch` defensively (no double-count if a future
  emitter changes).

Throughput reuses `pull_throughput` (0 until ~1s, then cumulative
average).

## Files changed

- `crates/blit-tui/src/f1push.rs`: `Running` gains live counters;
  `apply_progress`; tests.
- `crates/blit-tui/src/main.rs`: `F1PushProgress` +
  `accumulate_push_progress`; `f1_push_progress_tx` field +
  channel + inits; `spawn_f1_push` forwarder + `Some(&progress)`;
  progress select arm; bridge passes the counters; +2 accumulator
  tests.
- `crates/blit-tui/src/screens/f1.rs`: `PushStatusDisplay::Running`
  live counters; `render_push` shows `(N file(s) · X · R/s)`.

## Tests

544 total (was 541):

f1push.rs: `begin` starts at zero progress (extended);
`apply_progress_updates_running_counters` (+ stale-drop guard).

main.rs: `accumulate_push_progress_counts_files_and_bytes_from_file_complete`
(bytes from FileComplete); `accumulate_push_progress_ignores_payload_and_manifest`
(guards the push-vs-pull distinction).

The live forwarding needs a live daemon (manual); the accumulator
semantics, state machine, and select wiring are unit-tested.

## Known gaps

1. **No terminal auto-hide** (d-61 gap #3) — the Done/Error
   footer persists until the next push.
2. **Push mirror/move, remote→remote** still pending.

## Out of scope

- Push mirror/move; terminal auto-hide; remote→remote; F1 `d`
  diagnostics.

## Reviewer comments

(empty — pending grade)
