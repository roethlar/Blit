# cr-ecm5-5: Under a list of names, directories the list did not name were deleted

**Severity**: MEDIUM — a mirror removed empty directories outside its stated scope; no file was affected
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm5-5: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r5` over `3bc3ee47..585d3599` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r5.codex.md`.

## Evidence
With the pinned build at both ends, a comments-only list still caused the daemon to remove two unlisted empty directories. A local mirror also removed an empty directory present on both sides. The reviewer asked that subset deletion be short-circuited when the explicit list is empty.

Reproduced here before any change (2026-11-20, Linux, local `mirror --yes`), and wider than the empty list: with a list naming `a.txt` alone, the destination's `emptydir/` (also in the source), `only-dst-empty/` and `sub/emptysub/` were all removed.

Cause: under a list the walk admits every directory, because a listed file may lie anywhere below (`FileFilter::should_include_dir`, `crates/blit-core/src/fs_enum.rs`), and the deletion planner treats every directory the destination walk returns as a candidate unless a kept source file lies under it (`crates/blit-core/src/mirror_planner.rs`). Directories are then removed only if empty, so the damage is to empty directories. Older than this plan; cr-ecm4-2's guard asserted "nothing is in the deletion scope" under an empty list and checked files only.

Not the same thing, and unchanged: with NO list, a mirror removes an empty destination directory even when the source has it too, because transfers do not carry empty directories (stated in `blit check --help` — "Transfers don't preserve them today" — and nowhere in `mirror`'s own help; recorded for the owner in `docs/STATE.md`).

## Predicted observable failure
`blit mirror --files-from LIST SRC DST`: every empty directory at the destination that the list does not vouch for is removed, on every route.

## Intake
Admitted, for every list and not only the empty one: the rule the plan states is that the list is the scope.

## What
Under a list of exact names, a destination directory is a deletion candidate only when the list names it (`only_listed_directories`, applied by each planner that takes a filter). A list that names nothing therefore plans nothing; a listed file the source no longer has is deleted as before; a listed directory is a candidate like any listed entry.

## Guard proof
- `mirror_planner::…::under_a_list_of_names_a_directory_it_does_not_name_is_never_deleted`: one destination tree planned three ways — a list naming a file the source lacks and a directory (exactly those two are planned), a list naming nothing (nothing planned), no list (every extraneous entry, the empty directories among them — the older rule, pinned so the difference is the list's doing).
- `ignore_case::under_a_list_of_names_a_mirror_leaves_unlisted_directories_alone`, through the binary: locally, pushed to a real daemon and pulled from one, each with a list that names nothing and with one that names a file — an empty directory on both sides, one below a directory, one only at the destination and a stale file all stay; only the listed file is copied.
- Mutations, each restored byte for byte: every directory walked a candidate again → both guards red; a listed directory no candidate either → the first red.

## Files changed
- `crates/blit-core/src/mirror_planner.rs` — `only_listed_directories`; the guard.
- `crates/blit-cli/tests/ignore_case.rs` — the end-to-end guard.

## Known gaps
- A directory left empty because this same mirror deleted the listed files in it is not removed unless the list names the directory.
- The no-list behaviour above (empty directories are not carried, and a mirror removes them) is older than this plan and stated only in `check --help`; whether it should be so is the owner's question, not this finding's.
- The daemon-to-daemon route is not run; it plans deletions through the same function.
- Run on Linux only.
