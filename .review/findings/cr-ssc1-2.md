# cr-ssc1-2: Bound chunked FILE bodies by the manifest size before bytes reach the writer

**Severity**: HIGH — an authenticated peer can write unbounded bytes past a tiny advertised size before any check (disk exhaustion)
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `14a6bc3f`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range f74b0b1a..6bc3d08f, record .review/results/ssc-1-range.codex.json)

## Evidence
crates/blit-core/src/remote/transfer/pipeline.rs:1411 — ChunkedBody streams all chunks into the sink, while the cumulative size is checked only after the sentinel at line 1424 and is not checked at all for a failed status.

Verified against master after ssc-3 (2026-09-30) by the orchestrator before admission.

## Predicted observable failure
An authenticated peer can advertise a tiny file and send an arbitrary number of individually valid 64 MiB chunks. The destination writes them all before detecting an OK-size mismatch, or accepts them before a failed terminator, allowing disk exhaustion and excessive I/O far beyond the granted file size.

## Reviewer's suggested approach
Give ChunkedBody the advertised remaining length and reject any chunk whose length exceeds it before yielding bytes to the writer. Enforce the cumulative upper bound for both successful and failed records.

## What
`ChunkedBody` (`remote/transfer/pipeline.rs`) now carries the header's advertised size (`limit`) and the bytes announced so far (`total`); a chunk whose length prefix would push the total past the limit is rejected as `InvalidData` at the prefix — before one byte of it is yielded to the record writer — for ok and failed records alike. The FILE receive arm constructs it with `file_size`. The existing ok-requires-`header.size` check at the status byte stays (the bound is the upper half, that check the exact-match half).

## Guard proof
`file_body_exceeding_the_header_size_is_rejected_before_the_writer_sees_it` (pipeline.rs tests): header promises 4 bytes, the body streams two 4-byte chunks, once with an ok status and once with a failed status; asserts the error names the cumulative bound and the destination never holds more than 4 bytes. Mutation: `if announced > self.limit && false` → the guard FAILED (record accepted; `scratchpad/cr-ssc-mutations.txt`); restored → green, fuzz harness green.

## Known gaps
None. The chunk-length cap (`MAX_FILE_CHUNK_BYTES`) and the reason-length cap were already enforced; this closes the cumulative gap only.
