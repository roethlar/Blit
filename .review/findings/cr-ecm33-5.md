# cr-ecm33-5: what a failure left was read from the error's words

**Severity**: MEDIUM — two errors in opposite directions. (1) The writer's opener made a file where nothing stood. When its check then failed, the empty file was left at the name, reported nowhere. (2) Whether a failure left this run's copy behind is read from its reason's words (`INCOMPLETE_LEFT_IN_PLACE`). A source-only failure whose path contained those words put an untouched destination in the left-in-place set.
**Status**: Half (1) fixed on Unix, awaiting review; half (2) NOT FIXED — open (see Known gaps)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm33-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r34` over `70efef8a..0f912b06` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r34.codex.md`. It completes cr-ecm32-5 (`b91abdc1`) in part.

## Evidence
"I observed errors in both directions. Failed validation at a previously absent destination left an unreported zero-byte file. A source-only failure whose pathname contained the diagnostic phrase put an untouched destination in `left_in_place`. Track whether acquisition created a file, settle it on failure, and carry the outcome into the summary independently of the error text."

## Intake
Admitted. Half (1) is fixed here.

Half (2) is not, because the fix reaches a persisted format. The left-in-place set is read from a failure's reason in three places: the sink's summary, the job record (`job_record.rs`) and the CLI's job retry (`jobs.rs`). The job log stores only the reason. A flag independent of the words would have to travel through:
- `FileFailure`, built in 35 places;
- `SinkOutcome::record_failure`;
- the job log's `FileFailed` event: a new field, with old logs read by the words.

The misreading needs a path that contains the exact diagnostic sentence.

## What
`copy::create_regular` tries to make the file where nothing stands (`O_CREAT|O_EXCL`). Only when the name is taken does it open what is there. Two outcomes:
- A file it made whose check then fails is removed. If that removal fails, the error says, in the left-in-place words, that the empty file made there is left.
- A file that stood there is left as it was: it is cut only after its check, as before (cr-ecm32-2).

A link standing at the name is still written through, as before.

## Guard proof
- `file_copy::mode_tests::a_file_made_and_then_refused_is_not_left` (Unix): at an absent name, with the made file's check refused (`CREATE_LOOK_FAULTS`), nothing is left at the name.
- `a_file_opened_to_be_written_is_checked_before_it_is_cut` still holds: what stood there is left whole.
- Mutation, restored byte for byte: what was made and refused left behind → red.
- Checks:
  - All 1,165 `blit-core` tests and the four local-copy-mode CLI tests pass.
  - Native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean.
  - A plain local copy of 1,000 files measured 37–38 ms before and after, over existing files 36–38 ms both.

## Files changed
- `crates/blit-core/src/copy/file_copy/mod.rs` — `create_regular`; guard.

## Known gaps
- Half (2) open: left-in-place is still read from the reason's words, in the summary, the job record and the CLI's job retry. A failure whose reason, its path included, holds the exact sentence "an incomplete copy was left at the destination" is counted as left in place. This needs the structured flag described above; it is put to the owner.
