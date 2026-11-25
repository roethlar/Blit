# cr-ecm42-1: a reader took events after a log's end, and neither held a log to one summary then its end

**Severity**: HIGH — a retry overwrites a file that landed, and leaves out one that failed.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm42-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r43` over `e2c2e08d..4e12d761` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r43.codex.md`.

## Evidence
"I gave `jobs retry` a truncated record and a log with two consecutively numbered file events after `run-end`. It exited 0, overwrote `landed`, and left `missing` absent. Both readers should require a single terminal `summary` followed by `run-end` and end of file."

## Intake
Admitted as written, as one shape both readers ask of a log, as the reviewer's opening asks: "local and detached jobs should use one reader that verifies every contributing session".

## What
**`job_log::LogShape`** (new) is whether one log is whole, as every reader takes it:
- every event its writer numbered, in order, from 0;
- exactly one summary, holding the delivery evidence (cr-ecm41-1);
- nothing between the summary and `run-end`;
- nothing after `run-end`, a second end or a summary included;
- no line lost, no `log-incomplete`.

The retry's reading (`jobs.rs`) and the detached reader (`LogRead`, per session) both hold each log to it, in place of their own checks.

## Guard proof
- `job_log::tests::a_log_is_whole_only_in_its_shape`: whole; not after the end, a second end, a summary after the end, two summaries, an event between the summary and the end, no evidence, no summary, no end, a gap, a lost line.
- `jobs::retry_name_tests::a_cut_list_is_completed_only_from_a_whole_log`: the reviewer's log, with a landed file and the missing one numbered on after `run-end`, completes nothing.
- Mutations, restored byte for byte, each → red:
  - an event after the end taken;
  - two summaries taken;
  - an event between the summary and the end taken;
  - the retry's reader not held to the shape.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2,156 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/job_log.rs` — `LogShape`; guard.
- `crates/blit-core/src/job_record.rs` — `LogRead`.
- `crates/blit-cli/src/jobs.rs` — the log's reading; guard.

## Known gaps
- None known. A run writes its summary as the last event before `run-end` (`RunLog::prepare_close`); the suite's end-to-end retries read real logs in that shape.
