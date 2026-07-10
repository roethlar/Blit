# otp-7b-1 — resume over the TCP data plane

**What**: The unified session's resume block phase now rides the TCP data
plane (`docs/plan/OTP7_RESUME.md` staging otp-7b, first of two passes; the
D4 CLI fault-summary rider + cancel-during-resume e2e are otp-7b-2). The
responder no longer suppresses the data-plane grant for a resume session;
block records travel the sockets as the binary `BLOCK`/`BLOCK_COMPLETE`
record shapes the receive pipeline already decodes, while `BlockHashList`
stays a control-lane frame. `Push/PullSessionOptions` gained
`resume`/`resume_block_size`, so the session client can negotiate resume
end-to-end (needed by any e2e; CLI verbs still ride the old paths until
otp-10).

**Approach**:

- **One composite work item per resumed file.** The send half turns each
  correlated (need, hash-list) pair into `TransferPayload::ResumeFile
  {header, block_size, dest_hashes}` and queues it into the existing
  elastic pipeline. One payload = one worker = one socket: `DataPlaneSink`
  holds its session lock across the whole record, so every `BLOCK` and the
  closing `BLOCK_COMPLETE` are strictly serialized on one socket — no
  cross-socket reorder hazard against the completion's truncate+stamp.
  `prepare_payload` passes the variant through untouched (no blocking prep;
  the diff streams inside the sink write); every other sink and the old
  push paths reject it explicitly.
- **The block-diff is single-sourced** (`remote/transfer/resume_diff.rs::
  ResumeBlockDiff`): sequential read at the DEST-chosen block size, blake3
  per block, stale iff index-beyond-list / hash differs / malformed hash
  (D1: garbage degrades to sending, never trusting), EOF-short aborts.
  The otp-7a in-stream `send_resume_block_records` now drives the same
  iterator, so both carriers share one staleness semantic (same reasoning
  as the D3 free-helper decision).
- **DEST-side claims move to shared state.** `resume_headers` (grant map)
  and `files_resumed` became `Arc`-shared exactly like `outstanding`:
  the control loop inserts each grant BEFORE sending its `BlockHashList`
  (insert-before-send, same ordering rule as `outstanding`) and claims
  inline on the in-stream carrier; `NeedListSink` validates + claims on
  the data plane with the in-stream strictness replicated — blocks only
  against a live grant, in-bounds against the manifested size, completion
  claims exactly once, completion size must equal the manifest promise,
  resumed count increments only after the finalization write lands, and a
  file/tar delivery for a resume-flagged grant is refused (codex otp-7a F3
  parity). SourceDone still verifies both sets empty after the data plane
  drains.
- **Ceiling per carrier (D-2026-07-10-2, completes the D-2026-07-10-1
  revisit)**: DEST clamps to 2 MiB in-stream / 64 MiB data plane
  (`MAX_DATA_PLANE_RESUME_BLOCK_SIZE` = `MAX_WIRE_BLOCK_BYTES`, now
  pub(crate)); the block size is chosen after the carrier is settled.
  SOURCE arrival validation applies the same per-carrier ceiling. Floor
  and 65_536-hash cap unchanged (hash lists still ride the control lane).

**Files**:

- `crates/blit-core/src/remote/transfer/payload.rs` — `ResumeFile` on both
  payload enums; prepare pass-through; sort/count arms.
- `crates/blit-core/src/remote/transfer/resume_diff.rs` — NEW: the shared
  block-diff iterator.
- `crates/blit-core/src/remote/transfer/sink.rs` — `DataPlaneSink` sends
  the record (the real implementation); `FsTransferSink`/`NullSink`/both
  gRPC sinks reject the composite.
- `crates/blit-core/src/remote/transfer/pipeline.rs` — progress arm;
  `MAX_WIRE_BLOCK_BYTES` pub(crate); test-sink arm.
- `crates/blit-core/src/remote/transfer/source.rs`,
  `remote/push/client/mod.rs`, `remote/transfer/data_plane.rs` — explicit
  rejection arms on paths the composite never travels.
- `crates/blit-core/src/remote/transfer/session_client.rs` — resume
  options on `PushSessionOptions`/`PullSessionOptions` →
  `SessionOpen.resume`.
- `crates/blit-core/src/transfer_session/mod.rs` — grant un-suppression;
  send-loop routes ready pairs to `dp.queue` (peer-fault-preferring, as
  plain batches); per-carrier arrival validation + DEST clamp; shared
  resume state threading; in-stream helper on the shared diff.
- `crates/blit-core/src/transfer_session/data_plane.rs` — `ResumeHeaders`/
  `ResumeRecv`; `NeedListSink` resume validation + claims.
- Docs: `docs/TRANSFER_SESSION.md` (resume-on-data-plane transport note),
  `docs/plan/OTP7_RESUME.md` (D5 amendment, 7b staging split),
  `docs/DECISIONS.md` (D-2026-07-10-2).

**Tests** (suite 1540 → 1545):

- Roles suite (both initiator assignments, loopback data planes):
  `resume_over_the_data_plane_moves_only_the_changed_blocks` (plan pin 1 on
  the new carrier; plain file rides along so file + block records coexist),
  `resume_over_the_data_plane_stale_partial_falls_back_to_full_content`
  (D1 pin + zero-block complete on the wire).
- Daemon e2e: `push_session_resumes_partial_over_the_data_plane`,
  `pull_session_resumes_partial_over_the_data_plane` (real gRPC-served
  sessions, byte counts pin "only stale blocks move").
- Unit: `need_list_sink_enforces_the_resume_grant_contract` (ungranted /
  overrun / wrong-size / duplicate fault; mid-record blocks don't claim;
  completion claims + counts once; resume-flagged file delivery refused).
- Guard proofs by temporary revert: (a) neutered `ResumeBlockDiff`
  staleness (send everything) → both moves-only-changed-blocks pins FAIL
  (in-stream + data plane — proves the refactor kept the 7a pin live);
  (b) re-suppressed grant → both data-plane resume pins FAIL on
  `in_stream_carrier_used`. Restored; full suite green.

**Known gaps**:

- Session-wide block size only; per-file auto-scaling for partials past
  the hash cap (>4 TiB at 64 MiB blocks) stays future work — such
  partials degrade to the D1 full transfer (D-2026-07-10-2 notes this).
- The D4 CLI end-of-op fault summary and cancel-during-resume e2e are
  otp-7b-2 (next pass), per the staging split recorded in the plan.
- Send-side `SinkOutcome.files_written/bytes_written` for a resume record
  counts stale bytes only; the send-side totals are discarded by the
  session driver today (the DEST is the scorer), so this is cosmetic.
- On a violation inside `NeedListSink::claim_block_complete` the grant is
  consumed before the size check faults; harmless (the session aborts on
  the violation) but noted for the reviewer.
