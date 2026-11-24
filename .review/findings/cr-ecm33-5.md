# cr-ecm33-5: what a failure left was read from the error's words

**Severity**: MEDIUM — two errors in opposite directions. (1) The writer's opener made a file where nothing stood. When its check then failed, the empty file was left at the name, reported nowhere. (2) Whether a failure left this run's copy behind is read from its reason's words (`INCOMPLETE_LEFT_IN_PLACE`). A source-only failure whose path contained those words put an untouched destination in the left-in-place set.
**Status**: Fixed — half (1) on Unix (`b3d952e1`), half (2) in this commit; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: half (1) `b3d952e1`; half (2) this commit (`Fix cr-ecm33-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r34` over `70efef8a..0f912b06` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r34.codex.md`. It completes cr-ecm32-5 (`b91abdc1`) in part.

## Evidence
"I observed errors in both directions. Failed validation at a previously absent destination left an unreported zero-byte file. A source-only failure whose pathname contained the diagnostic phrase put an untouched destination in `left_in_place`. Track whether acquisition created a file, settle it on failure, and carry the outcome into the summary independently of the error text."

## Intake
Admitted.
- Half (1) was fixed first, in `b3d952e1`.
- Half (2) reaches a persisted format, the job log, and was put to the owner. The owner ruled (D-2026-11-24-1): "Display Name is never a unique identifier, everything is mapped to a GUID or SID in real software, so no, searching for a prose error message when you control the entire pipeline is not acceptable. ever. rule."

## What
### Half (1), `b3d952e1`
`copy::create_regular` tries to make the file where nothing stands (`O_CREAT|O_EXCL`). Only when the name is taken does it open what is there.
- A file it made whose check then fails is removed.
- A file that stood there is left as it was: it is cut only after its check, as before (cr-ecm32-2).
- A link standing at the name is still written through, as before.

### Half (2), this commit
Whether a failure left this run's own incomplete copy at the destination is now a structured fact. It is set where it is known and carried to every place that uses it. Nothing reads it from a message.

Where it is set:
- **On the error.** `sink::LeftInPlace` is a typed context. It is attached:
  - by settlement (`settle_failed_target` returns `Settled.left`, and `with_settle_note` wraps the note in `LeftInPlace` when the copy stayed);
  - by the resume's fresh partial (`fresh_as`: looking at it failed, or it could not be removed);
  - by `create_regular` when the file it made could not be removed (inside the I/O error).
- **Read by type.** `sink::leaves_in_place(&Report)` finds the marker anywhere in the chain, including inside an I/O error.

Where it travels:
- `FileFailure.left_in_place`. Failures are recorded with `SinkOutcome::record_failure_leaving` / `failed_leaving`, by:
  - `per_file_failure`;
  - a contained tar member;
  - a source's refusal after the writer settled;
  - a held resume-block failure (`failed_resume_reasons` keeps the reason and the flag);
  - a file kept where a copy was left (`A_FILE_WHERE_A_COPY_WAS_LEFT`).
- The summary's left-in-place sets are filled from the flag (`record_failure_leaving`), no longer from the reason's words.
- `ProgressEvent::FileFailed { left_in_place }`, via `report_file_failed_leaving` and `report_file_outcome`.
- The job log's `FileFailed` event gains `left_in_place` (omitted when false; `#[serde(default)]`). It is written from the live event, and at the run's end from the failure's flag or the totals' sets. An unnamed failure is checked by its own bytes when it has them, never by the text set, which may be another entry's.
- The job record (`job_record.rs`) and `blit jobs retry` (`jobs.rs`) read the event's field, no longer the reason.

The messages keep their human wording. Nothing reads them.

## Guard proof
Tests:
- Half (1): `file_copy::mode_tests::a_file_made_and_then_refused_is_not_left` (Unix).
- `sink::tests::left_in_place_is_said_by_its_flag_never_by_its_words`:
  - a failure whose path and reason both hold the sentence is not left in place;
  - an error carrying `LeftInPlace` under further context, or inside an I/O error, is.
- `job_record::tests::reading_a_log_notes_what_it_lost`: an event whose path and reason hold the sentence, without the field, marks nothing.
- `jobs::retry_name_tests::a_logs_words_mark_nothing_left_in_place` (CLI): likewise for the retry's reading of a log.
- `run_log::tests::an_unnamed_failure_is_left_in_place_by_its_own_bytes`.
- The existing left-in-place tests now also assert the flag:
  - a fresh partial left behind;
  - a read-only resume;
  - a made file that was removed (no flag);
  - a link to the held file;
  - an unremovable target.

Mutations, each restored byte for byte:

| Mutation | Red |
|---|---|
| The summary also reads the words | `left_in_place_is_said_by_its_flag_never_by_its_words` |
| The marker is not looked for | `a_fresh_partial_left_behind_is_reported_left_in_place` and the flag guard |
| The job record reads the words | `reading_a_log_notes_what_it_lost` |
| The CLI reads the words | `a_logs_words_mark_nothing_left_in_place` |
| An unnamed raw-named failure is looked up in the text set | `an_unnamed_failure_is_left_in_place_by_its_own_bytes` |

Checks:
- Native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean.
- The workspace passes on Linux.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs`:
  - `LeftInPlace`, `leaves_in_place`;
  - `FileFailure.left_in_place`, `record_failure_leaving`, `failed_leaving`, `left_in_place_holds`;
  - `Settled.left`, `with_settle_note`;
  - the resume-block failure;
  - guards.
- `crates/blit-core/src/copy/file_copy/resume.rs`, `crates/blit-core/src/copy/file_copy/mod.rs` — the marker where the fresh partial or a made file is left.
- `crates/blit-core/src/transfer_session/mod.rs` — `A_FILE_WHERE_A_COPY_WAS_LEFT` recorded with the flag.
- `crates/blit-core/src/remote/transfer/progress.rs` — the event's field.
- `crates/blit-core/src/run_log.rs` — the log's field, live and at the end; guard.
- `crates/blit-core/src/job_log.rs` — the event's field.
- `crates/blit-core/src/job_record.rs`, `crates/blit-cli/src/jobs.rs` — read the field; guards.
- `crates/blit-core/src/remote/transfer/payload.rs`, `crates/blit-core/src/transfers/failures.rs`, `crates/blit-cli/src/transfers/{failures,retry}.rs`, `crates/blit-cli/src/run_log.rs` — the new field (`false`: source skips, wire conversions whose left lists travel separately, placeholders, tests).

## Known gaps
- **Old logs.** A job log written before this change has no `left_in_place` field, and is read as leaving nothing in place: by the owner's rule, its words are not read.
  - The record's own left-in-place lists still name such files exactly.
  - Only a run whose record is short, or a pre-identity record that is ambiguous, consults its log for them.
  - 1.0.0 is not tagged, so no released build wrote such a log.
- **Half (1) is Unix only.** On Windows the opener is unchanged (cr-ecm33-3's Windows half and cr-ecm32-4 stay blocked: no Windows host).
- **Other prose classifications.** A scan for the same pattern found four other decisions read from words. Each is filed separately under "Found outside a review" in `REVIEW.md` and is not changed here:
  - `RETRIED_SUFFIX` in `blit-cli/src/transfers/retry.rs`;
  - two `endpoints.rs` message matches;
  - `payload.rs`'s "changed size while reading" / "Windows metadata changed";
  - `resume_diff.rs`'s `starts_with("source:")`.
