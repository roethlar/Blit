# `rm` on a daemon, before and after slice 2 — 2026-11-21

**Status:** Complete. The speed record `docs/plan/UTILITY_VERBS_LOCAL_PARITY.md`
AC-7 asks for, for the slice that changed how a tree is removed.

## Bottom line

Removing 200,000 empty files in 2,000 directories takes `blit rm -rf`
256 ms, beside 249 ms for the system `rm -rf` on the same tree and 246 ms
for `blit rm` before the slice. The removal is at local speed, as it was;
what changed is what it does (a directory needs `-r`, every entry that
cannot be removed is reported and passed, the exit status is `rm`'s).

## Method

Linux host `continuum`, tmpfs, loopback, release builds, each build's own
daemon serving the folder the tree is made in, `LC_ALL=C.UTF-8`. A fresh
tree for every run; the figure is the median of five wall-clock runs of
the whole command, the best beside it. "Before" is the tree at `a13238b6`
(`blit rm --yes`, which removed a tree without being told to); "after" is
slice 2. `bench_uv2.py` is the script; `result.txt` its output.

## Result

| Command | Median | Best |
|---|---|---|
| system `rm -rf t` | 248.6 ms | 242.9 ms |
| before: `blit rm --yes HOST:/bench/t` | 246.4 ms | 244.3 ms |
| after: `blit rm -rf HOST:/bench/t` | 255.5 ms | 253.7 ms |

## What the first measurement found

The slice's removal as first written took 296 ms — a fifth slower than
before. It removed each entry by its whole path, so the system walked
that path again for every file; the removal it replaced (the standard
library's) and `rm` itself remove an entry by name from the directory
they hold open. The slice now does the same (`HeldDirectory` in
`crates/blit-core/src/deletion.rs`, one `unlinkat` per entry on Unix),
and the figure above is with it. The cost by path grows with how deep
the module sits; this tree was about a dozen folders down.

## Limits

- One machine, one run of the script, Linux only. On Windows entries are
  removed by path, as before; nothing was measured there.
- Empty files on tmpfs: the cost of the removal calls and nothing else.
- Nothing pins the by-name removal but this measurement: removing by path
  gives the same result, slower.
