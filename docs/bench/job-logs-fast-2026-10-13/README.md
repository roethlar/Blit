# JOB_LOGS FAST acceptance — local A/B after jl-5, 2026-10-13

**Status**: Historical (evidence)

Question (plan acceptance "FAST"): with every command's log and job record
taken off the command's clock (JOB_LOGS jl-5), does logging still cost a
measurable slowdown on the existing small-file and large-file benches?
The 2026-10-08 run (`docs/bench/job-logs-fast-2026-10-08/`) failed the
1 GiB clone cell by a constant 2–4 ms per command.

Rig: this Mac (macOS, APFS), quiet (no build, test or review running; the
system's malware scan of fresh binaries allowed to settle first), release
builds. Script: `scripts/bench_otp11_local_ab.sh` (interleaved old/new per
round, RUNS=5; its own gate: new median ≤ old + 10%), each binary with a
fresh `--config-dir`. Old binary: `f6ca55c0` (jl-1a only — the CLI and
daemon log nothing), exported with `git archive`. New: the jl-5 commit that
adds this folder (every command keeps a log and a job record).

| cell | old `f6ca55c0` | new (jl-5) | verdict |
|---|---|---|---|
| huge (1 GiB, APFS clone) | 21 ms | 21 ms | PASS |
| tree (256 MiB + 32 dirs) | 29 ms | 29 ms | PASS |
| small (10,000 × 4 KiB) | 1260 ms | 1259 ms | PASS |
| noop (synced mirror) | 27 ms | 28 ms | PASS |

An earlier pass of the same binary, before the full test run: huge 21 / 22,
tree 29 / 29, small 1264 / 1261, noop 23 / 24 — all PASS.

Per command, measured directly (`per_command.py`: 30 interleaved runs each,
one 4 KiB file copied into a fresh destination, each binary with its own
scratch `--config-dir`, wall time around the process):

| history | old median | new median | difference |
|---|---|---|---|
| fresh folder (0–30 runs kept) | 5.80 ms | 6.56 ms | 0.76 ms |
| full (50 kept: every run prunes a log and a record) | 5.78 ms | 6.64 ms | 0.85 ms |

(Before jl-5 the same difference was ~2.7 ms: 8.8 vs 6.1 ms, 2026-10-08.)

Verification re-run by the orchestrating session, independent of the
implementing agent (`ab-verify.{out,runs}`): the old binary's source proven
identical to `f6ca55c0` (`git archive f6ca55c0` diffed against the build
tree), the new rebuilt from the clean commit (`blit 0.1.3+031e03526d9d`),
fresh config folders, the machine quiet after the full gate: huge 21 / 21 ms,
tree 28 / 29, small 1267 / 1275, noop 27 / 27 — every cell PASS. On the 1 GiB
cell the new binary trailed by 0, 1, 2, 0 and 1 ms over the five rounds.

Files: `ab.{out,runs}` (the table), `ab-first-pass.{out,runs}`,
`per-command.{out,runs}` (fresh folder) and `per_command.py`. The full-history
row is from the same binary against a folder already holding 50 runs.

What was cut (a throwaway timing build, never committed, marked each step):

- **Start: ~1.5 ms → 0.03 ms on the clock.** `CommandRun::start` touches no
  disk: the log's thread reads this machine's folder, settings and ID,
  finishes earlier commands' leftover logs and opens the log; a thread of
  its own writes the spec and the `running` record. The transfer's events
  wait in the log's bounded queue meanwhile.
- **Close: ~2.5 ms → ~0.7 ms.** The log's finish and the record's run at
  the same time instead of one after the other.
- **Record writes: ~0.45 ms each → ~0.1 ms.** `write_document` serialized
  pretty JSON straight into the file — a system call per token; it now goes
  through a bounded buffer (same bytes). The flushes themselves were never
  the cost (~0.03 ms each).
- **Record pruning** reads only the records beyond the newest `keep`,
  ordered by last write, instead of reading and parsing every kept record.

What remains (~0.7–0.85 ms per command) is the two closing chains, each
~0.5–0.6 ms: the log's `run-end` flush, compression, rename and folder
prune; the record's replace and the record folder's prune. Every flush
point is kept.
