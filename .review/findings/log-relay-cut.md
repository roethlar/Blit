# log-relay-cut: a log whose relay was cut at its close said it lost events only in words

**Severity**: HIGH — a retry completes a short record from a log that lost events: cr-ecm40-1's fault, by a third way.
**Status**: Reviewed, found incomplete — `combined-r42` (Codex openreview, 2026-11-25) reviewed `b6e6fab9..e2c2e08d` — the `combined-r41` fixes, log-relay-cut and GitHub CI's seven — with `guard_confirmed: true`: "I would keep the held-directory walk, the identity check when it reopens a parent, and the change that marks a collapsed failure list incomplete. I found six corrections the approach still needs." "I gave `jobs retry` a truncated record and a log modeled on the previous writer's relay-cut output. Its sequence was contiguous because the lost events never reached the writer. The retry exited 0, overwrote a file that had landed, and left the actual failed file missing. Record successful relay drainage explicitly and decline reconstruction from older logs that cannot establish it. Propagate that fact to the caller too. With the new writer, I observed `LogIncomplete` on disk alongside `LogReport.complete == true`." The remaining work is cr-ecm41-1
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `c52f2b59`
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
