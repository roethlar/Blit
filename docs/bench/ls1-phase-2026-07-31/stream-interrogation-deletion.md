# ls-6 — destination stream interrogation deleted from the default compare

**Status**: Evidence. Single run, 2026-08-01. Ruling: D-2026-08-01-4.
Plan: `docs/plan/LOCAL_SMALL_FILE_PATH.md` (ls-6).

## What was run

    BLIT_TRACE_LOCAL_PHASES=1 BLIT_TRACE_RUN_ID=ls6-smb-converged-1 \
      blit copy 'D:\Apps\' 'H:\apps\' --dry-run

Same protocol and tree as every run in this directory.
Log: `logs/ls6-smb-converged-1.log`.

## Result

    Up to date: 46041 files examined, 0 changed (dry run) (in 55.54s)

| run | wall | vs field complaint (283.92 s) |
|---|---:|---:|
| ls-1 step (0), serial diff | 273.57 s | 1.04× |
| adaptive checkers | 166.38 s | 1.71× |
| ls-5 directory sweep | 114.73 s | 2.47× |
| **ls-6 no stream interrogation** | **55.57 s** | **5.1×** |

COMPARE_METADATA — 915.8 s of summed checker-thread time yesterday —
is now **5.8 ms across all 46,041 files** (~126 ns each: a DWORD compare).
The compare is sweep-bound: COMPARE_SWEEP 24.5 s summed over 6,191
directories (~4 ms/directory round trip), COMPARE_STAT 37.5 s summed
(contains the sweeps and their lock waits, under checker concurrency).

## What changed, contractually

The default (size+mtime) compare no longer asks size/mtime-matched
destination files to enumerate their streams. Streams are carried whenever
a file transfers (unchanged, rel-4), attributes are still judged from the
sweep-supplied DWORD and repaired in place (unchanged, pfc-6), and
`--checksum` keeps the exhaustive per-file stream verdict. A destination
stream divergence on an otherwise-unchanged file is now INVISIBLE to a
default run — by ruling, not by accident: its only remedy was whole-file
replacement, so the interrogation never protected destination streams, it
found them sooner to destroy them sooner, at ~90 s per converged run. No
peer tool interrogates skipped files (robocopy's skip decision is
size+timestamp; streams ride `D=Data` when it copies).

Guards: `stream_divergence_is_invisible_to_the_default_compare` (local) and
the reworked pfc-6 wire guard both pin the skip-and-stale-stream-survives
contract and go red if the interrogation returns;
`checksum_compare_still_replaces_stream_divergence` and the CLI-level
`--checksum` legs pin the exhaustive path.

## Remaining structure

ENUMERATE_BACKPRESSURE 52.9 s — the source walk still spends the run
blocked on the diff, which is now bound by one `read_dir` round trip per
destination directory. Next measured lever if wanted: overlap/pipeline the
directory sweeps (prefetch), or the wire carrier's concurrent-session
measurement (pre-existing queue item). Caveats: single run, one SMB
server, one tree.
