# cr-ecm11-2: A saved job's leftovers were split under two names for one file

**Severity**: HIGH — `jobs retry` of an `--ignore-existing` single-file job exits 2 and leaves the run's own partial bytes in place
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the fix is `Fix cr-ecm11-1`, the commit before this one; this commit (`Fix cr-ecm11-2`) is its guard
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r11` over `d3e33acc..bfa537f9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r11.codex.md`. It completes cr-ecm10-3.

## Evidence
"I marked the root as an incomplete output in a failed `--ignore-existing` job record. `jobs retry` looked for the empty path on its repair pass, skipped `Report.txt` on its other pass, and exited 2 with the partial bytes still present. Both sets need to use the same identity when the split is made."

Cause. A retry of an `--ignore-existing` job runs in two parts (cr-jl4-1): the failed files whose write left the run's own incomplete copy, with the flag off, and the rest with it on. The two sets are the failed paths and, among them, the leftovers. cr-ecm10-3 put a single-file source's name in place of the empty path in the first set and not in the second; split, they were two different files. The part that repairs asked for the empty path — a line naming nothing — and the other part asked for `Report.txt` with the flag on and skipped the partial copy as a file to keep.

## Intake
Admitted as the reviewer put it. One cause with cr-ecm11-1, and one fix: the paths are handed on as recorded, so both sets hold the empty path, and the source itself answers to it. No code of this finding's own remained to change; what it needed is a guard.

## What
Nothing beyond cr-ecm11-1's change. The guard added here is the reviewer's scenario.

## Guard proof
- `job_retry.rs::an_ignore_existing_retry_of_a_single_file_replaces_its_own_incomplete_copy`: a single-file `--ignore-existing` copy fails; with the root marked in the record as left in place and a partial copy at the destination, `jobs retry` exits 0 and the file is whole; without the mark, a file put there since is kept, exit 0. The record's failure is under the empty path.
- Mutations, each restored byte for byte: the pass that repairs reading the empty path as a line naming nothing, as before → the guard red; the leftovers not split from the rest → red.

## Files changed
- `crates/blit-cli/tests/job_retry.rs` — the guard.

## Known gaps
- The guard writes the mark into the record by hand, as cr-jl4-1's does: a write that fails part way and leaves its own copy is not provoked.
- Run on Linux only.
