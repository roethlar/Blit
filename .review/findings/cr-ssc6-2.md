# cr-ssc6-2: Successful local retries retain the main pass's no-op outcome and duration

**Severity**: MEDIUM — a file landing only on retry is reported as 'Up to date … 0 changed' / outcome=up_to_date with the main pass's duration — wrong result in text and JSON
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `7c0dc993`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range b342d636..e14d1b72 (ssc-6), record .review/results/ssc-6-range.codex.json)

## Evidence
crates/blit-cli/src/transfers/mod.rs:344 — the local fold updates copied files, bytes, and failures but not outcome or duration; crates/blit-cli/src/transfers/local.rs:991 and crates/blit-cli/src/transfers/local.rs:1000 render those stale fields.

## Predicted observable failure
When the only file fails initially and lands on retry, the command prints “Up to date: ... 0 changed” and JSON reports outcome=up_to_date despite files_transferred=1. It also reports only the millisecond main-pass duration, excluding the retry wait and retry session; both behaviors reproduced in the reviewed snapshot.

## Reviewer's suggested approach
Measure the complete operation around all passes, set the final outcome to Transferred whenever a retry lands a file, and add a regression test where the sole file succeeds on retry.

## What
Both local retry folds share `fold_local_retry`: a file landed on retry makes the outcome Transferred and the duration is measured around the whole operation, waits included.

## Guard proof
CLI pin: a sole file landing only on the retry reports outcome=transferred, files_transferred ≥ 1 and duration_ms ≥ the wait (JSON); the text run never says "Up to date". Mutation: fold's outcome/duration lines removed → red.

## Known gaps
Remote routes derive "up to date" from files_transferred, which the fold already adds; they carry no duration.
