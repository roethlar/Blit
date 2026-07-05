# otp-3 — TransferSession core (role-parameterized, in-process)

**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-3.
**Status**: scoped — approach recorded 2026-07-05, implementation next.
**Contract**: `docs/TRANSFER_SESSION.md` (post-review, `f861579`).

## Scope (what otp-3 proves)

The role-parameterized session state machine exists in blit-core and
moves real bytes in-process with the roles swappable over the same
fixtures — the owner's invariance property enters the test suite
here. In otp-3 the byte carrier is the in-stream frame grammar only
(the TCP data plane + daemon serving land at otp-4); mirror, filters
beyond passthrough, resume, StallGuard/jobs wiring are later slices
per the plan.

## Approach (surveyed 2026-07-05)

- New module `crates/blit-core/src/transfer_session/`:
  - `transport.rs` — `FrameTransport` trait (`send(TransferFrame)`,
    `recv() -> Option<TransferFrame>`) + `in_process_pair()` built on
    bounded mpsc channels. otp-4 adds the gRPC-backed transport;
    otp-11 reuses the in-process one for local transfers.
  - `mod.rs` — `run_source(cfg, transport, Arc<dyn TransferSource>)`
    and `run_destination(cfg, transport, sink)` drivers, plus the
    shared hello/open/accept phase code (one implementation, both
    roles call it).
  - Hello: `session_build_id()` composed compile-time
    (`CARGO_PKG_VERSION` + `BLIT_GIT_SHA` emitted by blit-core's
    existing build script, fallback "unknown") + `CONTRACT_VERSION`
    const; exact-match check per contract, mismatch →
    `SessionError{BUILD_MISMATCH}` naming both ids.
- SOURCE driver: `TransferSource::scan` streams headers →
  `manifest_entry` frames (immediate start); need batches consumed
  incrementally; payloads planned via the existing
  `diff_planner::plan_push_payloads` on needed headers; in-stream
  records emitted per the contract grammar (file records:
  `file_begin` + `file_data`×N, completion at exactly `header.size`;
  tar records via the existing tar planner; payload records only
  after `ManifestComplete` per the carrier rule); `SourceDone`; await
  `TransferSummary`.
- DESTINATION driver: manifest entries diffed incrementally against
  the destination root using the `diff_planner::filter_unchanged`
  predicate (the existing single owner of compare_mode semantics —
  reused, not duplicated); `NeedBatch` emission with the engine's
  existing batching; `NeedComplete` only after ManifestComplete +
  all entries diffed (contract); in-stream records reassembled and
  written through `FsTransferSink::write_file_stream` (file records)
  and the existing tar-safety unpack path (tar records); summary
  computed destination-side.
- Tests (all in-process, both role assignments over the same
  fixtures — the suite runs each case twice via a role parameter):
  build-id mismatch refusal; small tree byte-identical; tiny-file
  tree (tar-shard records) byte-identical; incremental (pre-seeded
  destination) transfers only the need list; empty need list
  completes clean; protocol-violation fail-fast (payload record
  before ManifestComplete). Role-swap equality: for each fixture,
  the need-list set and summary counts must be IDENTICAL under both
  role assignments — the first executable form of the owner's
  invariant.

## Files (planned)

- `crates/blit-core/src/transfer_session/{mod.rs,transport.rs}` (new)
- `crates/blit-core/src/lib.rs` (module export)
- `crates/blit-core/build.rs` (BLIT_GIT_SHA emission)
- `crates/blit-core/tests/transfer_session_roles.rs` (new, the
  role-parameterized fixture suite)

## Known gaps (carried into implementation)

- Data plane, daemon serving, ActiveJobs/cancel, progress events:
  otp-4. Mirror: otp-6. Resume: otp-7. Delegated: otp-9.
- The in-process transport intentionally exercises the same frame
  grammar the wire will carry, so otp-4 is transport substitution,
  not new choreography.
