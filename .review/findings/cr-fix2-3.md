# cr-fix2-3: Lossy-name collisions shield only one physical destination path

**Severity**: HIGH — on a lossy-name collision the shield can resolve to the other raw entry, so a failed entry's populated destination directory is deleted under mirror
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range 44200834..be0b9fda (review-fix batch 2), record .review/results/ssc-fix2-range.codex.json)

## Evidence
crates/blit-core/src/mirror_planner.rs:300 — shield construction uses raw_entries.iter().find for a lossy text identity, even though multiple distinct raw paths can collapse to that same text.

## Predicted observable failure
On a byte-keyed destination, two non-UTF-8 source names can share one lossy text path. When one is rejected as the duplicate and its destination is a populated directory, the shield may resolve to the other raw name; mirror deletion then removes descendants beneath the failed entry even though that subtree was required to remain untouched.

## Reviewer's suggested approach
Carry the failed entry's structured raw identity into mirror planning. At minimum, conservatively shield the textual identity and every raw entry sharing that text instead of selecting the first match.

## What
(coder fills in)

## Guard proof
(red/green proof; mutation described)

## Known gaps
(none yet)
