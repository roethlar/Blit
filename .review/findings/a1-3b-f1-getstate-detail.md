# a1-3b-f1-getstate-detail: F1 detail block populated by GetState

**Severity**: Medium (follow-up split from `a1-3-f1-daemons`)
**Status**: Open
**Branch**: `phase5/a1`

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

## Reviewer comments

(empty — pending implementation)
