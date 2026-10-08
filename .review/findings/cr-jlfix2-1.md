# cr-jlfix2-1: the header check accepts a malformed current-version header

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jlfix2-r1 over `d06061ba..ff27f035` (record `.review/results/jlfix2-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-core/src/job_log.rs:1403 — check_header returns success for every numeric version <= VERSION after checking only kind and format; a temporary guard showed a version-1 header missing required RunStart fields became Unreadable while subsequent events were still consumed.

## Predicted observable failure
A corrupt or foreign log can masquerade as the current schema, causing typed consumers to continue interpreting later events instead of rejecting the log; the nonexistent version 0 is also accepted.

## Reviewer's suggested approach
After reading format and version as untyped JSON, dispatch only explicitly supported versions and deserialize the complete header into the corresponding RunStart schema before accepting any events.

## Intake
Admitted. `check_header` reads the envelope untyped (right) but then accepts any version up to the current one without parsing the header itself, and accepts version 0, which never existed. Fix: dispatch on explicitly supported versions only (today: 1), and for each require the whole line to parse as that version's `run-start` event.

## What
`check_header` dispatches on explicitly supported versions only:
- **Version 1:** the whole first line must parse as a version-1 `run-start` event, or the log is refused as "run-start is incomplete or damaged".
- **A newer version** is refused as newer.
- **Version 0, or any other number below the current one** that no blit writes, is refused by name.

Each future version adds its own arm.

## Guard proof
`a_newer_or_foreign_log_is_refused_not_misread` now also refuses a version-1 header missing its fields, and a version-0 header. Mutation — accept any version up to the current one without parsing the header — makes it fail; restored, green.
