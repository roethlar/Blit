# ue-r2-1g — review adjudication

**Slice commit**: `48e583e`
**Fix commit**: `4a2e58d`
**reviewer: gpt-5.5** (codex, raw output `.review/results/ue-r2-1g.codex.md`)
plus a 3-lens adversarial self-review panel (Claude subagents:
concurrency / compat / reliable) run alongside codex; its findings are
attributed separately below — it is a coder-side self-check, not the
loop's reviewer.

codex VERDICT: **NEEDS FIXES** (2 findings). Panel: 3 further
candidates, of which 2 accepted, 1 deferred (its other lens output
independently re-derived codex F2 and the gate-safety proof).

## codex findings

1. **Medium — cancellation-mid-transfer untested for multistream
   PullSync** (`remote/pull.rs` new test mod). **Accepted.** The
   absorbed `MULTISTREAM_PULL.md` criteria name "cancellation
   mid-transfer" among the required new multistream tests; the slice
   shipped clean-end + per-stream-failure pins only, and the existing
   `abort_on_drop_tests` are wrapper-generic (no live sockets).
   **Fixed** (`4a2e58d`): `cancellation_aborts_all_stream_workers` —
   2 authenticated live TCP streams, the receive future dropped via
   the same `AbortOnDrop` shape production uses, stub asserts BOTH
   sockets observe teardown (TCP-level observability).

2. **Low — conservative arm skips dial bookkeeping**
   (`pull_sync.rs:384`). **Accepted.** `negotiated_pull_streams`'s
   no-profile/unknown arm returned 1 without
   `set_negotiated_streams`, leaving the dial's constructor floor (4)
   as the recorded stream count — stale baseline for `ue-r2-2` resize
   and contrary to the finding doc's "recorded on the dial" contract.
   **Fixed** (`4a2e58d`): that arm returns
   `dial.set_negotiated_streams(1)`; both conservative-arm unit tests
   now assert `dial.initial_streams() == 1`.

## Self-review panel findings

3. **Low — client never enforces its own advertised stream ceiling**
   (`remote/pull.rs:455`; concurrency + reliable lenses converged).
   **Accepted.** `negotiation.stream_count.max(1)` had a floor but no
   cap: a hostile/buggy daemon advertising 100k streams would drive
   100k worker spawns + TCP connects. Pre-existing arm (deprecated
   Pull exercised it), but this slice makes it the mainline supported
   path, and REV4 Design §4 requires "the weak end protects itself in
   both directions". **Fixed** (`4a2e58d`): `bounded_stream_count`
   clamps to `local_receiver_capacity().max_streams` (the ceiling this
   client advertises); structural for honest peers (all committed
   daemons propose ≤ 16 < 32). Unit test pins floor + ceiling.

4. **Low — harvest changed the full-file token-mismatch gRPC code**
   (UNAUTHENTICATED → PERMISSION_DENIED; compat lens). **Accepted.**
   Undocumented wire-visible delta, and internally inconsistent with
   the resume path in the same file. A bad token is a credentials
   failure — UNAUTHENTICATED is correct. **Fixed** (`4a2e58d`): the
   harvested helper returns `Status::unauthenticated`, restoring
   pull_sync's pre-slice code exactly; the delta moves to the
   deprecated Pull path (no consumer keys on it; dies at `1h`).
   Verified: no committed client or middleware distinguishes the two
   codes (retry classification is io::ErrorKind-based).

5. **Low — sequential accept grows the worst-case bounded handler pin
   ~N×** (trickling client ≈ 12 min at 16 streams vs ≈ 45 s;
   concurrency lens). **Deferred.** Still bounded, precedented (the
   deprecated Pull path always had this exact shape), other RPCs
   unaffected (handler is a detached spawn). Filed to the W1
   socket-policy/timeout design-queue row; noted in the finding doc's
   Known gaps.

## Cross-checks recorded for the record

- Gate safety proven from git history (compat lens + coder
  independently): the client fan-out (`69d8599`, 2025-11-15) predates
  any client sending `receiver_capacity` (`a0d2c9f`, 2026-07-03) —
  no committed client can be stranded by the profile-presence gate.
- Old-client/new-daemon success-path negotiation bytes are
  field-for-field identical to pre-slice; new-client/old-daemon takes
  the untouched single-stream arm.
- Fail-whole verified end-to-end on both ends (client `join()??` +
  daemon pipeline first-error-wins); mirror purge unreachable after a
  failed transfer.
- e2e non-vacuity: marker exists at exactly one source location inside
  the >1-stream branch; revert-proof run recorded in the slice work
  (gate off → test fails, gate on → passes).

Validation after fixes: fmt/clippy clean, tests **1413 / 0 / 2**
(entering baseline 1403; +8 slice, +2 review).
