# ue-r2-1f: Push converge — retire the daemon `desired_streams` ladder

**Slice**: ue-r2-1f — sixth slice of `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`
**Status**: Reviewed (codex PASS, 1 Low fixed; Interpretation judged plan-conformant)
**Branch**: master (no agent branches — AGENTS.md §8)
**Commits**: `a4a9f70` + boundary-test review fix

## What

Complete push's convergence onto the engine's decision layer and
retire the second of the three stream-count ladders: the daemon-push
`desired_streams` table (`push/control.rs` — the ladder `tuning.rs`'s
own comment said "wins"). Its workload-shape awareness (file count AND
bytes) moves into the engine verbatim as the shared initial-stream
proposal, per REV4 Acceptance ("`desired_streams` already carries
partial shape-awareness today…; the engine generalizes this rather
than inventing it from nothing").

## Interpretation of "route push through the engine" (stated for review)

REV4 Design §1 draws the boundary explicitly: the engine owns strategy
selection, **dial creation/updates (subsuming the ladders)**, the
payload work queue, telemetry, and fallback/finish invariants;
path-specific code remains at protocol boundaries — and its boundary
list NAMES "source/destination manifest exchange" and "legacy
compatibility". Push's gRPC event loop (PushHeader, manifest frames,
need-list batching, negotiation frames, fallback timing) IS that
boundary. After this slice every tuning/decision input on the push
path is engine-owned:

- client-side dial + live tuner + per-batch planner chunk (landed at
  `ue-r2-1e`),
- daemon-side stream proposal (this slice, engine-owned shape table),
- the byte-moving execution already runs the shared pipeline
  (`execute_sink_pipeline_streaming`) the engine builds on.

What deliberately does NOT move: the manifest/need-list protocol loop
and the daemon's purge machinery (mirror/delete authority is boundary
work per Design §1). If the reviewer or owner reads "route through the
engine" as demanding a deeper structural move of the protocol loop
itself, that is a plan-level conversation to surface, not something to
smuggle in as churn.

## Design

- **`engine/dial.rs`**: new `pub fn initial_stream_proposal(total_bytes:
  u64, file_count: usize, ceiling: usize) -> u32` — the exact
  `desired_streams` table (empty → 1; 32 GiB/200k files → 16 … 32
  MiB/256 files → 2; else 1), clamped to the caller's ceiling. The
  RECEIVER proposes from workload shape; the sender's dial clamp
  (`set_negotiated_streams`, 1e) still bounds it sender-side. Live
  mid-transfer stream changes stay `ue-r2-2` (resize wire-up on the
  elastic queue).
- **`push/control.rs`**: both negotiation sites call the engine fn with
  the daemon's advertised ceiling (`local_receiver_capacity().
  max_streams`); the private `desired_streams` fn is deleted. Table
  values and today's negotiations are wire-identical (table max 16 <
  ceiling 32, so the clamp is structural, not behavioral).
- **Old/new compatibility**: negotiation shape unchanged
  (`stream_count = 4` field semantics identical); old clients keep
  clamping to their own ladder/dial exactly as before.

## Preserved properties (the slice's named invariants)

Manifest streaming, need-list batching, fallback timing,
scan-completeness purge safety, old/new compat — all untouched by this
change (no code in those paths moves); pinned by the existing push
wire/behavior tests, which must stay green.

## Tests

Baseline entering the slice: 1402 / 0 / 2 → after: **1403 / 0 / 2**
(+1 proposal-table test, extended per review to exact tier boundaries
±1; the retired ladder had no tests at all).

## Known gaps

- Remote transfers still record no perf history (1e gap, unchanged) —
  so the daemon's proposal cannot yet learn from history; it stays the
  static shape table, now engine-owned and single-home.
- `pull_stream_count` (ladder #3) retires at `1g`/`1h` as planned.
- The push event loop remains the protocol boundary per the
  Interpretation section — flagged there for review rather than
  restructured.
