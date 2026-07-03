# ue-r2-1e — adjudication of review findings

reviewer: gpt-5.5 (codex exec, read-only, headroom proxy)
slice range: `e1a21a1..15968f4` (4 sub-commits)
raw output: `ue-r2-1e.codex.md` (trimmed; findings + verdict retained)

VERDICT: NEEDS FIXES → fix-then-ship. All three Mediums verified
against source and **Accepted**; fixed in one commit; gate re-run
green.

1. **dial.rs — in-flight budget vs chunk ceiling — Accepted.** With
   `max_chunk_bytes` unknown, a `max_inflight_bytes` smaller than one
   chunk was not honored. **Fix**: the in-flight budget now bounds the
   chunk ceiling FIRST (floor 64 KiB, the session's minimum buffer),
   then prefetch; pinned by the tight-budget case in
   `profile_lowers_ceilings_but_never_raises_them` (8 MiB budget →
   8 MiB chunk ceiling, prefetch 1).
2. **dial.rs — idle ticks step the dial — Accepted.** Zero-traffic
   ticks read as "clean pipe" and ramped without evidence (manifest /
   preparation stalls). **Fix**: the tuner samples `bytes_sent` too
   and skips the step when a tick moved no bytes; the paused-clock
   test now asserts the idle tick holds at 16 MiB and only a
   byte-bearing tick steps.
3. **data_plane.rs — tar-shard writes invisible to the blocked signal
   — Accepted.** Shard chunk writes (the small-file workload) recorded
   bytes but no `write_blocked`, so a saturated link looked clean.
   **Fix**: shard writes get the same `P::ACTIVE`-gated timing as the
   file loop (NoProbe still reads no clock).

## Fix commit

- Fix sha: `46da929`. Gate after fixes: fmt clean,
  clippy clean, tests 1402 passed / 0 failed / 2 ignored.
