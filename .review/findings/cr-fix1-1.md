# cr-fix1-1: Empty failed path does not shield the destination root

**Severity**: HIGH — a failed single-file mirror source (empty relative path) leaves its populated destination root unshielded; the mirror can erase the whole destination directory while reporting one contained failure
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range ed4bc773..b342d636 (review-fix batch), record .review/results/ssc-fix1-range.codex.json)

## Evidence
crates/blit-core/src/mirror_planner.rs:397 — is_shielded breaks before checking an empty ancestor, although an empty relative path is the supported identity for a single-file source root.

## Predicted observable failure
If a single-file mirror source fails after enumeration while its destination root is a populated directory, every child is still planned for deletion; the mirror can erase the entire destination directory despite recording the source file as failed.

## Reviewer's suggested approach
Check the current path against shield_set before stopping at the empty path, and add a regression covering an empty failed path with populated destination descendants.

## What
(coder fills in)

## Guard proof
(red/green proof; mutation described)

## Known gaps
(none yet)
