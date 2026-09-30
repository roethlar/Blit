# cr-ssc1-1: Skipped source paths must shield their destination subtree from mirror deletion

**Severity**: HIGH — contained source failure silently destroys pre-existing destination data under mirror
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `8c1b8dad (+ follow-up 5d557004)`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range f74b0b1a..6bc3d08f, record .review/results/ssc-1-range.codex.json)

## Evidence
crates/blit-core/src/transfer_session/mod.rs:4431 and crates/blit-core/src/transfer_session/mod.rs:4872 — source skips are recorded, but the mirror pass receives only source_files; crates/blit-core/src/mirror_planner.rs:221 keeps each source path and its ancestors, not descendants protected by a failed path.

Verified against master after ssc-3 (2026-09-30) by the orchestrator before admission.

## Predicted observable failure
If the source manifest contains file `node`, opening it later fails, and the destination currently has directory `node/` with contents, mirror deletes every descendant under `node` while preserving only the empty directory. The transfer reports one contained failure but loses the destination subtree.

## Reviewer's suggested approach
Carry the uncapped failed-path set through every outcome merge and pass it to the mirror planner. Exclude each failed path and all component-wise descendants from deletion while still deleting unrelated extraneous entries.

## What
`SinkOutcome::merge_failures` now folds `failed_paths` in (the exact, uncapped set; only the wire report stays bounded) and exposes `failed_paths()`. `mirror_delete_pass` and `MirrorPlanner::plan_session_deletions` take that set as `shielded`; `plan_from_sets` skips every destination entry that is, or is a component-wise descendant of, a shielded path (`is_shielded`, casefolded like the keep-set). The session passes `contained_failures.failed_paths()` — source-side skips and retractions and destination-side containment alike — into the pass, on every route. Unrelated extraneous entries are still deleted.

## Guard proof
`mirror_shields_the_destination_subtree_of_a_skipped_source_file` (`tests/source_side_containment.rs`, in-stream and data plane: source FILE `node` skipped, destination DIRECTORY `node/` with two nested files kept byte-identical, `stale.txt` deleted, `entries_deleted == 1`) and `local::tests::mirror_shields_the_destination_subtree_of_a_vanished_source_file` (local route via `VanishingSource`, same shape). Mutation: the shield check `&& false` in `plan_from_sets` → both guards FAILED (`node/keep.txt` deleted; `scratchpad/cr-ssc-mutations.txt`); restored → green; the mirror, pfc and local_session suites green.

## Known gaps
None. `file_failed` on a merged outcome now answers per path (exact set) instead of conservatively; the one lane that read a merged outcome (the resume block record) is unaffected in behaviour.
