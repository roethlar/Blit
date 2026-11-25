# cr-ecm40-2: an older record's failure the log never mentions passed the disposition check

**Severity**: HIGH — a retry says "Up to date" over a partial copy. Brought in by cr-ecm39-3.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm40-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r41` over `d7f5f5e9..b6e6fab9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r41.codex.md`.

## Evidence
"With an older record and a finished log containing no event for its failed file, `--ignore-existing` reported \"Up to date\" and left the partial bytes in place. Require affirmative disposition evidence for each failed identity. The current check accepts any identity absent from `left_unknown`, including one the log never mentioned."

## Intake
Admitted as written.

## What
The log's reading keeps, per stream, the files whose events say what their failure left (`settled`):
- a failure that carries its disposition;
- a copy that landed the file;
- a removal.

`dispositions_known` takes an older record's dispositions as known only for a failed file in that set, and not left unknown. A file the log never mentions is not in it, so its record is declined.

## Guard proof
- `jobs::retry_name_tests::an_older_records_dispositions_are_settled_from_a_whole_log`: a whole log that names only another file → declined.
- Mutation, restored byte for byte: a failure the log never mentions taken as known → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's seven fixes: 2,146 tests over 57 binaries.

## Files changed
- `crates/blit-cli/src/jobs.rs` — `LoggedLeftovers::settled`, the log's reading, `dispositions_known`; guard and test literals.

## Known gaps
- None known.
