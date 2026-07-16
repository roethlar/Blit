# sf-2 — Shape-correction stream resize (dial file-count weighting, e2e)

**Plan**: `docs/plan/SMALL_FILE_CEILING.md` (Active, D-2026-07-04-4), slice sf-2.
**Status**: landed + graded — `c70c2ac` + review fix `7627e7b` (codex
1/1 accepted; verdict in `.review/results/`).

**Historical as of ldt-2 (2026-07-16):** this finding accurately records the
then-current static shape correction. `LIVE_DIAL_TUNING.md` retires it as
worker-count authority; live SOURCE telemetry now owns ADD and REMOVE.

## What

Makes a many-tiny-file push actually run at the stream count the
engine's shape table assigns it. The plan's diagnosis said
`initial_stream_proposal` was byte-weighted; in reality the table has
had file-count tiers since ue-r2-1f (`dial.rs`) — the defect is the
**input**: on push the daemon proposes the epoch-0 stream count at its
early manifest flush (`FILE_LIST_EARLY_FLUSH_ENTRIES` = 128,
`control.rs`), so a 10k-file push negotiated from a ~128-file prefix →
1 stream, and rode it for the whole transfer. That is the measured
10 GbE small-cell gap and the sf-1 loopback probe finding (1000 files →
1 stream where the table says 2).

Fix: **client-side shape-correction resize**. As the need list
accumulates (the true transfer shape — an incremental push may move a
tiny subset of a large manifest), the client re-runs
`initial_stream_proposal` over the accumulated need bytes/count and
corrects the live stream count upward through the existing ue-r2-2
resize wire, one ADD epoch at a time. No daemon change, no wire change.

## Approach

- `TransferDial::propose_shape_resize(desired)` (engine `dial.rs`, the
  single stream-policy owner per w2-2): one-in-flight, ceiling-clamped,
  one stream per epoch (the wire carries one `sub_token` per ADD),
  ADD-only. Unlike `resize_tick` there is **no sustain/cooldown** — the
  shape is a definite signal, not throughput inference. Epoch
  allocation switched from store to CAS in both proposers: two tasks
  (tuner + client loop) now allocate epochs, and a plain store could
  stack two live proposals onto one epoch number.
- Push client (`push/client/mod.rs`): correction fires at the three
  points where shape knowledge or send capacity changes — negotiation
  (need batches can predate it), each need-list batch (DataPlane mode),
  and each resize-ack settle (continues the ramp). Gated on
  resize-negotiated transfers; **flips off permanently the first time
  the tuner proposes REMOVE** — live throughput evidence outranks the
  static table, and re-adding what the tuner retired would flap.
- The tuner arm's inline ADD send is extracted into `send_resize_add`,
  shared with the shape path (identical wire behavior, one copy).
- `RemotePushReport.data_plane_streams: Option<usize>` — the dial's
  settled live count at finish (`None` on gRPC fallback). This is the
  e2e pin's observable; also useful diagnostics.
- Pull side checked and NOT touched: `negotiated_pull_streams`
  (`pull_sync.rs:344`) proposes from the complete post-diff
  `entries_to_send` — pull never had this defect.

## Files

- `crates/blit-core/src/engine/dial.rs` — `propose_shape_resize`, CAS
  epoch allocation in `resize_tick`, unit pins.
- `crates/blit-core/src/remote/push/client/mod.rs` — `send_resize_add`
  + `maybe_shape_resize` helpers, three correction call sites, REMOVE
  gate, report field.
- `crates/blit-core/src/remote/push/client/types.rs` — report field.
- `crates/blit-daemon/src/service/push/shape_resize_e2e.rs` (new) +
  `push/mod.rs` — loopback e2e pin.

## Tests

Suite 1479 → **1483 passed / 0 failed** (37 suites; same 2 ignored) —
count grew by 4, fmt + clippy clean.

- `shape_table_covers_the_small_file_ceiling_cells` — the plan's three
  measured cells mapped through the table (10k×4 KiB → 8 via the
  file-count tier; 1×1 GiB → 8 via bytes, unchanged; mixed → 8 via
  bytes) plus the sf-1 probe cell (1000 files → 2).
- `shape_resize_ramps_one_epoch_at_a_time_toward_the_target`,
  `shape_resize_clamps_to_the_profile_ceiling` — proposal semantics:
  no-op at/below live, one-in-flight blocks both proposers, no
  cooldown, receiver-ceiling clamp. ~~Refusal retries~~ was superseded by
  `otp-12-worker-parity`: a refusal now consumes its epoch and is terminal for
  the transfer, because retrying from a later batch reused the epoch/token
  contract and could loop without changing the live set.
- `many_tiny_file_push_opens_more_than_one_data_plane_connection`
  (blit-daemon, in-process loopback e2e): REAL push service served via
  `production_server_builder`, REAL `RemotePushClient` pushes 10,000
  tiny files, asserts `!fallback_used`, all files transferred, and
  `data_plane_streams > 1`. **Guard proven**: with
  `propose_shape_resize` forced to `None` (temporary revert) the test
  fails with "settled at 1" — the exact pre-fix behavior; restored and
  re-passed. Runtime ~0.35 s.

## Known gaps

- The ramp is one stream per acked epoch: 1→8 takes 7 control
  roundtrips. Negligible on LAN (the e2e settles multi-stream in
  well under its 0.35 s); on high-RTT links the ramp is slower — WAN
  tuning is a plan non-goal.
- Old daemon (no `resize_enabled`) or gRPC fallback: behavior
  unchanged by design — the correction needs the resize wire. Rig
  cells with both ends current are what sf-4 re-measures.
- The daemon's early-flush proposal itself still lowballs; the
  correction is client-side. Carrying workload totals in `PushHeader`
  would fix it at the source but is wire-visible (sf-6-class owner
  gate) and unnecessary while the ramp closes the gap.
- Shape corrections stop for the transfer after any tuner REMOVE; the
  ue-2/sf-5 backlog-signal feed (mid-transfer dynamics) stays a
  separate slice.
- Whether 8 streams reaches the *hardware* ceiling on the small cells
  is sf-4's rig question; this slice removes the policy binder only.
- Windows: touched code is platform-neutral (client loop + dial), but
  the parity run on the owner's machine per repo policy has not been
  done this slice.
