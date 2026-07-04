# w4-1-abortondrop-family — hoist AbortOnDrop; close the remaining detach-on-drop sites

**Branch**: `master`
**Commit**: `65ecb93`
**Source**: `docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` §W4.1 (ratified
D-2026-06-11-2), absorbing `design-2-orphaned-daemon-data-planes` and the
`async-push-client-pipeline-detach-on-drop` /
`async-daemon-push-stream-workers-detach-on-first-error` findings in
`docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`.

## What

`AbortOnDrop` (the RAII wrapper that aborts a spawned task on drop instead
of detaching it, R32-F2/R34-F2) existed only in
`blit-core/src/remote/pull.rs`, `pub(crate)`. Everywhere else that spawned
a task whose lifetime should be bounded by a calling future still used a
bare `JoinHandle`, so an early `?` return (or the whole future being
dropped/cancelled) left the spawned task running with no owner:

- `blit-daemon/src/service/push/control.rs` — `data_plane_handle`, the
  daemon's push accept/receive task (design-2's one remaining site; its
  two `service/pull.rs` sites were deleted with the legacy Pull RPC at
  `ue-r2-1h`).
- `blit-core/src/remote/push/client/mod.rs` — `MultiStreamSender`'s
  `pipeline_handle` (the sink-pipeline task) and `push()`'s
  `response_task` (the response-stream reader), both bare `JoinHandle`s.
- `blit-daemon/src/service/push/data_plane.rs` —
  `accept_data_connection_stream`'s per-stream workers were a bare
  `Vec<JoinHandle>`: the first worker to error dropped the remaining
  handles without aborting them, leaving live siblings running. The
  sibling `accept_data_connection_stream_resizable` (added at `ue-r2-2`)
  had already fixed this same class with a `JoinSet`.

## Approach

- Hoisted `AbortOnDrop<T>` (struct, `new`, `join`, `Drop`, and its four
  generic regression tests) from `pull.rs` into
  `blit-core::remote::transfer::abort_on_drop`, made `pub` (not
  `pub(crate)`) so `blit-daemon` can use it too, and re-exported at
  `blit_core::remote::transfer::AbortOnDrop`. `pull.rs` now imports it;
  behavior is unchanged (byte-identical wrapper).
- `push/control.rs`: `data_plane_handle` is now
  `Option<AbortOnDrop<Result<TransferStats, Status>>>`; both spawn sites
  (early-flush and post-manifest) wrap in `AbortOnDrop::new`. The
  post-manifest select loop pins `handle.join()` across loop iterations
  (`tokio::pin!`) instead of polling a bare `JoinHandle` — `.join()` holds
  `self` across its internal await, so dropping the pinned future mid-poll
  (the `stream.message()?` race erroring) drops the still-owned
  `AbortOnDrop` and aborts the task.
- `push/client/mod.rs` + `helpers.rs`: `spawn_response_task` now returns
  `AbortOnDrop<()>`; `MultiStreamSender::pipeline_handle` is
  `Option<AbortOnDrop<Result<SinkOutcome>>>`; `drain_pipeline_outcome`/
  `drain_pipeline_error` drain via `.join()` instead of `.await`. Six
  existing unit tests updated to construct `AbortOnDrop`-wrapped handles.
- `push/data_plane.rs`: `accept_data_connection_stream` converted from
  `Vec<JoinHandle>` + sequential `.await` to a `JoinSet` + `join_next()`
  loop, mirroring `accept_data_connection_stream_resizable`. Dropping the
  `JoinSet` on the first-error return aborts every remaining worker.

## Tests added

- `abort_on_drop::tests` (relocated, unchanged): drop-without-consume
  aborts; join returns value and drop becomes a no-op; drop after natural
  completion doesn't panic; cancellation during `.join()` await still
  aborts.
- `multi_stream_sender_drop_tests::dropping_sender_without_finish_aborts_pipeline_task`
  (blit-core): constructs a `MultiStreamSender` directly (bypassing
  `connect()`, which needs real TCP) with a long-running fake pipeline
  task, drops it without calling `.finish()` (the shape of `push()`'s
  early-`?`-return path), and asserts the task didn't run to completion.
  Verified against a reverted `Drop` impl (task then completes — test
  fails) before restoring the fix.
- `data_plane_handle_abort_tests::dropping_data_plane_handle_aborts_task`
  (blit-daemon): pins the same contract at the field-type level for
  `push/control.rs`'s `data_plane_handle` (a full gRPC-stream integration
  test to exercise the real drop path would be disproportionate to a
  wrapper-type change).
- `first_stream_error_aborts_sibling_worker` (blit-daemon): a real
  `TcpListener` + two real TCP client connections drive
  `accept_data_connection_stream` end-to-end — one client sends the token
  then closes without a `DATA_PLANE_RECORD_END` (triggers an immediate
  worker error), the other holds its socket open. Asserts the function
  returns `Err` and the surviving client observes its socket closed
  (EOF/reset) rather than hanging. Verified against the old
  `Vec<JoinHandle>` shape (probe times out — test fails) before restoring
  the `JoinSet` fix.

Full suite: fmt clean, clippy clean (workspace, all targets, `-D
warnings`), `cargo test --workspace` all green (blit-core 348, blit-daemon
162, both up from baseline by the new tests above; no other crate's count
changed).

## Known gaps

- `tuner_handle` (`MultiStreamSender`) stays a bare `JoinHandle` — out of
  scope per the design map (it already self-terminates via its
  `Weak<TransferDial>` if the sender is dropped without finishing, and
  `finish()` aborts it explicitly; it isn't a detach-on-drop bug of the
  same shape).
- No new regression test drives `spawn_response_task`'s returned
  `AbortOnDrop` through a real `tonic::Streaming` (constructing a fake one
  outside a live gRPC connection is disproportionate); its correctness
  follows from `AbortOnDrop`'s proven contract plus the mechanical
  `JoinHandle` → `AbortOnDrop` type change, which the compiler enforces at
  the `response_task.join()` call site.
