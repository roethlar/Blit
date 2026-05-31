# m2f-5-f2-fanout: F2 watches all discovered daemons (merged streams)

**Severity**: Feature (TUI_DESIGN §5.2 single-pane-of-glass)
**Status**: In progress / pending review (round 2)
**Branch**: `phase5/a1`
**Commit**: `49f1fce` (R1 `7a5e7a3`)

## What

The **visible** multi-daemon F2, built on the m2f-1..4 foundation.
F2 no longer watches just `parsed_remote` — the setup fans out to
**every watched daemon** and merges their Subscribe streams into one
channel, so transfers from every daemon on the network appear
together (the design's single-pane-of-glass).

## Approach

- **Watch set** (`f2_watched_endpoints`): `parsed_remote` (if set,
  first so it's watched immediately) + every discovered remote daemon
  (`DaemonsState::remote_endpoints`), deduped by `host_port_display()`
  identity.
- **Merged channel**: `open_subscribe_stream` now forwards into a
  caller-owned shared `mpsc::Sender<F2Event>` (instead of returning
  its own rx). `spawn_f2_setup_task` takes `Vec<RemoteEndpoint>`,
  opens one stream per daemon into that shared sender, fetches each
  daemon's `GetState`, and returns the merged rx + per-daemon
  snapshots. It drops its own sender handle so the receiver closes
  only when every watched stream ends.
- **Hydration**: the reply handler `merge_snapshot`s each daemon's
  snapshot (additive, m2f-3). A per-daemon `GetState` failure degrades
  the status only if *none* succeeded (the streams may still be live).
  A daemon whose *subscribe* fails is skipped; the reply is `Failed`
  only when no daemon could be reached.
- **(Re)fan triggers**: launch, the d-48 daemon-switch, and F2 `r`
  refresh all spawn with the current `f2_watched_endpoints`.

## Removed (now obsolete)

- `f2_source_label` — events carry their own daemon since m2f-4.
- `state::replace_from_snapshot` — every daemon hydrates additively
  via `merge_snapshot` (m2f-3); a clear-all replace can't coexist with
  fan-out. Its tests were repointed to `merge_snapshot` (identical for
  one daemon).

## Known edges / deferred to m2f-6

- **Identity reconciliation**: a daemon given by hostname as
  `parsed_remote` AND discovered by IP has two identities → watched
  twice (rows appear under both). Documented on `f2_watched_endpoints`.
- **Render**: the per-row source-daemon **column** isn't added yet —
  rows from all daemons appear, just not labeled by daemon. (m2f-6.)
- **Dynamic discovery**: daemons appearing *after* setup aren't
  auto-subscribed until the next `r`/d-48 (m2f-6).
- **Per-daemon reconnect / degraded UI** and **multi-daemon cancel**:
  m2f-6.

## Files changed

- `crates/blit-tui/src/daemons.rs`: `remote_endpoints()` + test.
- `crates/blit-tui/src/main.rs`: `open_subscribe_stream` (shared
  sender); `spawn_f2_setup_task` (fan-out); `F2SetupPayload.snapshots`;
  reply handler (per-daemon merge); `f2_watched_endpoints`; 3 call
  sites; removed `f2_source_label`; test updates.
- `crates/blit-tui/src/state.rs`: removed `replace_from_snapshot`;
  `merge_snapshot` doc.

## Tests

584 total. New: `remote_endpoints_skips_local_and_resolves_discovered`
(daemons), `f2_watched_endpoints_dedups_by_identity` (main). The
model's multi-daemon coexistence is covered by m2f-2/m2f-3 tests
(`same_id_from_two_daemons_stays_distinct`,
`merge_snapshot_is_additive_per_daemon`). The N-stream subscribe runs
against live daemons (manual).

## Round 2 (fix)

Two fan-out correctness findings:

1. **F2 `r` didn't re-fan while a stream was live** — the live
   branch only ran `refresh_via_get_state(parsed_remote)`, so newly
   discovered daemons were never subscribed until the stream tore
   down. Fixed: unified the refresh through `refan_f2_setup`, which
   restarts the merged setup against the current `f2_watched_endpoints`
   (drops the old merged rx → its forwarders exit → re-spawn),
   whether or not a stream is live. The pending guard still prevents
   duplicate setups. Removed the now-obsolete `should_spawn_f2_setup`
   and `refresh_via_get_state`.

2. **One daemon's Error dropped the whole merged receiver** — killing
   every other daemon's forwarder. Fixed: extracted `apply_f2_event`;
   `Error` keeps the receiver (status → Degraded), only `None` (all
   senders closed) drops it.

**Fix (`49f1fce`):** tests —
`refan_f2_setup_respawns_on_live_refresh_and_guards_pending` (live rx
+ discovered daemon → drops rx, bumps gen, sets pending; no respawn
while pending) and `apply_f2_event_error_keeps_receiver_none_drops_it`
(Error keeps rx + a later event from another daemon still applies;
None drops). 585 tests.

## Reviewer comments

(empty — pending round-2 grade)
