# c-7-watch-replay: `blit jobs watch` enables replay_recent

**Severity**: Feature (CLI polish — first production consumer of c-5b)
**Status**: In progress / pending review
**Branch**: `phase5/c`
**Commit**: filled by the sentinel commit

## What

c-5b shipped `SubscribeRequest.replay_recent` and the per-job
event ring. This slice flips the flag on for `blit jobs watch`
so the CLI's first production consumer of c-5b sees the row's
recent events on connect instead of waiting for the next live
tick.

## Why

Operator UX. Without replay the watch flow is:

1. `blit jobs watch t-123` — initial GetState renders the
   active line at T=0.
2. Daemon's progress ticker fires next at T+~100ms.
3. CLI emits its first `[progress]` line at T+~100ms.

That ~100ms gap between the active line and the first
progress is small but real. With c-5b's replay:

1. Initial GetState — active line at T=0.
2. Subscribe with `replay_recent: true` — the row's ring
   replays Started + recent Progress events at T=0+ε.
3. CLI emits `[progress]` lines IMMEDIATELY from the replay.
4. Live ticks continue from there.

Net effect: the operator sees byte progress with no perceived
delay between the active line and the first progress update.

## Approach

`blit-app::admin::jobs::subscribe()` gains a `replay_recent:
bool` parameter (was previously hardcoded to `false` per
c-5a / c-5b). `run_jobs_watch` passes `true`.

The c-6 stream-consumer loop already handles replayed
TransferStarted as a no-op (the active line covers it). The
replayed TransferProgress events flow through the existing
progress emitter unchanged.

## Files changed

- `crates/blit-app/src/admin/jobs.rs`:
  - `subscribe()` signature: `(remote, transfer_id_filter,
    replay_recent: bool)`.
- `crates/blit-cli/src/jobs.rs`:
  - `run_jobs_watch` passes `true`.

## Tests added

None new. c-5b's tests
(`subscribe_replay_recent_replays_per_row_ring_to_late_joiner`,
`subscribe_without_replay_recent_skips_ring`) cover the
replay machinery. This slice just wires up the first
production consumer — `blit jobs watch` exercising the
flag set to `true`.

Workspace: 575 passing serially (unchanged from c-5b).

## Known gaps

1. **No e2e CLI test.** Driving `blit jobs watch` against a
   real daemon to verify replay timing requires an
   in-process tonic server + a fixture transfer; that's an
   integration-test scope (m-jobs-6 has the same posture for
   the polling-era code path).

2. **TUI hasn't landed yet.** The TUI (A.1) is the bigger
   replay consumer (F2 Transfers pane). When A.1 lands,
   replay_recent will surface clearly in the pane's
   join-mid-transfer behavior.

## Out of scope (next slices)

- **c-1c-files-counter**: parallel to c-1a/c-1b for files
  (matches bytes byte-counter pattern).
- **bytes_total wiring from manifest**: lets renderers
  compute `bytes_completed / bytes_total = %`.
- **Throughput EWMA** smoothing.
- **c-8-module-and-heartbeat**: ModuleListChanged,
  DaemonHeartbeat event variants. Lower priority.

## Reviewer comments

(empty — pending grade)
