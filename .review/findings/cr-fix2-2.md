# cr-fix2-2: Unnamed scan failures can disappear after another retry

**Severity**: HIGH — when the scan-failure list overflows its wire budget the omitted failures are counted but the path set is reported complete; a further retry pass can clear them and exit 0 with files never landed (data loss under a success exit; move gate bypassed)
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range 44200834..be0b9fda (review-fix batch 2), record .review/results/ssc-fix2-range.codex.json)

## Evidence
crates/blit-core/src/remote/transfer/sink.rs:292 — record_unnamed_failures increments only files_failed_total; wire_failed_paths later returns truncated=false when its named paths fit, and crates/blit-cli/src/transfers/retry.rs:88 consequently treats that incomplete nonempty path list as exact.

## Predicted observable failure
If ManifestComplete.scan_failures exceeds its 1 MiB budget, omitted failures are counted but the summary claims its retry path set is complete. With another retry pass, only named paths are retried; if those converge, after_pass replaces the state with a clean result and the omitted files vanish from the failure count, producing exit 0 despite files never landing.

## Reviewer's suggested approach
Track unnamed-path presence in SinkOutcome and force failed_paths_truncated whenever files_failed_total exceeds the represented path identities; carry that unretried count forward until positively resolved.

## What
- `SinkOutcome::has_unnamed_failures()`: true when `files_failed_total` exceeds the represented path identities (a scoped scan's `scan_failures_dropped` is the only producer). `wire_failed_paths()` reports the set truncated whenever that holds, whatever the byte budget says.
- CLI `PassFailures::retry_set()` (retry.rs): the exact set is exact only when `files_failed <= failed_paths.len() + unretried`; otherwise the same paths are retried but flagged truncated, so `after_pass` carries the unrepresented count forward as `unretried`, which no later clean pass clears; `files_failed` includes it and the move gate refuses.
- Diagnostics-only test seam: hidden global `--diagnostics-scan-failure-name-cap <N>` installs `instrumentation::set_scan_failure_name_cap`; `requested_but_unscanned` drops (counts) every named entry past the cap, so a test forces the unnamed remainder without a megabyte of paths. Production is bounded by the byte budget alone.

## Guard proof
Guards: `sink::cr_fix2_2_tests::unnamed_failures_make_the_wire_retry_set_inexact` (unit), `retry::cr_fix2_2_tests::counted_but_unnamed_failures_survive_a_clean_retry_pass` (unit: 3 counted / 1 named / flag false → after a clean pass `unretried == 2`, `files_failed == 2`, synthesized entry, move gate refuses), `retry_pass::counted_but_unnamed_scan_failures_survive_a_clean_retry_pass` (CLI: three blocked files, all sources vanish during wait 1 with name cap 1, the named one returns during wait 2 and lands on pass 2 → exit 2, "2 file(s) were not retried", `retry_pass` counters `[1, 2]`).
Mutation (`scratchpad/cr-ssc-mutations-3.txt`): truncation signal forced false at both sites (sink `(out, unnamed)` → `(out, false)`; CLI `all_represented` → `true`) → all three guards FAILED (the CLI run exited 0 with two files never landed); restored → all green.

## Known gaps
The delegated summary carries no unnamed count of its own; it inherits the destination summary's `files_failed`/`failed_paths`, so the CLI-side representation rule is what protects the delegated route (unit-level guard).
