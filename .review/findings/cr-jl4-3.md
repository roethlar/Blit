# cr-jl4-3: completing a cut-short failure list adds back failures a later pass fixed

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl4-r1 over `3913ab70..2dab5d21` (record `.review/results/jl4-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/jobs.rs:323 — supplementation inserts every FileFailed event from the entire command log and never removes failures resolved by later retry-pass FileCopied or successful terminal events.

## Predicted observable failure
When the final failed list is truncated and an earlier in-command retry already fixed some paths, a later jobs retry includes those historical failures again, potentially overwriting files changed since they succeeded and violating the exactly-failed-files contract.

## Reviewer's suggested approach
Store the exact terminal failed set in the run record or a dedicated retry-state artifact; if log reconstruction remains necessary, reconstruct terminal per-identity state rather than taking a union of failure events.

## Intake
Admitted. Completing the record's list from the log takes every `file-failed` event the run ever wrote, including files an in-run retry pass then landed. The completion must be the run's terminal state — failed and not landed after.

## What
When a record's failure list is cut short, `failed_paths` completes it from this machine's log of the run read in order: a `file-failed` adds a (text, bytes) identity, a later `file-copied` or `file-sent` of the same identity removes it, so only what the run ended with still failed is added — not every failure it met.

## Guard proof
`a_retry_skips_what_a_later_pass_already_landed` (blit-cli `job_retry`): two blocked files, one freed during the run's retry wait and landed by its pass; the record's list then cut short; the landed file changed at the source; `jobs retry` sends only the still-failed file and leaves the landed one as it was. Mutation — the union of every failure, as before — overwrites it; restored, green.
