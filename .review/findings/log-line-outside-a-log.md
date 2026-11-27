# log-line-outside-a-log: a line given to the detached reader with no log begun was taken

**Severity**: MEDIUM — an incremental reader keeps a failure read after a whole log's end, in a read still called complete. cr-ecm43-4's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix log-line-outside-a-log`)
**Reviewer**: none — met by the reviewer's own probes in the event stream of `combined-r45` (Codex openreview over `34c77a48..ebc9d81c`, 2026-11-26; `.review/results/combined-r45.codex.events.jsonl`), which ended with no verdict when its service flagged the content; reproduced here by a guard; for the next openreview.

## Evidence
The reviewer's probe: a whole session read and ended (`end_log`), then one more consecutively numbered `file-failed` line — `outside_session complete=true recorded_failures=1`.

## Intake
Admitted.

## What
`LogRead::read_line` with no log begun takes nothing from the line and leaves the read not whole.

## Guard proof
- `job_record::tests::a_detached_log_is_whole_only_in_sequence_with_its_summary_then_its_end`: a line after a whole log's end, with no log begun — not complete, and its failure not kept.
- Mutation, restored byte for byte: the line taken → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2162 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case: 2161 passed, the one failure the setgid test
    the namespace's unmapped groups refuse, green outside it.

## Files changed
- `crates/blit-core/src/job_record.rs` — `LogRead::read_line`; guard.

## Known gaps
- None known.
