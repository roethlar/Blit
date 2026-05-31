# m2f-9-f2-discovery-refan: auto re-fan F2 when the daemon set changes

**Severity**: Feature / correctness (multi-daemon F2 — dynamic discovery)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `5e24918`

## What

m2f-5..8 made F2 watch every discovered daemon via merged Subscribe
streams, but the watch set was only (re-)fanned on F2 entry and on an
explicit `r`. So a daemon that appeared via mDNS *after* the operator
entered F2 didn't show its transfers until a manual refresh, and a
daemon that vanished left its now-dead streams in the merge. mDNS
rescans every ~5s, so this gap is hit routinely. m2f-9 makes a
discovery update that actually changes the watched-daemon set auto
re-fan the merged streams — closing the last single-pane-of-glass gap
the m2f-6/7 findings flagged as a follow-up.

## Approach

- `f2_watched_identities(app) -> BTreeSet<String>` — the current watch
  set keyed by `host_port_display`, the same identity
  `f2_watched_endpoints` dedups on (parsed_remote ∪ discovered
  remotes). A set, so order-independent equality is the change test.
- The `DiscoveryUpdate::Result` arm snapshots identities *before*
  `replace_from_discovery`, applies the update, then re-reads them. On
  a genuine change it calls the existing `refan_f2_setup`.
- Two gates keep this from thrashing live streams:
  - **Live-only:** re-fan only when `transfers_event_rx.is_some()` —
    i.e. F2 already has an active subscription. If the operator hasn't
    entered F2, there are no streams to refresh and we don't spawn any
    in the background.
  - **Change-only:** a steady discovery feed re-reports the same
    daemons every cycle; `before == after` then, so it's a no-op. Only
    an appearance/disappearance re-fans.
  - The pre-existing pending-guard inside `refan_f2_setup` (no respawn
    while `transfers_setup_pending`) absorbs a burst of updates.

## Files changed

- `crates/blit-tui/src/main.rs`: `f2_watched_identities`; the
  discovery-Result arm computes before/after and conditionally
  re-fans; one unit test.

## Tests

590 total (+1): `f2_watched_identities_changes_when_a_daemon_appears`
— a launch daemon (`nas`) plus a discovered one (`192.168.1.50:9050`)
asserts (a) the discovered daemon enters the set so the appearance is
detected, and (b) re-reporting the same daemon leaves the set
unchanged, so a steady feed won't churn. The discovery→auto-refan
wiring in the `select!` arm is integration (live daemons + mDNS).

## Scope

Auto re-fan on watch-set change only. Per-daemon reconnect / degraded
state (a daemon that stays listed but whose stream drops) remains a
separate concern; today such a stream's Error is absorbed by
`apply_f2_event` (m2f-5) and the row simply stops updating until the
next re-fan.

## Round 2 (commit `1e6e871`)

**Reopen finding:** the auto re-fan was gated on
`transfers_event_rx.is_some()`. A daemon discovered *while the startup
setup was still pending* (receiver not yet live) was lost: the change
couldn't re-fan (the gate skipped it) and wasn't recorded, so the
stale setup completed on the old watch set and later steady updates
compared equal — the daemon stayed unwatched until a manual `r`. The
same gate also skipped the mDNS-only launch (no `--remote`), where the
first discovered daemon never auto-watched.

**Fix:**
- `AppState.transfers_refan_after_setup` — records a watch-set change
  that arrives while `transfers_setup_pending` (when `refan_f2_setup`
  would no-op).
- `handle_discovery_watch_change(app, before, rx, tx)` — on a real
  change, re-fans immediately when no setup is pending, else sets the
  deferred flag. Replaces the `is_some()` gate, so the idle case (a
  failed/never-run setup, e.g. mDNS-only launch picking up its first
  daemon) now re-fans too.
- `apply_deferred_refan(app, rx, tx)` — the setup-reply arm runs the
  deferred re-fan once the pending setup lands and `pending` clears
  (both `Ready` and `Failed`), so a daemon discovered mid-flight ends
  up watched.

**Tests:** 591 total (+1 over R1).
`discovery_during_pending_setup_refans_after_it_lands` — startup setup
pending (no live receiver), discovery adds `192.168.1.50:9050`, the
change defers (no new gen spawned while pending); after the stale
setup lands and pending clears, the deferred re-fan spawns a fresh
fan-out (generation bumped, flag cleared) whose watch set includes the
mid-flight daemon. The two decision points were extracted into the
helpers above so the sequence is unit-testable without driving the
event loop.

## Round 3 (commit `9204a4d`)

**Reopen findings (round 2):**
1. An *emptying* watch set didn't drop the live receiver.
   `refan_f2_setup` returned early on `watched.is_empty()` *before*
   `*transfers_event_rx = None`, so when the last daemon vanished
   (mDNS-only / last-daemon case) its Subscribe stream stayed live and
   the daemon remained watched until the stream happened to close.
2. A *shrinking* watch set (`A+B → A`) stranded the removed daemon's
   active rows. The fresh setup for the remaining daemon only merges
   *its* snapshot, and `merge_snapshot` is per-daemon, so `B`'s
   in-flight rows — which can never receive a Complete/Error now that
   `B`'s stream is gone — lingered in the active table forever.

**Fix:**
- `TransfersState::retain_active_daemons(watched)` — drops active rows
  whose `source_daemon` left the watched set; **keeps recent rows**
  (finished transfers are history regardless of daemon presence);
  clears the active cursor if it was anchored to a pruned row.
- `refan_f2_setup` now reconciles via `retain_active_daemons` on every
  call (so a shrink prunes the removed daemon), drops the receiver
  unconditionally, and treats an empty watch set as a first-class
  outcome — receiver dropped, `transfers_status → NoRemote` — instead
  of returning before cleanup. (An empty set only occurs with no
  `parsed_remote`, so `NoRemote` is exactly right.)

**Tests:** 596 total (+3 over R2).
- `state::retain_active_daemons_drops_unwatched_active_keeps_recent` —
  prunes the unwatched daemon's active row, keeps its recent row, never
  leaves the cursor on a pruned daemon.
- `discovery_emptying_drops_receiver_and_clears_rows` — mDNS-only, one
  daemon with a live receiver; discovery goes empty → receiver dropped,
  active rows cleared, nothing watched, status `NoRemote`.
- `discovery_shrink_prunes_removed_daemon_active_rows` — `A+B → A`: the
  removed daemon's active row is pruned while a fresh setup for the
  remaining daemon goes pending.

## Reviewer comments

(empty — pending round-3 grade)
