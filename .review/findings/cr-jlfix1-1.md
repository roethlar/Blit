# cr-jlfix1-1: typed log reading still accepts a log with no header

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Admitted — open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jlfix1-r1c over `7d2bd7d3..efe53362` (record `.review/results/jlfix1-r1c.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-core/src/job_log.rs:1295 — malformed first-line JSON returns Ok; line 1298 likewise accepts any first event whose kind is not run-start, despite the typed-reader contract at lines 1262-1265.

## Predicted observable failure
A foreign JSONL file, or a newer log whose header is torn or missing, is silently interpreted using the current Event schema instead of being rejected, recreating the incompatible-schema misreading this change intends to prevent.

## Reviewer's suggested approach
Require a parseable run-start object with the expected format and a supported version before yielding typed events; users needing damaged-log forensics can use the existing raw decode path.

## Intake
Admitted. The cr-jl1a-2 fix let a log whose first line is not a `run-start` read as the current version, so a foreign JSON-lines file or a newer log with a torn header could still be misread. Fix: typed reading requires a valid, supported `run-start` first and refuses anything else; `decode` stays the raw path. Recovery, which must still finish a log whose start was lost (a crash before its first sync), writes a header synthesised from the log's key, marked as such, at the front of the finished file.
