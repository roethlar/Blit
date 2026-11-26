# cr-ecm43-4: the detached reader's `complete()` was not held to the session still being read

**Severity**: MEDIUM — an incremental reader of the public API takes a log that went on past its end for complete. cr-ecm42-1's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm43-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r44` over `4e12d761..34c77a48` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r44.codex.md`.

## Evidence
"Make `LogRead::complete()` honor the current session's validator. Through the public API, a consecutively numbered file event after `run-end` still gave `complete() == true` until I called `end_log()`. The daemon reader calls `end_log()`, but an incremental consumer can use the incorrect result."

## Intake
Admitted as written.

## What
`LogRead` knows whether a session is being read (`open`, set by `begin_log`, cleared by `end_log`). `complete()` asks that session's shape while it is open. And a session begun while another is open ends that one first, held to its shape like any other.

## Guard proof
- `job_record::tests::a_detached_log_is_whole_only_in_sequence_with_its_summary_then_its_end`: asked before `end_log`, a whole log is complete, and then an event after its end makes it not; a session never ended, then a whole one, is not complete.
- Mutations, restored byte for byte, each → red:
  - the open session's shape not asked;
  - a session begun over an open one leaves it unended.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2158 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case (`fold_run.sh`: a casefold tmpfs in a user
    namespace, the tests run as this user): 2157 passed, the one failure a setgid test the namespace's unmapped
    groups refuse (`a_fresh_partial_takes_its_setgid_directorys_group`, `EINVAL`), green outside it.

## Files changed
- `crates/blit-core/src/job_record.rs` — `LogRead`; guard.

## Known gaps
- None known.
