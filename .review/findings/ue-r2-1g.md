# ue-r2-1g: PullSync multistream through the engine

**Slice**: ue-r2-1g — seventh slice of `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`
**Status**: In review
**Branch**: master (no agent branches — AGENTS.md §8)
**Commits**: (filled at commit time)

## What

Route PullSync's tuning decisions through the engine and make its TCP
data plane multistream (it is single-stream today: the
`stream_count = 1u32` hardcode in `pull_sync.rs::stream_via_data_plane`),
harvesting the deprecated Pull RPC's daemon-side multi-accept pattern.
Absorbs `MULTISTREAM_PULL.md` (w2-3) acceptance criteria per REV4's
lineage header.

Key discovery that shaped the slice: **the client side of multistream
PullSync already exists.** `pull_sync_with_spec`'s negotiation arm
(`remote/pull.rs:955-966`) routes through `spawn_data_plane_receiver`,
which reads `negotiation.stream_count.max(1)` (`remote/pull.rs:455`)
and fans out to N `AbortOnDrop`-wrapped receive workers
(`receive_data_plane_streams_owned`, `remote/pull.rs:1613-1709`), each
running the shared `execute_receive_pipeline` under a per-stream
StallGuard, with per-stream failure propagating via `join()??` and
cancellation cascading through `AbortOnDrop`. The daemon simply never
advertises more than 1. So this slice is daemon-side negotiation +
fan-out, not a new client receive model.

## Interpretation of "route PullSync through the engine" (stated for review)

Same boundary as `ue-r2-1f` (judged plan-conformant there): REV4
Design §1 gives the engine strategy selection, **dial creation/updates
(subsuming the ladders)**, the payload work queue, and telemetry;
path-specific code remains at protocol boundaries, explicitly naming
"source/destination manifest exchange", "mirror/delete authority", and
"legacy compatibility". PullSync's control loop (spec, manifest
exchange, delete-list authority, need-list, block-hash resume
protocol, PullSyncAck checksum refusal) IS that boundary. After this
slice every tuning/decision input on the pull path is engine-owned:

- dial + receiver-profile clamp: `TransferDial::conservative_within`
  (landed `ue-r2-1e`, `pull_sync.rs:261`);
- stream count: `engine::initial_stream_proposal` (this slice —
  replaces the hardcode; same engine fn push uses since `1f`);
- payload prefetch: `dial.prefetch_count()` (this slice — replaces a
  leftover hardcoded `8`);
- chunking: `dial.chunk_bytes()` (already, `1e`);
- byte-moving: the shared `execute_sink_pipeline` with N sinks — the
  `1a` elastic work-stealing queue does the fan-out (no static file
  partition).

## Design

- **Stream-count decision** (new pure helper in `pull_sync.rs`,
  `negotiated_pull_streams`): on pull the daemon is the byte SENDER
  and the workload-shape-knower; it proposes from the engine's shape
  table, bounded by the client's advertised receiver ceiling:
  - client advertised no `TransferOperationSpec.receiver_capacity`,
    or `max_streams == 0` (unknown) → **1 stream** — today's behavior,
    exactly as REV4 Design §5 and the proto contract prescribe
    ("0 = unknown → sender stays at today's negotiated stream_count");
  - otherwise `initial_stream_proposal(bytes_to_send,
    entries_to_send.len(), dial.ceiling_max_streams())`, recorded on
    the dial via `set_negotiated_streams` (the `ue-r2-2` resize
    target).
  The engine fn's doc comment is updated: the proposer is the
  shape-knowing end (push: receiving daemon; pull_sync: sending
  daemon), always bounded by the byte receiver's profile.
- **Harvest**: `accept_and_wrap_sinks` (accept N, per-socket bounded
  token auth, shared buffer pool, N `DataPlaneSink`s) MOVES from the
  deprecated `service/pull.rs` into `pull_sync.rs`; the deprecated
  handlers now call it there. This is the REV4 "harvested into
  PullSync before deletion" criterion made literal — `1h` can delete
  `pull.rs` without touching the pattern.
- **`stream_via_data_plane`**: negotiation carries the computed
  `stream_count`; the single accept+token block is replaced by the
  harvested helper; `execute_sink_pipeline(source, sinks, payloads,
  dial.prefetch_count(), None)` distributes payloads across all
  streams via the elastic queue.
- **Wire**: NO proto changes. `stream_count` (field 4),
  `receiver_capacity` (spec field 12) and the client fan-out machinery
  all exist. Mixed versions degrade by construction:
  - old client (no profile in spec) → daemon proposes 1 → today's
    behavior byte-for-byte;
  - new client ↔ old daemon → daemon sends `stream_count: 1` → the
    client's `max(1)` single-stream arm.
- **Delegated daemon→daemon inherits automatically**: the dst daemon's
  handler stamps `receiver_capacity = local_receiver_capacity()` into
  the forwarded spec (`delegated_pull.rs:63-69`, landed `1e`) and its
  byte path IS the shared `pull_sync_with_spec` — no delegated change
  needed.

## Preserved properties (the slice's named invariants)

- **Resume**: resume-eligible transfers keep their dedicated
  single-stream path (`stream_via_data_plane_resume`, unchanged, its
  negotiation still says 1). The block-hash exchange is a strictly
  ordered JIT request/response protocol on the control stream —
  multistreaming it would reorder responses. This is an explicit
  RELIABLE exception in REV4's sense, stated here rather than smuggled.
  Pinned by `remote_resume.rs` (stays green).
- **Checksum refusal**: PullSyncAck handshake, negotiation-phase,
  untouched (`remote_checksum_negotiation.rs` stays green).
- **Delete-list authority**: daemon-computed, mirror-scoped,
  client-applied after a successful transfer; untouched
  (`remote_pull_mirror.rs` + `scope_deletions` unit tests stay green).
  Per-stream failure propagates BEFORE the CLI applies deletions
  (client joins all workers before returning the report).
- **Cancellation**: client hangup drops the handler future (control
  path) and `AbortOnDrop` cascades through the N receive workers
  (client); daemon accept waits stay bounded (30s/15s timeouts,
  harvested constants). `abort_on_drop_tests` + `jobs_lifecycle.rs`
  stay green.
- **Per-stream failure**: fail-whole-with-clean-error — any worker's
  error propagates via `join()??` (client) / pipeline error (daemon);
  this is `MULTISTREAM_PULL.md`'s proposed default. New test pins it.
- **gRPC fallback**: `force_grpc` arms untouched;
  single-logical-stream by design (REV4 non-goal). `remote_parity.rs`
  fallback tests stay green.

## Tests

Baseline entering the slice: 1403 / 0 / 2.

- `negotiated_pull_streams` unit tests (daemon): no profile → 1 even
  for a huge workload; `max_streams: 0` → 1; capable profile + small
  workload → table floor; capable profile + huge workload → table cap
  16; profile ceiling 6 clamps to 6.
- `multi_stream_receive_tests` (client, inline): a clean END-only
  multistream receive succeeds; one worker hitting EOF-without-END
  fails the whole receive (fail-whole pin).
- `test_pull_multistream_many_files` (e2e, real daemon): 300-file
  pull; asserts byte-identical content AND the client's
  `[pull-data-plane]` per-stream marker (printed only on the
  multi-stream branch) — the MULTISTREAM_PULL "negotiates >1 stream
  against a new daemon, observable" criterion.
- Everything named under Preserved properties stays green.

## Known gaps

- **Pull 1s-start is NOT met by this slice and cannot be, yet**: the
  stream proposal is workload-shape-keyed, and on PullSync the shape
  is only known after full server enumeration + manifest comparison —
  so negotiation inherently follows the scan. Starting earlier
  requires either a shape-blind proposal (contradicts the
  shape-aware criterion) or mid-transfer stream add/drop — which is
  exactly `ue-r2-2` (resize). Pull's 1s-start therefore rides on
  `ue-r2-2`; flagged as the plan's "feeds 1g (pull 1s-start)" residue
  rather than silently skipped.
- **No live tuner on the pull data plane**: pull_sync plans its
  payloads once (post-comparison), so the dial's live chunk/prefetch
  moves would have nothing to re-plan; sessions run `NoProbe` exactly
  as before. The tuner becomes meaningful for pull when streaming
  planning (1d's foundation) or resize (`ue-r2-2`) reaches this path.
  Remote perf-history lanes also still unrecorded (1e gap, unchanged).
- `DelegatedPullStarted.stream_count` stays the diagnostic `0`
  (`delegated_pull.rs:325`) — it is emitted before negotiation exists;
  documented as diagnostic-only in the proto.
- A single huge file still proposes up to 16 streams while planning
  may emit fewer payloads than streams (idle sinks close cleanly) —
  same property push has had since `desired_streams`; the elastic
  queue makes it harmless.
- Deliberate small deltas on the full-file TCP path, called out for
  review: payload prefetch `8` (hardcode) → `dial.prefetch_count()`
  (starts 4, receiver-bounded — the dial is the one tuning owner);
  buffer pool sizing now scales with stream count
  (`streams * 2 + 4` slots, the harvested helper's sizing) instead of
  the fixed 4-slot pool.
