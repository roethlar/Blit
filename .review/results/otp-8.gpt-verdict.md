# otp-8 — codex review adjudication

reviewer: gpt-5.6-sol (codex exec, read-only; loop doc names gpt-5.5 —
config default moved, see the 41st handoff's process note)
reviewed commit: `5ffc9be`
raw output: `.review/results/otp-8.codex.md`
verdict line: FAIL — "the new tests are otherwise sound; runnable test
count is confirmed at 1550 → 1552."
fix commit: `643294a` (both findings; suite 1552 → 1555)

## F1 (High) — in-stream sends don't race peer faults; cancellation can hang or surface INTERNAL

**Claim** (mod.rs:1183, 1224): the in-stream payload/resume sends never
poll queued peer faults, so a cancel can hang behind a stalled read or
decay to INTERNAL; the otp-8 finding doc's cancellation deferral
("fault frames ride the control lane identically on both carriers") is
unsound.

**Adjudication: ACCEPTED — and verified worse than the wrong-code
half.** The data-plane branch routes `dp.queue()` errors through
`prefer_peer_fault` and races the drain against `recv_peer_fault`
(mod.rs, otp-4b-3), but the in-stream branches ran `send_payload_records`
/ `send_resume_block_records` inline with bare `?`. The e2e cancel
fixture (client stuck inside `reader.read()`) proved a mid-transfer
cancel over the in-stream carrier HANGS the client — nothing ever
interrupts the blocked send half; the framed CANCELLED sits unread in
the events queue. My finding-doc deferral was wrong: the fault frames do
ride the control lane identically, but only the data-plane send path
ever *raced* them.

**Fix**: a `watch`-based fault side-channel (`SourceEventSender`
mirrors every `SourceEvent::Fault` onto it as the receive half queues
it), raced `biased` fault-first against both in-stream record sends —
the in-stream twin of the drain's `recv_peer_fault` arm. Non-fault
events are NOT consumed mid-send (the watch is a side channel), so
in-flight `Need`s stay queued for the next drain. Peer faults arrive
`peer_notified: true` (`from_wire`), so the error path does not echo.

**Guard**: `mid_transfer_cancel_surfaces_cancelled_over_in_stream_carrier`
(daemon e2e, mirrors the data-plane twin with `in_stream_bytes: true`).
Temporary revert of the select at the plain-batch site → the test fails
at its 10 s no-hang timeout; restored → passes.

**Residue noted, not fixed here** (same family as the deferred
otp-7b-1 F3 / STATE queue item 6): if the RPC teardown error surfaces
in the send arm before the framed fault is processed, the code decays
to INTERNAL — the biased fault-first select makes the framed reason win
whenever both are ready, and the e2e pins the dominant path.

## F2 (Medium) — one `TarShardHeader` frame can exceed tonic's 4 MiB decode limit

**Claim** (mod.rs:1716): `TarShardHeader.files` carries every member
header in one protobuf frame; the planner caps a shard at 4096 files
and bounds content bytes, but not the encoded header-list size — legal
long-path workloads can push the single frame past the 4 MiB limit on
the in-stream carrier. So resume was not the only real-wire residue.

**Adjudication: ACCEPTED.** Verified against `transfer_plan.rs`
(`count_target` clamps to [128, 4096]; no path-length term in the shard
budget) and the absence of any `max_decoding_message_size` override
(tonic default 4 MiB applies). 4096 headers × ~600-byte paths ≈
2.5 MiB encoded — real trees can double that. Note the same exposure
exists in the OLD gRPC fallback lane (`payload.rs` sends the same
header-list shape), so this is a pre-existing grammar gap the session
inherited, not a regression — but post-cutover the session is the only
path, so it must be fixed here, and the carrier slice is exactly where
it belongs. The data plane is unaffected (binary records, 64 MiB
per-record cap; content chunks were already bounded).

**Fix**: `MAX_IN_STREAM_TAR_HEADER_BYTES` (2 MiB, same posture as the
resume block ceiling) + `bound_in_stream_tar_headers` — a pure splitter
applied ONLY on the in-stream send path, after the planner: an
offending shard becomes consecutive smaller shard records (same
grammar, same planner decisions; only record boundaries move). A single
header can never exceed the bound (OS path limits), and a one-file
remainder cannot hit the empty-`relative_path` hazard (that identity
only exists for single-file roots, which the planner never tar-shards).

**Guards**: `tar_shard_headers_split_under_the_in_stream_bound` (pure
splitter: split-under-bound, order/file-set preservation, passthrough,
degenerate single-header case) and
`in_stream_send_splits_oversized_tar_header_frames` (wiring: 4096
one-byte files with ~590-byte paths — >2 MiB encoded — through
`send_payload_records` with a frame-capturing `FrameTx` must emit
multiple `TarShardHeader` frames, each under the bound, no file lost).
Temporary revert of the wiring line → the wiring test fails on a single
oversized frame; restored → passes.

## Also from the raw output

Codex confirmed the two reviewed tests sound (fixture math, clamp
observability unique to the asserted block size) and the runnable count
1550 → 1552; no other findings.
