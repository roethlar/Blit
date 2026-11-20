# cr-ecm3-2: A `move` refused for a pattern file had already saved a job without the filter

**Severity**: HIGH — running the saved job moved the whole tree, the file the user meant to leave included
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm3-2: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r3` over `3bc3ee47..467773b4` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r3.codex.md`.

## Evidence
The reviewer ran `move -y --exclude-from skip --save sample src/ dst/`: it reported the refusal and saved `sample`. `jobs run sample` then moved the entire tree, including the file the exclusion named.

Cause: `run_command` (`crates/blit-cli/src/main.rs`) saves the job before the verb runs, and `move` refused its filters only when it was dispatched. For the older filter options that order was harmless — the job recorded the option and refused again when run. A pattern file is different by design (slice 3b, Q6): a job records patterns, never a file, and `move` leaves its files unread — so the saved job held no filter at all.

## Predicted observable failure
`blit move --exclude-from FILE --save NAME …` fails as it should and leaves a job that, run, moves everything.

## Intake
Admitted. Slice 3b created the hole.

## What
`move` refuses every filter option first — before a job is saved, a record started or a file read — through the one function its dispatch already used (`reject_move_filters`, `crates/blit-cli/src/transfers/mod.rs`), now also called by `run_command`. Nothing is saved or exported from a `move` that takes a filter, whichever filter it is.

## Guard proof
- `saved_jobs::a_move_refused_for_a_filter_saves_no_job`: for `--exclude-from`, `--include-from`, `--exclude`, `--files-from` and `--min-size`, each with `--save` and `--export` — the refusal is reported, no job file is exported, `jobs run NAME` finds no job, the source is intact and the destination was never made; at the end the saved-job list is empty.
- `pattern_files::move_refuses_pattern_files_and_help_offers_them_where_they_are_taken` (slice 3b) is unchanged and green.
- Mutation (the early refusal removed, the dispatch one kept): the guard red; restored byte for byte.

## Files changed
- `crates/blit-cli/src/transfers/mod.rs` — `reject_move_filters`, called where the inline check was.
- `crates/blit-cli/src/main.rs` — called first for `move`.
- `crates/blit-cli/tests/saved_jobs.rs` — the guard.

## Known gaps
- Run on Linux only.
- A `move` refused for one of its OTHER reasons (`--dry-run`, `--force`, `--ignore-existing`, a case mode …) still saves its job first; that job records the option and is refused again when run, so nothing is moved — but it is a job that can never run.
