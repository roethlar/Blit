# ue-r2-1e: Live cheap dials replace the `determine_remote_tuning` ladder

**Slice**: ue-r2-1e — fifth slice of `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`
**Status**: Coded; under GPT review (`docs/agent/GPT_REVIEW_LOOP.md`)
**Branch**: master (no agent branches — AGENTS.md §8)
**Commits**: `3be9105` (dial type), `a0d2c9f` (profile stamping),
`98943b7` (ladder retirement), `15968f4` (live tuner + measurement fix)

## What

Replace the static size-keyed `determine_remote_tuning` ladder
(`remote/tuning.rs`) with the single mutable **dial** (REV4 Design §4):
sender-owned, bounded by the receiver's advertised `CapacityProfile`
(the `ue-r2-1b` wire fields, stamped for real starting this slice),
started conservative, with the cheap dials (chunk bytes, prefetch)
adjusted live from the PR1 telemetry landed at `ue-r2-1a`. The dial is
a mutable object read by both ends from this slice onward — the
C-ready seam `ue-r2-2` wires stream resize onto.

Ladder-retirement scope per REV4 "Slice dependencies": this slice
retires **`determine_remote_tuning`** (client push + daemon pull/
pull_sync callers). The daemon-push `desired_streams` ladder retires at
`ue-r2-1f`; `pull_stream_count` at `1g`/`1h`.

## Design (frozen before implementation)

- **`engine/dial.rs`** (new): `TransferDial` — Arc-shared atomics:
  `chunk_bytes`, `prefetch_count`, `tcp_buffer_bytes` (0 = unset),
  `initial_streams`/`max_streams` (set at negotiation; live changes
  arrive with `ue-r2-2` resize), plus immutable ceilings/floors.
  Constructors:
  - `conservative()` — the floor tier (16 MiB chunk, 4/8 streams, no
    explicit tcp buffer/prefetch), per D-2026-06-20-1/-2: start
    immediately at conservative defaults, tune live, no probe phase and
    no size-gated start.
  - `clamp_to_profile(&CapacityProfile)` — receiver bounds: ceilings
    become `min(default ceiling, profile value)` for max_streams /
    max_chunk_bytes / prefetch-via-max_inflight; 0 = unknown = keep the
    default ceiling (per the 1b proto contract, unknown ≠ unlimited —
    defaults ARE conservative).
  - Ceilings default to today's top ladder tier (64 MiB chunk, 32
    streams, prefetch 32, 8 MiB tcp buffer) so a fully-ramped dial
    matches today's best static behavior.
- **Receiver profile stamping** (first real senders of the 1b fields):
  - daemon push negotiation stamps `receiver_capacity` on
    `DataTransferNegotiation` (both control.rs negotiation sites);
  - pull_sync client stamps `TransferOperationSpec.receiver_capacity`
    in `build_spec_from_options`.
  - Profile values: honest system facts only — `cpu_cores` =
    `num_cpus`, `max_streams` = 32 (today's accept ceiling),
    `max_chunk_bytes` = 64 MiB, `max_inflight_bytes` = 2 GiB (today's
    effective top-tier chunk×prefetch), `drain_class`/`load_percent`/
    `drain_rate` = 0 (unknown — no fabricated numbers). Old peers skip
    the field (1b compat tests).
- **Ladder retirement**: `determine_remote_tuning` deleted with its 4
  tier tests (replaced by dial tests — count called out below). Callers
  switch to a per-transfer `Arc<TransferDial>`:
  - push client: `ensure_remote_tuning` becomes dial construction at
    first need, clamped by `negotiation.receiver_capacity` when
    present; use sites read the dial.
  - daemon pull.rs / pull_sync.rs: dial constructed at
    negotiation/setup (clamped by the spec's `receiver_capacity` on
    pull_sync — there the DAEMON is the sender). The deprecated Pull
    path gets the same mechanical swap (it dies at `1h`).
- **Live tuner** (`engine/dial.rs` or `dial_tuner`): a sampling task
  per data-plane session reading the PR1 `StreamTelemetry` snapshots
  every 500 ms and stepping the cheap dials with hysteresis:
  blocked-ratio < 5% → step up (chunk ×2 toward ceiling, prefetch
  +50%); > 30% → step down toward floors; one step per tick.
  `tcp_buffer_bytes` is a connect-time dial this slice (applied to
  sockets opened after a change; no setsockopt on live sockets yet).
  Requires switching the sender data plane from `NoProbe` to
  `LiveProbe` wiring where the tuner is attached.
- **Carried `ue-r2-1a` finding**: fix `write_blocked_nanos` join!
  over-measurement in the data-plane send path while wiring its first
  consumer (the tuner) — over-measure would bias the tuner
  conservative.
- **Workload-shape awareness** (planner input to the dial start):
  local/1d `InitialPlan` strategy and push manifest hints may seed
  `initial_streams` within bounds — kept minimal this slice: dial
  starts at the conservative tier regardless; shape-keyed starts are
  planner work layered later (documented, not silent).
- **Dead local tuning window** (1d discovery): `derive_local_plan_
  tuning`/`select_tuning_window` never fire at HEAD (streaming
  summaries carry no bucket stats). This slice does NOT revive or
  delete it — that fold-or-retire decision is w2-2-adjacent and
  surfaced to the owner in STATE; the dial does not consume it.

## Sub-commits (each gated)

1. Dial type + conservative start + profile clamp + tests.
2. Receiver-profile stamping (daemon push negotiation, pull_sync spec)
   + tests (profile arrives; old-peer compat already pinned at 1b).
3. Ladder retirement: all `determine_remote_tuning` callers → dial;
   delete `tuning.rs` ladder + its 4 tests (replacement dial tests
   keep the count non-decreasing).
4. Live tuner + LiveProbe wiring + `write_blocked_nanos` accuracy fix
   + behavior tests (step-up on clean telemetry, step-down on blocked,
   ceilings respected).

## Tests

Baseline entering the slice: 1399 / 0 / 2 → after: **1402 / 0 / 2**.
The 4 deleted ladder-tier tests (tuning.rs) are offset by 7 dial tests
(defaults, profile clamps both directions, step/hysteresis, negotiated
clamp, blocked-ratio edges, paused-clock tuner loop incl. Weak
self-termination) + the profile-stamp asserts in existing suites.

## Deviations from the frozen design (called out for review)

- The tuner attaches on the push client's real data-plane path only
  (the flagship sender). Daemon pull paths construct dials and read
  them at their (mostly connect-time) seams but do not yet run a
  tuner: pull_sync's single-stream/resume paths and the deprecated
  Pull RPC gain live tuning when they converge through the engine
  (`1g`) or die (`1h`).
- The daemon pull_sync data-plane paths still hardcode prefetch 8 and
  their receive-pool sizes exactly as before — only their previously
  tuning-derived values (chunk) moved to the dial.

## Known gaps

- Live chunk/prefetch reads INSIDE a running MultiStreamSender: the
  dial ramps and the planner's per-batch `chunk_bytes_override`
  follows it, but a session's pool buffer size and its stored
  `chunk_bytes`/`payload_prefetch` remain connect-time values (the
  ramped values apply to payload PLANNING immediately and to future
  sockets; per-write chunking inside live sessions follows at
  `ue-r2-2`'s resize wiring, which adds sockets that pick up dialed
  values).
- Remote transfers still record no perf history (`perf_history` is
  local-only: `TransferMode::{Copy,Mirror}`), so the remote
  known-workload replay of Design §3 has no data source; the dial
  always novel-starts on remote paths. Surfaced to the owner —
  recording remote runs is unplanned work (candidate for `1f`/`1g`).
- The daemon-push `desired_streams` ladder still proposes stream
  counts (retires at `1f`); `pull_stream_count` retires at `1g`/`1h`.
- Dead local tuning window untouched (1d discovery; fold-or-retire at
  w2-2/owner call).
- The receiver profile advertises static ceilings; live load/drain
  measurement (load_percent, drain_rate) stays 0 = unknown until a
  measurement source exists.
