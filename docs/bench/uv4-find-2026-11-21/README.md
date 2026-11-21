# `find` on a daemon: size, age and `-delete`, slice 4 — 2026-11-21

**Status:** Complete. The speed record `docs/plan/UTILITY_VERBS_LOCAL_PARITY.md`
AC-7 asks for, for the slice that gave `find`'s walk tests that ask each
entry about itself, and an action that removes what it finds.

## Bottom line

Over 200,000 empty files in 2,000 directories on Btrfs, `blit find …
-delete` removed everything in 1177 ms beside 1120 ms for the system
`find -delete` on the same tree, and `blit rm -rf` in 1108 ms. A test
that asks every entry about itself (`-size -1`) took 187 ms beside the
system's 140 ms; one that asks only the entries whose name already held
(`-name 'f00*' -size -1`, a tenth of them) 38 ms beside 40 ms. Listing
every path costs what it cost before the slice (59 ms; before: 58 ms).
On tmpfs, where removing costs far less, `-delete` takes 304 ms beside
the system's 248 ms: nearly a quarter slower there, a twentieth on Btrfs.

## Method

Linux host `continuum`, loopback, release builds, each build's own daemon
serving the tree, `LC_ALL` as the session had it, best of five wall-clock
runs of the whole command with its output read whole; the tree planted
afresh and synced before every removal. Run twice: with the tree on tmpfs
and with it on Btrfs — the two filesystems this project's gate runs on,
and removal costs more than four times as much on the second. "Before" is the tree
at `cade8d6c` (after slice 3, before this one), which has no `-size`,
`-mtime` or `-delete`; "after" is slice 4. `bench_uv4.py` is the script;
`result.txt` its output for both runs, medians included.

## Result (best of five)

| Command | tmpfs | Btrfs | Result |
|---|---|---|---|
| system `find t` | 34.1 ms | 25.5 ms | 202,001 lines |
| system `find t -size -1` | 131.0 ms | 140.4 ms | 200,000 lines |
| system `find t -type f -mtime -1` | 132.2 ms | 138.5 ms | 200,000 lines |
| system `find t -name 'f00*' -size -1` | 45.8 ms | 40.0 ms | 20,000 lines |
| system `find t -delete` | 247.8 ms | 1120.5 ms | all removed |
| system `find t -name 'f00*' -delete` | 71.2 ms | 152.6 ms | 20,000 removed |
| before: `blit find HOST:/bench/tree` | 68.9 ms | 57.7 ms | 202,001 lines |
| after: `blit find HOST:/bench/tree` | 73.2 ms | 58.7 ms | 202,001 lines |
| after: `blit find … -size -1` | 170.8 ms | 187.4 ms | 200,000 lines |
| after: `blit find … -type f -mtime -1` | 171.0 ms | 183.5 ms | 200,000 lines |
| after: `blit find … -name 'f00*' -size -1` | 42.9 ms | 38.2 ms | 20,000 lines |
| after: `blit find … -delete -y` | 304.5 ms | 1176.8 ms | all removed |
| after: `blit find … -name 'f00*' -delete -y` | 76.8 ms | 155.6 ms | 20,000 removed |
| after: `blit rm -rf HOST:/bench/tree`, for scale | 247.5 ms | 1107.9 ms | all removed |

`-delete` removes each entry as the walk gives it, by its name in a
directory held open (`blit_core::deletion::Remover`): one directory is
opened per directory emptied, not per entry, and nothing found is held or
sent. `blit rm -rf`, which lists and empties each directory through one
descriptor, is the faster of the two.

## What is pinned by a guard, not by this stopwatch

An entry is asked about itself only when a test needs it, and only once
its name and kind have held:
`contents_come_first_when_asked_and_an_entry_is_asked_only_when_needed`
(`crates/blit-core/src/find.rs`) counts the lookups — one for a walk with
name tests alone, one more for each entry whose name held once a size
test is added. `only_the_start_is_looked_up` still holds for a walk with
neither.

## Limits

- One machine, one run of the script per filesystem, Linux only. Nothing
  was run on Windows or macOS.
- Empty files: the walk's and the removal's own cost, and nothing else.
- A test that asks every entry is about a third slower than the system's
  on both filesystems, and listing every path about twice the system's,
  as slice 3 recorded. Neither was profiled here.
