# d-69-f1-delegated-progress: live footer for remote→remote copy

**Severity**: Feature (parity with the F3 pull / F1 push footers)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `2f1f5d2`

## What

d-68 shipped remote→remote delegated copy with only a terminal
summary in the F1 footer (`Running` showed 0 files / 0 bytes
until it finished). d-69 adds the live `files` / `bytes` /
`bytes_per_sec` counters while it runs — parity with the F3 pull
(d-37) and F1 push (d-63) footers. Mirrors the d-63 staging:
d-61 shipped push, then d-63 added its progress; d-68 shipped
delegated copy, now d-69 adds its progress.

## Why a third accumulator

The three transfer directions report progress differently, so
each needs its own fold:

- **Pull (receive)** — `accumulate_pull_progress`: bytes from
  `Payload`, files from `FileComplete` (Payload's `files` field
  is unused; counting both would double-count).
- **Push (send)** — `accumulate_push_progress`: both from
  `FileComplete` (send emits no `Payload`).
- **Delegated** (new) — `accumulate_delegated_progress`: both
  from `Payload`. The destination daemon reports cumulative
  `BytesProgress`, which `remote::report_bytes_progress`
  delivers as `report_payload(file_delta, byte_delta)` — so a
  `Payload` carries BOTH counts and no `FileComplete` arrives.

## Approach

- `accumulate_delegated_progress(files, bytes, event)` — sums
  `files` + `bytes` from `Payload`; ignores `FileComplete` /
  `ManifestBatch` defensively.
- `spawn_f1_delegated_pull` gains a `progress_tx` param and a
  forwarder (same shape as the pull/push forwarders): a
  `RemoteTransferProgress` channel, accumulate per event, ship
  `F1PushProgress` snapshots via `try_send` (lossy), then
  `drop(progress)` + `forwarder.await` before the terminal
  reply. `run_delegated_pull` now gets `Some(&progress)`.
- `plan_f1_delegated` passes `app.f1_push_progress_tx`.
- The footer already renders `Running { files, bytes,
  bytes_per_sec, verb }` (verb = "delegating"), so no render
  change — the counters were simply always 0 before.

## Files changed

- `crates/blit-tui/src/main.rs`: `accumulate_delegated_progress`
  + test; `spawn_f1_delegated_pull` forwarder + `progress_tx`;
  `plan_f1_delegated` call.
- `crates/blit-tui/src/f1push.rs`: module-doc refresh (delegated
  copy now has live progress, no longer "summary only").

## Tests

567 total (was 566, +1):
`accumulate_delegated_progress_sums_files_and_bytes_from_payload`
— two Payload deltas accumulate both counts; a stray
`FileComplete` doesn't double-count. (The forwarder wiring is
exercised end-to-end only against a live daemon; the fold is the
unit-testable piece, matching how the pull/push accumulators are
covered.)

## Known gaps / follow-ups

1. remote→remote **mirror/move** delegation (destructive).
2. **detach + F2 visibility** (needs multi-daemon F2).

## Reviewer comments

(empty — pending grade)
