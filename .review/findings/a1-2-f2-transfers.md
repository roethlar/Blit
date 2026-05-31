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

## Round 3 (sha `455ba2e`)

Reviewer caught the third startup race: round 2's
ordering ran `GetState` BEFORE registering the Subscribe
Receiver. A transfer that started after the snapshot but
before the stream registered was invisible — Progress
events would discard against an unknown id, and a Complete
would land in `recent[]` with blank kind/peer/module/path.

Fix mirrors c-6 round 2 on the CLI side: **subscribe
first**.

1. `run_event_loop` opens Subscribe (spawns the forwarder)
   BEFORE awaiting `jobs::query`. Events broadcast during
   the GetState RPC's flight buffer in the mpsc.
2. After applying the snapshot, `drain_startup_events`
   `try_recv`s buffered events and applies each onto state.
3. Falls into the normal select loop.

For the merge to be safe:

- `TransfersState::apply_event(TransferStarted)` now uses
  `entry().or_insert_with(...)` instead of `insert(...)`.
  Started events for ids already in the snapshot are no-ops
  (preserves snapshot bytes/throughput). Returns `true`
  only when the row was newly inserted.
- The daemon's broadcast is FIFO per receiver, so a
  transfer that Started+Completed entirely within the
  startup window has its Started buffered first. Replay
  inserts the row with metadata, then Complete moves it
  to recent. The "blank metadata" failure mode is closed.

+2 regression tests:

- `apply_event_started_does_not_clobber_snapshot_progress`
  — snapshot has bytes=500_000; buffered Started arrives;
  bytes preserved.
- `buffered_started_then_complete_preserves_metadata` —
  transfer started+completed in the race window arrives
  in recent with full metadata (kind/peer/module/path).

`apply_event_started_inserts_idempotently` updated to
match the new return semantics (true on first insert,
false on duplicate).

Workspace: 591 passing serially.

## Round 4 (sha filled by sentinel)

Reviewer caught two follow-ups on round 3's subscribe-first
ordering:

### 1. Subscribe-first was not causally ordered (Medium)

Round 3 issued `tokio::spawn(async move { subscribe(...).await
... })` and immediately returned. The spawned task still had
to be scheduled, connect over gRPC, and have the daemon
register its broadcast receiver — all of which can happen
*after* the caller resumes and fires `GetState`. A transfer
started in that gap was still invisible.

Fix: **inline-await** the subscribe RPC.

```rust
async fn open_subscribe_stream(
    endpoint: &RemoteEndpoint,
) -> Result<mpsc::Receiver<EventOrError>, String> {
    let stream = jobs::subscribe(endpoint, "", false).await?;
    let (tx, rx) = mpsc::channel::<EventOrError>(TUI_EVENT_BUFFER);
    let _ = tx.send(EventOrError::Connected).await;
    tokio::spawn(forward_subscribe_stream(stream, tx));
    Ok(rx)
}
```

When this function returns `Ok(rx)`, the daemon's broadcast
sender is registered and any subsequent transfer event is
in our mpsc. The spawned `forward_subscribe_stream` task is
just the inner pump — no setup work remains.

`run_event_loop` now calls `open_subscribe_stream(&endpoint).await`
BEFORE `jobs::query(...)`. Failure of subscribe lands the TUI
in Degraded state without ever firing GetState (no point
fetching a snapshot we can't keep live).

### 2. Buffered terminal events duplicate snapshot recent[]
   rows (Medium)

A transfer that started, completed, and was recorded in the
daemon's `RecentRing` **before** the GetState RPC returned
would appear in `snapshot.recent[]`. The same transfer's
Started + Complete pair also broadcast through the Subscribe
stream during the startup window and buffered in our mpsc.
Replaying those buffered events ran:

- Started → `active.entry(id).or_insert_with(...)` — inserted
  a fresh active row (id not in active).
- Complete → `move active→recent` — pushed a SECOND recent
  row with the same `transfer_id`.

Net: the recent table showed two rows for one transfer.

Fix: **terminal-id dedup at the top of `apply_event`**.

```rust
let event_id = match event.payload.as_ref() {
    Some(Payload::TransferStarted(s))  => Some(s.transfer_id.as_str()),
    Some(Payload::TransferProgress(p)) => Some(p.transfer_id.as_str()),
    Some(Payload::TransferComplete(c)) => Some(c.transfer_id.as_str()),
    Some(Payload::TransferError(e))    => Some(e.transfer_id.as_str()),
    None => None,
};
if let Some(id) = event_id {
    if self.recent.iter().any(|r| r.transfer_id == id) {
        return false;
    }
}
```

Once an id is in `recent[]`, no further event for that id
can mutate state. Closes the duplicate-row failure mode for
both the snapshot/stream overlap and any future Complete →
late Error scenarios.

### Files

- `crates/blit-tui/Cargo.toml`: + tonic = "0.14" (needed
  because `forward_subscribe_stream`'s signature names
  `tonic::Streaming<DaemonEvent>`).
- `crates/blit-tui/src/main.rs`: subscribe ordering refactor
  + extracted `forward_subscribe_stream` helper.
- `crates/blit-tui/src/state.rs`: terminal-id dedup at
  `apply_event` entry.

### Tests

+1 regression test in `state::tests`:

- `buffered_events_dedupe_against_snapshot_recent`:
  populates snapshot.recent[] with id "race-id"; pushes
  Started + Complete for that id through `apply_event`;
  asserts active_count=0, recent_count=1, both events
  return `false`.

19 unit tests in `blit-tui` total (was 18). Workspace
passing serially: 592 tests.

## Round 5 (sha filled by sentinel)

Reviewer flagged a status regression introduced in round 4:

### Connected masked an initial GetState failure (Low)

Path: subscribe succeeds (Connected pre-sent into the
buffer), initial `GetState` fails → `status =
Degraded("initial GetState failed: ...")`. Then
`drain_startup_events` runs, sees the buffered
`EventOrError::Connected`, and unconditionally flips
`status = Live`. The footer reads "live" even though the
active/recent snapshot is missing — F2 silently shows a
partial state.

Root cause: Connected is a *stream-health* signal, not a
*snapshot-health* signal. Round 4 conflated the two.

Fix: only let `Connected` transition `Connecting → Live`;
preserve any existing `Degraded(...)`.

```rust
Ok(EventOrError::Connected) => {
    if matches!(status, ConnectionStatus::Connecting) {
        *status = ConnectionStatus::Live;
    }
}
```

The same rule applies to the first event in the drain (and
in the main select loop's Connected/first-event arms — those
were already gated for the first-event case, the Connected
arm now matches).

### Files

- `crates/blit-tui/src/main.rs`: gate Connected on
  `matches!(status, Connecting)` in two places — the
  startup drain and the main select loop's Connected arm
  (kept consistent so the comment matches the code).

### Tests

+2 regression tests in `tests`:

- `drain_startup_events_connected_preserves_degraded`:
  pre-set `Degraded("initial GetState failed: timeout")`,
  buffer one `Connected`, assert status stays `Degraded`
  with the same message.
- `drain_startup_events_connected_flips_connecting_to_live`:
  pre-set `Connecting`, buffer one `Connected`, assert
  status flips to `Live`.

21 blit-tui unit tests (was 19); workspace passing serially.

## Reviewer comments

(empty — pending grade)
