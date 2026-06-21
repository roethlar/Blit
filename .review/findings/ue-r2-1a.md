# ue-r2-1a: Salvage adaptive PR1+PR2 substrate (telemetry Probe + work-stealing queue)

**Slice**: ue-r2-1a — first slice of `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`
**Status**: Coded; under GPT review (`docs/agent/GPT_REVIEW_LOOP.md`)
**Branch**: master (no agent branches — AGENTS.md §8)
**Commits**: `e569eea` (PR1), `3844a15` (PR2), `ec561f2` (PR2 fix), plus the
tests/finding commit.

## What

Land the adaptive-streams substrate that was trapped behind the `-s ours`
octopus (D-2026-06-07-2): per-stream telemetry with a zero-cost `Probe`
(PR1), the shared work-stealing pipeline queue (PR2), and the
forwarder-halt-on-error fix (PR2 review). This is the C-ready seam REV4
builds on — an elastic work-stealing stream-set, work not pinned to a
stream. No operator-visible behavior change: the default `NoProbe`
monomorphization compiles to today's hot path.

## Approach

Cherry-picked (`-x`) the three code commits onto master rather than merging
— the octopus made them ancestors, so a plain merge no-ops
(D-2026-06-07-2):

- `e6ef095` → `e569eea` (PR1): `DataPlaneSession<P: Probe = NoProbe>`,
  `StreamTelemetry`/`StreamProbe`, `tcp_info` module. Conflicts in
  `data_plane.rs` (master's audit-h3b `StallGuardWriter` stream vs PR1's
  generic struct) and `mod.rs` (re-exports) hand-resolved: the stream
  stays `StallGuardWriter<TcpStream>` and the struct gains `<P: Probe>` +
  a `probe` field; `from_stream_with_probe` wraps the stream in the stall
  guard. `mod.rs` re-exports drop `Phase`/`TransferProgress`/
  `TransferProgressSnapshot` (master had already removed those types) and
  add the telemetry types. Added the missing `AtomicU8` import in
  `progress.rs`.
- `af66ff5` → `3844a15` (PR2): shared `flume` work-stealing queue. Applied
  cleanly (master's `pipeline.rs` matched the cherry-pick base).
- `b797b73` → `ec561f2` (PR2 fix): forwarder halts promptly on sink error
  via a shared `cancelled` flag. Applied with `-n` to drop the bundled
  `reviews/PR2-workqueue.codex.md` artifact (absolute `C:/Users` paths;
  review provenance, not substrate).

`eafb187` ("backup": doc moves + stray-file delete) and `d9d4ec7` (PR3 WIP,
does not build) excluded per REV4.

## Files changed

- `data_plane.rs` — generic `DataPlaneSession<P: Probe>`; hot-loop
  telemetry gated on the compile-time `P::ACTIVE`; StallGuard composition
  preserved.
- `progress.rs` — `StreamId`, `StreamState`, `StreamTelemetry`,
  `StreamProbe`, `Probe`/`NoProbe`/`LiveProbe` (+ `AtomicU8` import).
- `tcp_info.rs` — new; best-effort `getsockopt(TCP_INFO)` on Linux, `None`
  stub elsewhere.
- `sink.rs` — `DataPlaneSink<P: Probe = NoProbe>` (default keeps call
  sites unchanged).
- `pipeline.rs` — shared work-stealing queue + forwarder-halt fix + two
  new behavior tests.
- `mod.rs` — re-exports.

## Tests added

PR1/PR2 brought their own; this slice adds two to `pipeline.rs`
`workqueue_tests` to complete REV4's "work-stealing as behavior" set:

- `byte_and_file_totals_correct_under_work_stealing` — two sinks pull the
  shared queue; distinct per-file sizes pin byte + file totals and that
  every byte lands on exactly one sink (no double-count / drop).
- `producer_cancel_winds_down_pipeline_promptly` — producer drops the
  channel mid-stream; the executor winds down without hanging (the 5s
  timeout is the no-hang assertion) and writes only the fed payloads.

Kept: `fast_sink_steals_work_from_slow_sink` (slow-sink),
`forwarder_stops_promptly_on_worker_error` +
`pipeline_streaming_surfaces_underlying_sink_error` (failing-sink),
`receive_pipeline_aborts_on_stall` + `stall_guard.rs` (StallGuard),
`pipeline_copies_files_end_to_end` (byte-identical).

Validation: `cargo fmt --check` clean; `clippy -D warnings` clean;
`cargo test --workspace` **1377 passed / 0 failed / 2 ignored** (baseline
1370; +7).

## Known gaps

1. **Hard-abort on cancel is w4-1, not here.** The work-stealing workers
   are bare `tokio::spawn`; dropping a `JoinHandle` does not abort the
   task, so dropping the *pipeline future itself* detaches in-flight
   workers rather than aborting them — the AbortOnDrop family (REV4 w4-1,
   ratified High). ue-r2-1a pins the graceful producer-close cancellation
   path the substrate provides;
   `producer_cancel_winds_down_pipeline_promptly` documents the boundary.
2. **PR1 telemetry-accuracy findings carried forward → ue-r2-1e.** The
   original PR1 codex review (verdict: fix-then-ship) left two unaddressed:
   (Medium) `write_blocked_nanos` times the whole `tokio::join!(write,
   read)` step, so a slow *source read* inflates the write-backpressure
   signal; (Low) tar-shard chunk writes record bytes but no write-block
   time. The telemetry has no live consumer until the dial in ue-r2-1e, so
   the inaccuracy is latent — land faithfully now, fix when ue-r2-1e wires
   the signal to a decision.
3. PR1 telemetry types (`StreamProbe`, `StreamTelemetry`, `tcp_info`,
   `LiveProbe`) are substrate with no live consumer yet (wired in
   ue-r2-1e / the controller). Re-exported for that future use.
