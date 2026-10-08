# cr-jl1a-2: the job-log reader never checks the format and version it records

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl1a-r1 (record `.review/results/jl1a-r1.codex.json`); dispatched under the owner's goal of 2026-10-07 ("review with codex each slice")

## Evidence
crates/blit-core/src/job_log.rs:989 — `open_log` only detects compression, and line 1028 deserializes each line directly as the current `Event`; it never validates the required first `run-start`, `format`, or `version` described at lines 6–11.

## Predicted observable failure
After an incompatible schema bump, an older text reader can silently interpret known event kinds with obsolete semantics or report arbitrary lines as corruption instead of identifying an unsupported version or applying a migration.

## Reviewer's suggested approach
Read and validate the header before yielding typed events, select a version-specific decoder or migration path, and expose raw-line streaming separately for `--json` retrieval and damaged-log forensics.

## Intake
Admitted. `open_log` parses every line as the current `Event` without reading `run-start`'s `format`/`version`, so a log from a newer, incompatible schema would be misread rather than refused. Fix: typed reading validates the header and refuses a foreign format or a newer version with a plain error; `decode` stays the raw path for `--json`.

## What
`LogLines` checks the first line as the `run-start` header before any typed parsing — read as untyped JSON so a newer header's fields are never forced into this version's shape. A foreign `format` or a `version` above this build's yields an `InvalidData` error naming it ("this job log is version 2; this blit reads versions up to 1"); a log whose head was lost still reads as the current version. Older versions dispatch through the same check (a migration slot when VERSION moves past 1). `decode` stays the raw path (`blit jobs log --json`).

## Guard proof
`a_newer_or_foreign_log_is_refused_not_misread`: a v2 header and a foreign format are refused, the current version and a damaged head read. Mutation — skip the header check — makes it fail; restored, green.
