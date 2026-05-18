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

## Reviewer comments

(empty — pending grade)
