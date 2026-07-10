# otp-7b-1 — codex verdict adjudication

Reviewer: gpt-5.6-sol (codex exec, read-only, ultra effort) — the
config.toml default moved from gpt-5.5 since the loop doc was written;
signed as observed. Raw output: `otp-7b-1.codex.md`. Codex VERDICT:
FAIL (6 findings). Adjudicated against source per the loop's step 5.

- **F1 (High) — silent hash scan vs the receiver's StallGuard:
  ACCEPTED, fixed.** Real: a mostly-matching large partial produces no
  socket traffic while the source reads+hashes (`spawn_receive` guards
  the dest socket with `TRANSFER_STALL_TIMEOUT` = 30 s), so a healthy
  resume could abort. (The exposure technically predates the session —
  the old data-plane resume had the same shape — but the slice's
  StallGuard invariant makes it ours.) Fix: `ResumeBlockDiff` gained
  armable keepalive ticks (`ResumeDiffEvent::KeepAlive`, emitted when
  stall/3 passes without a stale block); `DataPlaneSink` answers each
  with a zero-length `BLOCK` record — a no-op in-place write the
  receive path already handles — so the socket shows liveness. The
  in-stream carrier stays unarmed (no stall guard on the control
  lane). Unit-pinned (`all_matching_scan_emits_keepalives_when_armed`).
- **F2 (High) — block writes not flushed: ACCEPTED — already fixed in
  `071799a`** (otp-7b-2), where this slice's own validation gate
  independently surfaced it as a ~50% full-suite flake of the 7a
  mid-resume pin. See `otp-7b-2-fault-summary.md`.
- **F3 (High) — bounded `dp.queue()` not raced against control events:
  DEFERRED.** The shape is pre-existing since otp-4b — the plain-batch
  `dp.queue` at the same loop is identical and rode through the
  otp-4b-3 cancel review; cancel today propagates via the transport
  break (dest teardown → worker send error → queue error →
  `prefer_peer_fault` picks the framed CANCELLED), pinned by BOTH
  cancel e2es (file records otp-4b-3, resume otp-7b-2). The new long
  silent-scan window this slice added is bounded by the F1 keepalive
  (a worker touches its socket at least every stall/3, so a torn-down
  session surfaces promptly). The general queue/event race restructure
  is filed in STATE's unowned-residue list.
- **F4 (Medium) — resume-only batches never propose resize: ACCEPTED,
  fixed.** `maybe_propose_resize` now runs before the resume queue,
  same as plain batches (the shape totals already counted resume
  needs at `ResumeNeed` arrival).
- **F5 (Medium) — per-worker 64 MiB diff buffer outside BufferPool:
  REJECTED as blocking, recorded.** The worst case requires the user
  to explicitly request the 64 MiB ceiling AND a receiver advertising
  a high stream count; the default block size is 1 MiB (32 workers →
  32 MiB) and the buffer is transient per resume file. Recorded in
  the finding doc's Known gaps; pool integration can ride a later
  slice if a real workload hits it.
- **F6 (Low) — 64 MiB data-plane ceiling unpinned: ACCEPTED, fixed.**
  New pin `resume_data_plane_honors_block_sizes_above_the_in_stream_
  ceiling` (4 MiB blocks honored on the data plane; guard-proven: with
  the per-carrier ceiling reverted to 2 MiB the pin fails at
  2 MiB ≠ 4 MiB moved).

Fix sha: (appended after the fix commit) — see below.
Suite after fixes: 1548 → 1550 (both new pins; none dropped).
