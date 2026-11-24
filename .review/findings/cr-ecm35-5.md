# cr-ecm35-5: whether a failure left its copy was unknown per list, not per file

**Severity**: HIGH — a recovered `--ignore-existing` retry can skip its own partial, or be refused for good. Two errors in opposite directions:
- **Logged as known.** A failure past the reason cap, missing from a left-in-place list cut short, was logged as known to have left nothing (`Some(false)`). A retry recovered from that log could keep its own partial as a file.
- **Unknown for the whole list.** One unknown event — a log from before the field — kept the record's whole list marked incomplete, even after that very file had landed.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm35-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r36` over `dc14afc4..d2494965` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r36.codex.md`. It completes cr-ecm34-5 (`183408ae`) and cr-ecm34-6 (`4db619b2`).

## Evidence
"A summary-only failure missing from a truncated leftover list was logged as `Some(false)`, which can make a recovered `--ignore-existing` retry skip its own partial. Conversely, an old unknown event kept the entire list marked incomplete even after that file landed. Preserve unknown for the affected identity and clear it when a later copy settles that identity."

## Intake
Admitted as the reviewer put it.

## What
- **The writer.** `FailedFile.left_in_place` is an `Option`:
  - A failure the report kept says it itself (contract 30).
  - A path past the cap is told by the lists alone. In a list it left its copy; missing from a whole list it left nothing; missing from a list cut short it is not known, and is logged with no field.
  - A path named by bytes heard live is told by those bytes, the same way.
- **The job record's reader** (`LogRead.left_unknown`) keeps the unknown files by identity. A later copy of one, or a later event of it that says, settles it. A record rebuilt from the log marks its list not whole only while some file stays unknown.
- **`blit jobs retry`** (`LoggedLeftovers.left_unknown`) keeps the unknown files by identity, per stream, settled the same way. An older record's ambiguous raw-named failure is refused only when that file's own disposition is unknown.

## Guard proof
- `run_log::tests::a_failure_missing_from_a_cut_list_is_not_known`: a file in the cut list is `Some(true)`, one missing from it `None`, and one missing from a whole list `Some(false)`.
- `job_record::tests::reading_a_log_notes_what_it_lost`: an unknown event, then a copy of the same file, leaves the list whole.
- `jobs::retry_name_tests::an_old_records_raw_named_leftover_is_ambiguous`:
  - an unknown for this file refuses;
  - an unknown for another file leaves this one settled by the complete log.
- `an_unnamed_failure_is_left_in_place_by_its_own_bytes` still holds.
- Mutations, each restored byte for byte:
  - a cut list taken for whole → red;
  - a copy does not settle it → red;
  - any unknown refuses → red.
- Checks:
  - native clippy is clean;
  - the workspace passes on Linux: 2,095 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/run_log.rs` — `FailedFile.left_in_place: Option<bool>`; the end pass; guard.
- `crates/blit-core/src/job_record.rs` — `LogRead.left_unknown` by identity; guard.
- `crates/blit-cli/src/jobs.rs` — `LoggedLeftovers.left_unknown` by identity; guard.

## Known gaps
- None known.
