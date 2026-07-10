# otp-7a-resume-in-stream — codex review adjudication

**Commit reviewed**: `4e5ff58` (otp-7a: resume block phase over the in-stream carrier)
**Raw review**: `.review/results/otp-7a-resume-in-stream.codex.md` (VERDICT: FAIL, 6 findings)
**reviewer: gpt-5.5** (codex CLI reports model `gpt-5.6-sol`; provider headroom)

## Findings

1. **High — resume frames unbounded vs tonic's 4 MiB limit; block_size=1
   amplification (mod.rs). Accepted.** Verified: no
   `max_decoding_message_size` anywhere in the workspace, so the served
   in-stream carrier really does have the default limit, and the D5 clamp
   (64 MiB) allowed frames 16× over it; a 1-byte block size hashed a
   partial into a 32×-sized list. Fixed (D-2026-07-10-1, plan D5 amended):
   DEST clamps block size into [64 KiB, 2 MiB]; one BlockHashList capped
   at 65_536 hashes with the over-cap partial degrading to the empty-list
   full-transfer fallback (D1); SOURCE range-validates at arrival. Pinned:
   `resume_block_size_floor_clamps_tiny_requests` +
   `..._ceiling_clamps_oversized_requests` (guard-proven by removing the
   clamp — both fail) + `resume_hash_list_cap_boundary` (pure-fn cap
   boundary; a live fixture would be 4 GiB).
2. **Medium — hash lists accumulate in `resume.ready` before payloads may
   begin. Accepted in part.** Real observation; with F1's cap each list is
   ≤ 2 MiB, so the aggregate is O(resume-flagged needs) — the same shape
   as the existing `pending: Vec<FileHeader>` accumulation, now bounded
   per item. Sending earlier is impossible (payload records may not
   precede the source's ManifestComplete — contract). Residual aggregate
   bound documented in the finding doc's Known gaps; revisit at 7b where
   the data plane changes the buffering picture.
3. **Medium — FileBegin/tar records could claim resume-granted paths;
   `resume_headers` unchecked at SourceDone. Accepted.** Verified real:
   `outstanding.remove` succeeded for resume paths, so a whole-file record
   bypassed the hash choreography. Fixed: both record arms reject
   resume-flagged paths as violations, and SourceDone verifies
   `resume_headers` is empty (belt-and-braces). Pinned:
   `file_record_for_resume_flagged_path_is_protocol_violation`
   (guard-proven: with the check removed the destination absorbs the
   record and hangs — the pin now fails on a bounded timeout).
4. **Medium — hash phases not cancellation-aware. Deferred.** Accurate
   observation, but it is the session's existing payload-phase model, not
   a 7a regression: `send_payload_records` streams a whole batch without
   draining events, and the destination's diff chunks / mirror pass are
   equally non-cancellable `spawn_blocking` work; the cancel-race
   machinery (otp-4b-3) is data-plane-specific by design. Carried into
   otp-7b's scope (plan Staging edited): cancel-during-resume e2e in the
   daemon harness.
5. **Low — oversized block_size validated only at send time. Accepted.**
   Fixed: the range check moved to `BlockHashList` arrival
   (`process_source_event`), so an invalid frame faults before pending
   plain files transmit; the send helper now trusts the validated value.
6. **Low — two pins infer rather than observe. Accepted in part.**
   (b) accepted: the mid-fault pin now proves the fault was genuinely
   mid-record — it asserts block 0 landed in the partial and byte `bs` is
   untouched. (a) rejected: `bytes_transferred` on the in-stream carrier
   counts exactly the block payload bytes written, so zero bytes with a
   completed `files_resumed=1` IS the "zero blocks transferred" mandate's
   observable, and guard proof A (neutered diff) demonstrated the pin
   trips; a frame-count observation would require transport interception
   for no additional discrimination.

Fix sha: (appended after commit)
