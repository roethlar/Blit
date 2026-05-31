# a1-3b-f1-getstate-detail: F1 detail block populated by GetState

**Severity**: Medium (follow-up split from `a1-3-f1-daemons`)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

`a1-3-f1-daemons` shipped the F1 Daemons table and the
synthetic Local row but left the detail block as mDNS TXT
data only. The TUI_DESIGN owner-signoff also calls for a
per-daemon detail pane lit by `GetState`:

> ```
> ┌─ Selected: mycroft ──────────────────────────────────────────┐
> │ Version: 0.1.0  │ Uptime: 2d 4h 17m  │ Active: 1            │
> │ Modules: home (12.3 TiB / 16.0 TiB), backups, media          │
> │ Counters: 142 push / 88 pull / 3 purge │ errors: 1           │
> ```

This slice wires that up.

## Scope

1. On cursor change, fire `jobs::query` against the
   selected daemon (or the loopback port for the Local
   row).
2. Cache the most recent `DaemonState` per `instance_name`
   so cursor flicks don't re-query each time.
3. Surface the values in the detail block: active count,
   recent-completed count, version. Module capacity / push-
   /pull/purge counters require either daemon-side
   counters (none today) or per-module disk stats — both
   tracked separately; this slice renders what is already
   on the wire.
4. Status indicator in the detail block when GetState is
   in flight, succeeded, or failed.

## Out of scope

- Per-module disk stats (would need `df`/`du` per module).
- Push/pull/purge counters (would need new daemon-side
  counters).
- mDNS-based endpoint resolution. The detail-pane query
  builds a `RemoteEndpoint` from the selected daemon's
  first advertised address + port; Local uses a loopback
  endpoint.

## Why split out

Round-1 review of `a1-3-f1-daemons` flagged this as a
Medium gap. Rather than land it inside the same slice
(which would have ballooned `run_f1_event_loop` with a
debounced query manager, per-row caching, and the
endpoint-resolution scaffold) the work is split out so:

1. `a1-3` lands cleanly with state model + render + Local
   row + viewport behavior.
2. `a1-3b` adds the GetState integration as a focused
   slice that's easy to review on its own.

## Implementation

### State (`daemons.rs`)

New `DaemonDetail` enum next to the existing `DaemonsState`:

```rust
pub enum DaemonDetail {
    Pending,
    Loaded { state: Box<DaemonState>, fetched_at: Instant },
    Error { message: String },
}
```

`DaemonsState` gains:

- `details: HashMap<String, DaemonDetail>` — per-row cache,
  keyed by `instance_name`. Survives mDNS rescans (regression
  test pins this) so a flicker in/out of a daemon doesn't
  retrigger a fetch.
- `detail_for(instance_name) -> Option<&DaemonDetail>`
- `set_detail(instance_name, detail)`
- `endpoint_for_row(row) -> Option<RemoteEndpoint>` —
  loopback `127.0.0.1:9031` for Local; first advertised
  address + port for Remote; `None` for Remote with no
  addresses.

### Event loop (`run_f1_event_loop` in `main.rs`)

Selection-change-triggered fetch:

1. At the top of every loop tick, `maybe_kick_detail_fetch`
   compares `state.selected_row().instance_name` against the
   last-fetched name. On change: sets `DaemonDetail::Pending`
   for the new row and spawns `spawn_detail_fetch(endpoint,
   name, detail_tx)`.
2. `spawn_detail_fetch` runs `jobs::query(endpoint, 0)`
   off-thread; the result is tagged with the row's name and
   delivered through a bounded(8) mpsc.
3. The select! loop pulls `DetailUpdate { instance_name,
   result }` and `set_detail`s the result. The renderer
   reads back whatever is currently keyed under the
   selected row's name.

`r` (Refresh) keystroke now also re-fires the detail fetch
by resetting `last_fetched = None`. Periodic detail refresh
(time-based) is **not** in scope — operator can hit `r` to
re-fetch.

### Render (`screens/f1.rs`)

`detail_lines` signature gains `detail: Option<&DaemonDetail>`
and `now: Instant`. Render paths:

**Remote row + `Loaded`**:
```
mycroft · 192.168.1.10:9031 · 0.2.0
active: 1 · recent: 3 · push: 12 · pull: 4 · errors: 0
modules: home, media, backups
uptime: 1d 1h · as of 2s ago
```

**Remote row + `Pending`**:
```
mycroft · 192.168.1.10:9031 · 0.2.0
fetching GetState...
modules: home, media, backups       (mDNS fallback)
delegation: enabled                 (mDNS fallback)
```

**Remote row + `Error`**: red "GetState failed: …" line
plus the same mDNS fallback lines.

**Local row + `Loaded`**: green "local daemon detected · vX
· uptime Y" header + counters line + "as of Xs ago".

**Local row + `Error`**: yellow "no local daemon detected"
+ message + dimmed hint pointing at `blit-daemon`.

**Local row + `None`**: dimmed "GetState not yet attempted"
(transient — `maybe_kick_detail_fetch` fires the request
on the next loop tick).

## Files changed

- `crates/blit-tui/src/daemons.rs`: `DaemonDetail` enum,
  `details` field on `DaemonsState`, helper methods
  (`detail_for`, `set_detail`, `endpoint_for_row`).
- `crates/blit-tui/src/screens/f1.rs`:
  `detail_lines`/`local_detail_lines`/`detail_body_for_remote`
  consume `DaemonDetail`; mDNS-only fallback helpers
  (`mdns_modules_line`, `mdns_delegation_line`);
  `format_uptime` helper.
- `crates/blit-tui/src/main.rs`: `maybe_kick_detail_fetch`,
  `spawn_detail_fetch`, `DetailUpdate` envelope, detail-rx
  arm in the F1 select loop; `r` keystroke invalidates
  `last_fetched`.

## Tests added

12 new unit tests:

In `daemons::tests`:
- `endpoint_for_row_returns_loopback_for_local`
- `endpoint_for_row_uses_first_advertised_address`
- `endpoint_for_row_returns_none_when_remote_has_no_address`
- `detail_for_returns_set_value`
- `set_detail_replaces_prior_value_for_same_name`
- `details_survive_discovery_rescan`

In `screens::f1::tests`:
- `detail_lines_for_remote_loaded_shows_counters`
- `detail_lines_for_remote_pending_shows_spinner_and_mdns_fallback`
- `detail_lines_for_remote_error_shows_message_and_fallback`
- `detail_lines_for_local_loaded_shows_live`
- `detail_lines_for_local_error_shows_no_daemon_hint`
- `format_uptime_picks_correct_unit`

Existing `detail_lines_*` tests updated for the new
`detail` + `now` parameters and the additional body line
(GetState-not-fetched-yet hint in the `None` path).

56 blit-tui unit tests (was 44). Workspace passes serially.

## Known gaps

1. **No periodic detail refresh.** Operator triggers a
   re-fetch with `r`. Time-based refresh (e.g. every 10s on
   the selected row) could land in a future polish slice.

2. **No selection-debounce.** Cursoring quickly through
   the list fires one fetch per intermediate row. Tasks are
   cheap and the names tag results, but a debounce of
   ~100ms would avoid burning RPCs on transient cursor
   moves. Out of scope for this slice.

3. **`active_transfers` Counters field is unused.** The
   detail block reads `state.active.len()` directly — same
   number, but the wire-side counter is technically the
   authoritative one. Today they always match.

4. **Per-module disk capacity (`12.3 TiB / 16.0 TiB` in the
   design) isn't here.** Would need per-module `df` queries;
   tracked as a separate concern.

5. **No render test against TestBackend for the full F1
   pane with a loaded detail.** Existing tests cover each
   variant of `detail_lines` independently; a full-pane
   golden test could land alongside future polish.

## Round 2 (sha filled by sentinel)

Reviewer caught two correctness gaps:

### 1. Cache not used to avoid re-fetching (Medium)

Round-1 `maybe_kick_detail_fetch` always overwrote the
cached `Loaded` entry with `Pending` whenever the selected
name differed from `last_fetched`. Cursor flick away and
back triggered a redundant RPC AND visibly flickered the
detail block from loaded data back to "fetching..."

Fix: consult `state.detail_for(&name)` before deciding to
spawn. If *any* cached entry exists (`Loaded`, `Pending`,
OR `Error`), the kick just updates `last_fetched` and
returns — no RPC, no Pending overwrite.

The `r` keystroke remains the explicit invalidation: it
calls `DaemonsState::invalidate_detail(name)`, which drops
the cache entry AND bumps the row's request_id so an
in-flight reply from before `r` won't write back.

Regression test (`maybe_kick_detail_fetch_preserves_loaded_on_revisit`):
1. Manually inject a `Loaded { version: "9.9.9" }` detail
   for daemon "alpha".
2. Run kick → cursor moves to alpha.
3. Cursor off, then back to alpha.
4. Run kick again.
5. Assert detail is still `Loaded { version: "9.9.9" }`
   AND that no `DetailUpdate` landed on the detail_tx
   channel.

Companion test (`maybe_kick_detail_fetch_spawns_when_cache_empty`)
pins the inverse: empty cache → kick sets Pending.

### 2. Older same-row replies could overwrite newer ones (Medium)

A press of `r` (or any path that re-fires a fetch for the
same row) starts a second RPC. If the first RPC's reply
arrives after the second's, the stale data overwrites the
fresh data.

Fix: per-row generation counter in `DaemonsState`.

- New field `request_ids: HashMap<String, u64>`.
- `begin_fetch(name) -> u64`: bumps the row's id, stores
  Pending, returns the new id. Replaces the old `set_detail(_,
  Pending)` call site in the kick.
- `apply_detail_update(name, request_id, detail) -> bool`:
  only writes the detail if `request_id` matches the row's
  current id. Returns true on apply, false on drop (used
  by tests; the main loop ignores the return).
- `invalidate_detail(name)`: removes the cache entry AND
  bumps the request_id (so an in-flight reply from before
  the invalidation is silently dropped).

`spawn_detail_fetch` now takes the request_id and embeds
it in the `DetailUpdate` reply. The select! apply arm calls
`state.apply_detail_update(name, request_id, detail)`.

Regression tests:
- `begin_fetch_increments_request_id_per_row`: two bumps on
  the same row → ids 1, 2; bumping a different row starts
  at 1.
- `apply_detail_update_writes_current_generation`:
  begin_fetch → reply with matching id → applied.
- `apply_detail_update_drops_stale_generation`: begin_fetch
  twice for the same row; reply with the older id is
  dropped; Pending (from the second begin_fetch) stays.
- `invalidate_detail_drops_in_flight_reply`: begin_fetch →
  invalidate → reply with the now-stale id is dropped;
  cache stays empty.

### Files changed (round 2)

- `crates/blit-tui/src/daemons.rs`: `request_ids` field,
  `begin_fetch`, `apply_detail_update`, `invalidate_detail`
  methods; existing `set_detail` kept as a synchronous
  bypass (tests + invalidation Pending writes).
- `crates/blit-tui/src/main.rs`: `maybe_kick_detail_fetch`
  consults `detail_for` before spawning; `r` keystroke
  calls `invalidate_detail`; `DetailUpdate` carries
  `request_id`; reply arm calls `apply_detail_update`.

### Tests

+6 unit tests:

In `daemons::tests`:
- `begin_fetch_increments_request_id_per_row`
- `apply_detail_update_writes_current_generation`
- `apply_detail_update_drops_stale_generation`
- `invalidate_detail_drops_in_flight_reply`

In `main::tests`:
- `maybe_kick_detail_fetch_preserves_loaded_on_revisit`
- `maybe_kick_detail_fetch_spawns_when_cache_empty`

62 blit-tui unit tests (was 56). Workspace passes serially.

## Reviewer comments

(empty — pending grade)
