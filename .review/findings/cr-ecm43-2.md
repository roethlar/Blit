# cr-ecm43-2: neither log reader asked whether a log was the run's own

**Severity**: HIGH — a retry recopies a file that landed and leaves out the one that failed.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm43-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r44` over `4e12d761..34c77a48` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r44.codex.md`.

## Evidence
"Bind each log to its run before using its file events. I placed one genuine run's finished log under another run's log filename. With a truncated record, `jobs retry` exited 0, recopied `blue.txt`, and left the actual failure, `red.txt`, absent. Check the `run-start` run ID, participant, role and attempt against the expected key in both readers."

## Intake
Admitted as written.

## What
**`LogShape::of(key)`**: the shape of the log a reader asked for. A log is whole only when it opens with its `run-start`, once, naming that key — run, participant, role, attempt; a shape made without one (`Default`) is never whole.
- **The retry's reader** asks of each log the key its file name says (`LogShape::of(log.key)`).
- **The detached reader** (`LogRead::begin_log(expected)`) asks for this run's log, as a destination, kept by the participant and in the attempt the daemon's header names; a header that names no valid key gives none, and the log is not whole.

## Guard proof
- `job_log::tests::a_log_is_whole_only_in_its_shape`: another run's, participant's, role's or attempt's log; no `run-start` first; a `run-start` after another event; a second one; a shape without a key — each not whole.
- `jobs::retry_name_tests::a_cut_list_is_completed_only_from_a_whole_log`: the reviewer's — a whole log of one run copied under another run's name completes nothing.
- `job_record::tests::a_detached_log_is_whole_only_in_sequence_with_its_summary_then_its_end`: another run's whole log, and a log asked for by no key, are not complete.
- Mutations, restored byte for byte, each → red:
  - the run-start's key not compared (all three);
  - the run not compared (all three);
  - the participant, the role, the attempt not compared, each;
  - a log that does not open with its run-start.
- The suite's end-to-end retries and detached runs read real logs whose run-starts name the key each reader asks for: green.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2158 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case (`fold_run.sh`: a casefold tmpfs in a user
    namespace, the tests run as this user): 2157 passed, the one failure a setgid test the namespace's unmapped
    groups refuse (`a_fresh_partial_takes_its_setgid_directorys_group`, `EINVAL`), green outside it.

## Files changed
- `crates/blit-core/src/job_log.rs` — `LogShape`; guard.
- `crates/blit-core/src/job_record.rs` — `LogRead::begin_log`, `ask_daemon`; guards.
- `crates/blit-cli/src/jobs.rs` — the retry's reader; guard.

## Known gaps
- A log recovered after its first line was lost has its run-start rebuilt from its name; it is not whole anyway (its run never wrote its summary).
