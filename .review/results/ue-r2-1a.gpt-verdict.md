# ue-r2-1a — GPT review adjudication

**Reviewer**: gpt-5.5 (codex, read-only) — raw output in `ue-r2-1a.codex.md`
**Reviewed**: slice commits `515fb76..771a632`
**Codex verdict**: fix-then-ship (4 findings)
**Adjudicator**: claude (Opus 4.8) — verified each finding against source/tests
**Outcome**: all 4 accepted; fixes in commit `<address-review>`. Re-validated:
fmt clean, clippy -D warnings clean, `cargo test --workspace` 1378 passed / 0
failed / 2 ignored.

| # | Codex severity | Verdict | Disposition |
|---|---|---|---|
| F1 | High | **Accepted** (downgraded → Medium) | Fixed |
| F2 | Low | **Accepted** | Fixed |
| F3 | Low | **Accepted** | Fixed (test hardened) |
| F4 | Low | **Accepted** | Fixed (test added) |

## F1 — workers don't re-check `cancelled` (pipeline.rs)

**Codex (High):** after a sink errors, survivor workers keep pulling queued
payloads until the queue drains, so first-error-wins is delayed.

**Verified:** real. The worker loop was `while let Ok(payload) =
work_rx.recv_async().await` with no `cancelled` check, so survivors drained
the bounded queue (≤ `prefetch * sink_count`) before observing
`Disconnected`.

**Severity downgraded High → Medium:** the error still wins and byte/file
accounting stays correct; `b797b73` already bounded the delay to the queue
capacity (it removed the *unbounded* drain of the whole input). The residue
is bounded wasted work on an already-failing transfer.

**Fix:** the worker loop now re-checks `cancelled` before each
`recv_async`, so a survivor stops at the next payload boundary — post-error
work is bounded to one in-flight payload per survivor. Interrupting an
*in-flight* prepare/write (true prompt cancellation, and hard-abort on
dropping the pipeline future) remains the **w4-1 AbortOnDrop family**, by
design — not duplicated here.

## F2 — `send_block` doesn't record probe bytes (data_plane.rs:537)

**Codex (Low):** `send_block` records CLI outbound bytes + `bytes_sent` but
not `self.probe.record_bytes`, so `LiveProbe` undercounts resume-block data.

**Verified:** real — PR1 added `record_bytes` to the main double-buffered
loop and the tar-shard path but missed the resume-block write path.

**Fix:** added `self.probe.record_bytes(content.len() as u64)` after the
block-content write, completing PR1's own byte-counting pattern. (The
separate write-block *timing* gaps from the PR1 review — Medium `join!`
over-measure, Low tar-shard — remain deferred to ue-r2-1e where the
telemetry is first consumed; see the finding doc Known Gaps.)

## F3 — byte/file total test only checks aggregates (pipeline.rs)

**Codex (Low):** a balanced duplicate+drop that preserves the sums could
pass; record paths/IDs and assert each appears once.

**Verified:** partially mitigated already (distinct per-file sizes make a
sum-preserving dup/drop nearly impossible), but the point is fair —
aggregates alone don't *prove* exactly-once.

**Fix:** `ByteSink` now records each written relative path;
`byte_and_file_totals_correct_under_work_stealing` asserts the path set has
length `n` and de-dups to `n` (every file delivered to exactly one sink,
once).

## F4 — no multi-sink cancel-under-backpressure test (pipeline.rs)

**Codex (Low):** the cancel test covers clean EOF with one fast sink, not
the `cancelled` error path under backpressure.

**Verified:** fair — and it's the natural test for the F1 fix.

**Fix:** added `one_sink_error_bounds_survivor_work_under_backpressure` —
a failing sink + a slow sink + a loaded queue; asserts the error wins, the
pipeline doesn't hang (timeout), and the survivor processes far fewer than
all payloads (proving the F1 `cancelled` re-check bounds its work).
