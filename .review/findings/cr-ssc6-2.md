# cr-ssc6-2: Successful local retries retain the main pass's no-op outcome and duration

**Severity**: MEDIUM — a file landing only on retry is reported as 'Up to date … 0 changed' / outcome=up_to_date with the main pass's duration — wrong result in text and JSON
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range b342d636..e14d1b72 (ssc-6), record .review/results/ssc-6-range.codex.json)

## Evidence
crates/blit-cli/src/transfers/mod.rs:344 — the local fold updates copied files, bytes, and failures but not outcome or duration; crates/blit-cli/src/transfers/local.rs:991 and crates/blit-cli/src/transfers/local.rs:1000 render those stale fields.

## Predicted observable failure
When the only file fails initially and lands on retry, the command prints “Up to date: ... 0 changed” and JSON reports outcome=up_to_date despite files_transferred=1. It also reports only the millisecond main-pass duration, excluding the retry wait and retry session; both behaviors reproduced in the reviewed snapshot.

## Reviewer's suggested approach
Measure the complete operation around all passes, set the final outcome to Transferred whenever a retry lands a file, and add a regression test where the sole file succeeds on retry.

## What
(coder fills in)

## Guard proof
(red/green proof; mutation described)

## Known gaps
(none yet)
