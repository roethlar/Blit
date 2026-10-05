# cr-rework-1: --ignore-existing retries can falsely clear leftovers beyond the 64-entry named report

**Severity**: HIGH — a run with more than 64 incomplete copies left behind under `--ignore-existing` can exit 0 with incomplete files at the destination
**Status**: Fixed — red/green on macOS; Windows ARM64 VM suite green; a re-review of the fix is the owner's call
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `75462ba9`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (owner-approved codereview over c4b69fa6..2be91345, the cr-win-1 rework + D8; record .review/results/ssc-rework-range.codex.json)

## Evidence
crates/blit-cli/src/transfers/retry.rs:118 — left_in_place derives its override set only from the capped failures sample, while retry_set at lines 98-102 uses the larger failed_paths set; crates/blit-core/src/remote/transfer/sink.rs:280 caps failures at 64.

## Predicted observable failure
If more than 64 files leave incomplete destination copies during an --ignore-existing run, later paths retain --ignore-existing on retry and are skipped because their partials now exist. If the first 64 converge, the pass reports no failures and exits 0 while incomplete files remain.

## Reviewer's suggested approach
Carry a typed left-in-place disposition for every retry path alongside failed_paths. If that identity must be truncated, preserve the unknown remainder as unretried instead of inferring behavior from the capped human-readable report.

## Intake
Admitted. The cr-win-1 record listed "leftovers beyond the report cap retry under the user's flag" as a known gap but understated it: under `--ignore-existing` that retry SKIPS the leftover (it exists), the skip clears the failure, and the run can exit 0 — the false success cr-win-1 exists to remove. Reachable wherever a run leaves more than 64 incomplete copies with `--ignore-existing`: a local `--resume` copy of new files that fail, or targets another process keeps from being removed.

## What
The owner chose the complete fix, option A. The "left an incomplete copy here" set now travels exactly beside the failed-path set, instead of being read off the capped report.

- `SinkOutcome` records it where every failure passes through (the reason `settle_failed_target` annotates) and merges it like `failed_paths`.
- `TransferSummary.left_in_place` (fields 10/11) and `DelegatedPullSummary` (fields 11/12) carry it, bounded by `MAX_WIRE_LEFT_IN_PLACE_ENCODED_BYTES` (256 KiB). It is built at the summary's single construction site, re-encoded verbatim for the delegated topology, and copied onto `LocalMirrorSummary`.
- No contract bump: a wire change before the next release lands under contract 7, which is unreleased (v0.1.3 is contract 6). This follows the plan's Constraints and the cr-fix2-1 precedent.
- The CLI's `PassFailures` reads the set from every route.
- If even this set was truncated, the paths it does not name cannot be classified. Under `--ignore-existing` none of them is retried; they stay reported as not retried, never cleared by a skip. This is the reviewer's fallback.
- The unretried note now reads "the retry set could not name them all"; tests assert only its prefix.

## Guard proof
- `transfer_session::local::tests::the_left_in_place_set_reaches_the_summary_past_the_report_cap`: 70 new 1 MiB files copied with resume, every metadata tail faulted. Result: report 64, set 70, not truncated.
- `delegated_summary::tests::the_left_in_place_set_is_re_encoded_verbatim`.
- Retry unit tests:
  - `leftovers_past_the_named_report_cap_are_still_recognised`: 70 leftovers, report truncated to 64; all 70 retry without `--ignore-existing`.
  - `an_unclassifiable_remainder_is_not_retried_under_ignore_existing`
  - `every_route_carries_the_left_in_place_set`
  - `ignore_existing_retries_this_runs_own_leftovers_without_it`, now driven by the exact set.

Mutations, one per link, each RED alone, restored byte-identical → green:
1. The CLI reading only the leftovers the capped report names. This is the defect itself.
2. The unclassified rule dropped.
3. The sink not recording the set.
4. The summary build dropping it.
5. The local summary copy dropping it.
6. The delegated re-encode dropping it.
7. The CLI wire conversion dropping it.

Gate (macOS, at `75462ba9`): fmt clean; clippy `-D warnings` clean native, linux-cross, and windows-msvc-cross (`blake3/pure`); `cargo test --workspace --no-fail-fast` 1361/0/2 (1356 before).
Gate (Windows 11 ARM64 VM, at `75462ba9`): `cargo test --workspace --no-fail-fast` 1332/1/2 (1327 before). The one failure is the known elevated-token `metadata_repair` test.

## Known gaps
- A truncated leftover set (more than 256 KiB of leftover paths) under `--ignore-existing` leaves the unclassifiable failures un-retried rather than retried. They are reported and the run exits 2, so there is no false success.
