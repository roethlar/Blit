# cr-ecm41-2: the detached reader trusted a log with a gap, or without its own summary and end

**Severity**: HIGH — a detached run's record names a file that landed as the one that failed, and calls itself whole.
**Status**: Reviewed, found incomplete — `combined-r43` (Codex openreview, 2026-11-25) reviewed `e2c2e08d..4e12d761` — the `combined-r42` fixes and GitHub CI's three — with `guard_confirmed: true`: "I would keep the granted-header identity, the failure-bounded disposition tracking, the explicit log-delivery field, and the independent `openat2` probe. I would require the following corrections." "I gave `jobs retry` a truncated record and a log with two consecutively numbered file events after `run-end`. It exited 0, overwrote `landed`, and left `missing` absent. Both readers should require a single terminal `summary` followed by `run-end` and end of file." The remaining work is cr-ecm42-1
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `9dabfe07`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r42` over `b6e6fab9..e2c2e08d` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r42.codex.md`.

## Evidence
"A complete destination log identified `missing` as the sole failed file. After I omitted two middle events, the detached reader still reported the log complete and produced an untruncated record naming `landed` instead. Validate sequence continuity and terminal summary/end ordering for each session before either reader trusts it."

## Intake
Admitted as written.

## What
`LogRead` reads each session's log on its own terms:
- **`begin_log`** starts a session's log: events numbered from 0, no summary or end yet.
- **Each event** must be the next in sequence. Nothing may follow the end, a summary included.
- **`end_log`** requires that log's own summary and its end; another session's do not stand for it.

The daemon-log reader wraps each session's lines in the two.

## Guard proof
- `job_record::tests::a_detached_log_is_whole_only_in_sequence_with_its_summary_then_its_end`:
  - the reviewer's log is whole, and names `missing`;
  - with two events out of its middle, it is not;
  - nor with the summary after the end, an event after the end, or a second session's log without its own summary and end.
- Mutations, restored byte for byte, each → red:
  - a gap not looked for;
  - an event after the end taken;
  - a session's log without its own summary and end taken.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's six fixes: 2,152 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/job_record.rs` — `LogRead`; the daemon-log reader; guards.

## Known gaps
- None known.
