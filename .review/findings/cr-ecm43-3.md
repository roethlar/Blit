# cr-ecm43-3: a second close while the first waited found no relay, and a cancelled close put its relay back over a newer lane's

**Severity**: HIGH — the log says it holds every event, and holds none of the ones still in the lane. cr-ecm42-3's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm43-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r44` over `4e12d761..34c77a48` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r44.codex.md`.

## Evidence
"Give one close operation ownership of the relay drain. With one failure queued and the first close pending, a second close wrote zero failure events and returned both `all_events_logged` and `LogReport.complete` as true. Cancelling a close can also overwrite the handle of a newer lane. Coordinate close and lane registration through shared state so every accepted lane contributes to the result."

## Intake
Admitted as written.

## What
- **The newest relay is watched, never taken** (`Relay`: a `watch` of whether it, and every relay before it, drained, and its task's abort handle). Every close waits on its own watch of it, and the next lane's relay on another; a relay cut, or stopped with its runtime, ends without a word, which is not drained. Taken by the close that waited for it, a second close at once found none; and a close cancelled put its relay back (`PendingRelay`, gone) over a newer lane's.
- **Lanes are counted**: before it takes the writer, a close checks under the lock whether a lane was accepted while it waited, and waits for that one too.
- **Once a close has the writer, no lane is accepted** (`closed`, cleared when the log starts again): no close would wait for its events.

## Guard proof
- `run_log::tests::every_close_waits_for_every_lane_the_log_accepted` (paused clock):
  - two closes at once both wait for the held lane; let go, one takes the log, which holds the lane's failure and is whole; the other is given none; then no lane is accepted;
  - a close cancelled after a newer lane was accepted: the next close waits for that lane; a lane accepted while it waits is waited for too, and its failure is in the log, which is whole.
- cr-ecm42-3's guards (`a_close_tried_again_waits_for_the_relay_the_first_left`, `a_relay_stopped_with_its_runtime_is_not_taken_for_drained`): green.
- Mutations, restored byte for byte, each → red:
  - the close takes the relay (and cr-ecm42-3's first guard with it);
  - a lane accepted while the close waits not waited for;
  - a lane accepted after the close.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2158 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case (`fold_run.sh`: a casefold tmpfs in a user
    namespace, the tests run as this user): 2157 passed, the one failure a setgid test the namespace's unmapped
    groups refuse (`a_fresh_partial_takes_its_setgid_directorys_group`, `EINVAL`), green outside it.

## Files changed
- `crates/blit-core/src/run_log.rs` — `State.relay`, `lanes`, `closed`; `lane`, `prepare_close`, `Relay`; guard.

## Known gaps
- Past the limit only the newest relay is aborted, as before; one before it may run on, and what it observes after the close is dropped, as an event for a closed log always was.
