# w8-1b-zero-copy-fast-eval — FAST evaluation of zero_copy.rs

**Branch**: `master` (owner-authorized branchless session 2026-06-11)
**Commit**: `6189d82`
**Source**: W8.1 embedded owner decision (D-2026-06-11-2 excluded
`zero_copy.rs` from the w8-1 sweep pending this evaluation)

## What

Analysis-only slice: evaluate wiring `splice` into the receive pipeline
(owner flagged FAST potential) vs deleting the 219-line dead module.
Deliverable is `docs/plan/ZERO_COPY_RECEIVE_EVAL.md` (**Status: Draft**;
the owner's verdict on it is the decision gate).

## Verdict (recommended)

**Delete**, folded into w8-1 once the lib.rs sentinel (w5-1) is graded.
Honest both ways:

- *For wiring*: the wire path is genuinely splice-friendly — raw TCP, no
  TLS/compression, exact-length `DATA_PLANE_RECORD_FILE` payload runs,
  and no inline hashing in `receive_stream_double_buffered`.
- *Against*: the dead code is unshippable as-is (EAGAIN
  `thread::sleep(10ms)` busy-wait against tokio's nonblocking fd — needs
  an `AsyncFd` rewrite, so nothing is saved by keeping it); the
  `AsyncRead`-generic pipeline forces a raw-fd downcast + permanent
  buffered fallback (gRPC fallback can never splice); the CPU win at
  10 GbE is a fraction of one core, Linux-only, raw-file payloads only,
  and unmeasured (the 10 GbE bench rig hasn't run); it is the only
  unsafe libc I/O in blit-core.
- *Revisit gate*: 10 GbE benchmarks showing receive-side CPU saturation.

## Files changed

- `docs/plan/ZERO_COPY_RECEIVE_EVAL.md` (new, Draft) — no code changes.

## Tests added

None (analysis-only; suite unchanged at 1368).

## Known gaps

- No empirical benchmark — the recommendation rests on static analysis
  (memcpy-vs-wire-rate arithmetic) plus the busy-wait defect. The doc
  names the measurement that would overturn it.
- The conditional deletion slice is sentinel-blocked on w5-1 (lib.rs);
  executing it belongs to w8-1.
