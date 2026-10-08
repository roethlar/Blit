# cr-jl4-1: a retry of an `--ignore-existing` job skips its own incomplete files

**Severity**: HIGH (reviewer: HIGH)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl4-r1 over `3913ab70..2dab5d21` (record `.review/results/jl4-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/jobs.rs:280 — job_to_retry restores the original options and installs only retry_only, while crates/blit-cli/src/transfers/retry.rs:451 documents and implements the required left_in_place split for --ignore-existing.

## Predicted observable failure
If a failed write leaves Blit's own incomplete destination and the original job used --ignore-existing, jobs retry treats that partial file as pre-existing, skips it, exits successfully, and records an apparently clean child run while the damaged file remains. The required red/green guard reproduced this behavior.

## Reviewer's suggested approach
Persist the exact left-in-place set and truncation flag in RunRecord; retry those paths with ignore_existing disabled, retry the remaining failed paths with the original flag, and refuse unclassifiable paths.

## Intake
Admitted. The in-command retry passes retry a path whose failed write left this run's own incomplete copy with `--ignore-existing` off; `jobs retry` restores the original options and sends only the retry set, so that copy is skipped as existing and the run reports success. The record must keep which failures left a copy in place (and whether that list is whole), and the retry must split as the passes do — refusing what it cannot classify.
