# log-events-before-start: events handed to a log before it started were dropped, and the log said it held every event

**Severity**: HIGH — the log says it holds every event, and holds none of those. cr-ecm43-3's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix log-events-before-start`)
**Reviewer**: none — met by the reviewer's own probes in the event stream of `combined-r45` (Codex openreview over `34c77a48..ebc9d81c`, 2026-11-26; `.review/results/combined-r45.codex.events.jsonl`), which ended with no verdict when its service flagged the content; reproduced here by a guard; for the next openreview.

## Evidence
The reviewer's probe: a lane opened before the log's start, one more failure sent than its queue holds, the log then started and closed — `prestart_lane accepted=1025 recorded=0 all_events_logged=Some(true) report_complete=true`. `observe_from` returned in silence where no log was open.

## Intake
Admitted.

## What
An event handed over with no log open to take it — and none closed before it — is noted (`State.unlogged`); the next close marks the log `log-incomplete` (`the transfer reported events before the log was started; they are missing`), its summary holds no evidence, and the caller's report is incomplete with that reason. An event for a log already closed is that log's, as before.

## Guard proof
- `run_log::tests::events_before_the_start_leave_the_log_incomplete`: the reviewer's — the report incomplete, the summary without evidence, the loss marked; and a lane opened before the start whose events all come after it leaves the log whole.
- Mutation, restored byte for byte: the loss not noted → red.
- The workspace's own runs, none of which hands a log an event before its start: green.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2162 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case: 2161 passed, the one failure the setgid test
    the namespace's unmapped groups refuse, green outside it.

## Files changed
- `crates/blit-core/src/run_log.rs` — `State.unlogged`, `observe_from`, `prepare_close`, `ClosingLog`; guard.

## Known gaps
- The events themselves are still lost: a log cannot take what it is given before it has a writer. It no longer says otherwise.
