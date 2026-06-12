# w4-2-delete-push-upload-channel — delete the 262,144-slot drain-and-discard channel

**Branch**: `master` (owner-authorized session 2026-06-12, "Continue with 1")
**Commit**: `03bcb1d`
**Source finding**: async-push-upload-channel-fallback-wedge (reviewer: high) — `docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`

## What

`handle_push_stream` queued every needs-upload `FileHeader` into a
262,144-slot mpsc channel before any transfer-mode branch. In gRPC-fallback
mode nothing read it — send #262,145 wedged daemon and client forever with
no timeout in scope. In TCP mode, N stream workers shared the receiver
behind `Arc<AsyncMutex>` solely to drain it into the void (headers travel
on the wire post-Phase-5), with a companion cache explicitly voided.
All of it is deleted.

## Approach

Pure plumbing removal, no behavior change on the live paths:
- control.rs: `FILE_UPLOAD_CHANNEL_CAPACITY`, channel creation,
  `upload_tx.send` in the manifest loop (and its false "only the gRPC
  fallback path uses this queue" comment), `drop(upload_tx)`, both
  `upload_rx_opt.take()` sites.
- data_plane.rs: `files`/`cache` params of `accept_data_connection_stream`
  and `handle_data_plane_stream`, the per-stream drain task +
  `drain_handle.abort()`, the voided cache. Unused imports cleaned
  (mpsc/AsyncMutex; HashMap retained — still used by the fallback path).

## Discovery during the regression net (filed as design-4)

The spec'd regression test ("force-grpc push with >capacity manifest
completes") FAILED — and stash-bisect against the unmodified tree showed
it fails identically pre-change. **Forced-gRPC pushes are broken at
≥128 files** (exactly `FILE_LIST_EARLY_FLUSH_ENTRIES`; ~100 is
timing-flaky — 97/100 files landed, then "failed to send push request
payload"; ≤80 reliable on loopback). Mechanism hypothesis (unverified):
the need-list batcher's mid-manifest early flush triggers the
`tcp_fallback=true` negotiation while the daemon's manifest loop still
rejects `FileData` as premature. Filed as **design-4** in REVIEW.md with
repro. Consequence: the 262k wedge this slice removes was unreachable in
practice — pushes died ~2000× earlier. The Phase B finding verified the
wedge by code-reading; execution finds the earlier cliff.

## Files changed

- `crates/blit-daemon/src/service/push/control.rs`
- `crates/blit-daemon/src/service/push/data_plane.rs`
- `crates/blit-cli/tests/remote_tcp_fallback.rs` (+2 tests)
- `REVIEW.md` (design-4 row), `docs/STATE.md` (session authorization)

## Tests added

- `forced_grpc_push_many_files_completes` (50 files — multi-file fallback
  coverage inside the zone design-4 doesn't break; suite 1368 → 1369).
- `forced_grpc_push_overflows_old_upload_channel_capacity` (270k files,
  `#[ignore]`): the joint acceptance test for design-4 + w4-2 — currently
  cannot pass until design-4 is fixed; documented in its comment.

## Known gaps

- The full >262k regression proof is deferred behind design-4 (the ignored
  test is the contract). The structural fix is still sound: no bounded
  channel exists on the manifest path at all post-deletion (grep-verifiable).
- design-4's mechanism is a hypothesis; the finding row says so. Fixing it
  is new queue work for the owner to ratify (not part of the ratified 38).
