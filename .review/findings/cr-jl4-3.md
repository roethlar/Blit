# cr-jl4-3: completing a cut-short failure list adds back failures a later pass fixed

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
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
