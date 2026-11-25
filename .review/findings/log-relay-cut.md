# log-relay-cut: a log whose relay was cut at its close said it lost events only in words

**Severity**: HIGH — a retry completes a short record from a log that lost events: cr-ecm40-1's fault, by a third way.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix log-relay-cut`)
**Reviewer**: none — found outside a review, while fixing cr-ecm40-1; for the next openreview.

## Evidence
`RunLog::prepare_close` waits `RELAY_DRAIN_LIMIT` (30 s) for the transfer's audit lanes to drain. Where one is still held, it aborts the relay, and the events not handed over are lost. The log said so in a `diagnostic` event only: "the transfer still held its log lane open when the run ended; events it had not handed over are missing".

Such a log is otherwise whole, as far as structured facts go:
- it is finished;
- it holds the run's summary;
- it ends with `run-end`;
- every event the writer numbered is there.

So the retry's reading (cr-ecm40-1) took it as whole, and could complete a record from it. Its readers act on facts, never on a diagnostic's words (D-2026-11-24-1).

## Intake
Admitted. Found while fixing cr-ecm40-1, whose finding names two of the three ways a log loses events (recovery, a gap). This is the third.

## What
- The cut is recorded as `log-incomplete`, with the same words as its reason. Every reader of a log already takes that event as "events were lost":
  - the retry's reading sets the log not whole;
  - `job_record`'s reading marks the record incomplete.

  A blit before this one reading such a log does the same.
- `log-incomplete`'s documentation names both of its causes now. Its line in `blit jobs log` no longer adds "nothing after this was kept", which was untrue of a cut relay. The writer's own stop still says in its reason that it could not write the log.

## Guard proof
- `run_log::tests::a_lane_held_open_at_the_close_leaves_the_log_incomplete` (paused clock): a lane held across the close; the finished log holds `log-incomplete`.
- Mutation, restored byte for byte: the cut not marked → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux.

## Files changed
- `crates/blit-core/src/run_log.rs` — `prepare_close`; guard.
- `crates/blit-core/src/job_log.rs` — `LogIncomplete`'s documentation and its line.

## Known gaps
- None known.
