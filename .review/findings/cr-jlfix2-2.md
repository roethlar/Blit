# cr-jlfix2-2: recovering an empty partial gives its header and run-end the same sequence number

**Severity**: LOW (reviewer: LOW)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jlfix2-r1 over `d06061ba..ff27f035` (record `.review/results/jlfix2-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-core/src/job_log.rs:994 — close_interrupted assigns sequence 0 when no event survived, while rebuilt_header also assigns sequence 0 at line 1024; the guard observed recovered sequences [0,0].

## Predicted observable failure
A process killed after creating the partial but before flushing its header produces a recovered log whose sequence metadata cannot uniquely order its header and run-end.

## Reviewer's suggested approach
When the start was lost and no prior event survived, reserve sequence 0 for the synthesized header and append the interrupted run-end with sequence 1.

## Intake
Admitted. With no surviving event, `close_interrupted` numbers the run-end 0, the same as the rebuilt header. Fix: when nothing survived, the run-end is 1 — the rebuilt header holds 0.

## What
When no event survived in a partial, the interrupted `run-end` recovery appends is numbered 1. Nothing surviving means the start was lost too, and the rebuilt header holds 0. With surviving events it follows the last one, as before.

## Guard proof
`startup_numbers_an_empty_logs_lines_in_order`: an empty partial recovers as a rebuilt header and a `run-end` numbered `[0, 1]`. Mutation — number it 0 — makes it fail; restored, green.
