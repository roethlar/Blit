# cr-rework-2: a retry-side source failure drops a path's leftover classification

**Severity**: HIGH — with `--ignore-existing` and two or more retries, a copy/mirror can exit 0 with this run's incomplete copy still at the destination
**Status**: Fixed — red/green on macOS; retry tests green on the Windows ARM64 VM; a re-review is the owner's call
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `a60368b1`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (owner-approved re-review of the cr-rework-1 fix, range 6e0e5a7b..4cdef0a8; record .review/results/rework-fix-range.codex.json)

## Evidence
crates/blit-cli/src/transfers/retry.rs:171 — after_pass returns the next pass's PassFailures without carrying self.left_in_place; lines 353-396 then use that dropped set to decide whether to disable --ignore-existing.

## Predicted observable failure
With --ignore-existing and at least two retries, the main pass can leave an incomplete destination copy, then the first retry can fail while scanning or reading the source before touching that copy. Its summary has an empty left_in_place set, so the next retry restores --ignore-existing, skips the still-existing partial, clears the failure, and exits 0. For a move, the zero failure count also permits deletion of the source, causing data loss.

## Reviewer's suggested approach
Preserve prior left-in-place membership for paths that remain failed across a retry, clearing it only when the path succeeds or the destination positively reports that the old target was removed. Add a three-pass guard covering leftover creation, a source-side retry failure, and subsequent recovery under --ignore-existing.

## Intake
Admitted for copy/mirror. The move consequence does not apply: `blit move` refuses `--ignore-existing` up front (`run_move_inner`, R51-F1), so a move never takes this path. A leftover is this run's own copy for as long as its path keeps failing; a pass that never touched it cannot un-classify it.

## What
`PassFailures::after_pass` (crates/blit-cli/src/transfers/retry.rs) keeps a prior leftover in the set for as long as its path keeps failing. Only a path that succeeded, meaning it is absent from the next pass's failed paths, leaves the set, and the truncation flag carries forward. A pass that failed a path on the source side never touched that path's copy, so the report naming no leftover for it no longer un-classifies it.

## Guard proof
`retry::tests::a_leftover_stays_classified_through_a_source_side_retry_failure` uses the reviewer's three-pass shape. The main pass leaves the copy (`left_in_place = ["left"]`). Retry 1 fails the path again with an empty leftover set, as a source-side failure would. Retry 2 must still re-send the path without `--ignore-existing`.
- Carry-over removed: RED. `[false, true]`: retry 2 ran under `--ignore-existing`, which would skip the copy and clear the failure.
- Restored byte-identical: green.

Gate (macOS, at `a60368b1`): fmt clean; clippy `-D warnings` clean native, linux-cross, and windows-msvc-cross (`blake3/pure`); `cargo test --workspace --no-fail-fast` 1362/0/2 (1361 before).
Windows 11 ARM64 VM at `a60368b1`: retry unit tests 19/19 and `retry_pass` 14/14. The change is platform-independent CLI logic, so the full Windows evidence comes from CI after the push.

## Known gaps
(none)
