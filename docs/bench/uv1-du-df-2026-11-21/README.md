# `du` and `df` on a daemon, before and after slice 1 — 2026-11-21

**Status:** Complete. The speed record `docs/plan/UTILITY_VERBS_LOCAL_PARITY.md`
AC-7 asks for, for the slice that changed `du`'s walk.

## Bottom line

Over 200,000 empty files in 2,000 directories, the total that took
`blit du` 882 ms now takes 154 ms, beside 130 ms for the system `du -s` on
the same tree. With `-a` — a line for every file — `blit du` takes 233 ms
beside the system's 199 ms. In all three forms the output is the system
`du`'s, byte for byte, once the operand is renamed.

## Method

Linux host `continuum`, tmpfs, loopback, release builds, each build's own
daemon serving the tree read-only, `LC_ALL=C.UTF-8`, best of five wall-clock
runs of the whole command. "Before" is the tree at `a13238b6`; "after" is
that tree with slice 1. `bench_uv1.py` is the script; `result.txt` its
output.

## Result

| Command | Time | Lines |
|---|---|---|
| system `du -s tree` | 129.9 ms | 1 |
| system `du tree` | 125.1 ms | 2,001 |
| system `du -a tree` | 199.2 ms | 202,001 |
| before: `blit du HOST:/bench/` | 882.1 ms | 202,002 |
| before: `blit du --max-depth 1 HOST:/bench/` | 266.3 ms | 2,002 |
| after: `blit du -s HOST:/bench/` | 154.4 ms | 1 |
| after: `blit du HOST:/bench/` | 154.6 ms | 2,001 |
| after: `blit du -a HOST:/bench/` | 232.9 ms | 202,001 |
| system `df -k tree` | 0.4 ms | 2 |
| before: `blit df HOST:/bench/` | 2.1 ms | 4 |
| after: `blit df HOST:/bench/` | 1.6 ms | 2 |

Before, the daemon walked the whole tree, kept a total for every path in
memory, sorted them, and sent a row for every file whatever was asked;
the client printed all of them. After, it walks once in the order `du`
prints, keeps one running total per open directory, and sends only the
lines that will be printed.

## What is pinned by a guard, not by this stopwatch

`du` without `-a` sends no row for a file:
`du_sends_a_row_for_each_line_it_prints_and_no_other`
(`crates/blit-daemon/src/service/admin.rs`) counts the rows the daemon
sends for each form of the request.

## Limits

- One machine, one run of the script, Linux only; nothing measured on
  Windows, where counting a hard-linked file once costs an open per file.
- Empty files on tmpfs: the walk's cost and nothing else. No file has
  content, no directory is on a disk.
- The first attempt at the "before" figure failed outright: the old `du`
  ended with an HTTP/2 error on its second run. That was the defect
  recorded in `docs/bench/h2-small-frames-2026-11-20/`; the figure above
  was taken after its fix.
