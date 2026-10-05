# cr-rework-2: a retry-side source failure drops a path's leftover classification

**Severity**: HIGH — with `--ignore-existing` and two or more retries, a copy/mirror can exit 0 with this run's incomplete copy still at the destination
**Status**: Open (admitted at intake)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (owner-approved re-review of the cr-rework-1 fix, range 6e0e5a7b..4cdef0a8; record .review/results/rework-fix-range.codex.json)

## Evidence
crates/blit-cli/src/transfers/retry.rs:171 — after_pass returns the next pass's PassFailures without carrying self.left_in_place; lines 353-396 then use that dropped set to decide whether to disable --ignore-existing.

## Predicted observable failure
With --ignore-existing and at least two retries, the main pass can leave an incomplete destination copy, then the first retry can fail while scanning or reading the source before touching that copy. Its summary has an empty left_in_place set, so the next retry restores --ignore-existing, skips the still-existing partial, clears the failure, and exits 0. For a move, the zero failure count also permits deletion of the source, causing data loss.

## Reviewer's suggested approach
Preserve prior left-in-place membership for paths that remain failed across a retry, clearing it only when the path succeeds or the destination positively reports that the old target was removed. Add a three-pass guard covering leftover creation, a source-side retry failure, and subsequent recovery under --ignore-existing.

## Intake
Admitted for copy/mirror. The move consequence does not apply: `blit move` refuses `--ignore-existing` up front (`run_move_inner`, R51-F1), so a move never takes this path. A leftover is this run's own copy for as long as its path keeps failing; a pass that never touched it cannot un-classify it.

## What
(coder fills in)

## Guard proof
(red/green proof)

## Known gaps
(none yet)
