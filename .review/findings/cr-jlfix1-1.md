# cr-jlfix1-1: typed log reading still accepts a log with no header

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed — awaiting re-review
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

## What
- **Reading:** typed reading (`open_log` / `LogLines`) now requires the first line to be a parseable `run-start` of this format and a version this build reads. Another kind, a damaged first line, or a file that is not a job log is refused with `InvalidData` ("not a blit job log, or its first line is damaged — `blit jobs log --json` shows its lines as stored"). `decode` stays the raw path.
- **Recovery** still finishes a log whose start was lost, after a crash before its first sync:
  - `close_interrupted` reads the partial raw and reports whether the header is missing.
  - `recover_one` then writes a `run-start` rebuilt from the log's name (run, participant, role and attempt; verb `unknown`; stamped with the partial's time) at the front of the finished file.
  - The `run-end` detail says the start was lost.
  - The same applies when the partial already ends with its `run-end`.

## Guard proof
- `a_newer_or_foreign_log_is_refused_not_misread`: a torn header, a first event of another kind, and plain text are all refused, as is a newer version or a foreign format; the current version reads.
- `startup_rebuilds_a_lost_start`: a partial without its header is recovered with a rebuilt `run-start` and reads.
- The other recovery tests now write a real header first.
- Mutations, each red, then restored:
  - accepting a non-`run-start` first line (two tests)
  - skipping the rebuilt header (the lost-start test)
