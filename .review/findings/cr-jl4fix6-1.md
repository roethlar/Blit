# cr-jl4fix6-1: a later attempt cannot clear a stale leftover of its own stream

**Severity**: HIGH (reviewer: HIGH)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl4fix6-r1 over `9d380e61..8e2bbd74` (record `.review/results/jl4fix6-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/jobs.rs:441 — a fresh leftover set is created for every log and unconditionally united at line 474, although `logs_for_run` returns attempts ordered within each role and participant and `CommandRun::next_session` creates multiple numbered sessions for one run.

## Predicted observable failure
If attempt 1 leaves an incomplete file, attempt 2 successfully copies it, and a later attempt fails that identity without leaving a partial, the attempt-1 marker survives the union. For an ambiguous legacy record, `jobs retry --ignore-existing` can then treat a current file as Blit's own incomplete copy and overwrite it. A temporary focused guard failed at the reviewed head, passed when state was folded across same-role/participant attempts, and the worktree was restored to the head SHA.

## Reviewer's suggested approach
Group logs by participant and role, fold each group's attempts in order using one persistent leftover set, then union only each group's terminal set; retain the current rule that `FileSent` never clears leftover state.

## Intake
Admitted. cr-jl4fix5-1 made every log fold on its own, but a run's sessions on one machine and role are one stream in attempt order (`logs_for_run` lists them so): a copy in attempt 2 must clear a leftover of attempt 1, and a failure in attempt 3 without a leftover must not revive it. The fold keeps one set per participant-and-role stream across its attempts, and unites only each stream's terminal set.
