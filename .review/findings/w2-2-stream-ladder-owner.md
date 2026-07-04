# w2-2-stream-ladder-owner — dial is the single stream/chunk owner

**Branch**: `master` (D-2026-06-07-1 branchless policy)
**Commit**: see REVIEW.md row
**Source findings**: boundaries-stream-count-policy-minted-three-times,
constants-three-disagreeing-stream-ladders,
duplication-byte-total-tuning-ladders —
`docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`; slice spec
`docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` W2.2.

## What

The row as written (2026-06-11) targeted four surfaces. REV4 delivered
the three stream-count legs before this slice ran: the
`determine_remote_tuning` ladder died at `ue-r2-1e` (live dial), the
daemon `desired_streams` ladder at `ue-r2-1f`
(`engine::initial_stream_proposal`, byte- **and** file-count-keyed per
the spec's "takes file_count"), and `pull_stream_count` with the Pull
RPC at `ue-r2-1h`. D-2026-06-20-1 recorded that absorption in v1 slice
IDs ("w2-2 … is ue-1b"); the REVIEW.md row stayed open for the leg REV4
never enumerated: **the `transfer_plan.rs` chunk ladder**.

A 5-agent audit workflow + hand verification established that the
remaining chunk lane through the planner was entirely dead policy:

- The 16/32 MiB ladder (`transfer_plan.rs:221-229` pre-change) lost to
  `PlanOptions.chunk_bytes_override` on every remote path — push client
  and daemon pull_sync always set `Some(dial.chunk_bytes())`.
- Where the ladder *did* win (local engine path, test callers), the
  output was discarded: `PlanUpdate` carries payloads only.
- The single workspace read of the planned value
  (`push/client/mod.rs:1442-1446` pre-change) sat behind a
  `chunk_bytes == 0` guard no live caller could trigger — every caller
  passes `dial.chunk_bytes()`, floored at 64 KiB.
- All live wire chunking already reads the dial directly
  (`DataPlaneSession` construction, sinks, buffer pools).

The spec's proposed fix ("make transfer_plan take chunk_bytes as a
required input") predates REV4; with a live dial and zero consumers of
the planned value, threading a chunk size through the planner would be
plumbing with no reader. The honest single-owner outcome is deletion.

## Approach

- `transfer_plan.rs`: deleted the chunk ladder, `Plan.chunk_bytes` (and
  the now single-field `Plan` wrapper — `build_plan` returns
  `Vec<TransferTask>`), `PlanOptions.chunk_bytes_override`, and
  `plan_to_daemon_format` (zero callers ever, per `git log -S` back to
  the initial commit; its "used by server pull mode" comment was never
  true). The write-only kickoff histogram (`bins_bytes`/`bins_count`)
  collapsed to the `total_bytes` accumulator that was its only read;
  the two sub-1 MiB small bins merged (identical classification).
- `payload.rs`: deleted `PlannedPayloads` (its `chunk_bytes` field was
  write-only); `plan_transfer_payloads` returns
  `Result<Vec<TransferPayload>>`. Signature ripple through
  `diff_planner.rs`, `streaming_plan.rs`, `pipeline.rs` tests,
  `transfer/mod.rs` re-exports.
- Push client: deleted the five per-batch
  `plan_options.chunk_bytes_override = Some(dial.chunk_bytes())`
  refresh statements (dead stores), `ensure_dial`'s `plan_options`
  param + override write, and the unreachable `chunk_bytes == 0`
  fallback in `stream_fallback_from_queue`. Two arms kept their
  `ensure_dial` call as a bare statement — first-need dial creation and
  first-wins ceiling semantics are unchanged. `plan_options` is now an
  immutable `PlanOptions::default()`.
- Daemon pull_sync: both `PlanOptions` literals lose the override
  (now `PlanOptions::default()`); sinks/pools keep reading
  `dial.chunk_bytes()` directly (unchanged).
- `auto_tune/mod.rs`: deleted `TuningParams` — orphaned since
  `ue-r2-1e` deleted its only producer; zero references remained.
  Module doc rewritten (module is now solely the history-derived local
  plan tuner).
- Comment-truth sweep on touched surfaces: `dial.rs` mutability-model
  doc no longer claims chunk/prefetch are "read at each use site" (the
  per-batch planner refresh this slice deleted was one alleged site;
  the other consumers snapshot at session/pipeline/batch setup — steps
  reach epoch-N sockets and later fallback batches, not live sessions).
  `buffer.rs` doc example now cites the dial instead of the deleted
  `TuningParams`.

Behavior: byte-identical on every live path. Remote chunking came from
the dial before and after; the deleted values had no readers.

## Files changed

- `crates/blit-core/src/transfer_plan.rs`
- `crates/blit-core/src/remote/transfer/payload.rs`
- `crates/blit-core/src/remote/transfer/diff_planner.rs`
- `crates/blit-core/src/remote/transfer/mod.rs`
- `crates/blit-core/src/remote/transfer/pipeline.rs` (tests)
- `crates/blit-core/src/engine/streaming_plan.rs`
- `crates/blit-core/src/remote/push/client/mod.rs`
- `crates/blit-core/src/auto_tune/mod.rs`
- `crates/blit-core/src/buffer.rs` (doc)
- `crates/blit-core/src/engine/dial.rs` (doc)
- `crates/blit-daemon/src/service/pull_sync.rs`

## Tests

4 new unit tests in `transfer_plan.rs` (the module had none): tier
classification + interleave order, the single-small-file no-tar rule,
`force_tar` single-file wrap, and `small_count_target` shard splitting
with the 128 clamp floor. They pin the batching behavior that survives
the restructuring; the deletion itself is compile-guarded (the removed
types/fields cannot be referenced), so revert-style mutation
verification does not apply — same evidence shape as w2-1. Zero tests
deleted. Workspace 1448 → 1452 across 37 suites.

## Known gaps

- `docs/WHITEPAPER.md` §§ around 309/606/641 still describe
  `determine_remote_tuning`/`TuningParams` — drift that predates this
  slice (stale since `ue-r2-1e`); w10-docs-batch territory.
- W3.1 (BufferPool) was sequenced "after W2.2 settles the tuning
  owner": settled — the owner is `engine::TransferDial`; W3.1 should
  build `for_data_plane(dial, streams)` against it.
- The local engine path plans with `PlanOptions::default()` targets
  unless perf-history tuning fires; `derive_local_plan_tuning` is
  dynamically dead at HEAD (no producer fills the tar/raw buckets since
  `4ce4898`) — separate fold-or-retire residue already in the STATE
  queue, untouched here.
- gRPC fallback sinks clamp to 1 MiB regardless of the dial value they
  receive (pre-existing, intentional per audit-h3c).
