# log-not-its-own: another run's log, rightly not whole, still gave its events to the reader

**Severity**: HIGH — a detached run is ended as another run ended, and a retry is given the files another run failed. cr-ecm43-2's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix log-not-its-own`)
**Reviewer**: none — met by the reviewer's own probes in the event stream of `combined-r45` (Codex openreview over `34c77a48..ebc9d81c`, 2026-11-26; `.review/results/combined-r45.codex.events.jsonl`), which ended with no verdict when its service flagged the content; reproduced here by a guard; for the next openreview.

## Evidence
The reviewer's probe: a whole log whose `run-start` names another run, read for this one — `foreign_log complete=false settled_as=Some(Ok) truncated=true`: not complete, as cr-ecm43-2 made it, and then `settle`, the daemon no longer knowing the job, ended this run from that log.

And read here in the retry's reader: with the record's own list whole, another run's log under this run's name added the files that run had failed to the ones retried (`known.extend(still_failed)`).

## Intake
Admitted.

## What
**`LogShape::its_own`**: the log is the one asked for as far as it has been read — its first event is its `run-start`, naming the key expected. A reader takes a log's events only while that holds:
- the detached reader (`LogRead::read_line`) takes nothing from such a log — no failure, no summary, no end — and notes it (`not_its_own`); `settle` then has no end to settle by, and says the log kept under the run's name is not its own;
- the retry's reader stops at such a log's first event.

## Guard proof
- `job_log::tests::a_log_is_whole_only_in_its_shape`: its own; not another run's; not one whose `run-start` is not its first event; not before anything is read.
- `job_record::tests::a_detached_log_is_whole_only_in_sequence_with_its_summary_then_its_end`: another run's whole log, and a log asked for by no key — nothing taken, and `settle` with the job unknown to the daemon is an error.
- `jobs::retry_name_tests::a_cut_list_is_completed_only_from_a_whole_log`: with the record's own list whole, another run's log gives the retry no file and no evidence.
- Mutations, restored byte for byte, each → red:
  - the detached reader takes another run's events;
  - the retry's reader takes them;
  - any first `run-start` makes the log its own (all three guards);
  - a `run-start` after another event makes it its own.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2162 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case: 2161 passed, the one failure the setgid test
    the namespace's unmapped groups refuse, green outside it.

## Files changed
- `crates/blit-core/src/job_log.rs` — `LogShape`; guard.
- `crates/blit-core/src/job_record.rs` — `LogRead`, `settle`; guard.
- `crates/blit-cli/src/jobs.rs` — the retry's reader; guard.

## Known gaps
- None known.
