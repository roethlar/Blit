# cr-ecm8-3: A list line holding a NUL refused the whole request through a daemon

**Severity**: MEDIUM — one malformed line of a `--files-from` list changed the outcome by route: reported and passed over between local folders, the whole transfer refused through a daemon
**Status**: Fixed — awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm8-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r8` over `3bc3ee47..f4203c31` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r8.codex.md`.

## Evidence
With an embedded NUL in one `--files-from` entry followed by `ok.txt`, local copy reported the missing entry and copied `ok.txt`. A daemon pull rejected the entire request with `PROTOCOL_VIOLATION`. The reviewer: classify NUL-bearing entries as unnameable before building the request, so valid entries still transfer.

Cause: cr-ecm7-1 told a list's lines apart — the names, and the lines that name nothing, which travel apart — by one test: a relative path whose every component is a name. A line holding a NUL passes it (to a path parser a NUL is a character like another), so it was sent among the names; and the receiver, by the same fix, refuses a NAME that holds one, and with it the list. The two halves of one fix disagreed about one kind of line.

## Predicted observable failure
Any list holding a NUL in a line — a file written with `find -print0` and read as lines, a UTF-16 file — pushed, pulled or sent daemon to daemon: nothing is transferred, where the same command between local folders copies every other name.

## Intake
Admitted.

## What
A line holding a NUL is one of the lines that name nothing: no entry's name holds one. `split_listed` puts it with them, so it travels in `FilterSpec.files_from_unnamed`, is matched against nothing, and is reported by the source as a listed name it did not find — and the names beside it are transferred. A retry's list is told apart the same way.

With that, every entry a sender of this contract writes among the names is one the receiver accepts: components that are names, none empty, none `.` or `..`, none holding a NUL.

## Guard proof
- `transfers::filter::tests::a_list_line_holding_a_nul_travels_apart_from_the_names`: a list of two such lines (one in a folder) and `ok.txt` is sent as one name and two lines apart; the receiver takes the spec; the filter here and the filter there hold the same two parts and pass `ok.txt`; a retry's list of such a line is sent apart too, and taken.
- `tests/ignore_case.rs::a_list_line_that_names_nothing_is_reported_on_every_route`, through the binary and a real daemon, now with `nul<NUL>name.txt` beside the three earlier lines: local, push and pull alike end unsuccessfully, report the line, and copy the name beside it.
- Mutation, restored byte for byte: a line holding a NUL sent among the names → both guards red (the end-to-end one on push and pull, where the request is refused).

## Files changed
- `crates/blit-core/src/transfers/filter.rs` — `split_listed`; the guard.
- `crates/blit-cli/tests/ignore_case.rs` — the end-to-end case.

## Known gaps
- The report prints the line with the NUL shown as a replacement character, as any name is printed.
- A list that holds NULs is very likely a list that was not written one name per line; blit reports each such line as not found and does not say so. `--exclude-from` and `--include-from` refuse a file holding a NUL outright; whether `--files-from` should is not decided here.
- Run on Linux only.
