# ldt-1 — acknowledged elastic membership

**Slice**: `LIVE_DIAL_TUNING` ldt-1. The active plan requires the SOURCE
pipeline to settle each resize from the exact worker membership that actually
took effect, including payload-closing races.

**Status**: In progress — implementation and coder guard proofs are green;
neutral Claude Fable 5/max review is pending.

**Branch**: `master`

**Commit**: candidate commit pending

## What

The elastic pipeline previously accepted one-way `Add`/`RetireOne` commands.
The SOURCE could settle its dial without knowing whether the requested worker
joined, which worker retired, whether the matching telemetry probe changed, or
whether a closing pipeline had already ended that worker. ADD delivery failure
was silently cleaned up and reported as success. Probe storage was positional,
so it could not prove exact non-tail removal.

## Approach

- Replace the fire-and-forget control sender with one non-cloneable
  `ElasticPipelineControl`. ADD and REMOVE await typed `MembershipOutcome`
  replies containing the exact `StreamId` and authoritative logical count;
  `Seal` is ordered and idempotent.
- Give each worker a readiness/admission gate. The supervisor waits until the
  task is running, registers its keyed probe, releases the worker under the
  sampler's registry mutex, and only then acknowledges a live join. A terminal
  ADD runs the ordinary no-payload `finish()`/END path on a private disconnected
  queue and returns `JoinedThenEnded`.
- Keep a supervisor-local LIFO ledger keyed by `StreamId`. REMOVE marks and
  signals that exact worker, unregisters that exact probe under the same sampler
  mutex, and returns `RetireMarked`. If the worker had already ended but its join
  was not reaped, settlement waits for that exact join and returns
  `AlreadyEnded`; it never retires a second worker.
- Preserve the first pipeline error, fail outstanding membership replies, and
  make command-delivery or acknowledgement loss observable errors. Rejected
  unadmitted sinks receive exactly one `finish()` cleanup.
- Give both `SourceDataPlane` constructors the same identified-member pipeline.
  Fresh ADD sockets receive monotonic IDs, exact membership counts are checked
  before dial settlement, payload closing sends ordered `Seal`, and final
  logical membership is cross-checked against the settled dial.

## Files changed

- `crates/blit-core/src/remote/transfer/pipeline.rs` — acknowledged control,
  member ledger, admission gate, exact LIFO retirement, terminal outcomes,
  first-error handling, and deterministic guards.
- `crates/blit-core/src/remote/transfer/progress.rs` — keyed, duplicate-safe
  `StreamProbeRegistry` and shared registry type.
- `crates/blit-core/src/dial.rs` — tuner samples the keyed registry while the
  existing fixed-probe entry point remains source-compatible.
- `crates/blit-core/src/transfer_session/data_plane.rs` — symmetric identified
  initial members, monotonic ADD identity, acknowledged admission, ordered seal,
  and final count check.
- `crates/blit-core/src/transfer_session/mod.rs` — settle accepted ADD only from
  a join outcome whose logical count equals the accepted target.
- `crates/blit-core/src/remote/transfer/mod.rs` — export the acknowledged
  membership and registry APIs.

## Tests and guard proof

- `elastic_busy_retire_acks_exact_lifo_member_at_payload_boundary`: exact busy
  LIFO member retires after its claimed payload and all payloads remain
  exactly-once. Mutating LIFO selection from last to first failed on member 10
  versus expected member 99; restoration passed.
- `elastic_idle_retire_wakes_exact_member_and_unregisters_its_probe`: exact idle
  member wakes, emits one END, and only its probe leaves. Mutating unregister to
  the next ID failed closed with a lost-probe error; restoration passed.
- `sealed_running_remove_is_retire_marked_not_already_ended`: a sealed but still
  running worker reports `RetireMarked`, distinct from a completed worker's
  `AlreadyEnded`. Mutating the marked result to `AlreadyEnded` failed on the
  exact outcome; restoration passed.
- `elastic_add_after_end_of_stream_just_finishes_the_sink`: terminal ADD returns
  `JoinedThenEnded`, emits one END, and a later REMOVE of that same logical
  member returns `AlreadyEnded` without another finish.
- `membership_delivery_failures_are_errors_and_finish_unadmitted_sink_once`:
  closed-command and lost-ack paths cannot fabricate membership. Mutating the
  closed-command path to return `Joined` failed immediately; restoration passed.
- `first_error_slot_keeps_the_original_failure`: overwriting instead of
  preserving the first error failed; restoration passed.
- `registry_refuses_duplicate_without_replacing_existing_probe`: replacing a
  duplicate probe failed; restoration passed.
- `registry_unregisters_exact_non_tail_member`: removing ID 30 for requested ID
  20 failed; restoration passed.
- Focused elastic, telemetry, and role guards pass. Full repository gates pass:
  `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace`.

## Known gaps

- This slice deliberately does not start the production live tuner or attach
  `LiveProbe` to production `SourceDataPlane` members. Both constructors still
  pass an empty registry and unprobed `SinkMember`s; ldt-2 owns that symmetric
  cutover.
- The session wire still proposes only ADD and the static shape ramp remains
  worker-count authority. Shared ADD/REMOVE validation, telemetry-driven
  proposals, receiver-bounded startup, and deletion of shape authority are
  ldt-2.
- Broader accepted/unaccepted terminal-race, cancellation, observer, and final
  logical-versus-peak proofs remain ldt-3. No endpoint, SSH, or hardware test is
  part of ldt-1.

## Reviewer comments

Pending neutral Claude Fable 5/max review of the exact committed candidate.
