# log-restart-after-cut: a log started again after a cut was called incomplete with every event of its own in it

**Severity**: MEDIUM — a whole log reported as incomplete; nothing lost. cr-ecm43-3's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix log-restart-after-cut`)
**Reviewer**: none — met by the reviewer's own probes in the event stream of `combined-r45` (Codex openreview over `34c77a48..ebc9d81c`, 2026-11-26; `.review/results/combined-r45.codex.events.jsonl`), which ended with no verdict when its service flagged the content; reproduced here by a guard; for the next openreview.

## Evidence
The reviewer's probe: a log cut at its close (a lane held), then started again for another run with a lane of its own and one failure — `restart_after_cut failures_sent=1 failures_recorded=1 report_complete=false`. The new lane's relay was chained after the cut one, which had not drained.

## Intake
Admitted.

## What
When a close takes the writer, the relays go with the log now closed: the next log's first lane starts a chain of its own.

## Guard proof
- `run_log::tests::a_log_started_again_after_a_cut_is_whole_on_its_own` (paused clock): the second log is complete, its summary holds the evidence, its one failure is in it.
- Mutation, restored byte for byte: the relay left in place → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2162 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case: 2161 passed, the one failure the setgid test
    the namespace's unmapped groups refuse, green outside it.

## Files changed
- `crates/blit-core/src/run_log.rs` — `prepare_close`; guard.

## Known gaps
- None known.
