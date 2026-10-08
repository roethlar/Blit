# cr-jl1a-1: job-log file events lose the exact bytes of a non-UTF-8 name

**Severity**: MEDIUM (reviewer: HIGH)
**Status**: Admitted — open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl1a-r1 (record `.review/results/jl1a-r1.codex.json`); dispatched under the owner's goal of 2026-10-07 ("review with codex each slice")

## Evidence
crates/blit-core/src/job_log.rs:214 — copied, deleted, and failed events contain only a `String` path; crates/blit-core/src/raw_name.rs:5 — Blit's manifest text is explicitly lossy and exact non-UTF-8 bytes are carried separately because distinct names can collapse to the same text.

## Predicted observable failure
Two supported Unix filenames whose invalid UTF-8 bytes collapse to the same replacement text can produce indistinguishable forensic events unless every future caller invents and consistently applies an undocumented escaping convention, violating the requirement to name every affected file.

## Reviewer's suggested approach
Introduce a `LoggedPath` containing a human-readable value plus optional exact raw bytes encoded for JSON, or formally require one reversible encoding for all paths; provide constructors from native paths and transfer headers and test colliding non-UTF-8 names.

## Intake
Admitted. Blit transfers names that are not valid UTF-8 (contract 7 `raw_relative_path`); the log keeps only the lossy text, so it cannot say which file on disk an event means, and two such names that collapse to one text read the same. Within one transfer the lossy text is unique (intake refuses a duplicate), so this is a forensic loss, not a wrong retry — rated MEDIUM. Fix: an optional `raw` field on every file event carrying the exact bytes in Blit's existing reversible escape (`raw_name::escape_raw`), fed from the manifest's raw names and from the mirror pass's own paths.
