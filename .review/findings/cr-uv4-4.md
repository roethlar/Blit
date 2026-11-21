# cr-uv4-4: `-mtime` cut each time to a whole second before it subtracted them

**Severity**: MEDIUM — a file not yet a day old is selected by `-mtime +0`, and with `-delete` it is removed before its day is up
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv4-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r15` over `b8273ac1..35adeacb` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r15.codex.md`. The defect is in utilities slice 4 (`35adeacb`).

## Evidence
"Blit selected a file for `-mtime +0` when it was 86,399.12 seconds old. GNU `find` excluded it. With `-delete`, Blit can remove it before the 24-hour cutoff. Subtract full-precision timestamps, then round the elapsed duration."

Cause. The walk took the entry's time in whole seconds and the daemon its clock in whole seconds, and `AgeTest::holds` cut their difference to whole days. A file written at .88 of a second and looked at 86,399.12 seconds later — the clock on the second — gave two whole seconds 86,400 apart: one day, where less than a day had passed. The error is at most one second, always toward OLDER, and it is the wrong side for a test that decides what `-delete` removes.

## Intake
Admitted as the reviewer put it.

## What
A moment is its nanoseconds since 1970 (`blit_core::find::nanos_of`; before 1970, less than none). `AgeTest::holds` takes two such moments, subtracts them as they are, and cuts what has ELAPSED to whole 24-hour periods. The walk reads an entry's time as the filesystem keeps it (`Metadata::modified`), and only when a `-mtime` was asked for; the daemon reads its clock once, to the nanosecond, when the request arrives (`FindOptions::now_nanos`). Nothing on the wire changes: the daemon counts by its own clock, as before.

## Guard proof
- `find::tests::an_age_is_counted_from_the_whole_of_both_times`: the reviewer's case as arithmetic — written at .88, looked at 86,399.12 seconds later on the second, the two whole seconds 86,400 apart — is not `-mtime +0` and is `-mtime 0`; one nanosecond short of a day is not a day and the day itself is; a time to come by one nanosecond is less than no age; a moment before 1970.
- `find::tests::the_walk_reads_an_entrys_time_whole`: a file whose time is set to .88 of a second, walked with `-mtime +0` from a clock 86,399.12 seconds later (found nothing) and from one a day later (found).
- `size_and_age_are_counted_as_find_counts_them` — the rows GNU find 4.11.0 printed — holds unchanged, its whole seconds given as nanoseconds.
- Mutations, each restored byte for byte: each time cut to a whole second before the two are subtracted (both guards red); the walk reading the entry's time in whole seconds (the walk's guard red).

## Files changed
- `crates/blit-core/src/find.rs` — `nanos_of`; `AgeTest::holds` on nanoseconds; `FindOptions::now_nanos` (was `now_seconds`); the walk's reading of an entry's time; guards.
- `crates/blit-daemon/src/service/core.rs` — the clock, read whole.

## Known gaps
- Not compared with the system `find` at the edge: the two commands would have to run within the same part of one second. The rule is pinned as arithmetic and at the walk, with a time set to the nanosecond.
- On a filesystem that keeps whole seconds the walk's guard returns before it asserts: there is nothing finer to read.
- Linux only; nothing was run on Windows or macOS.
