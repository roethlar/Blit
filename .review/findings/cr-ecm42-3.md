# cr-ecm42-3: a close interrupted and tried again, or a relay failed or stopped with its runtime, counted as drained

**Severity**: HIGH — the log says it holds every event, and holds none of the ones lost.
**Status**: Reviewed, found incomplete — `combined-r44` (Codex openreview, 2026-11-26) reviewed `4e12d761..34c77a48` — the `combined-r43` fixes, GitHub CI's three and gate-put-race — with `guard_confirmed: true`: "I would keep the shared `LogShape` validator, per-session delivery checks, scripted manifest order and synchronized `--ignore-existing` fixture. The following cases still need changes." "Give one close operation ownership of the relay drain. With one failure queued and the first close pending, a second close wrote zero failure events and returned both `all_events_logged` and `LogReport.complete` as true. Cancelling a close can also overwrite the handle of a newer lane. Coordinate close and lane registration through shared state so every accepted lane contributes to the result." The remaining work is cr-ecm43-3
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `b848405e`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r43` over `e2c2e08d..4e12d761` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r43.codex.md`.

## Evidence
"I queued one failure, interrupted a pending close, and retried the close. The log contained zero file failures, yet both `all_events_logged` and `LogReport.complete` were true. Shutting down the relay's runtime before closing produced the same result. Preserve the pending drain across close attempts and propagate cancellation or failure from every relay in the chain."

## Intake
Admitted as written.

## What
- **Each relay tells whether everything before it drained** (`JoinHandle<bool>`). A lane's relay awaits the one before it and carries that on: a relay cut, failed or stopped with its runtime is not drained. It had awaited the earlier one and gone on regardless.
- **The close counts as drained only `Ok(true)`.** Anything else is a cut, with the log marked and the caller told, as log-relay-cut and cr-ecm41-1 made them.
- **`PendingRelay`**: the relay a close has taken to wait for is put back if that close is dropped before it settles, so a close tried again waits for it in turn.

## Guard proof
- `run_log::tests::a_close_tried_again_waits_for_the_relay_the_first_left` (paused clock): a lane holding a failure; the first close interrupted.
  - With the lane still held, the second close waits past the limit, and the relay is cut: the report is incomplete, and the summary holds no evidence.
  - With the lane let go, the second close waits for the relay: its failure is in the log, and the log is whole.
- `run_log::tests::a_relay_stopped_with_its_runtime_is_not_taken_for_drained`: a lane's relay stopped with its runtime, and a later lane chained after it, on another; the close is told the relay was cut.
- Mutations, restored byte for byte, each → red:
  - a relay that ended otherwise taken for drained;
  - the chain not carried on;
  - a dropped close forgets the relay.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2,156 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/run_log.rs` — the relay chain, `prepare_close`, `PendingRelay`; guards.

## Known gaps
- A close dropped after the relay settled, and after it took the log's writer, leaves nothing for a second close: that one returns no report, as before.
