# cr-jl4fix4-1: an ambiguous legacy leftover is never resolved from a complete log

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl4fix4-r1 over `beab73c9..e522afe8` (record `.review/results/jl4fix4-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/jobs.rs:393 — `short` only identifies an incomplete failure list; crates/blit-cli/src/jobs.rs:402 — logs are scanned only when that predicate is true, while lines 357-369 refuse every unresolved legacy raw/text match.

## Predicted observable failure
A legacy record with a complete failure list is refused by `jobs retry` even when its complete local log identifies the exact raw leftover; a log proving that a UTF-8 sibling owned the text entry cannot authorize the safe retry either.

## Reviewer's suggested approach
Detect ambiguous legacy identities independently of failure-list completeness, fold a complete terminal log into an exact terminal leftover set, and refuse only if that log is absent, incomplete, or unreadable.

## Intake
Admitted. The log is read only when the record's failure list is cut short, so a legacy record with a whole list is refused even when this machine's complete log names the exact leftover — or proves the text entry was a UTF-8 sibling's. The log must be read whenever a legacy identity is ambiguous, and a complete log settles the question both ways; only an absent or incomplete log leaves it unknowable.
