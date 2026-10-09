# JOB_LOGS FAST acceptance — local A/B, 2026-10-08

**Status**: Historical (evidence)

Question (plan acceptance "FAST"): does logging cost a measurable slowdown
on the existing small-file and large-file benches?

Rig: this Mac (macOS, APFS), quiet (no review or build running), release
builds. Script: `scripts/bench_otp11_local_ab.sh` (interleaved old/new per
round, RUNS=5; its own gate: new median ≤ old + 10%). Old binary: `f6ca55c0`
(jl-1a only — the CLI and daemon log nothing). New: `2dab5d21` (jl-1b..jl-4:
every command keeps a log and a job record).

| cell | old | new `2dab5d21` | new + plain `fsync` |
|---|---|---|---|
| huge (1 GiB, APFS clone) | 21 ms | 73 ms | 25 ms |
| tree (256 MiB + 32 dirs) | 29 ms | 79 ms | 31 ms |
| small (10,000 × 4 KiB) | 1235 ms | 1302 ms | 1223 ms |
| noop (synced mirror) | 25 ms | 81 ms | 26 ms |

Files: `ab-before-fsync.{out,runs}`, `ab-after-fsync.{out,runs}`.

Finding: a constant ~50 ms per command, all of it logging (the same copy
with logging unable to start: 6 ms; with it: 54 ms). The cost was about ten
`F_FULLFSYNC`s — Rust's `sync_all`/`sync_data` on Apple, which also empty
the drive's write cache — in job logs and records; blit flushes no ordinary
copied file. With logs and records flushed by POSIX `fsync(2)` on Apple
(every flush point kept), the cost is ~2–3 ms per command: the 1 GiB clone
cell still misses the script's +10% gate by ~2 ms on a 21 ms run; the other
three pass, the 10,000-file cell within noise.

Where the rest goes (throwaway timing build, 50 runs of history): ~1.5 ms
starting the run's log and record, ~2.5 ms closing them; flushing is now
~0.4 ms of it (a build with flushing skipped: 8.4 ms vs 8.8 ms, old 6.1 ms).
About 1 ms of it grows with history (pruning reads every kept record).
Further cuts are possible (prune by file time, write the starting record
without a flush) and are the owner's call under FAST.
