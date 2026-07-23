# Terminal data-plane attribution

**Status**: Draft
**Created**: 2026-07-23
**Supersedes**: nothing
**Decision ref**: pending owner approval

## Goal

Make the telemetry already collected by every live TCP SOURCE stream survive
short-transfer shutdown. When an explicitly traced transfer ends before the
500 ms dial sampler produces a tick, its terminal session record must still
report aggregate payload bytes and aggregate time spent awaiting payload socket
writes. Combined with the existing first-write, completion, and membership
timeline, that record must distinguish a socket/backpressure-bound run from
time spent outside socket writes well enough to choose the next investigation
for 10, 25, and 40 Gb/s links. This plan changes observation only and requires
no new hardware transfer.

## Non-goals

- No Thunderbolt, Ethernet, SSD, RAM-disk, rsync, iperf, payload, repeat, or
  comparison run. A later physical validation requires a separate exact owner
  approval after this instrument is accepted.
- No stream-count, dial, buffer, prefetch, socket, carrier, planner,
  filesystem, retry, or transfer-policy change.
- No new source-read, destination-read, destination-write, scheduler, kernel,
  or process CPU timing. A low terminal blocked-write fraction identifies the
  remaining non-socket class; dividing that class is a later evidence-driven
  plan if needed.
- No release, packaging, tag, publication, version, wire, proto, public CLI,
  metrics, persisted-history, or always-on telemetry change.
- No claim that `write_blocked_nanos` is pure kernel blocking. It retains its
  existing meaning: elapsed time awaiting payload `write_all` calls.

## Constraints

- The large-file data-path files have no diff from the fast candidate
  `d1f1152d` through plan-draft HEAD. The 35.578 Gb/s and 19.153 Gb/s RAM
  observations therefore do not identify a product-code regression.
- Reuse the existing `LiveProbe` counters for `bytes_sent` and
  `write_blocked_nanos`. Add no payload-loop clock read, atomic update, buffer
  touch, log call, or measurement branch.
- Preserve a separate diagnostic-only clone of each initial and ADDed SOURCE
  stream probe only when `BLIT_TRACE_SESSION_PHASES` is active. Trace-off
  construction and the payload hot path must retain their current behavior.
- The existing live `StreamProbeRegistry` remains the sole membership and dial
  authority. Terminal retention may observe a probe but must never register,
  unregister, settle, resize, keep a task alive, or affect worker eligibility.
- Snapshot retained probes only after the elastic send pipeline has joined, so
  terminal counters are final. Counter mismatch, serialization failure, or
  writer failure is diagnostic-only and cannot alter the transfer result.
- Extend the existing schema-1 session-phase vocabulary without changing its
  prefix or required fields. Emit one SOURCE-only `dial_terminal_sample` with
  `sample_bytes`, `sample_blocked_ns`, `sample_streams`, final/peak stream
  counts, and `sample_valid`. Do not invent a `blocked_ratio` inside the
  product: analysis derives stream-time capacity from the existing monotonic
  phase and membership timeline.
- `sample_valid` is true only after successful pipeline completion when the
  probe byte sum equals the successful send outcome. Zero-byte and mismatch
  records remain visible but invalid; they do not fail or retry the transfer.
- SOURCE-initiator and SOURCE-responder layouts use the same retention,
  aggregation, event, and validity code. Connection topology may not enter the
  calculation.
- Tests use captured events and synthetic counters, never wall-clock sleeps or
  hardware timing. New behavior guards receive red/restored-green mutation
  proof before acceptance.
- Build and test scratch should be RAM-backed where practical. No SSD payload
  allocation is authorized. Formal review, if selected by the repository's
  risk policy, uses Claude Opus 4.8/max; Fable is not used.

## Acceptance criteria

- [ ] A short successful TCP session that produces no periodic `dial_sample`
      emits exactly one SOURCE `dial_terminal_sample` after all send workers
      finish and before the SOURCE `data_plane_complete` record.
- [ ] The terminal record's payload bytes and blocked-write nanoseconds equal
      the saturating sum of every initial and ADDed stream's final existing
      `LiveProbe` counters. Its stream count names all probes included in that
      sum; final and peak membership retain their existing meanings.
- [ ] Terminal retention is independent of live membership. Normal completion,
      REMOVE, and terminal ADD/END may unregister probes from the live registry
      without erasing their final diagnostic counters or changing settlement.
- [ ] The record is valid only when successful probe bytes equal successful
      send-outcome bytes. Mismatch and zero-byte cases emit an invalid record
      without changing the returned result or creating a retry.
- [ ] Trace-off sessions emit no terminal sample and allocate no retained-probe
      collection. The payload copy loop, `Probe` implementations, tuner policy,
      and live registry have no new work or decision input.
- [ ] Deterministic captured-event tests cover exact aggregate math,
      short-session terminal emission, trace-off silence, mismatch invalidity,
      initial-plus-ADD retention, REMOVE retention, and identical
      SOURCE-initiator/SOURCE-responder semantics.
- [ ] Every new guard is mutation-proved by temporarily reverting its
      production behavior, observing the targeted guard fail, restoring it,
      and observing it pass.
- [ ] `cargo fmt --all -- --check`, strict workspace Clippy, the full workspace
      suite, `bash scripts/agent/check-docs.sh`, and `git diff --check` pass
      without reducing the prior test baseline.
- [ ] Product changes are confined to terminal observation and its tests. No
      transfer policy, payload framing, proto, result, progress, filesystem,
      or error/cancellation contract changes.
- [ ] The implementation, mutation proof, selected review record, plan/state
      closure, and any admitted review fixes are committed one coherent slice
      at a time. Nothing is pushed, tagged, published, or run on hardware
      without separate exact owner approval.

## Design

### Preserve existing counters

`SourceDataPlane` receives an optional terminal-probe collection only when its
bound session-phase trace exists. Construction adds a cheap clone of each
epoch-0 `StreamProbe`; accepted ADD construction adds the new probe through the
same helper. The collection is not the live registry and is never consulted by
the tuner or elastic membership code. A clone retains only the counter
allocation and cannot keep a socket, worker, queue, or task alive.

After `SourceDataPlane::finish` joins the elastic pipeline, fold the retained
probe snapshots with saturating arithmetic. Compare aggregate probe bytes with
`ElasticPipelineOutcome.outcome.bytes_written`, then emit one
`dial_terminal_sample` through the existing bound phase trace. Emit
`data_plane_complete` afterward as today. The terminal record uses the existing
optional schema-1 sample and membership fields; it carries no calculated ratio
and has no effect on success or failure.

### Interpretation boundary

For a session whose membership is fixed, offline analysis divides terminal
`sample_blocked_ns` by the first-socket-write-to-data-plane-complete duration
times the stream count. For a resized session, the existing monotonic
`dial_settlement` timeline supplies the membership-time integral. A high
fraction means the SOURCE workers spent most available stream time awaiting
socket writes, directing the next work to the receiver/network side. A low
fraction proves material time was outside socket writes and directs the next
observer to source reads, queueing, or scheduling. The terminal record alone
must not claim which member of that second class dominates.

### Affected code and tests

- `crates/blit-core/src/transfer_session/data_plane.rs` — optional retained
  probes, exact terminal fold, validity comparison, and SOURCE terminal event.
- `crates/blit-core/src/remote/transfer/session_phase.rs` — reuse the existing
  optional sample fields for the closed `dial_terminal_sample` event; add no
  required schema field.
- `crates/blit-core/src/remote/transfer/progress.rs` only if a small pure helper
  beside `StreamProbe` is the narrowest way to expose saturating terminal
  aggregation. The live counters and `Probe` hot-path contract do not change.
- Existing `blit-core` data-plane and role tests — captured event order,
  counter math, validity, trace-off silence, membership retention, and role
  parity.

## Slices

1. **tdp-1 — terminal telemetry.** Retain diagnostic probe clones only for
   traced SOURCE sessions, aggregate their existing final counters after the
   pipeline join, emit the validity-checked terminal sample, add deterministic
   role/membership guards, and mutation-prove each new behavior.
2. **tdp-2 — verification and review closure.** Run the complete repository
   gates, adjudicate any risk-selected review findings one per commit, record
   exact accepted heads and mutation evidence, then close the plan and current
   state without performing a hardware transfer.

## Open questions

- None. Hardware validation is deliberately outside this plan and remains
  blocked on a fresh exact owner approval.
