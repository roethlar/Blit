# cr-ecm34-4: a source's abort dropped what the destination knew it left

**Severity**: MEDIUM — two places where the source's failure replaced the destination's account. A discarding stream writer (its destination already failed) and a resume record holding a destination failure both met a source that closed the record failed. Both reported the source's reason with `left_in_place = false`, though the destination had left its copy.
**Status**: Reviewed, found incomplete — `combined-r36` (Codex openreview, 2026-11-24) reviewed `dc14afc4..d2494965` — the `combined-r35` fixes and prose-live-phase — with `guard_confirmed: true`: "The retained Linux copy handle, contract-v30 disposition flag, explicit display phase and separate unretried count are sound choices. The following gaps still affect what gets written, reported or retried." "After the receiver created a fresh partial and wrote a block, a source abort left nine bytes for an eight-byte source and reported `left_in_place = false`. Retain whether the receiver created the held file and use that fact when closing the record." The remaining work is cr-ecm35-4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `13675159`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r35` over `0f912b06..dc14afc4` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r35.codex.md`. It completes cr-ecm33-5 (`33808972`) in part.

## Evidence
"I exercised both a discarding stream writer and a resume with a pending destination error marked `LeftInPlace`. Both reported `left_in_place = false` after the source closed the record as failed. Keep the source's diagnostic and propagate the destination's known disposition into the outcome and progress event."

## Intake
Admitted as the reviewer put it.

## What
- **`FsRecordWriter::abort`.** A writer in `Discard` mode whose pending error carries `LeftInPlace` settles as left, with the destination's account as its note. The outcome reads "{source's reason}; {destination's account}" with the flag set. A discarding writer whose failure left nothing still says nothing left.
- **`FsTransferSink::fail_resume_record`.** It takes the failure held for the record before letting the record go. With one held, the outcome reads "{source's reason}; {held reason}" and carries the held flag.
- **The data-plane pipeline.** It reports a resume record its source failed after the sink settles it, through `report_file_outcome`, so the progress event, and through it the job log, carries the flag. The control lane reported it that way already.

## Guard proof
- `sink::tests::a_sources_abort_keeps_what_the_destination_left` checks three cases:
  - a discarding writer whose error left its copy: flagged, in the left-in-place set, the source's reason first;
  - one whose error left nothing: not flagged;
  - a resume record with a held `LeftInPlace` failure: flagged, in the set, the source's reason first.
- `a_failed_resume_record_lets_its_partial_go` still holds.
- Mutations, each restored byte for byte:
  - the discarding writer forgets → red;
  - the resume record forgets → red.
- Checks: native clippy is clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `abort`, `fail_resume_record`; guard.
- `crates/blit-core/src/remote/transfer/pipeline.rs` — the report after the sink's outcome.

## Known gaps
- The pipeline's reordering has no guard of its own: it would need a data-plane socket fixture with a held failure. It reports through `report_file_outcome`, whose reading of the flag is guarded (cr-ecm33-5).
