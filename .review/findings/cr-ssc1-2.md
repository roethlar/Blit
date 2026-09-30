# cr-ssc1-2: Bound chunked FILE bodies by the manifest size before bytes reach the writer

**Severity**: HIGH — an authenticated peer can write unbounded bytes past a tiny advertised size before any check (disk exhaustion)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range f74b0b1a..6bc3d08f, record .review/results/ssc-1-range.codex.json)

## Evidence
crates/blit-core/src/remote/transfer/pipeline.rs:1411 — ChunkedBody streams all chunks into the sink, while the cumulative size is checked only after the sentinel at line 1424 and is not checked at all for a failed status.

Verified against master after ssc-3 (2026-09-30) by the orchestrator before admission.

## Predicted observable failure
An authenticated peer can advertise a tiny file and send an arbitrary number of individually valid 64 MiB chunks. The destination writes them all before detecting an OK-size mismatch, or accepts them before a failed terminator, allowing disk exhaustion and excessive I/O far beyond the granted file size.

## Reviewer's suggested approach
Give ChunkedBody the advertised remaining length and reject any chunk whose length exceeds it before yielding bytes to the writer. Enforce the cumulative upper bound for both successful and failed records.

## What
(coder fills in)

## Guard proof
(red/green proof of the new guard; mutation described)

## Known gaps
(none yet)
