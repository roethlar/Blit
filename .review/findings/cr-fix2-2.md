# cr-fix2-2: Unnamed scan failures can disappear after another retry

**Severity**: HIGH — when the scan-failure list overflows its wire budget the omitted failures are counted but the path set is reported complete; a further retry pass can clear them and exit 0 with files never landed (data loss under a success exit; move gate bypassed)
**Status**: Open
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
(coder fills in)

## Guard proof
(red/green proof; mutation described)

## Known gaps
(none yet)
