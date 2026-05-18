# a1-6b-state-preservation: preserve per-pane state across F-key navigation

**Severity**: Medium (follow-up split from `a1-6-screen-router`)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

a1-6 landed in-app F-key routing but each navigation
re-enters the destination pane's event loop from scratch.
That means:

- **F1**: mDNS discovery task restarts. The first scan can
  block up to 1.5s; the operator sees `scanning...` every
  time they revisit F1 from F2/F3/F4.
- **F2**: Subscribe stream is reopened and GetState is
  re-fired. Roughly 2× control-plane RTT every visit.
- **F3**: Browse path is forgotten. Operator's mid-tree
  position is gone; they restart at the module list.
- **F4**: `perf_local.jsonl` is re-read from disk.

This makes the router feel sluggish even though the
mechanics work. State preservation is the natural follow-up.

## Scope

1. New `AppState` struct holding all four panes' states
   AND their background-channel handles
   (mDNS discovery rx, Subscribe rx, browse fetch rx,
   profile fetch rx).
2. Replace the four `run_fN_event_loop` functions with a
   single `run_app_event_loop` that owns `AppState` and
   selects! across all background channels plus the
   keystroke channel.
3. Per-pane keystroke handlers extracted as
   `handle_f1_key(&mut app, action)` etc.
4. The render dispatch reads `app.current_screen` and
   routes to the appropriate `render_into`.

## Why split out

- a1-6 landed the routing primitive (`LoopOutcome::Navigate`
  + F-key recognition + tab strip) cleanly.
- The state-preservation refactor consolidates four event
  loops into one with a unified select!, which is a
  substantial code movement (hundreds of lines).
- Splitting keeps each PR easy to review: a1-6 is "did the
  F-keys work"; a1-6b is "did the consolidation preserve
  semantics."

## Implementation

### `AppState` struct in `main.rs`

```rust
struct AppState {
    parsed_remote: Option<RemoteEndpoint>,
    remote_label: String,

    // F1
    daemons: DaemonsState,
    daemons_last_fetched: Option<String>,
    detail_tx: mpsc::Sender<DetailUpdate>,
    discovery_refresh_tx: mpsc::Sender<()>,

    // F2
    transfers: TransfersState,
    transfers_status: ConnectionStatus,

    // F3
    browse: BrowseState,
    browse_last_fetched_view: Option<browse::BrowseView>,
    browse_fetch_tx: mpsc::Sender<BrowseFetchReply>,

    // F4
    profile: ProfileState,
    profile_reply_tx: mpsc::Sender<ProfileReply>,
}
```

State for every pane lives inside `AppState`. Senders for
the per-pane reply mpscs stay on `AppState` (cloned into
spawned fetcher tasks); the receivers live in
`run_router`'s scope and get borrowed into the pane loops.

### `run_router` is now the all-time setup site

Everything that used to be done by each pane's
`run_fN_event_loop` setup block is now in `run_router`:

- `parsed_remote = RemoteEndpoint::parse(args.remote)`.
- `spawn_discovery_task` for the F1 mDNS feed.
- `open_subscribe_stream + jobs::query` for F2's initial
  GetState (subscribe-first ordering preserved).
- `spawn_profile_fetch` for F4's initial read.
- F3's "no remote" / "parse failed" banner is set on
  `app.browse` directly.

Background tasks now live for the whole TUI session —
navigation through F-keys doesn't restart any of them.
Discovered daemons survive across F-key bounces; the
Subscribe stream keeps feeding `transfers_event_rx` while
the operator is on F1 or F3; the profile data stays
loaded.

### Per-pane loops renamed and slimmed

`run_fN_event_loop` → `run_fN_pane_loop`. New signatures:

```rust
run_f1_pane_loop(terminal, key_rx, app, disco_rx, detail_rx)
run_f2_pane_loop(terminal, key_rx, app, event_rx)
run_f3_pane_loop(terminal, key_rx, app, fetch_rx)
run_f4_pane_loop(terminal, key_rx, app, reply_rx)
```

Each loop:
- Uses `app.{daemons,transfers,browse,profile}` for its
  pane state instead of `let mut state = ...::new()`.
- Reads from the borrowed `*_rx` receivers; never spawns a
  task that creates a new channel-pair.
- Reads `app.parsed_remote` / `app.remote_label` so all
  four panes share the same endpoint view.
- Returns `Result<LoopOutcome>` (unchanged from a1-6).

The Subscribe error path now sets `*event_rx = None`
through the `&mut Option<mpsc::Receiver<...>>` parameter so
the router's stored value is cleared — without this, the
next F2 visit would re-enter the select! arm with a stale
empty receiver and panic.

## Files changed

- `crates/blit-tui/src/main.rs`:
  - New `AppState` struct.
  - `run_router` does all setup + spawns all background
    tasks once.
  - Each `run_fN_event_loop` renamed `run_fN_pane_loop`
    and refactored to borrow from `AppState` + receivers.

## Tests

No new tests required — the refactor preserves the
existing semantics. Every helper function tested by the
existing suite (`maybe_kick_detail_fetch`,
`handle_f3_refresh`, `views_differ`,
`drain_startup_events`, etc.) keeps its same shape and
behavior; the difference is purely in *where* the state
lives.

94 blit-tui unit tests pass (unchanged). Workspace passes
serially.

## Known gaps

1. **No test of state preservation across navigation.**
   The contract is "navigate away from F1, come back, your
   discovered rows + cursor survive." Verifying that
   end-to-end would need a TestBackend-driven integration
   test that drives the router through multiple
   transitions. Out of scope for this slice.

2. **`AppState` is not split across modules.** Everything
   lives in `main.rs`. A future polish slice could move
   the struct + setup helpers into a dedicated `app.rs`.

## Round 2 (sha filled by sentinel)

Reviewer flagged three issues on round 1:

### 1. Hidden F2 setup blocked the first draw (Medium)

`run_router` awaited `open_subscribe_stream` + initial
`GetState` inline before entering the event loop. With
default `--screen f1`, a slow or unreachable remote
nevertheless stalled the first draw for up to the
combined RTT of those two operations.

**Fix:** background-spawn the F2 setup. New
`spawn_f2_setup_task(endpoint, tx)` does both RPCs in a
detached `tokio::spawn` and posts an `F2SetupReply` (either
`Ready { event_rx, snapshot_result }` or `Failed(msg)`)
through a bounded mpsc(1). The unified loop has a select!
arm for `f2_setup_rx.recv()` that wires the returned
event_rx + snapshot into the shared state. F1 renders
immediately on TUI start regardless of remote latency.

### 2. Hidden panes' channels could back up (Medium)

Round 1 kept the four per-pane loops; only the active
pane's loop ran the select! that drained its receivers.
That meant F1's bounded(4) discovery channel and F2's
bounded(256) Subscribe channel could fill while the
operator browsed other panes, eventually back-pressuring
the producer tasks.

**Fix:** consolidate into a single `tokio::select!` in
`run_router`. The unified loop drains every channel
(disco_rx, detail_rx, transfers_event_rx, browse_fetch_rx,
profile_reply_rx, f2_setup_rx, key_rx) on every iteration
regardless of `app.current_screen`. Key dispatch happens
via a new `handle_pane_action(action, &mut app, ...)`
async helper that routes per-action based on the active
pane.

The four `run_fN_pane_loop` functions are deleted; their
state-mutation logic moved into `handle_pane_action`'s
per-screen match arms or into the unified select!'s
data-channel arms.

For the optional Subscribe receiver, the select! uses an
`if transfers_event_rx.is_some()` guard plus an inner
`async {…}` block that yields `pending()` when the
receiver is None — keeps the arm inactive until F2 setup
completes.

### 3. Remote parse errors collapsed to "invalid endpoint" (Low)

Round 1's `RemoteEndpoint::parse(raw).ok()` discarded the
actual error message. F2 and F3 banners then synthesized
a generic "parse '<raw>': invalid endpoint" string,
losing the backslash-guidance / module-path-missing hints
that the parser produces.

**Fix:** keep the parse `Result`. Round 2 builds a tuple
`(Option<RemoteEndpoint>, Option<String>)` where the
second element is the actual parse error message. Both
F2's `transfers_status` and F3's banner consume this
string directly.

### Files changed (round 2)

- `crates/blit-tui/src/main.rs`:
  - Removed the four `run_fN_pane_loop` functions and
    `LoopOutcome` enum.
  - New `run_router` body: unified select! with one arm
    per channel + key dispatch through
    `handle_pane_action`.
  - `AppState` gains `current_screen` field.
  - `spawn_f2_setup_task` + `F2SetupReply` enum.
  - `handle_pane_action(action, app, transfers_event_rx,
    f2_setup_tx)` — async; routes per active pane.
  - Parse-error string preserved through
    `parse_error_message: Option<String>`.

### Tests

Existing 94 blit-tui unit tests all pass — the helper
functions tested (`maybe_kick_detail_fetch`,
`handle_f3_refresh`, `views_differ`,
`drain_startup_events`, `apply_browse_reply`, etc.) keep
their shapes; the consolidation is purely call-site
restructuring around them.

Adding new end-to-end tests for the unified loop's
multi-channel draining would need a TestBackend + a
substantial fixture-task harness. Out of scope for this
round; tracked implicitly under E (Polish).

## Reviewer comments

(empty — pending grade)
