# cr-jl1c-1: `jobs watch` re-downloads an unfinished log to name the failed files

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Admitted — open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl1c-r1 (record `.review/results/jl1c-r1.codex.json`); dispatched under the owner's goal of 2026-10-07

## Evidence
crates/blit-cli/src/jobs.rs:578 — print_failed_files performs up to 50 complete GetJobLog reads and rescans every event; crates/blit-daemon/src/service/core.rs:519 — the terminal event is broadcast before job_log.close at line 530, so a live watcher normally enters this loop while finalization is still running.

## Predicted observable failure
For a large job log, jobs watch can repeatedly transfer and parse the entire log while gzip finalization runs, causing substantial CPU, disk, and network amplification and delaying exit; if finalization outlasts the attempts, it can silently print a partial or empty filename list despite the authoritative failed count.

## Reviewer's suggested approach
Add an internal wait-for-finalized mode or completion notification to GetJobLog so watch waits efficiently and streams the log once; preserve immediate partial reads for ordinary jobs log calls, deduplicate failure entries, and report when retrieved details do not match files_failed.

## Intake
Admitted. The daemon broadcasts the job's end before it closes the log, so a live `watch` polls `GetJobLog` — up to 50 whole-log downloads — until the log finishes, and can give up with a partial list. Fix: `GetJobLogRequest.wait_finished` (contract 7) makes the daemon wait, bounded, for the job's logs to finish (woken when a log closes) and then stream them once; `watch` reads once, de-duplicates, and says when the log names fewer files than the count.
