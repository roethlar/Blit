# otp-7a — resume over the in-stream carrier

**What**: The unified session now executes the resume block phase
(`docs/plan/OTP7_RESUME.md`, Active per D-2026-07-09-1) over the in-stream
carrier. A `SessionOpen` with `ResumeSettings{enabled}` is accepted (the four
otp-7 refusal stubs are gone); the DESTINATION flags resume-eligible needs
(plan D2: non-empty regular-file partial + compare says transfer), sends each
flagged need's `BlockHashList` right after its `NeedBatch`, and applies
`BlockTransfer`/`BlockTransferComplete` records in place; the SOURCE holds a
resume-flagged need until its hash list arrives (the contract's strict
ordering), then Blake3-diffs the live file block-by-block and sends only
stale blocks. `TransferSummary.files_resumed` is now real.

**Approach** (choreography and decisions transcribed from the plan doc):

- **Both open validators un-stubbed**; `resume_negotiated()` is the one
  reading of the flag. Invariance (D6): the flag lives in the open, so both
  initiator layouts run identical halves — pinned by the role suite.
- **Responder grants no data plane when resume is negotiated** (7a): block
  records exist only as control-lane frames until otp-7b ports them onto the
  data plane. The grant is always the responder's, so one suppression covers
  push- and pull-shaped layouts. Defensive fail-fast in the send half if a
  resume session somehow holds a plane.
- **SOURCE**: `SourceEvent::{ResumeNeed, BlockHashes}`; `ResumeSendState`
  holds needs awaiting hashes (`held`) and correlated pairs (`ready`).
  `NeedComplete` with a still-held need is a protocol violation (ordered
  lane: every list precedes NeedComplete). `send_resume_block_records` is
  the plan-D3 free helper: sequential read at the DEST-chosen block size
  (wire-validated ≤ `MAX_BLOCK_SIZE`, 0 ⇒ default), Blake3 per block, send
  iff index-beyond-list or hash differs (malformed hash entry ⇒ differs —
  D1: stale/garbage hashes degrade to sending, never trusting); EOF-short
  aborts exactly as whole-file records do; ends with
  `BlockTransferComplete{total_bytes = header.size}`.
- **DESTINATION**: `destination_needs` widened from bool to `NeedVerdict`
  (Skip / Transfer{resume_eligible}); `diff_chunk_and_send_needs` emits
  `NeedEntry{resume}` + post-batch `BlockHashList`s
  (`compute_resume_block_hashes`, blocking pool, containment-checked path,
  vanished file ⇒ empty list ⇒ full transfer). Records are claimed
  fail-fast (`claim_resume_record`: resume session only, control lane only,
  post-ManifestComplete, granted-with-resume only, exactly once — the
  `outstanding` completion set is claimed here, so the SourceDone
  every-need-delivered check covers resume records too). A record may open
  with `BlockComplete` directly (zero stale blocks). Completion validates
  `total_bytes == header.size` and finalizes via the existing sink
  (`write_file_block_payload`/`write_file_block_complete` — truncate +
  fsync + mtime/perms stamp from the retained manifest header).
  `Frame::Error` inside an open block record surfaces the peer's fault
  (D4), not a position violation.

**Files**:
- `crates/blit-core/src/transfer_session/mod.rs` — all of the above.
- `crates/blit-core/tests/transfer_session_roles.rs` — 6 new pins.
- `crates/blit-core/Cargo.toml` — `async-trait` added to dev-dependencies
  (the fault-injection `TransferSource` in the role suite).

**Tests** (all four plan guard-proof targets, run live):
- `resume_moves_only_the_changed_blocks` — 6-block file, 4 landed; exactly
  2 blocks' bytes move (plus a coexisting plain file record).
  **Guard proof A (run)**: neutering the diff (`stale = true`) fails this
  pin and the zero-blocks pin.
- `resume_identical_content_moves_zero_blocks_and_stamps_mtime` — mtime-only
  touch: zero payload bytes, `files_resumed = 1`, source mtime stamped
  (asserted on disk, both roles).
- `resume_stale_partial_falls_back_to_full_content` — no shared blocks ⇒
  full content lands + shrunk-to-empty source truncates; never an abort
  (D1/Q1). **Guard proof B (run)**: trusting stale hashes
  (`stale = index-beyond-list only`) fails byte-identity with corrupt
  output.
- `resume_ineligible_targets_are_plain_full_transfers` — absent + empty
  dest files: `resume=false`, `files_resumed = 0` (D2).
- `mid_resume_source_fault_surfaces_cleanly_to_both_ends` — a source
  reader truncated mid-block-2 ⇒ clean `SessionFault{Internal}` naming the
  file at BOTH ends, no deadlock, no summary (so no false `files_resumed`)
  (D4). **Guard proof C (run)**: removing the in-record `Frame::Error` arm
  fails the destination-fault assertion.
- `block_hashes_without_a_held_resume_need_fault_the_source` — uncorrelated
  hash list is a protocol violation.
- Existing `resume_flagged_need_is_refused_in_non_resume_session` still
  passes unchanged (the violation is now conditional on the open).

Added by the codex fix pass (see the verdict file):
- `resume_block_size_floor_clamps_tiny_requests` /
  `resume_block_size_ceiling_clamps_oversized_requests` — the
  D-2026-07-10-1 wire bounds, guard-proven by removing the clamp.
- `resume_hash_list_cap_boundary` (lib) — the 65_536-hash cap boundary as
  a pure-fn test (a live fixture would be 4 GiB).
- `file_record_for_resume_flagged_path_is_protocol_violation` — the
  choreography-bypass rejection (guard-proven: without the check the
  destination absorbs the record and hangs; the pin fails bounded).
- The mid-fault pin now also observes the partial patch (block 0 landed,
  byte `bs` untouched) — the fault is provably mid-record.

**Known gaps** (all deliberate, plan-scoped):
- Data-plane resume is otp-7b; resume sessions are forced onto the
  in-stream carrier by grant suppression.
- The CLI end-of-operation fault summary (D-2026-07-09-1 Q2 rider) is
  otp-7b's deliverable (plan Staging).
- No CLI/daemon wiring sets `SessionOpen.resume` yet (`session_client.rs`
  builds opens without it) — the flag is exercisable only via the session
  API and the role suite until 7b/e2e.
- Blake3 hashing runs inline on the async task at both ends' send/apply
  paths (1 MiB default blocks; matches the old gRPC path's pattern) — a
  perf pass belongs to the benchmark-gated slices, not 7a.
- Hash lists for correlated resume needs buffer in the source's
  `resume.ready` until ManifestComplete lets the payload phase run
  (payload-before-ManifestComplete is a contract violation, so earlier
  sending is impossible). Each list is ≤ 2 MiB after D-2026-07-10-1; the
  aggregate is O(resume-flagged needs) — same shape as the `pending`
  need vector, bounded per item (codex F2, partial). Revisit at 7b.
- Hash/block phases inherit the session's payload-phase cancel latency
  (codex F4, deferred): a queued peer fault is seen at the next event
  drain, exactly as for file-record batches. otp-7b's daemon e2e pins
  cancel-during-resume (plan Staging updated).
- Whole-file records inside `receive_file_record` still report a peer
  `SessionError` mid-record as a position violation (pre-existing); block
  records got the from-wire treatment (needed by D4's pin). Aligning file
  records is a candidate cleanup, not regressed by this slice.
- `files_resumed` counts also into `files_transferred` (the finalize
  reports `files_written = 1`), so `files_transferred` remains "files the
  session completed" across both record kinds.
