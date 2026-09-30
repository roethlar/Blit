# cr-ssc1-1: Skipped source paths must shield their destination subtree from mirror deletion

**Severity**: HIGH — contained source failure silently destroys pre-existing destination data under mirror
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range f74b0b1a..6bc3d08f, record .review/results/ssc-1-range.codex.json)

## Evidence
crates/blit-core/src/transfer_session/mod.rs:4431 and crates/blit-core/src/transfer_session/mod.rs:4872 — source skips are recorded, but the mirror pass receives only source_files; crates/blit-core/src/mirror_planner.rs:221 keeps each source path and its ancestors, not descendants protected by a failed path.

Verified against master after ssc-3 (2026-09-30) by the orchestrator before admission.

## Predicted observable failure
If the source manifest contains file `node`, opening it later fails, and the destination currently has directory `node/` with contents, mirror deletes every descendant under `node` while preserving only the empty directory. The transfer reports one contained failure but loses the destination subtree.

## Reviewer's suggested approach
Carry the uncapped failed-path set through every outcome merge and pass it to the mirror planner. Exclude each failed path and all component-wise descendants from deletion while still deleting unrelated extraneous entries.

## What
(coder fills in)

## Guard proof
(red/green proof of the new guard; mutation described)

## Known gaps
(none yet)
