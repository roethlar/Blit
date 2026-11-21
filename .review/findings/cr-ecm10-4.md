# cr-ecm10-4: `check` of a single file against a destination of another name looked for the wrong name

**Severity**: MEDIUM — a copy that succeeded was reported as "Missing on destination", exit 1
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm10-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r10` over `df682eea..41b4bfdd` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r10.codex.md`. Completes cr-ecm9-3 with cr-ecm10-3.

## Evidence
A listed copy from `Report.txt` to `Saved.bin` exited 0. A checksum check with the same list reported "Missing on destination" and exited 1. The unfiltered checksum check found one match. The reviewer: use the source basename to select both operands, then compare their actual paths.

Cause. cr-ecm9-3 made the walk ask its filter about a single-file root under the file's own name. `check` walks both roots with the same filter, so the destination file was asked about under ITS name: `Saved.bin`, which a list naming `Report.txt` does not hold. The source's file was found, the destination's filtered away. A pattern went wrong the same way — it reads an entry's name off the path it is given.

## Predicted observable failure
`check` after any single-file copy to another name, with `--files-from` naming the file: "Missing on destination". With `--exclude` naming the destination's own name: the destination filtered away and the same report; with `--exclude` naming the source: the source filtered away and the destination reported "Missing on source".

## Intake
Admitted as the reviewer put it.

## What
- `FileEnumerator::answering_to(name)`: a root that is one file is filtered under `name` in place of its own — for a list and for a pattern alike. A root that is a directory is not affected.
- `check::compare_trees`: when the source root is a file, the destination is walked answering to the source's name. The two files are then the one entry a copy of that source made, and are compared.

## Guard proof
- `check::tests::a_single_file_is_compared_under_the_sources_name` (the module's first unit guard): `Report.txt` against an equal and an unequal file of another name — listed by the source's name (compared: one match; one difference), listed by the destination's (nothing compared, as a copy with that list copies nothing), excluded by the source's name and by a glob over it (left out on both sides), excluded by the destination's own name and by a glob over it (names nothing: compared), unfiltered; and two trees, each entry by its path as before.
- `single_file_copy.rs::a_file_copied_under_another_name_checks_under_the_sources`, through the binary — the reviewer's commands: the listed copy exits 0; `check --checksum` with and without the list finds one match; after the destination is changed it reports one difference and no missing file; `--exclude '*.bin'` compares the two and finds them differing; `--exclude '*.txt'` leaves the entry out.
- Mutations, each restored byte for byte: the destination asked about under its own name → both guards red; a pattern still reading the destination's own name off its path → both.

## Files changed
- `crates/blit-core/src/enumeration.rs` — `FileEnumerator::answering_to`, the depth-0 file.
- `crates/blit-core/src/check.rs` — `compare_trees`; the unit guard.
- `crates/blit-cli/tests/single_file_copy.rs` — the end-to-end guard.
- `CHANGELOG.md`.

## Known gaps
- A single file checked against a DIRECTORY is still "Missing on destination": the directory's entries are known by their paths, and none is the empty one. `check` does not resolve a destination the way `copy` does (into the directory).
- `check` reports the one entry of two single files with an empty name (noted under cr-ecm9-3).
- Run on Linux only.
