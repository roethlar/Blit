# prose-retried: whether a failure was marked retried was read from its words

**Severity**: LOW — a display defect, and a breach of D-2026-11-24-1. A failure that survived `--retries` is meant to read "… (retried)". The marking skipped any failure whose reason already ended in " (retried)". A file whose name ends so, named last in its reason, was therefore reported as never retried.
**Status**: Verified — `combined-r35` (Codex openreview, 2026-11-24) reviewed `0f912b06..dc14afc4` — the `combined-r34` fixes and the three prose fixes — with `guard_confirmed: true`: no change required of it; its guard was the reviewer's own red/green proof.
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `7d2635d1`
**Reviewer**: none yet. Found by the working agent on 2026-11-24, applying the owner's rule D-2026-11-24-1 (no decision is read from prose), which names `RETRIED_SUFFIX`.

## Evidence
`PassFailures::mark_retried` (`crates/blit-cli/src/transfers/retry.rs`) appended `RETRIED_SUFFIX` only `if !failure.reason.ends_with(RETRIED_SUFFIX)`. The reason's words decided whether the failure had been marked. A reason such as `source: cannot open a (retried)` (the file `a (retried)`) already ends so, and stayed unmarked.

## What
`PassFailures` carries `retried: bool`.
- `mark_retried` marks the report's failures once, and says so on the report.
- Asked again, it does nothing.
- It no longer reads a reason. The suffix is display only.

## Guard proof
- `transfers::retry::tests::a_retried_failure_is_marked_by_the_report_not_its_words`. After one pass, a failure of the file `a (retried)`, whose reason ends in " (retried)", is marked all the same. Marking the report again changes nothing.
- `zero_retries_or_zero_failures_run_nothing` also asserts that an unretried report is not flagged.
- Mutations, each restored byte for byte:
  - marked by the words → red;
  - marked again → red.
- Checks: native clippy is clean, and the CLI's retry tests pass.

## Files changed
- `crates/blit-cli/src/transfers/retry.rs` — `PassFailures.retried`, `mark_retried`; guard.

## Known gaps
- None known. The suffix text is what people read in the summary, the record and the log. Nothing reads it back.
