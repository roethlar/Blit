# ue-r2-2 — review adjudication

**Slice commits**: `042ca4b` (engine) + `ce0e396` (pipeline) +
`04c9c6d` (push) + `b08d80e` (pull/delegated) + `0788e83` (fmt +
wire-test fix + finding doc)
**Fix commit**: (sha in the records commit note below)
**reviewer: gpt-5.5** (codex exec, read-only, headroom proxy; raw
findings tail preserved in `.review/results/ue-r2-2.codex.md`). A
3-lens adversarial self-review panel (concurrency / wire-security /
invariants+tests, Claude subagents) ran in parallel; attributed
separately. Overlap between the two was high — every codex finding was
independently found by at least one panel lens.

codex VERDICT: **NEEDS FIXES** (3). Panel: 4 more real findings,
including a High neither codex nor I had.

## Adjudication (deduplicated)

1. **High — pull resize was DEAD on the primary CLI path** (panel
   wire-F1 + invariants-F1; `pull.rs:551`). The `build_spec_from_options`
   capability flip was one of two edits that failed with a
   file-modified error mid-session and only the other was re-issued —
   so every direct `blit pull` still advertised `false` and the
   daemon's fold never enabled resize; only delegated pulls got it.
   The finding doc's "every pull e2e negotiates resize" claim was
   false (the wire tests passed because they hand-build specs).
   **Accepted — fixed**: bit flipped with a review-catch note; the
   claim is now true.
2. **High — no cumulative stream bound at either acking end** (codex
   #1 + panel wire-F2; `push/control.rs`, `pull.rs` command arm).
   Per-command target checks let replayed ADDs with fresh credentials
   grow workers/sockets past the advertised `max_streams`.
   **Accepted — fixed**: `resize_live` (push daemon, seeded at
   negotiation, +1 per armed ADD — conservatively counts lapsed dials)
   and `data_plane_live` (pull client) refuse ADDs at the local
   ceiling; REMOVE decrements with a floor of 1.
3. **Medium — pull controller validated epoch-N sockets INLINE in its
   select** (codex #2 + panel concurrency-F5/wire-F3). A stray dial
   stalling the 15s token read froze pipeline-result propagation,
   acks, and expiry. **Accepted — fixed**: validation runs in a
   spawned task that settles its own epoch (handles aborted at
   teardown). Trade-off accepted and documented: the accept now
   consumes the armed slot, so a stray dial that beats the real one
   costs that epoch (settled refused) and the real socket degrades
   non-fatally client-side — bounded harm instead of a frozen
   controller.
4. **Medium — settle-accepted on a dead pipeline dropped an authorized
   socket without its END** (codex #3). **Accepted — fixed** on both
   controllers: pull's validation task and push's `add_stream` now
   close the just-authorized sink with a clean `finish()` (END) and
   settle refused when the pipeline is gone.
5. **Medium — an ADD racing end-of-transfer was transfer-FATAL**
   (panel concurrency-F1/F2 + invariants-F2), three interleavings:
   supervisor could break with a queued Add (unbiased select) → sink
   dropped END-less → peer's authorized worker fatal; client epoch-N
   worker post-connect errors were fatal even with zero bytes
   received; the finding doc claimed "non-fatal on both ends" for a
   race where that held only for the connect-refused variant.
   **Accepted — fixed**: the supervisor select is biased
   control-first AND drains `control_rx` after break (finishing any
   queued Add's sink); an OPTIONAL pull worker that dies having
   received nothing is non-fatal (real loss still surfaces
   sender-side); finding doc corrected.
6. **Low — tuner idle ticks bypassed the sustain reset** (panel
   concurrency-F3): the unit-tested "idle resets the streak" semantics
   were dead in the composed system; a streak could survive a 30s
   stall. **Accepted — fixed**: idle and re-baseline ticks now call
   `resize_tick(0, 0.0)`.
7. **Low — push acceptor's armed-expiry never woke the loop** (panel
   concurrency-F4): the accept gate could stay open long past the TTL
   on a quiet acceptor. **Accepted — fixed**: earliest-expiry
   `sleep_until` select arm (mirroring the pull controller).
8. **Low — push REMOVE retired before the ack and popped its probe
   before the pipeline accepted the retire** (panel invariants-F4):
   a refused accounting ack would diverge dial state from the real
   worker count. **Accepted — fixed**: REMOVE settles at retire time
   (the retire is fait accompli; the daemon's ack is accounting and
   now lands as unsolicited-by-design), and the probe pops only after
   `RetireOne` was accepted.
9. **Low — DEFERRED: push epoch-0 phase can fatally consume an early
   epoch-N socket** (panel wire-F4). Requires an ADD acked within
   ~2.5s (cooldown+sustain floor) while an epoch-0 dial still
   straggles — improbable at current constants. Fixing means servicing
   `arm_rx` during the epoch-0 loop; recorded in the finding doc's
   Known gaps as post-REV4 hardening rather than grown into this
   stack.
10. Cosmetic (panel): a lapsed optional ADD printed a misleading
    `0.00 Gbps` per-stream line — now skipped.

## Cross-checks recorded for the record

Verified clean by one or both reviewers: flume `RecvFut` cancel-safety
under the biased retire race (checked against flume source); tonic
`Streaming::message()` cancel-safety in all three new selects;
resize-OFF byte/behavior parity with 1g on both directions (one
bounded error-path-only delta noted: the retained `work_rx` clone lets
the forwarder run up to `capacity` further payloads when ALL workers
panic); mixed-version matrix (old peer never sees a suffix or a resize
frame; every `DataPlaneResize` construction site sits behind the
negotiated gate); credential lifecycle (epoch-0 sub never valid for
armed accepts, consume-on-match, TTL re-check at consume);
StallGuard on ADDed streams in all three legs; byte accounting
exactly-once across add/retire; the ack-before-arm race is absorbed
by registration-before-ack plus the OS backlog; R25-F2 delegated
override still pinned; +12 test count claim verified; the refusal
wire test proven non-vacuous.

Validation after fixes: fmt/clippy clean; tests (Windows host)
**1405 / 0 / 3** — the two new wire tests and ten policy/pipeline
tests from the stack included; full suite green with pull resize now
genuinely negotiated end-to-end.
