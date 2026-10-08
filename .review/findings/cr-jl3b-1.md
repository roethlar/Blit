# cr-jl3b-1: a file in the current folder shadows a saved job of the same name

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl3b-r1 over `0333cc3e..f3d685fa` (record `.review/results/jl3b-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/jobs.rs:193 — `job_to_run_blocking` tests `Path::is_file()` before consulting `SavedJobs`, so a bare saved-job name is resolved differently depending on the caller's working directory.

## Predicted observable failure
Running `blit jobs run nightly` from a directory containing an ordinary file named `nightly` fails with “not a blit-job document”; if that file is a valid machine-bound job file, it silently runs that different transfer instead, potentially including a destructive mirror or move, violating cwd-independent replay.

## Reviewer's suggested approach
Resolve valid bare names from the saved-job store first and require an explicit filesystem path such as `./job.json` for job files; also reject saved names shaped like run IDs or otherwise provide explicit namespace selectors.

## Intake
Admitted. `jobs run` (and jl-2's `jobs log`) read a target as a file whenever one of that name exists in the current folder, so the same command means different jobs in different folders. One rule for every `jobs` target: a path — anything with a separator — is a file; a bare word is a saved job's name or a run ID; and a saved job may not take a name shaped like a run ID.
