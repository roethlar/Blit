# d-47-f1-browse-nav: enter on a daemon row browses it

**Severity**: Feature (designed — TUI_DESIGN §5.1 `[enter] browse`)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `065fd95`

## What

Until now the F3 browser only ever showed the daemon passed via
`--remote` at launch. TUI_DESIGN §5.1 specifies `[enter] browse`
on the F1 daemon list — select any discovered daemon, press
enter, and browse it. d-47 delivers that: `enter`/`l`/`→` on an
F1 daemon row retargets F3 to that daemon and jumps to F3.

## Approach

### The `browse_target` seam

The core change is decoupling the **F3 browse target** from the
**F2 transfers target**. Previously both were `parsed_remote`.
Now:

- `parsed_remote` — the F2 Subscribe target, bound to the launch
  remote for the session (unchanged).
- `browse_target` (new) — the F3 daemon. Initialized to
  `parsed_remote`, but retargetable. F3 browse-fetch, pull, du,
  delete, the cursor pull-spec, and the F3 header all key off it.

The F2 cancel ops (`K`/`X`/confirm) deliberately stay on
`parsed_remote` — they cancel transfers on the daemon F2 is
watching, not the one F3 is browsing.

### F1 enter

`UserAction::Descend` on F1 resolves the selected row to a
`RemoteEndpoint` (`DaemonsState::endpoint_for_row`) and, when it
resolves, calls `retarget_browse`: set `browse_target`, reset the
browse view to a fresh `Modules` list, clear
`browse_last_fetched_view` (so the loop's fetch driver kicks a
listing for the new daemon), and switch to F3. The Local row
(and any row without an endpoint) is a no-op — F3 is a remote
browser.

### Header

The F3 header now shows `browse_target.host_port_display()` so it
always names the daemon being browsed, not the launch label.

## Files changed

- `crates/blit-tui/src/main.rs`:
  - `browse_target` field on AppState (init = launch remote).
  - 8 F3 sites repointed from `parsed_remote` to `browse_target`
    (browse-fetch driver, pull-spec, header label, F3 refresh,
    delete-refresh, F3PullBegin, F3DeleteBegin, F3DuBegin).
  - F1 `Descend` arm + `retarget_browse` helper.
  - 5 AppState test fixtures gain `browse_target: None`; 1 test.
- `crates/blit-tui/src/help.rs`: `Enter / → / l` row notes the
  F1 browse action.

## Tests

+1 test (458 → 459):

- `retarget_browse_switches_f3_target_and_navigates` — retarget
  sets `browse_target`, resets the view to Modules, clears the
  fetched-view marker, jumps to F3, and leaves `parsed_remote`
  (F2) untouched.

`DaemonsState::endpoint_for_row` (the row→endpoint resolution) is
already covered in `daemons::tests`.

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

## Known gaps

1. **F2 does not follow the browse target.** Switching the F3
   browse daemon does NOT re-point F2's Subscribe stream — F2
   keeps showing the launch daemon's transfers. This is a
   deliberate scope boundary: re-subscribing F2 mid-session
   touches the generation-guarded setup/teardown lifecycle and
   the live `transfers_event_rx`, which is its own slice. Each
   pane's header names its own daemon, so there's no hidden
   inconsistency — F3 says "Browse · skippy", F2 says
   "Transfers · nas". A follow-on can make F2 follow (or watch
   all daemons).
2. **Local row can't be browsed.** `enter` on Local is a no-op;
   F3 is a remote browser. Local-filesystem browsing would be a
   separate feature.
3. **Browse target resets to a fresh fetch each enter.** Even
   re-entering the same daemon re-fetches its module list (no
   per-daemon view cache). Cheap and always correct.

## Out of scope

- F2 following the browse target / multi-daemon F2.
- Local-filesystem browsing on F3.
- Per-daemon browse-view caching.

## Reviewer comments

### Round 1 verdict — reopened (`.review/results/d-47-f1-browse-nav.reopened.md`)

One finding:

- **Enter on the Local row was not a no-op.** The finding claimed
  Local was inert because F3 is a remote browser, but
  `DaemonsState::endpoint_for_row` returns the loopback
  `127.0.0.1:9031` for Local (so the daemon's own RPCs work) —
  *not* `None`. Round 1 retargeted on any `Some`, so Enter on
  Local jumped to F3 and browsed loopback instead of doing
  nothing. My "Local is a no-op" assertion was simply wrong about
  the helper's return.

### Round 2 fix

- Extracted `f1_browse_target(daemons) -> Option<RemoteEndpoint>`,
  which `filter`s out `row.is_local()` before resolving the
  endpoint — so Enter on Local returns `None` (genuine no-op).
  The F1 `Descend` arm calls it.

### Round 2 tests

+2 tests (459 → 461):

- `f1_browse_target_is_none_for_local_row` — the reviewer's
  specific ask: a fresh `DaemonsState` (cursor on Local) resolves
  to `None`.
- `f1_browse_target_is_some_for_remote_row` — a discovered remote
  daemon resolves to `Some`.

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

### Lesson restated

Don't assert a helper's behavior from its name or intent —
`endpoint_for_row` *sounds* like it'd return `None` for a
non-remote row, but it returns loopback so the local daemon is
reachable. Read the callee, and gate on the actual domain
predicate (`is_local()`) rather than assuming a sentinel return.
