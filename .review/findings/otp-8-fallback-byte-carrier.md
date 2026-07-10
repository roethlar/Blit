# otp-8 — fallback byte-carrier (assessment + wire residue pins)

**What**: Plan slice otp-8 — "fallback byte-carrier (control-stream
frames) as the session's alternate transport." Executed as the handoff
directed: **assess what already exists before writing new code**. The
verdict: the slice's substance landed incrementally across otp-3..7b
and is already pinned; the genuine residue was two missing wire-level
pins (in-stream **resume** over the real gRPC transport), which this
slice adds. No production code changes.

**Assessment (what already exists, with the pin that proves it)**:

- **The carrier itself** — the in-stream payload record grammar
  (frames 9–15: file records, tar-shard records, resume block records)
  has been the session's fallback since otp-3 and is the default
  fixture of the whole in-process role suite
  (`transfer_session_roles.rs` — byte-identical, tar-shard,
  incremental, mirror, filters, resume, all across BOTH role
  assignments).
- **Selection at negotiation** (contract §Transport selection): via
  `SessionOpen.in_stream_bytes`, or granted by the responder when it
  cannot bind a data plane (grant-less `SessionAccept`) —
  `transfer_session/mod.rs` (`prepare_responder_data_plane` returns
  `None` on bind/RNG failure ⇒ no grant ⇒ both ends read in-stream
  from grant absence). There is deliberately NO mid-session carrier
  switch: selection is settled at negotiation (plan §Design "selected
  at negotiation — not a separate transfer path"), so a post-grant
  dial failure is a session fault, not a silent downgrade. The old
  drivers' mid-flight `tcp_fallback_used` semantic dies with them at
  otp-10.
- **Over the real wire, both directions** —
  `session_lands_bytes_over_in_stream_carrier` (push) and
  `pull_session_lands_bytes_over_in_stream_carrier` (pull) force the
  carrier over a daemon-served RPC and assert
  `summary.in_stream_carrier_used`.
- **Resume on the carrier** — otp-7a's role-suite pins (block diff,
  zero-block mtime stamp, stale-partial full-content fallback,
  floor/ceiling clamps) all run in-stream.
- **Summary reporting** — `TransferSummary.in_stream_carrier_used`
  populated in the session teardown, asserted on both carriers.
- **Client seam** — `PushSessionOptions.in_stream_bytes` /
  `PullSessionOptions.in_stream_bytes` (the `--force-grpc`-shaped
  option, session_client.rs).

**The residue this slice closes**: in-stream **resume** was pinned only
on the in-process transport. The in-stream block-size ceiling
(`MAX_IN_STREAM_RESUME_BLOCK_SIZE` = 2 MiB, D-2026-07-10-1) exists
because tonic's default 4 MiB frame decode limit applies to every
`Transfer` frame when the daemon serves — but the in-process transport
has no frame limit, so no existing test could falsify the ceiling
where it matters. Two e2e pins added:

- `push_session_resumes_partial_over_in_stream_carrier` — the exact
  fixture of the otp-7b data-plane twin, forced in-stream: the
  destination partial is patched block-wise over the RPC, only the 2
  missing blocks move, trees byte-identical, carrier flag set.
- `pull_session_resume_clamps_oversized_blocks_to_in_stream_ceiling` —
  roles flipped, and the clamp made observable: an 8 MiB block-size
  request over a 6 MiB file whose dest copy has ONE corrupt byte at
  offset 3 MiB must move exactly one 2 MiB block
  (`bytes_transferred` == 2 MiB). Unclamped, the single 6 MiB
  `BlockTransfer` frame is rejected by tonic's 4 MiB decode limit;
  any other effective block size moves a different byte count.

**Files**:

- `crates/blit-daemon/src/service/transfer_session_e2e.rs` — the two
  pins above (new otp-8 section after the otp-7b resume e2es).
- `.review/findings/otp-8-fallback-byte-carrier.md` — this record.

**Tests** (suite 1550 → 1552): the two e2e pins. Guard proofs by
temporary mutation (see the verdict file for the run evidence):

- `MAX_IN_STREAM_RESUME_BLOCK_SIZE` 2 MiB → 1 MiB: the clamp pin FAILS
  (1 MiB moves, not 2 MiB) — the assertion is sensitive to the exact
  ceiling, not merely to "some clamp happened".
- Ignoring the open's `in_stream_bytes` request (always grant): both
  pins FAIL on the carrier-flag assertion.

**Known gaps**:

- **CLI plumbing is deferred to otp-10 by design**: no CLI verb rides
  the session until cutover, so the `--force-grpc`-shaped flag wiring
  onto the session options lands with the verb switch
  (session_client.rs already carries the seam and documents exactly
  this). Not new debt; it is the plan's staging.
- The organic bind-failure path (`prepare_responder_data_plane` →
  `None`) is exercised for grant-absent handling by the forced-request
  tests (same accept-side path), but the bind failure itself is only
  log-covered — forcing a real `bind("0.0.0.0", 0)` failure in a test
  is not portably possible. Accepted as untestable-by-construction.
- Cancel/fault e2es remain data-plane-only over the wire; the
  in-stream fault path is pinned in the role suite
  (`mid_resume_source_fault_surfaces_cleanly_to_both_ends`,
  in-stream fixtures). Judged sufficient: the fault frames ride the
  control lane identically on both carriers.
