# cr-rework-3: a leftover the retry removed stays classified, so a file recreated during the wait is overwritten

**Severity**: HIGH (reviewer) — under `--ignore-existing`, a file another process creates at a failed path during the retry wait can be overwritten
**Status**: Fixed — red/green on macOS; Windows ARM64 VM suite green; closing is the owner's call
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `7c607672`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (owner-approved re-review of the cr-rework-2 fix, range 343ddf6f..9118a88e; record .review/results/rework2-fix-range.codex.json)

## Evidence
crates/blit-cli/src/transfers/retry.rs:189 — every previously classified leftover is re-added whenever its path still fails, even when the retry touched the destination and positively removed the old partial before reporting that failure.

## Predicted observable failure
If retry 1 opens the old partial, encounters a source failure, and successfully removes the partial during abort, the path is nevertheless retained as a leftover. If another process creates that destination during the wait before retry 2, retry 2 forcibly disables --ignore-existing and overwrites the newly created file, violating the requested protection and causing data loss.

## Reviewer's suggested approach
Carry an explicit per-path disposition from each retry—untouched, left in place, or positively removed—and preserve prior membership only for untouched failures. Clear it after confirmed removal, and add a regression test covering removal followed by destination recreation before the next retry.

## Intake
Real, and introduced by the cr-rework-2 fix (`a60368b1`), which keeps a leftover classified while its path keeps failing without knowing whether a later pass removed it. Narrow: it needs `--ignore-existing`, two or more retries, a retry that touches and removes this run's leftover and still fails, and another process creating a file at that exact path within the retry wait. The main pass has the same diff-to-write race over milliseconds; this one spans the retry wait (30 s by default). The precise fix is a third per-path disposition, "positively removed", sent exactly beside `left_in_place`.

## What
The owner chose the precise fix, option A. The destination now reports, per path, that it positively removed this run's incomplete copy, as an exact set beside `left_in_place`.

- `settle_failed_target` returns `Settled { note, removed }`.
- The local copy and tar-shard writers carry `removed` beside their error (`FailedWrite`), never in the reason. A removal note at the front of reasons would have broken the `source:`-first convention, and a suffix cannot be added without flattening the error chain the fatal-class classifier reads.
- `SinkOutcome::removed_incomplete` is recorded at the streamed commit and abort, the local copy, and the shard fold.
- It is merged like `left_in_place`, carried in `TransferSummary` 12/13, `DelegatedPullSummary` 13/14 and `LocalMirrorSummary`, and re-encoded verbatim. It lands under unreleased contract 7, like cr-rework-1.

In the CLI, `after_pass` keeps a prior leftover classified only while its path keeps failing and its copy was not removed. If the removed set did not fit, a prior leftover it does not name is ambiguous. That path is neither carried nor retried; it stays reported as not retried. It is never overwritten and never cleared by a skip.

## Guard proof
Retry unit tests:
- `a_leftover_the_retry_removed_is_no_longer_classified`: retry 1 removes the copy and fails; retry 2 runs with `--ignore-existing` on.
- `an_ambiguous_leftover_is_not_retried_under_ignore_existing`: no second pass for the ambiguous path, which stays failed.

Sink tests now assert the reported removal for the streamed tail, the shard member and the local copy, plus `an_aborted_record_reports_its_removed_target` (whose reason still starts with `source:`).

End to end: `local_session::the_removed_set_reaches_the_summary_past_the_report_cap`, with 70 removed, a report of 64, and nothing left in place. The re-encode and every-route tests carry the set.

Ten mutations, one per link, each RED alone; restored byte-identical → green:
1. The CLI ignoring the removed set.
2. The ambiguous rule dropped.
3. The settle step never reporting a removal.
4. `failed_write_outcome` not marking.
5. The abort not marking.
6. The shard fold not marking.
7. The summary build dropping the set.
8. The local summary copy dropping it.
9. The re-encode dropping it.
10. The CLI wire conversion dropping it.

Gate (macOS, at `7c607672`): fmt clean; clippy `-D warnings` clean native, linux-cross, and windows-msvc-cross (`blake3/pure`); `cargo test --workspace --no-fail-fast` 1366/0/2 (1362 before).
Gate (Windows 11 ARM64 VM, at `7c607672`): `cargo test --workspace --no-fail-fast` 1337/1/2. The one failure is the known elevated-token `metadata_repair` test.

## Known gaps
- A truncated removed set (more than 256 KiB of removed paths) under `--ignore-existing` leaves an ambiguous prior leftover un-retried. It is reported and the run exits 2: never overwritten, never falsely cleared.
- The underlying race remains for a file another process creates at a failed path between a destination's diff and its write. The main pass has the same in-place, no-staging window (D-2026-09-29-2).
