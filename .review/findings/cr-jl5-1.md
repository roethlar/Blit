# cr-jl5-1: Snapshot --files-from before the transfer can overwrite it

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled in after the fix)
**Reviewer**: codex / gpt-6-astra / xhigh / standard (inline, session-only — the owner's word `astra`), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-jl5-031e0352`; codereview defect hunt jl5-r1 over `df5fd1cf..031e0352` (record `.review/results/jl5-r1.codex.json` + `.events.jsonl`); 2026-10-13 18:03–18:18Z; capability_ok=true; guard_confirmed=true; SHAs pin-verified; the local ptk hook redirected the reviewer's shell calls (a note, not an invalidation)

## Evidence
crates/blit-cli/src/run_log.rs:173 — begin_record now runs concurrently with the transfer, but job_spec still reads --files-from from disk at line 733. Reproduced with the unmodified binary by holding the machine-id read until a copy overwrote its destination-side list: the transfer copied a.txt and list.txt, but --export recorded files_from as ["b.txt"], the replacement list's contents.

## Predicted observable failure
Without --save, a transfer that overwrites its --files-from file before deferred initialization reaches job_spec silently stores a different job. Running the exported job subsequently selects b.txt instead of the original a.txt and list.txt.

## Reviewer's suggested approach
Capture the list contents before transfer dispatch, preferably sharing the parsed selection with the transfer, and defer only persistence of that captured specification.

## Intake
Admitted. Before jl-5, `job_spec` ran before the transfer started, so the list it recorded was the list the run used. jl-5 moved `begin_record` to the `blit-job-record` thread, which runs beside the transfer; `job_spec` still reads the `--files-from` file from disk there, so a transfer that writes that file (a list kept inside its own destination tree, a mirror's deletion of it) can be recorded with another list than it ran. The plan's acceptance says a saved job reproduces the run after the list file changed; this breaks that. The reviewer reproduced it on the landed binary. Fix shape: capture the list's contents once, before the transfer can touch the file — sharing the transfer's own read where it has one — and defer only the writing of the spec; nothing else `job_spec` reads races with the transfer (the current folder does not change).

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
