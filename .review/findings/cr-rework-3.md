# cr-rework-3: a leftover the retry removed stays classified, so a file recreated during the wait is overwritten

**Severity**: HIGH (reviewer) — under `--ignore-existing`, a file another process creates at a failed path during the retry wait can be overwritten
**Status**: Open — real; fix or decline is the owner's call
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (owner-approved re-review of the cr-rework-2 fix, range 343ddf6f..9118a88e; record .review/results/rework2-fix-range.codex.json)

## Evidence
crates/blit-cli/src/transfers/retry.rs:189 — every previously classified leftover is re-added whenever its path still fails, even when the retry touched the destination and positively removed the old partial before reporting that failure.

## Predicted observable failure
If retry 1 opens the old partial, encounters a source failure, and successfully removes the partial during abort, the path is nevertheless retained as a leftover. If another process creates that destination during the wait before retry 2, retry 2 forcibly disables --ignore-existing and overwrites the newly created file, violating the requested protection and causing data loss.

## Reviewer's suggested approach
Carry an explicit per-path disposition from each retry—untouched, left in place, or positively removed—and preserve prior membership only for untouched failures. Clear it after confirmed removal, and add a regression test covering removal followed by destination recreation before the next retry.

## Intake
Real, and introduced by the cr-rework-2 fix (`a60368b1`), which keeps a leftover classified while its path keeps failing without knowing whether a later pass removed it. Narrow: it needs `--ignore-existing`, two or more retries, a retry that touches and removes this run's leftover and still fails, and another process creating a file at that exact path within the retry wait. The main pass has the same diff-to-write race over milliseconds; this one spans the retry wait (30 s by default). The precise fix is a third per-path disposition, "positively removed", sent exactly beside `left_in_place`.

## What
(coder fills in)

## Guard proof
(red/green proof)

## Known gaps
(none yet)
