# cr-ecm40-1: a log recovery finished, or one with a gap, completed a short record

**Severity**: HIGH — a retry overwrites a file that landed, and leaves out one that failed.
**Status**: Reviewed, found incomplete — `combined-r42` (Codex openreview, 2026-11-25) reviewed `b6e6fab9..e2c2e08d` — the `combined-r41` fixes, log-relay-cut and GitHub CI's seven — with `guard_confirmed: true`: "I would keep the held-directory walk, the identity check when it reopens a parent, and the change that marks a collapsed failure list incomplete. I found six corrections the approach still needs." "I gave `jobs retry` a truncated record and a log modeled on the previous writer's relay-cut output. Its sequence was contiguous because the lost events never reached the writer. The retry exited 0, overwrote a file that had landed, and left the actual failed file missing. Record successful relay drainage explicitly and decline reconstruction from older logs that cannot establish it. Propagate that fact to the caller too. With the new writer, I observed `LogIncomplete` on disk alongside `LogReport.complete == true`." "A complete destination log identified `missing` as the sole failed file. After I omitted two middle events, the detached reader still reported the log complete and produced an untruncated record naming `landed` instead. Validate sequence continuity and terminal summary/end ordering for each session before either reader trusts it." The remaining work is cr-ecm41-1 and -2
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `15fcd228`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r41` over `d7f5f5e9..b6e6fab9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r41.codex.md`.

## Evidence
"`jobs retry` accepted a crash-recovered log, overwrote an already-landed file, omitted a file that still needed copying, and exited 0. A log with a sequence-number gap produced the same result. Check sequence continuity and terminal summary coverage before completing a short record. Recovery can append `run-end` to a partial log."

## Intake
Admitted as written. The `run-end` outcome cannot tell recovery from a Ctrl-C: both are `interrupted`, and recovery's own note is words. A run's own close writes its `summary` before `run-end`, and neither recovery nor the close of a log whose handles all went away writes one. So the summary is the structured fact.

## What
The retry's reading of a log takes it as whole (`LoggedLeftovers::complete`) only where all of these hold:
- it is finished;
- it holds its run's own `summary`;
- it ends with `run-end`;
- it says nothing was lost;
- every event its writer numbered is there, in order: each one's `seq` is the one after the last, from the header's 0.

The test logs that model a run closing its own log now record the summary such a run writes.

## Guard proof
- `jobs::retry_name_tests::a_cut_list_is_completed_only_from_a_whole_log`:
  - A partial log left by a crash is recovered by `job_log::recover`: completes nothing.
  - A whole log with one event cut out of its middle: completes nothing.
  - A whole log still completes the record.
- Mutations, restored byte for byte:
  - a log with no summary of its run's taken whole → red;
  - a gap in the sequence not looked for → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's seven fixes: 2,146 tests over 57 binaries.

## Files changed
- `crates/blit-cli/src/jobs.rs` — the log's reading; guard; test logs.

## Known gaps
- A run that stops before it has totals writes no summary, so its log completes nothing: its record is declined, "run the whole job again".
- The relay cut at close is still marked only in a diagnostic's words; it is `log-relay-cut`, found outside a review.
