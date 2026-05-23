# d-68-f1-remote-remote-copy: delegated copy from the F1 trigger

**Severity**: Feature (TUI_DESIGN §1 "between any two endpoints")
**Status**: In progress / pending review (round 2)
**Branch**: `phase5/a1`
**Commit**: `96075cb` (R1 `1dcdc66`)

## What

The F1 trigger handled remote→local (pull, d-58…d-60) and
local→remote (push, d-61/d-65), but a **remote source + remote
destination** — a remote→remote delegated transfer — was the
missing arm. Worse, it was *silently mis-routed*: the
`Endpoint::Remote(source)` branch in `plan_f1_trigger` always
treated the source as remote→local and passed the raw `dest`
string into the F3 pull machine as a literal **local** path, so
`nas:/a/ → skippy:/b/` would have tried to pull into a local
directory named `skippy:`.

d-68 detects remote→remote up front and routes copy to the
verified delegated execution path (`run_delegated_pull` /
`DelegatedPullExecution`, the `a0-delegated-execution` module),
where the destination daemon pulls from the source.

## Scope (atomic)

- **Copy only.** Mirror/move delegation is rejected with a clear
  message ("remote→remote supports copy only for now") rather
  than mis-routed — a follow-up, mirroring how push shipped copy
  (d-61) before mirror/move (d-65).
- **Attached** (`detach: false`, the CLI default). The design's
  `detach: true` + F2 visibility depends on multi-daemon F2
  (not yet built), so that's deferred.
- **No live byte progress.** Delegated transfers report via the
  pull data-plane (`report_payload`), not the push `FileComplete`
  path, so the existing push accumulator doesn't fit. Passes
  `None` for progress; the authoritative summary still lands on
  the terminal reply (same staging as d-61 push, before d-63
  added push progress).

## Approach

- `plan_f1_trigger`: before the existing source-based match, if
  source **and** dest both parse as `Endpoint::Remote`, hand off
  to the new `plan_f1_delegated`. Otherwise behaviour is
  unchanged (remote→local pull, local→remote push).
- `plan_f1_delegated`: rejects non-copy kinds; gates the dest
  with `ensure_remote_destination_supported` (needs a module,
  same as push); `begin_delegated` + `spawn_f1_delegated_pull`.
- `spawn_f1_delegated_pull`: builds `DelegatedPullExecution`
  (copy `PullSyncOptions` via the existing `f3_pull_options`,
  `detach: false`, `relay_fallback_suggestable: false`), runs
  `run_delegated_pull(.., None, |_| {})`, flattens the outcome
  (`summary.files_transferred` / `bytes_transferred`) into the
  reused `F1PushReply`.
- `f1push.rs`: a `delegated: bool` on `Running`/`Done`/`Error`
  (threaded via a new `begin_delegated`; existing `begin` sets
  it `false`, so no churn to the d-61…d-65 call sites/tests). The
  verb bridge maps `delegated` → "delegating"/"delegated".

## Files changed

- `crates/blit-tui/src/main.rs`: `plan_f1_trigger` remote→remote
  detection; `plan_f1_delegated`; `spawn_f1_delegated_pull`; verb
  bridge + `push_present_verb`/`push_past_verb` gain a `delegated`
  arg; `TriggerOutcome` derives `Debug` (for tests); 2 tests.
- `crates/blit-tui/src/f1push.rs`: `delegated` field +
  `begin_delegated`; module doc; 2 tests.
- `crates/blit-tui/src/f1trigger.rs`: module-doc refresh — the
  trigger now wires all three directions (was stale: claimed
  push + delegated were "follow-ups", and local-dest only).

## Tests

562 total (was 558, +4):

- `f1push::begin_delegated_marks_running_and_flows_to_done` /
  `begin_push_is_not_delegated`.
- `plan_f1_trigger_remote_to_remote_copy_delegates`
  (`#[tokio::test]`) — remote+remote copy → `Launched`,
  `F1PushStatus::Running { delegated: true }`, and the F3 pull
  machine is NOT engaged (no mis-route).
- `plan_f1_trigger_remote_to_remote_mirror_move_rejected` —
  mirror/move → `Rejected("…copy only…")`.

## Known gaps / follow-ups

1. remote→remote **mirror/move** delegation.
2. **detach + F2 visibility** (needs multi-daemon F2).
3. **live byte progress** for delegated copy.

## Round 2 (fix)

**Reviewer (Medium):** the remote→remote detection only matched
`Ok(Remote)` and dropped `Err(_)` — so a remote-*shaped* typo dest
(e.g. `skippy:/backup`, a module path missing its trailing slash,
which `parse_transfer_endpoint` deliberately rejects) still fell
through into the remote→local pull as a literal local path. Same
mis-route class the slice meant to close.

**Fix (`96075cb`):** parse the destination once for a remote source
and branch on all three outcomes — `Ok(Remote)` delegates,
`Ok(Local)` stays remote→local, `Err(_)` → `Rejected("invalid
destination: …")`. Two new tests: malformed remote-dest is rejected
(no pull, no delegate); a genuine local dest still pulls. 564 tests.

## Reviewer comments

(empty — pending round-2 grade)
