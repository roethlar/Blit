# cr-jl3b-1: a file in the current folder shadows a saved job of the same name

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed — awaiting re-review
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

## What
One rule for every `blit jobs` target (`job_record::names_a_path`): a path — anything holding a separator, `./job.json`, `/tmp/x.jsonl.gz` — is a file; a bare word is a saved job's name (`jobs run`) or a run ID (`jobs log`, and the detached-run lookup), whatever files the current folder holds. A bare name with no saved job, beside a file of that name, gets a hint to give the file as a path. `valid_job_name` refuses a name shaped like a run ID (32 lowercase hex), so a bare word names one job only. Help text, man page and plan say so.

## Guard proof
`a_saved_jobs_name_means_the_saved_job_wherever_it_is_typed` (blit-cli `saved_jobs`): a saved job and an exported job file share the name `nightly`; `jobs run nightly` runs the saved one, `jobs run ./nightly` the file's; a file named like a run ID does not shadow `jobs log <id>`; a run-ID-shaped `--save` name is refused (plus the core name test). Three mutations each red alone — file-first `jobs run`, file-first `jobs log`, run-ID-shaped names allowed — restored green.
