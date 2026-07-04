# Adjudication — w4-4-blocking-work-off-runtime

Slice commit: `0feca34`
Review record: `.review/results/w4-4-blocking-work-off-runtime.codex.md`
reviewer: gpt-5.5 (codex exec, read-only sandbox)
Adjudicated: 2026-07-04

## Verdict

**NEEDS FIXES (1 Medium) — accepted, fixed in `768e7e3`.**

1. **Accepted** — verified against the batcher source: its
   64 KiB/5 ms early-flush triggers are only evaluated inside
   `push()`, and chunking confines `push()` calls to drain time, so
   between chunk boundaries nothing could flush. The practical
   regression is real for slowly-enumerating clients (streaming
   manifest while the scan proceeds — the ue-r2-1d design): the
   first `FilesToUpload` and mid-manifest TCP spin-up would wait for
   128 trickled entries (potentially many seconds) instead of ~5 ms.
   Fix: `manifest_drain_due(pending_len, oldest_buffered)` — drain on
   chunk-full OR oldest-buffered age ≥ `MANIFEST_CHECK_MAX_DELAY`
   (= `FILE_LIST_EARLY_FLUSH_DELAY`), evaluated on the next arrival,
   which is exactly the granularity of the batcher's own push-time
   flush semantics (a lone entry followed by silence also waited for
   the next event before this change). Fast streams hit the 128 cap
   well inside 5 ms, so syscall batching is preserved where it
   matters. +1 trigger-contract unit test.

Rejected: none.
Deferred: none.

Coder-side verification: both halves re-verified at HEAD before
coding (the sites moved since the 2026-06-11 audit — daemon pull.rs
died at ue-r2-1h, its enumeration relocated into pull_sync.rs);
decision-parity + containment-escape + empty-drain + trigger pins
(blit-daemon 170 → 174), mutation-verified (all-`true` batched check
fails the parity pin; restored green); fmt + clippy clean; workspace
1472 → 1476 passed / 0 failed / 2 ignored across 37 suites (1475 at
the slice commit, +1 with the fix).
