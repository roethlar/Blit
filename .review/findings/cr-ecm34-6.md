# cr-ecm34-6: a log without the left-in-place field was read as leaving nothing

**Severity**: MEDIUM — the known gap cr-ecm33-5 left. The job log's `FileFailed.left_in_place` was omitted when false, and a log written before the field has none. A missing field therefore read as a known "nothing left", and a complete old log settled an ambiguous raw-named failure that way. An `--ignore-existing` retry can then skip its own incomplete copy as a file to keep.
**Status**: Reviewed, found incomplete — `combined-r36` (Codex openreview, 2026-11-24) reviewed `dc14afc4..d2494965` — the `combined-r35` fixes and prose-live-phase — with `guard_confirmed: true`: "The retained Linux copy handle, contract-v30 disposition flag, explicit display phase and separate unretried count are sound choices. The following gaps still affect what gets written, reported or retried." "A summary-only failure missing from a truncated leftover list was logged as `Some(false)`, which can make a recovered `--ignore-existing` retry skip its own partial. Conversely, an old unknown event kept the entire list marked incomplete even after that file landed. Preserve unknown for the affected identity and clear it when a later copy settles that identity." The remaining work is cr-ecm35-5
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `4db619b2`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r35` over `0f912b06..dc14afc4` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r35.codex.md`. It completes cr-ecm33-5 (`33808972`) in part.

## Evidence
"New logs omit `false`, and old logs omit the field altogether. I confirmed that a complete old-format log caused an ambiguous raw-named failure to be classified as having left nothing. An `--ignore-existing` retry can then skip its own incomplete copy. Distinguish explicit `false` from historical absence and refuse the ambiguous retry when the saved record cannot settle it."

## Intake
Admitted as the reviewer put it. Unknown is not false, and per D-2026-11-24-1 the old reason's words are not read to fill the gap.

## What
- **The log field.** `job_log::EventBody::FileFailed.left_in_place` is `Option<bool>`. The run log writes it always, `false` included. An event without it — a log written before the field — reads as `None`, not known.
- **The job record.** `LogRead.left_unknown` is set by any such event. A record built from that log then marks its left-in-place list not whole (`left_in_place_truncated`), so an `--ignore-existing` retry of it is refused, as for any list that is not whole.
- **`blit jobs retry`.** `LoggedLeftovers.left_unknown` likewise. A complete log whose events do not say cannot settle an older record's ambiguous raw-named failure, and the retry is refused with the message an absent log gives.

## Guard proof
Tests:
- `job_log::tests::a_failures_left_in_place_is_written_always_and_absent_is_unknown`: `false` is written, and an event without the field reads as `None`.
- `job_record::tests::reading_a_log_notes_what_it_lost`: an event that says `false` leaves the rebuilt list whole, and one that does not say marks it not whole.
- `jobs::retry_name_tests::an_old_records_raw_named_leftover_is_ambiguous`: a complete log whose events do not say is refused.
- `job_log::tests::events_use_the_documented_wire_names` pins the new line: `"left_in_place":false`.

Mutations, each restored byte for byte:

| Mutation | Red |
|---|---|
| The field is not written | the write guard, and `an_unnamed_failure_is_left_in_place_by_its_own_bytes` |
| The run log omits false | `an_unnamed_failure_is_left_in_place_by_its_own_bytes` |
| The record ignores an unknown | `reading_a_log_notes_what_it_lost` |
| The retry ignores an unknown | `an_old_records_raw_named_leftover_is_ambiguous` |

Checks:
- Native clippy is clean.
- The workspace passes on Linux: 2,076 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/job_log.rs` — the field's type; guard; the pinned line.
- `crates/blit-core/src/run_log.rs` — written always.
- `crates/blit-core/src/job_record.rs` — `LogRead.left_unknown`; the rebuilt list; guard.
- `crates/blit-cli/src/jobs.rs` — `LoggedLeftovers.left_unknown`; the refusal; guard.

## Known gaps
- A log written by a build between `33808972` and this commit omits `false`. Read now, such a log's failures that left nothing read as unknown. That errs toward refusal: such a run's ambiguous or rebuilt `--ignore-existing` retry is refused, never misread.
