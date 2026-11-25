# cr-ecm41-1: a log held no positive evidence that every event reached it

**Severity**: HIGH — a retry overwrites a file that landed, and leaves out one that failed. log-relay-cut's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm41-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r42` over `b6e6fab9..e2c2e08d` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r42.codex.md`.

## Evidence
"I gave `jobs retry` a truncated record and a log modeled on the previous writer's relay-cut output. Its sequence was contiguous because the lost events never reached the writer. The retry exited 0, overwrote a file that had landed, and left the actual failed file missing. Record successful relay drainage explicitly and decline reconstruction from older logs that cannot establish it. Propagate that fact to the caller too. With the new writer, I observed `LogIncomplete` on disk alongside `LogReport.complete == true`."

## Intake
Admitted as written. The reviewer's next point, cr-ecm41-2, asks the same test of the detached reader; the evidence itself is required by both readers here.

## What
- **`Summary.all_events_logged`** (new; `false` when absent): every event the transfer reported reached the log before the summary. `RunLog::prepare_close` writes it where it is known, as the relay drained or not.
- **The caller is told.** `ClosingLog` carries the cut, so `LogReport.complete` is false where events were lost before the writer had them, with the reason.
- **The retry's reading** takes a log as whole only with a summary that holds the evidence.
- **The detached reader's** `LogRead::complete` asks the same.
- The test logs that model a run whose relay drained now say so.

## Guard proof
- `run_log::tests::a_lane_held_open_at_the_close_leaves_the_log_incomplete`:
  - a cut run's report is incomplete, and its summary holds no evidence;
  - a drained run's is complete, and its summary holds it.
- `jobs::retry_name_tests::a_cut_list_is_completed_only_from_a_whole_log`: an older blit's cut log — an unbroken sequence, a summary, the cut in words only — completes nothing.
- `job_record::tests::a_log_without_evidence_that_it_has_every_event_is_not_complete`.
- Mutations, restored byte for byte, each → red:
  - the retry takes a summary without the evidence;
  - a cut summary says every event is here;
  - the caller not told;
  - the detached reader takes a summary without the evidence.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's six fixes: 2,152 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/job_log.rs` — `Summary::all_events_logged`.
- `crates/blit-core/src/run_log.rs` — `prepare_close`, `ClosingLog`; guard.
- `crates/blit-cli/src/jobs.rs` — the log's reading; guard; test logs.
- `crates/blit-core/src/job_record.rs` — `LogRead::complete`; guard; test logs.

## Known gaps
- A log from any earlier blit holds no evidence, so it completes no record. A retry that needs one is declined ("run the whole job again"), and a detached run with an older daemon is settled by the daemon's job state.
