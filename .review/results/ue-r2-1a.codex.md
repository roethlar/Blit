# ue-r2-1a — codex (gpt-5.5) review output

Reviewer: `codex exec -s read-only` on `gpt-5.5` (headroom proxy), slice
`515fb76..771a632`. Findings excerpt (the full agent transcript is not
retained — adjudication in `ue-r2-1a.gpt-verdict.md`):

---

- `crates/blit-core/src/remote/transfer/pipeline.rs:120` — **High** — Workers
  never observe `cancelled` before taking more queued work, and the forwarder
  only checks it before `send_async`, so a sink error can still wait for
  queued/backpressured survivor work before surfacing; fix with a cancellation
  token/watch used in `tokio::select!` around worker receive and forwarder
  send/recv, and stop workers from draining queued payloads after first error.
- `crates/blit-core/src/remote/transfer/data_plane.rs:534` — **Low** —
  `send_block` records CLI outbound bytes but not `probe.record_bytes`, so
  `LiveProbe` undercounts block-resume data-plane bytes; add
  `self.probe.record_bytes(content.len() as u64)` after successful
  block-content writes.
- `crates/blit-core/src/remote/transfer/pipeline.rs:1217` — **Low** — The
  byte/file total test can pass balanced duplicate/drop cases because it asserts
  only aggregate counts and sums; record relative paths/IDs in the test sink and
  assert the exact expected set appears once.
- `crates/blit-core/src/remote/transfer/pipeline.rs:1284` — **Low** —
  `producer_cancel_winds_down_pipeline_promptly` covers clean producer EOF with
  one fast sink, not the shared `cancelled` error path under backpressure; add a
  multi-sink test with one failing sink, one slow sink, and a full queue.

VERDICT: fix-then-ship
