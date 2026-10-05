# cr-rework-1: --ignore-existing retries can falsely clear leftovers beyond the 64-entry named report

**Severity**: HIGH — a run with more than 64 incomplete copies left behind under `--ignore-existing` can exit 0 with incomplete files at the destination
**Status**: Open (admitted at intake; fix approach awaits the owner's go)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (owner-approved codereview over c4b69fa6..2be91345, the cr-win-1 rework + D8; record .review/results/ssc-rework-range.codex.json)

## Evidence
crates/blit-cli/src/transfers/retry.rs:118 — left_in_place derives its override set only from the capped failures sample, while retry_set at lines 98-102 uses the larger failed_paths set; crates/blit-core/src/remote/transfer/sink.rs:280 caps failures at 64.

## Predicted observable failure
If more than 64 files leave incomplete destination copies during an --ignore-existing run, later paths retain --ignore-existing on retry and are skipped because their partials now exist. If the first 64 converge, the pass reports no failures and exits 0 while incomplete files remain.

## Reviewer's suggested approach
Carry a typed left-in-place disposition for every retry path alongside failed_paths. If that identity must be truncated, preserve the unknown remainder as unretried instead of inferring behavior from the capped human-readable report.

## Intake
Admitted. The cr-win-1 record listed "leftovers beyond the report cap retry under the user's flag" as a known gap but understated it: under `--ignore-existing` that retry SKIPS the leftover (it exists), the skip clears the failure, and the run can exit 0 — the false success cr-win-1 exists to remove. Reachable wherever a run leaves more than 64 incomplete copies with `--ignore-existing`: a local `--resume` copy of new files that fail, or targets another process keeps from being removed.

## What
(coder fills in)

## Guard proof
(red/green proof)

## Known gaps
(none yet)
