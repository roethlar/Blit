# cr-ecm38-7: a list its log completed was not checked against the record's count

**Severity**: MEDIUM — a retry asks for a file that does not exist. cr-ecm37-6 refuses a record whose own entries name more files than it counts. A record whose list was cut is completed from the run's log, and that log is checked only for naming too few. An earlier blit's log could name a long file twice: once by its name, once by a shortened rendering of it. A cut record counting one failure then yielded two retry names.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm38-7`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r39` over `6ee7ed5e..86d69ee3` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r39.codex.md`. It completes cr-ecm37-6 (`1bc7aaa9`).

## Evidence
"A truncated record counting one failure yielded two retry names when its log supplied both the full and shortened spellings. Require the recovered identity count to equal the recorded failure count after merging the log."

## Intake
Admitted as the reviewer put it.

The check relies on one file reaching the log under one identity, and cr-ecm38-1 restored that: a retry scan's text failures are text again, not bytes. A log made of several streams (a daemon's destination log beside the command's own) names a file under the same identity in each.

## What
`failed_identities` (`blit-cli/src/jobs.rs`) checks the list after the log completes it, both ways:
- **Too few**, as before: refused.
- **Too many**: refused as ambiguous. The refusal says that a log from an earlier blit can name a long file twice, and how to run the whole job again.

## Guard proof
- `jobs::retry_name_tests::a_list_its_log_completes_names_no_more_files_than_it_failed`: a cut record counting one failure.
  - Completed from a whole log that names a long file and its rendering, it is refused.
  - Completed from a log that names the file once, it is retried as that file.
- `jobs::retry_name_tests::an_old_records_ambiguity_is_settled_from_a_whole_log`: its "crossed" part has two role logs that each fail a different file. Its record now counts the two files its logs fail; it had reused a count of one from the part before.
- Mutation, restored byte for byte: the completed list not checked → red.
- Checks:
  - native clippy is clean;
  - the workspace passes on Linux: 2,124 tests over 57 binaries.

## Files changed
- `crates/blit-cli/src/jobs.rs` — the check after the log; guard; the crossed part's count.

## Known gaps
- None known.
