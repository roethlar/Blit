# cr-ecm9-3: A source that is one file was filtered under different names in different places

**Severity**: HIGH — `check` reported success on two unequal files, and `copy` wrote a file and exited 2 reporting it missing
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm9-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r9` over `3bc3ee47..ab7331c3` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r9.codex.md`.

## Evidence
With two unequal files named `Report.txt`, `copy --ignore-case --exclude './report.txt'` skipped the file, while `check` with those options reported a difference. With `Report.txt` in `--files-from`, `check` reported zero files and success on the unequal pair. `copy` wrote the file and then exited 2, reporting it missing. The reviewer: use the basename consistently for filtering and missing-file accounting, preserving the empty relative path used on the wire.

Cause. A source that IS one file has one entry, whose path relative to the source is empty. Three places decide about it, and each used a different name:
- the transfer's filter (`filter_manifest`) asks about the file's own name — since cr-ecm7-1;
- the local walk `check` runs (`enumerate_local_streaming`, depth 0) asked about the empty path, so a `--files-from` list never held it and a rule naming it never matched;
- the account of what a list asked for and the scan never gave (`requested_but_unscanned`) compared the list's name with the paths sent, and the one file was sent under the empty path.

## Predicted observable failure
For a single-file source: `check` with a list naming the file compares nothing and exits 0 whatever the two files hold; `check` with an exclude naming the file compares it though `copy` leaves it out; `copy` with a list naming the file copies it and ends with exit 2 and "did not land".

## Intake
Admitted as the reviewer put it.

## What
- **The walk asks about the file's own name.** At depth 0, for a root that is a file, the filter is asked about the root's base name; what is emitted is still the empty relative path. `copy` and `check` of the same source and options now decide alike.
- **The account knows the one file by its name.** `requested_but_unscanned` takes the source's base name; a list's name equal to it is not missing when the source sent a file under the empty path. Anything else the list names is still reported, and so is the name itself when nothing was sent.

Through the binary after the fix: `copy --ignore-case --exclude './report.txt'` and `check` with the same options both leave the file out (the destination unchanged; `0 files match, no differences found`); with `Report.txt` in `--files-from`, `check` reports the difference (exit 1) and `copy` copies the file and exits 0.

## Guard proof
- `enumeration::tests::a_root_that_is_one_file_is_filtered_by_its_own_name`: a root that is `Report.txt` is emitted under the empty path with no filter; listed by its name it is emitted, listed under another name or by an empty list it is not; six excludes in both case modes (`./Report.txt`, `./report.txt` exact and ignoring case, `Report.txt`, `*.txt`, `other.txt`); an include that names it and one that does not.
- `transfer_session::tests::the_one_file_of_a_single_file_source_is_not_missing_when_it_was_sent`: listed and sent, not missing, and the other listed name is; a name in another case is another file; with no source name, or nothing sent under the empty path, both are missing.
- `single_file_copy::a_single_file_source_is_one_name_to_copy_and_to_check`, through the binary: the reviewer's two commands for `copy` and for `check`, the exit codes, what is reported and what the destination then holds.
- Mutations, each restored byte for byte: the walk asks about the empty path → the walk's guard and the end-to-end guard red; the one file, listed and sent, still counted missing → the account's guard and the end-to-end guard; the name excused whether or not the file was sent → the account's guard.

## Files changed
- `crates/blit-core/src/enumeration.rs` — the depth-0 file; the guard.
- `crates/blit-core/src/transfer_session/mod.rs` — `requested_but_unscanned` and its caller; the guard.
- `crates/blit-cli/tests/single_file_copy.rs` — the end-to-end guard.
- `CHANGELOG.md` — under Patterns.

## Known gaps
- `check` of two single files prints the differing entry with an empty name (`*  (size (7 B vs 21 B))`): the entry's relative path is empty and the report prints that. Older than this finding and not changed by it; it is an imperfection of the report, noted here so that it is on record.
- The account's exemption is by the source's base name as text. A source whose name is not UTF-8 is compared through a lossy copy of it; a list cannot name such a file in any case (a list is text).
- Through a daemon the same account runs on the sending side; the end-to-end guard is local to local. The unit guards cover the function every route calls.
- Run on Linux only.
