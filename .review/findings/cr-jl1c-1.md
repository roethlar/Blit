# cr-jl1c-1: `jobs watch` re-downloads an unfinished log to name the failed files

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed — awaiting re-review
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

## What
- **Contract 7:** `GetJobLogRequest.wait_finished` (3). The daemon waits, at most 60 s, until every log of the job is finished and then streams them once.
  - It is woken by a `Notify` that each log's close fires, with a one-second re-check for a log finished another way, such as startup recovery.
  - A job with no log at all gets none by waiting; its log starts at its open.
- **`jobs watch`:** `print_failed_files` makes one read with `wait_finished`. It de-duplicates the failed names, raw bytes first, and still shows at most 20. It says when the log names fewer files than the count, or was still being written.
- **`jobs log`:** keeps reading at once (`wait_finished` false), so a running job's log is shown as it stands.

## Guard proof
- `job_logs::tests::wait_finished_sends_the_log_once_it_closes`: the reply waits while the log is open, then arrives within 400 ms of the close, whole and marked finished.
- Mutations, each red, then restored:
  - ignoring `wait_finished` (the reply comes at once)
  - not waking on close (the reply waits for the one-second re-check)
- The CLI acceptance test `a_detached_job_with_a_failed_file_is_not_reported_as_success` still passes.
