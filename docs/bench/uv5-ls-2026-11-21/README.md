# `ls` on a daemon, before and after slice 5 — 2026-11-21

**Status:** Complete. The speed record `docs/plan/UTILITY_VERBS_LOCAL_PARITY.md`
AC-7 asks for, for the slice that changed how a directory is listed.

## Bottom line

Listing one directory of 200,000 files on Btrfs took `blit ls` 253 ms and
now takes 58 ms, beside 28 ms for the system `ls` on the same directory. With
`-l` it takes 232 ms beside the system's 319 ms. On tmpfs: 71 ms beside 50 ms,
and with `-l` 232 ms beside 260 ms. A directory of 1,000 files is
listed in about 2 ms either way, which is the cost of starting the command
and reaching the daemon.

## Method

Linux host `continuum`, loopback, release builds, each build's own daemon
serving the directories read-only, the system `ls` under `LC_ALL=C.UTF-8`,
best of five wall-clock runs of the whole command with its output read
whole. Run twice: with the directories on tmpfs and with them on Btrfs.
"Before" is slice 4 (`35adeacb`), whose `ls` printed a header and a
`FILE`/`DIR` row for each entry, looking each one up; "after" is slice 5.
`bench_uv5.py` is the script — it works in a directory of its own inside
the one it is given, and removes nothing else; `result.txt` its output
for both runs, medians included.

## Result (best of five)

| Command | tmpfs | Btrfs | Printed |
|---|---|---|---|
| system `ls big` | 50.3 ms | 28.4 ms | 200,000 names |
| system `ls -l big` | 260.5 ms | 319.4 ms | 200,000 entries |
| system `ls small` | 0.5 ms | 0.9 ms | 1,000 names |
| system `ls -l small` | 1.7 ms | 2.0 ms | 1,000 entries |
| before: `blit ls HOST:/bench/big/` | 255.6 ms | 252.9 ms | 200,000 rows, each looked up |
| before: `blit ls HOST:/bench/small/` | 2.6 ms | 2.8 ms | 1,000 rows |
| after: `blit ls HOST:/bench/big/` | 71.0 ms | 57.6 ms | 200,000 names |
| after: `blit ls -l HOST:/bench/big/` | 231.9 ms | 232.1 ms | 200,000 entries |
| after: `blit ls HOST:/bench/small/` | 1.9 ms | 1.9 ms | 1,000 names |
| after: `blit ls -l HOST:/bench/small/` | 2.7 ms | 2.5 ms | 1,000 entries |

Before, the daemon looked every entry up, sorted the rows and sent them
in one reply. After, a listing without `-l` is names and nothing else: no
entry is looked up, names travel 512 to a message as the directory gives
them, and this end sorts and prints. With `-l` each entry is looked up
once, as `ls -l` looks it up.

## What is pinned by a guard, not by this stopwatch

A listing without its facts looks up the path given and no entry:
`a_directory_is_listed_into_and_its_entries_are_not_looked_up`
(`crates/blit-core/src/listing.rs`) counts the lookups — one, with or
without `-a` — and one more for each entry once facts are asked for.

## Limits

- One machine, one run of the script per filesystem, Linux only. Nothing
  was run on Windows or macOS.
- Empty files, short names: the listing's own cost and nothing else.
- Names alone take about twice the system's time on Btrfs and half as
  long again on tmpfs: 200,000 names cross a connection, are sorted here
  and printed. Nothing here was profiled.
- As first written this slice wrote each line to standard output by
  itself and took 104 ms for the 200,000 names on Btrfs (124 ms on
  tmpfs); it now writes through a buffer. No guard pins that.
