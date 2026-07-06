# otp-5b-1 — codex review adjudication

**Slice**: otp-5b-1 — single-stream SOURCE-responder TCP data plane
(`docs/plan/ONE_TRANSFER_PATH.md` slice otp-5; finding
`.review/findings/otp-5b-source-responder-data-plane.md`).
**Reviewed commit**: `e6a0b3b`.
**Reviewer**: gpt-5.5 (`codex exec -s read-only`, xhigh; raw output
`.review/results/otp-5b-source-responder-data-plane.codex.md`).
**Verdict**: codex FAIL — 1 Medium finding, adjudicated **Accepted** and fixed.
**Fix commit**: `13485ee`.

## Findings

### F1 — DESTINATION initiator: grant-without-host silently degrades (Medium) — ACCEPTED

`crates/blit-core/src/transfer_session/mod.rs:1735` (as reviewed). The
`None => match (&negotiated.accept.data_plane, data_plane_host)` block used a
`_ => (None, …)` catch-all, so a DESTINATION initiator that received a
data-plane grant (`accept.data_plane = Some`) but held `data_plane_host = None`
fell through to the in-stream branch. That is not a real fallback: the SOURCE
responder, seeing `!in_stream_bytes`, has already bound its listener and blocks
up front in `accept_source_data_plane()` waiting for this end to dial. The
DESTINATION would instead sit in its manifest loop while the SOURCE waited out
its 30s `DATA_PLANE_ACCEPT_TIMEOUT`, then faulted — a bounded hang, not the
fail-fast the contract's "the initiator dials when a grant exists" rule wants.

**Verified against source**: the SOURCE side already fails fast on the exact
symmetric inconsistency — `source_send_half` returns
`SessionFault::internal("responder granted a TCP data plane but this initiator
has no host to dial")` (`mod.rs:892`). The DESTINATION branch lacked the mirror.
Real, and worth fixing for symmetry + fail-fast.

**Fix** (`13485ee`): split the catch-all into `(Some(_), None)` → return an
INTERNAL `SessionFault` (mirroring the SOURCE guard), and `(None, _)` → the
in-stream carrier (a genuine no-grant). `(Some, Some)` dials as before.

**Test**: no dedicated test added — this is a defensive guard for an
inconsistent initiator config the production caller (`run_pull_session`) never
produces (it always pairs `in_stream_bytes=false` with
`data_plane_host=Some`), and the symmetric SOURCE guard is likewise untested. A
dedicated test would pair the misconfigured DESTINATION with a real SOURCE
responder and incur that responder's full 30s accept timeout inside the run.
The happy-path dial (grant+host) is covered by
`pull_data_plane_single_stream_lands_bytes` (roles) and
`pull_session_lands_bytes_over_the_data_plane` (e2e).

## Not-findings (checked, no issue)

- **Push path unchanged**: codex confirmed the DESTINATION-responder /
  SOURCE-initiator data plane is behaviorally unchanged (the bind gate now
  reads `!in_stream_bytes` for either role; the DESTINATION-responder still
  binds+accepts+receives, the SOURCE-initiator still dials+sends).
- **Resize suppression**: the SOURCE responder's `SourceDataPlane` is built
  with `resizable = false`, so `propose_resize` returns `None` and no
  `DataPlaneResize` frame flows; a DESTINATION initiator treats a `Resize` as a
  protocol violation. Confirmed correct.
- **Byte-accounting / scorer, StallGuard, AbortOnDrop, same-build handshake,
  in-stream fallback, test count (1519 → 1521)**: all confirmed intact.

Signed: reviewer gpt-5.5; adjudication by the coding agent against source.
</content>
