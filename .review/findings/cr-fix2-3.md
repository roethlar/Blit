# cr-fix2-3: Lossy-name collisions shield only one physical destination path

**Severity**: HIGH — on a lossy-name collision the shield can resolve to the other raw entry, so a failed entry's populated destination directory is deleted under mirror
**Status**: Verified
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
`plan_session_deletions` (`mirror_planner.rs`) builds the failure shield conservatively: for each failed text identity it shields the text path itself AND every raw-named entry that collapses to that text (decoded through `path_from_received_raw` where the host can store raw names), instead of a first-match `find` between colliding raw entries. A failed entry known only by its text can therefore never leave one of its physical identities unshielded.

## Guard proof
Guard: `mirror_planner::shield_tests::a_lossy_collision_shields_every_raw_entry_sharing_the_text` — two raw entries (`first.bin`, `second`) share one lossy text; the text is shielded; the destination has `first.bin`, a populated `second/child.bin`, and `extraneous.bin` → only `extraneous.bin` is planned, no directories. ASCII raw bytes keep it portable (ungated, runs on every platform).
Mutation (`scratchpad/cr-ssc-mutations-3.txt`): first-match `find` restored → the guard FAILED (`second/child.bin` planned for deletion); restored → all five shield tests green.

## Known gaps
The shield is still keyed by text at the recording site (`record_failure` takes the text path); carrying a structured raw identity from the failure record itself would let the shield be exact rather than conservative — not needed for safety, noted for a future tidy.
