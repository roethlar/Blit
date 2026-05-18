# a1-2-f2-transfers: F2 Transfers pane with live Subscribe stream

**Severity**: Feature (second slice of milestone A.1 — first real TUI screen)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Replaces a1-1's placeholder splash with the F2 Transfers
screen. When `--remote <host>` is set, the TUI:

1. Fetches an initial `GetState` snapshot.
2. Spawns a background `Subscribe` forwarder task that
   sends `DaemonEvent`s into an mpsc.
3. Runs an event loop that `tokio::select!`s between a
   keystroke `spawn_blocking` and the mpsc.
4. Renders header + Active table + Recent table + Footer
   on every loop iteration.

This is the first end-to-end TUI consumer of the c-2..c-5b
Subscribe wire surface. Exercises the daemon's filter +
forwarder + per-row event ring (via empty-filter
all-events subscription) without taking a dependency on
the daemon process during unit tests.

## Why F2 first

A.1 has four screens. F2 is the most relevant to milestones
B (GetState), M-Jobs (CancelJob/detach), and C (Subscribe)
since they were all built to feed it. Landing F2 first
validates the wire-surface decisions end-to-end with a real
client. F1 (Daemons), F3 (Browse), F4 (Profile/Verify) land
in subsequent slices.

## Approach

### State model (`state.rs`)

```rust
pub struct TransfersState {
    active: HashMap<String, ActiveRow>,  // by transfer_id
    recent: VecDeque<RecentRow>,         // newest-first, bounded
}
```

- `replace_from_snapshot(DaemonState)` — initial connect + 'r' refresh.
- `apply_event(DaemonEvent) -> bool` — incremental Subscribe updates:
  - `TransferStarted` → `active.insert(id, row)` (idempotent for
    snapshot-replays).
  - `TransferProgress` → in-place update of active row's
    bytes/throughput. Returns false if id not active (event
    raced with row drain — discard).
  - `TransferComplete` → move active→recent with ok=true.
  - `TransferError` → move active→recent with ok=false + message.
- `recent` ring bounded by `TUI_RECENT_CAP = 50` (matches the
  daemon's `DEFAULT_RECENT_LIMIT`). Newest-first ordering so
  the renderer can iterate without re-sorting.

### Rendering (`screens/f2.rs`)

Pure free function: `render(frame, state, remote_label, status)`.
Stacked layout:

```
header (1 line)   ─ blit-tui · F2 Transfers · <remote> · N active · M recent
active table      ─ id  kind  peer  module/path  bytes  throughput
recent table      ─ id  kind  peer  module/path  bytes  duration
footer (1 line)   ─ status · q/Esc quit · r refresh
```

`ConnectionStatus` enum carries banner state:
`NoRemote` / `Connecting` / `Live` / `Degraded(msg)`.
Recent rows colored green/red by `ok` flag.

### Event loop (`main.rs`)

```rust
let keystroke = tokio::task::spawn_blocking(|| { /* event::poll + read */ });
if let Some(rx) = event_rx.as_mut() {
    tokio::select! {
        k = keystroke => { /* handle quit/refresh */ }
        ev = rx.recv() => { /* state.apply_event(ev) */ }
    }
} else {
    /* keystroke-only path for --no-remote */
}
```

The forwarder task (`spawn_subscribe_forwarder`):

- Calls `jobs::subscribe(&endpoint, "", false)` — empty filter
  (watch every transfer), no replay (don't need history
  for the start-of-day state; GetState handled that).
- Loops over `stream.message().await`. Forwards events
  via `tx.send(EventOrError::Event(_))`.
- Stream errors / end → `EventOrError::Error(msg)` sent once,
  task exits.
- Sender drop on TUI close → exits via send-failure.

### Refresh hook

Pressing `r` re-issues `jobs::query` and atomically replaces
the local state. Status banner flips back to Live on success;
Degraded on failure. Lets the operator recover after a
Subscribe stream error without restarting the TUI.

### CLI args

Same `--remote <host>` as a1-1; now consumed. No flag changes.

## Files changed

- `crates/blit-tui/Cargo.toml`: unchanged (deps fit a1-1).
- `crates/blit-tui/src/main.rs`: event loop rewritten (~250 LOC).
- `crates/blit-tui/src/state.rs` (new): `TransfersState` +
  `ActiveRow` + `RecentRow` + 6 unit tests.
- `crates/blit-tui/src/screens/mod.rs` (new): module declaration.
- `crates/blit-tui/src/screens/f2.rs` (new): pure render
  function + 3 unit tests for formatting helpers.

Removed: a1-1's `render_splash` / `center_within` and the
`center_within_returns_middle_band` test (splash replaced by
the F2 layout). Kept: `TuiGuard` / `take_active_for_restore`
/ panic hook / `should_quit` — the lifecycle scaffold
unchanged.

## Tests added

9 new unit tests:

In `state::tests`:
- `replace_from_snapshot_populates_active_and_recent`
- `apply_event_progress_updates_row_in_place`
- `apply_event_progress_for_unknown_id_returns_false`
- `apply_event_complete_moves_row_to_recent`
- `apply_event_error_moves_row_to_recent_with_message`
- `apply_event_started_inserts_idempotently`
- `recent_ring_drops_oldest_on_overflow`

In `screens::f2::tests`:
- `format_bytes_picks_correct_unit`
- `format_ms_picks_correct_unit`
- `module_path_handles_each_empty_combination`

Workspace: 589 passing serially (was 580; +9).

## Known gaps

1. **No render test against TestBackend.** A future test
   could call `render` against a ratatui `TestBackend` and
   compare the resulting buffer against a golden snapshot.
   Out of scope for this slice — the state model + format
   helpers are covered; the layout is mostly visual.

2. **No periodic redraw outside events.** The screen
   redraws only when a keystroke fires OR a Subscribe event
   arrives. If both are quiet (no active transfers, no
   keystrokes), the screen sits idle. That's fine — there's
   nothing to display that's changing. A future slice could
   add a tokio interval for clock updates if any time-based
   display lands.

3. **Subscribe with empty filter sees ALL transfers.** Means
   the TUI receives events even for transfers the operator
   isn't watching specifically. Today that's exactly the F2
   behavior — "show everything on this daemon." A future
   per-row-detail screen would filter via `transfer_id_filter`.

4. **No screen routing.** The whole UI is F2. F1/F3/F4 +
   routing land in subsequent slices.

5. **Forwarder Degraded path is one-shot.** When the stream
   errors, the receiver gets one Degraded message, then the
   loop drops the receiver. Manual refresh (`r`) brings
   state up to date but doesn't re-establish the live
   stream. A future slice could add automatic reconnect.

## Out of scope (next A.1 slices)

- **a1-3-f1-daemons**: F1 Daemons via mDNS + per-daemon detail.
- **a1-4-f3-browse**: F3 Browse via List/Find/DiskUsage.
- **a1-5-f4-profile**: F4 reads `~/.config/blit/perf_local.jsonl`.
- **a1-6-screen-router**: F-keys to navigate between panes.

## Round 2 (sha `da7646e`)

Reviewer caught two real medium findings:

### 1. Detached keystroke polls (Medium)

Each loop iteration's `spawn_blocking` keystroke poll
could detach when the select arm preferred a Subscribe
event. crossterm `event::poll`/`event::read` are sync and
not cancellable, so dropped JoinHandles left blocking
tasks still polling — and a detached task could silently
consume a q/Esc/r keystroke. Under active progress
traffic the lossy quit/refresh was a real bug.

Fix: **single-owner input task**. `spawn_input_task` runs
ONE blocking task that loops on `event::poll`/`event::read`
and forwards every key press through an mpsc. The main
loop selects on `key_rx.recv()` and the Subscribe rx — no
per-iteration spawn_blocking, no detached blocking tasks.
Input task exits when the mpsc Sender becomes closed (TUI
quitting) — checked via `tx.is_closed()` on each
poll-timeout cycle.

### 2. Connecting status stuck on idle daemons (Low)

The status only flipped from Connecting → Live on the
first event. A successful Subscribe with no transfer
activity left the footer reading "connecting..." forever.

Fix: forwarder emits `EventOrError::Connected` immediately
after the subscribe RPC returns. The main loop flips to
Live on that control message. First-event path keeps the
Live transition as a defensive fallback in case Connected
hit mpsc backpressure.

### Code structure

- `handle_keystroke(join_result)` replaced by
  `key_action(&KeyEvent) -> Option<UserAction>` — pure
  function, no async/JoinResult plumbing.
- `EventOrError` enum grew a `Connected` variant.

### Tests

+2 in main tests:

- `key_action_maps_quit_and_refresh`
- `key_action_returns_none_for_unmapped_keys`

16 unit tests in `blit-tui` total. Workspace passing.

## Reviewer comments

(empty — pending grade)
