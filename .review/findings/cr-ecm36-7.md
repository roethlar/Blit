# cr-ecm36-7: a record rebuilt from a log's file events was not marked as naming files only

**Severity**: MEDIUM — a retry the daemon could settle is refused. cr-ecm35-6 marks every record this build starts with `failures_are_files`. In an older record, an entry named `(not retried)` is a file only by its log. A record settled from a complete daemon log, whose failure list is replaced by the log's file events, kept the older record's `false`. A client without a local log of the run then refused a retry the daemon's log had already settled.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm36-7`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r37` over `d2494965..2c55cb3a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r37.codex.md`. It completes cr-ecm35-6 (`8735e2fa`).

## Evidence
"A complete daemon log naming a real file called `(not retried)` rebuilt the record with `failures_are_files = false`. A client without a local log then refuses a retry its daemon already identified. Set the flag when file events supply the replacement list."

## Intake
Admitted as the reviewer put it.

## What
Where a record's failure list is replaced by the one a log's file events make, the record is marked `failures_are_files = true`. There are two such places:
- `LogRead::ended_record`, a run settled from its log;
- the daemon's answer for a run it finished (`settle`).

A log names files only: the synthetic count was never logged.

## Guard proof
- `job_record::tests::reading_a_log_notes_what_it_lost`: a record from before the marker, rebuilt from the log, is marked.
- Mutation, restored byte for byte: a rebuilt list not marked → red.
- Checks: native clippy is clean.

## Files changed
- `crates/blit-core/src/job_record.rs` — the two rebuilds; guard.

## Known gaps
- The daemon's-answer rebuild has no guard of its own. It sets the flag the same way, beside the list it replaces.
