# d-68-f1-remote-remote-copy: delegated copy from the F1 trigger

**Severity**: Feature (TUI_DESIGN §1 "between any two endpoints")
**Status**: In progress / pending review (round 4)
**Branch**: `phase5/a1`
**Commit**: `c93bcd6` (R3 `9531dde`, R2 `96075cb`, R1 `1dcdc66`)

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

## Round 3 (fix)

**Reviewer (Medium):** R2 delegated *any* `Ok(Remote)` dest, but a
bare relative local dest (`backup`) parses as
`RemotePath::Discovery` (`RemoteEndpoint::parse` treats bare
`host` / `host:port` as discovery). So `nas:/photos/ → backup`
stopped falling through to the remote→local pull and was rejected
as "needs a module" — a regression of the pre-d-68 behavior, where
`start_pull` accepts any nonblank local dest.

**Fix (`9531dde`):** narrow the delegated branch with a guard —
`Ok(Remote(dst)) if ensure_remote_destination_supported(&dst).is_ok()`
(Module/Root only). A discovery parse now falls through to the pull
alongside genuine local paths; `Err` still rejects. New test
`plan_f1_trigger_remote_source_bare_dest_pulls_not_delegates`
pins it. 565 tests.

The three dest buckets for a remote source are now: remote
module/root → delegate; discovery-or-local → pull; remote-shaped
typo (`Err`) → reject.

## Round 4 (fix)

**Reviewer (Medium):** R3's `Err → reject` arm rejected Windows-style
local destinations like `C:/tmp/out`. `RemoteEndpoint::parse` classifies
`C:/path` / `C:\path` as local (`check_local_path → IsLocal`), but
`parse_transfer_endpoint` (blit-app) re-converted that error to `Err`
because the string contains `:/` — so a remote-source pull to a Windows
local dest never reached `start_pull`. The CLI shared the same latent
bug.

**Fix (`c93bcd6`):** root-caused in `parse_transfer_endpoint` — it now
honors the lower-level "input appears to be a local path" verdict and
returns `Ok(Local)` *before* the `:/` remote-typo guard. Remote-shaped
typos (`skippy:/backup`) still `Err`; genuine module dests still parse
remote. Tests: blit-app unit tests
(`windows_drive_paths_are_local`, `remote_shaped_typo_still_errors`,
`module_dest_is_remote`) + the requested TUI regression
`plan_f1_trigger_remote_source_windows_local_dest_pulls`. 566 blit-tui
tests; blit-app endpoints suite green.

Note: this fixes a shared blit-app classifier, so the CLI's
`blit copy nas:/photos/ C:/tmp/out` benefits too.

## Reviewer comments

(empty — pending round-4 grade)
