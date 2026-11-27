# log-close-of-another-run: a close left waiting ended the next run's log, with the first run's outcome

**Severity**: HIGH — a run's log ends as another run ended, and the run's own close finds none. cr-ecm43-3's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix log-close-of-another-run`)
**Reviewer**: none — met by the reviewer's own probes in the event stream of `combined-r45` (Codex openreview over `34c77a48..ebc9d81c`, 2026-11-26; `.review/results/combined-r45.codex.events.jsonl`), which ended with no verdict when its service flagged the content; reproduced here by a guard; for the next openreview.

## Evidence
The reviewer's probe: a close waiting on a held lane; the lane let go; a second close ends the log; the log started again for the next run; then the first close resumes — `old_close filename=Some("second.daemon.destination.1.jsonl.gz") recorded_outcome=Some(Failed) second_close_had_writer=false`.

## Intake
Admitted.

## What
Each start of a log is counted. A close ends only the log that was open when it was asked: finding another started since, it ends nothing and returns none.

## Guard proof
- `run_log::tests::a_close_ends_only_the_log_it_was_asked_to_end`: the close left waiting returns none; the second run's own close ends its log, under its own name, as that run ended.
- Mutation, restored byte for byte: any open log ended → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2162 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case: 2161 passed, the one failure the setgid test
    the namespace's unmapped groups refuse, green outside it.

## Files changed
- `crates/blit-core/src/run_log.rs` — `State.started`, `begin`, `prepare_close`; guard.

## Known gaps
- None known.
