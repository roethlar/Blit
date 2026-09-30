# cr-fix3-1: Collision shielding preserves an unrelated lossy-text subtree

**Severity**: MEDIUM — while a raw-named source entry keeps failing, a distinct valid-UTF-8 destination path equal to its lossy rendering (and its subtree) is never deleted by mirror, so the destination cannot converge
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range be0b9fda..5251584c (review-fix batch 3), record .review/results/ssc-fix3-range.codex.json)

## Evidence
crates/blit-core/src/mirror_planner.rs:305 — The new unconditional insertion shields the lossy text path even when raw_names_storable is true; lines 275-282 establish that without a representable source entry this text path is a distinct extraneous destination identity.

## Predicted observable failure
On a byte-keyed Linux/BSD destination, if a raw non-UTF-8 source entry fails while the destination also contains a valid UTF-8 path equal to its lossy rendering, mirror skips that unrelated path and its descendants. The partial run leaves stale destination data, and repeated runs cannot converge while the source failure persists.

## Reviewer's suggested approach
On storable destinations, shield every matching decoded raw path but add the text path only when source_files contains that representable identity; on unstorable destinations, continue shielding the text identity. Carrying the failed entry's structured raw identity would make this exact.

## What
(coder fills in)

## Guard proof
(red/green proof; mutation described)

## Known gaps
(none yet)
