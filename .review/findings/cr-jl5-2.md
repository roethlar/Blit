# cr-jl5-2: Preserve the command's start timestamp across deferred initialization

**Severity**: LOW (reviewer: LOW)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled in after the fix)
**Reviewer**: codex / gpt-6-astra / xhigh / standard (inline, session-only — the owner's word `astra`), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-jl5-031e0352`; codereview defect hunt jl5-r1 over `df5fd1cf..031e0352` (record `.review/results/jl5-r1.codex.json` + `.events.jsonl`); 2026-10-13 18:03–18:18Z; capability_ok=true; guard_confirmed=true; SHAs pin-verified; the local ptk hook redirected the reviewer's shell calls (a note, not an invalidation)

## Evidence
crates/blit-cli/src/run_log.rs:173 — deferred begin_record calls RunRecord::starting at line 596, which assigns started_ms from the current time. With a delayed machine-id read, the unmodified binary exported a record whose started_ms was 1,206 ms after the copied file was already present; its log correctly retained the earlier run-start timestamp.

## Predicted observable failure
Slow initialization records the worker's initialization time as the command's start. jobs list displays incorrect start times and can reorder overlapping commands; exported timestamps can imply that a completed transfer began after its file operations.

## Reviewer's suggested approach
Capture the start timestamp in CommandRun::start and pass it to the background record initializer.

## Intake
Admitted. `RunRecord::starting` stamps `started_ms` with the current time on the record thread, which jl-5 moved off the command's clock, so the record's start is the start of its own writing (normally a millisecond later; under a slow start, seconds). The log's `run-start` is stamped at start already. Fix shape: take the start time in `CommandRun::start` and hand it to the record thread so the record and the log agree.

## What
(filled in with the fix)

## Approach
(filled in with the fix)

## Files changed
(filled in with the fix)

## Guard proof
(filled in with the fix)

## Known gaps
—

## Reviewer comments
(the re-review's verdict, when dispatched)
