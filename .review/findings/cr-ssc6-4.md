# cr-ssc6-4: Remote retry aggregation drops carrier metadata

**Severity**: LOW — retry passes lose in_stream_carrier_used / files_resumed, so the final summary can misreport the carrier and resume counts
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `c1cf9117`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range b342d636..e14d1b72 (ssc-6), record .review/results/ssc-6-range.codex.json)

## Evidence
crates/blit-cli/src/transfers/retry.rs:124 — PassResult retains only transferred counts and failures; crates/blit-cli/src/transfers/retry.rs:144 folds those fields without propagating in_stream_carrier_used or files_resumed from retry summaries.

## Predicted observable failure
If a retry session falls back to the in-stream gRPC carrier after the main session used TCP, the combined JSON and human summary still report tcp_fallback=false even though some reported bytes used the fallback; analogous retry-pass resume counts are also lost.

## Reviewer's suggested approach
Carry all aggregatable summary metadata through PassResult/RetryOutcome, summing files_resumed and OR-ing carrier-use flags into the final summary.

## What
`PassResult`/`RetryOutcome` carry `in_stream_carrier_used` (OR) and `files_resumed` (sum); the wire fold applies both, the delegated fold applies the carrier fact as `tcp_fallback_used`, local passes report neither.

## Guard proof
Unit pin: two passes, one in-stream, each resuming one file → summary says in-stream used and 2 resumed; delegated says fallback used. Mutation: loop drops the facts → red.

## Known gaps
The delegated wire carries no resume count, so a delegated retry's block-wise resumes are not reported.
