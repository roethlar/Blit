# cr-ecm39-3: an older record's whole leftover list was trusted over a log that said "not known"

**Severity**: HIGH — a saved `--ignore-existing` retry keeps a partial as a file someone put there. cr-ecm38-3 made the record writer mark the leftover account incomplete when a failure's disposition is not known. A record written before that may call the account whole although its run's log says a disposition was not known. With such a record, `jobs retry` under `--ignore-existing` exited 0 and left a nine-byte partial for an eight-byte source.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm39-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r40` over `86d69ee3..d7f5f5e9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r40.codex.md`. It completes cr-ecm38-3 (`929ce1a3`).

## Evidence
"With a previous-format record that marked the leftover list complete and a log that recorded the disposition as unknown, `jobs retry` under `--ignore-existing` exited 0. It left a nine-byte partial for an eight-byte source. Distinguish records written with reliable disposition information. For older records, recover that information from sufficient evidence or decline the affected retry."

## Intake
Admitted as the reviewer put it.

## What
- **The record says so.** `RunRecord.dispositions_exact` (serde default false) means the leftover account says, for every failed file, whether its failure left the run's copy. It is set wherever a record is ended now:
  - the CLI's record writer;
  - the daemon-log rebuild;
  - the daemon-answer rebuild.

  A record from before lacks it.
- **An older record's retry** (`jobs.rs`). Under `--ignore-existing`, such a record has its run's log read whatever else the record says (`failed_identities`' `need_log`). The retry is made only when that log is whole and none of the failed files is "not known" in it (`dispositions_known`). Otherwise it is declined, and the refusal says why.

## Guard proof
- `jobs::retry_name_tests::an_older_records_dispositions_are_settled_from_a_whole_log`: an older record with a whole leftover account.
  - A whole log that says the failure's disposition is not known → declined.
  - One that knows it → retried.
  - No log → declined.
  - The same record written with the flag → retried with no log.
- `run_log::tests::a_failure_not_known_to_have_left_its_copy_leaves_the_account_incomplete` (cr-ecm38-3's guard) now also checks that a record written now says `dispositions_exact`.
- Mutations, each restored byte for byte:
  - an older record's dispositions trusted → red;
  - a record not said to be written so → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,141 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/job_record.rs` — `RunRecord.dispositions_exact`; set by both rebuilds.
- `crates/blit-cli/src/run_log.rs` — the writer sets it; the guard's check.
- `crates/blit-cli/src/jobs.rs` — `dispositions_known`; `failed_identities`' `need_log`; guard.

## Known gaps
- The daemon-side rebuilds set the flag with no guard of their own; they fold "not known" into the account already (cr-ecm35-5, cr-ecm36-6).
