# ldt-2 — live controller cutover

**Slice**: `LIVE_DIAL_TUNING` ldt-2. Replace the static workload-shape worker
target with one SOURCE-owned telemetry controller that adjusts the same elastic
membership in both connection layouts.

**Status**: Accepted — all local guards/gates are green; neutral Claude Fable
5/max openreview returned one Low cleanup suggestion declined at intake because
it predicted no observable failure.

**Branch**: `master`

**Commit**: `65a0f9f0bb3225a2b81f8c668f6bda41545f5efa`

## What

Production still used the retired file-count/byte-count shape table as worker
authority, grew only through ADD, and constructed the two SOURCE socket layouts
separately. The nominal live tuner did not own `TransferSession` membership.
Epoch zero could therefore differ from the receiver-bounded design, and the
old exact-eight parity result proved only symmetry of a static target.

## Approach

- Route both SOURCE layouts through `start_source_data_plane`. Only socket
  acquisition is a `SourceSockets::{Dial,Accept}` branch; the dial, registry,
  pipeline, initial membership, tuner, proposal receiver, ID allocator, and
  settlement path are shared.
- Start epoch zero at the conservative floor clamped to the DESTINATION's
  resolved receiver safety ceiling. Treat zero capacity as unknown/default,
  not a one-stream limit.
- Attach the exact same `LiveProbe` allocation to every epoch-zero/ADD socket
  and its keyed pipeline member. The production sampler folds per-member byte
  and blocked-write deltas and rebaselines membership changes.
- Consume one live proposal stream. Validate and settle monotonic one-step ADD
  and REMOVE epochs from actual pipeline membership; ADD mints one fresh
  credential and socket, REMOVE retires one worker at a payload boundary.
- Delete `initial_stream_proposal`, `propose_shape_resize`, the workload need
  accumulators, and forced post-payload convergence. Workload shape remains a
  planning/carrier input only.
- Make final `data_plane_streams` the settled logical count rather than
  cumulative sockets opened. Stop and join the tuner before teardown.
- Correct the wire comments, session contract, active plans/state, historical
  exact-target records, dormant scale-harness labels, and neutral review
  workflow pointers in the same cutover.

## Files changed

- `crates/blit-core/src/dial.rs` — receiver-bounded floor/safety limit, live
  sampling and settlement, ADD/REMOVE policy, and removal of shape authority.
- `crates/blit-core/src/transfer_session/data_plane.rs` — one SOURCE
  constructor, exact live probes, one tuner, socket-layout-only branching,
  membership application, and prompt tuner shutdown.
- `crates/blit-core/src/transfer_session/mod.rs` — shared op-aware SOURCE and
  DESTINATION protocol state, actual logical counts, and deterministic
  two-layout session guards.
- `crates/blit-core/src/remote/transfer/{pipeline,data_plane,abort_on_drop}.rs`
  — admission barrier, probe identity check, resume-block telemetry, logical
  count inspection, and abort-plus-join lifecycle primitive.
- `crates/blit-core/src/remote/transfer/session_client.rs`,
  `crates/blit-core/src/transfer_session/local.rs`, and session role tests —
  explicit receiver profile plumbing and adaptive assertions.
- `crates/blit-daemon/src/service/transfer_session_e2e.rs` and
  `crates/blit-cli/tests/remote_parity.rs` — remove static worker-count claims
  while retaining carrier/integrity coverage.
- `proto/blit.proto`, `docs/TRANSFER_SESSION.md`, active plan/state/review
  records, and `scripts/bench_tripwires.sh` — exact live membership contract
  and historical-policy annotations.
- `.agents/repo-guidance.md`, `docs/agent/PROTOCOL.md`, `.review/README.md`,
  and `.agents/governance-inventory.md` — map the refreshed split workflow to
  this repo's owner-required neutral Claude openreview and guard gate.

## Tests and guard proof

- `live_dial_clean_trace_grows_both_layouts_through_seventeen` runs real paired
  sessions in both socket layouts and requires the identical 13-step ADD trace
  `4→17`, fresh 16-byte tokens, equal summaries/needed paths, and actual final
  membership. Production stream-ceiling mutations to 8 and 16 failed before
  restoration. Replacing either epoch-zero layout or an ADD socket with a
  distinct/no probe failed the exact probe/membership guard. Reusing one ADD
  token failed the distinct-token assertion.
- `live_dial_blocked_trace_shrinks_both_layouts_to_one` requires identical
  REMOVE `4→3→2→1` traces and data integrity. A no-op REMOVE and reporting
  initial rather than settled membership each failed before restoration.
- `live_dial_idle_and_hysteresis_traces_hold_both_layouts` rejects idle and
  in-band resizing. Treating idle as signal and raising the clean threshold
  into the hysteresis sample each failed before restoration.
- `receiver_bounds_seed_both_layouts_identically` holds an explicit ceiling of
  two and proves zero/default capacity can grow through 17. Treating zero as
  one, ignoring the low receiver bound, and clamping production at 8/16 each
  failed before restoration.
- `workload_shape_has_no_resize_authority_in_either_layout` transfers 10,000
  one-byte files under an idle trace and requires zero resize frames/final
  floor membership. A signal-producing mutation made the same capture path
  fail; restoration passed. The retired shape symbols are absent repo-wide.
- `production_tuner_folds_blocked_telemetry_into_remove` drives the real
  sampler on paused time. Dropping blocked-nanosecond deltas made the expected
  REMOVE disappear; restoration passed.
- `resume_block_records_bytes_and_socket_write_time` proves resume payloads
  feed both counters. Removing the blocked-time update left bytes nonzero and
  blocked time zero, turning the guard red; restoration passed.
- SOURCE ACK, epoch-zero grant, DESTINATION duplicate/stale/future/op/target/
  token/bound/refusal tests cover both sides of each exact target. Mutations
  accepting the wrong ACK count, bypassing the epoch-zero floor, relaxing an
  ADD target, replaying duplicates as new work, and making refusal nonterminal
  each failed before restoration.
- `abort_and_join_observes_task_cleanup_before_returning` proves cancellation
  cleanup is complete before teardown continues. Removing `abort()` made the
  focused test exceed an external eight-second bound; restoration passed all
  five wrapper tests.
- Four obsolete static-policy tests are intentionally removed:
  `initial_stream_proposal_matches_the_retired_daemon_table`,
  `shape_table_covers_the_small_file_ceiling_cells`,
  `shape_resize_ramps_one_epoch_at_a_time_toward_the_target`, and
  `shape_resize_clamps_to_the_profile_ceiling`. The library suite grows to 366
  tests despite those removals.
- Current local validation: focused mutations restored green;
  `cargo fmt --all -- --check`, strict workspace clippy,
  `cargo test --workspace`, `scripts/agent/check-docs.sh`, shell syntax, and
  diff checks pass. The final workspace run includes all 366 `blit-core`
  library tests, all 42 `transfer_session_roles` tests, 160 daemon tests, and
  649 TUI tests. The first workspace attempt had one daemon transport startup
  failure in `test_utils_df`; its exact filtered rerun and two complete
  workspace reruns passed.

## Known gaps

- ldt-3 owns accepted/unaccepted need-completion races in both layouts,
  cancellation/fault closure, default-off decision observation, peak-versus-
  final reporting, and the complete debug/release/CI matrix.
- Cheap chunk/prefetch/tcp dial values remain construction-time snapshots for
  existing sockets/queues. Future ADD sockets read current values, but the
  shared buffer pool retains its epoch-zero buffer size. This does not block
  live ADD/REMOVE or role parity, but its performance-policy fidelity needs a
  separate measured design rather than expansion inside ldt-2.
- ldt-4 remains the first endpoint/SSH rig-W `q`↔`netwatch-01` evidence slice. No endpoint,
  Time Machine, mount, daemon, network, benchmark, push, or deletion action is
  part of ldt-2.

## Reviewer comments

Claude Fable 5/max reviewed exact range
`602941f2aa1194b4fe12faa3b9c7d049f99c8343..65a0f9f0bb3225a2b81f8c668f6bda41545f5efa`
in retained detached worktree `/tmp/blit-openreview-ldt2-65a0f9f-r1` under the
neutral best-way question. The result was schema-valid with exact SHAs and
`guard_confirmed=true`.

Claude returned one Low candidate: `settle_inflight_resize` still accepts and
immediately discards an unused `FrameTx`. The evidence is correct, but the
predicted-failure field explicitly says “No runtime failure” and describes only
a possible future-reader misunderstanding. It is therefore DECLINED at intake
as style/maintainability without a current observable failure; no code change
is admitted. The slice has no material finding and is accepted. Full record:
`.review/results/ldt-2-r1.claude-verdict.md` (raw
`.review/results/ldt-2-r1.claude.json`).
