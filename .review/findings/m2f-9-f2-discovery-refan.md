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

## Reviewer comments

(empty — pending grade)
