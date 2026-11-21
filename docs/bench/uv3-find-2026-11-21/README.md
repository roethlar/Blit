# `find` on a daemon, before and after slice 3 — 2026-11-21

**Status:** Complete. The speed record `docs/plan/UTILITY_VERBS_LOCAL_PARITY.md`
AC-7 asks for, for the slice that changed `find`'s walk.

## Bottom line

Over 200,000 empty files in 2,000 directories, listing every path took
`blit find` 307 ms and now takes 62 ms, beside 33 ms for the system
`find` on the same tree. With a test that selects a tenth of the entries
(`-name 'f00*'`) it takes 32 ms beside the system's 38 ms, and listing
the directories alone 26 ms beside 25 ms: what is not found is never
sent.

## Method

Linux host `continuum`, tmpfs, loopback, release builds, each build's own
daemon serving the tree read-only, `LC_ALL=C.UTF-8`, best of five
wall-clock runs of the whole command with its output read whole. "Before"
is the tree at `a13238b6`, whose `find` took `--pattern` and no test of
`find`'s; "after" is slice 3. `bench_uv3.py` is the script; `result.txt`
its output.

## Result

| Command | Time | Lines |
|---|---|---|
| system `find .` | 32.5 ms | 202,001 |
| system `find . -name 'f00*'` | 37.9 ms | 20,000 |
| system `find . -type d` | 25.0 ms | 2,001 |
| before: `blit find HOST:/bench/` | 306.7 ms | 202,001 |
| before: `blit find HOST:/bench/ --pattern 'f00*'` | 252.1 ms | 20,001 |
| after: `blit find HOST:/bench/` | 61.8 ms | 202,001 |
| after: `blit find HOST:/bench/ -name 'f00*'` | 32.2 ms | 20,000 |
| after: `blit find HOST:/bench/ -type d` | 25.9 ms | 2,001 |

Before, the daemon read the metadata of every entry (a `stat` each), sent
one message per entry with its size and time, and the client printed a
padded table. After, an entry's kind comes with its directory's listing,
no entry but the start is looked up, the paths found travel 512 to a
message, and the client prints each as it is.

## What is pinned by a guard, not by this stopwatch

`find` with no test that needs it looks up no entry but the start:
`only_the_start_is_looked_up` (`crates/blit-core/src/find.rs`) counts the
lookups of a walk over 421 entries — one.

## Limits

- One machine, one run of the script, Linux only. Where a directory's
  listing does not say what kind an entry is (some filesystems; the
  standard library then asks), a lookup per entry returns and this guard
  cannot see it.
- Empty files on tmpfs: the walk's cost and nothing else.
- The full listing is still about twice the system's: 200,000 paths cross
  a connection and are printed with the operand before each.
