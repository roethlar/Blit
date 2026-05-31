# a1-3-f1-daemons: F1 Daemons pane via mDNS discovery

**Severity**: Feature (third slice of milestone A.1 — adds F1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Adds the F1 Daemons screen to `blit-tui`. Operator selects
it with `blit-tui --screen f1`. The pane:

1. Spawns a background mDNS discovery task that loops every
   5s and forwards `Vec<MdnsDiscoveredService>` (or a
   `String` error) into the event loop via mpsc.
2. Renders a table of discovered daemons with cursor
   selection + a per-row detail block underneath.
3. Accepts `↑`/`↓` (and vim-style `k`/`j`) to navigate the
   cursor and `r` to nudge an immediate rescan.

a1-2's F2 screen stays the default (`--screen f2`). A.1's
final slice (a1-6) replaces `--screen` with in-app F-key
routing.

## Why F1 third (not first)

a1-2's reviewer landed F2 first because every wire-surface
slice (B / M-Jobs / C) was built to feed it. F1 needs only
the mDNS discovery primitive that already exists in
`blit_app::scan` — no daemon-side work. Landing it now
proves the second screen plugs into the existing terminal
lifecycle without a router refactor, sets the pattern for
F3 / F4, and gives the operator a list of daemons to point
F2 at.

## Approach

### State model (`daemons.rs`)

```rust
pub struct DaemonRow {
    instance_name: String,
    addresses: Vec<Ipv4Addr>,
    port: u16,
    module_count: Option<u32>,    // None = pre-§3.2 daemon
    delegation_enabled: Option<bool>,  // ditto
    version: Option<String>,
    modules: Vec<String>,
}

pub struct DaemonsState {
    rows: Vec<DaemonRow>,
    selected: usize,
    status: DiscoveryStatus,
}

pub enum DiscoveryStatus {
    Scanning,
    Live { last_scan_at: Instant },
    Degraded { message: String },
}
```

Reducer surface is small:

- `replace_from_discovery(&[Service], scanned_at)` — replaces
  rows AND preserves cursor on the same `instance_name` when
  the daemon survives the rescan. Sorts deterministically by
  `instance_name` so a re-scan doesn't shuffle the table on
  the operator.
- `note_discovery_error(msg)` — sets status to Degraded
  WITHOUT clearing rows (operator still sees the previous
  result + an error banner).
- `select_next` / `select_prev` — bounded cursor movement;
  no-ops at the ends and on an empty list.

### Rendering (`screens/f1.rs`)

Pure free function `render(frame, &state, now: Instant)`.
Layout mirrors F2's: header / table / detail block /
footer. Header counts discovered daemons; footer renders
status with `last scan Xs ago` when Live.

Detail block per cursor row shows: name + addr:port +
version line; modules line (advertised names → fallback to
count → fallback to "(daemon does not advertise)"); and
delegation line (yes / no / "unknown (pre-§3.2 daemon)").

Address formatting collapses multi-address daemons to
`first.addr+N` for table width — the detail block could
expand this in a future slice but no operator pain today.

### Discovery task

```rust
fn spawn_discovery_task(
    interval: Duration,
    scan_timeout: Duration,
    refresh_rx: mpsc::Receiver<()>,
    tx: mpsc::Sender<DiscoveryUpdate>,
)
```

`tokio::time::interval(5s)` with `MissedTickBehavior::Skip`
so paused TUIs (suspend / detach) don't fire a flurry on
resume. Each tick:

1. Wait on `interval.tick()` OR `refresh_rx.recv()` — the
   `r` keystroke pushes `()` via a bounded(1) channel for
   non-blocking nudges. When a manual scan fires, `ticker.reset()`
   pushes the next automatic scan forward so we don't
   double-fire.
2. `blit_app::scan::discover(scan_timeout=1.5s)` — wrapped
   in spawn_blocking by `blit_app::scan`, so the discovery
   doesn't stall the runtime.
3. Forward result/err. Receiver close → return (F1 loop
   exited).

### Event loop (`run_f1_event_loop`)

```rust
loop {
    terminal.draw(|f| screens::f1::render(f, &state, Instant::now()));
    tokio::select! {
        key = key_rx.recv() => key_action match ...
        update = disco_rx.recv() => state.replace_from_discovery(...) | note_discovery_error(...)
    }
}
```

Mirrors `run_f2_event_loop`'s shape but without the
GetState / Subscribe wiring. Keystrokes go through the same
`spawn_input_task` + `key_action` plumbing from a1-2 — only
the action handler differs.

### Shared `UserAction` enum

Extended with `SelectNext` / `SelectPrev`. F2 ignores both
(no cursor today). F1 maps them to `select_next` /
`select_prev`. `key_action` now recognises arrow keys plus
vim `j`/`k` for navigation.

## Files changed

- `crates/blit-tui/src/daemons.rs` (new): `DaemonRow`,
  `DaemonsState`, `DiscoveryStatus` + 8 unit tests.
- `crates/blit-tui/src/screens/f1.rs` (new): render
  function + format helpers + 7 unit tests.
- `crates/blit-tui/src/screens/mod.rs`: `pub mod f1;` added.
- `crates/blit-tui/src/main.rs`:
  - `mod daemons;` declaration.
  - `--screen f1|f2` clap arg; main dispatches on it.
  - `run_event_loop` renamed to `run_f2_event_loop`;
    `run_f1_event_loop` added.
  - `UserAction` gained `SelectNext` / `SelectPrev`;
    `key_action` recognises arrows + j/k; F2 match arms
    ignore navigation actions; +1 unit test for the
    new keymap.
  - `spawn_discovery_task` + `DiscoveryUpdate` enum.
  - `F1_DISCOVERY_INTERVAL` (5s) and
    `F1_DISCOVERY_SCAN_TIMEOUT` (1.5s) constants.

## Tests added

16 new unit tests:

In `daemons::tests`:
- `replace_from_discovery_sorts_by_instance_name`
- `replace_from_discovery_preserves_selection_on_rescan`
- `replace_from_discovery_clamps_selection_when_prior_row_disappears`
- `select_next_prev_bounded`
- `from_service_pulls_txt_keys`
- `from_service_handles_pre_3_2_daemon`
- `note_discovery_error_preserves_rows_and_sets_status`
- `empty_discovery_clamps_selected_to_zero`

In `screens::f1::tests`:
- `format_address_handles_zero_one_many`
- `format_module_count_distinguishes_zero_from_unknown`
- `format_delegation_renders_three_states`
- `format_since_picks_correct_unit`
- `detail_lines_label_unknown_delegation_for_pre_3_2_daemon`
- `detail_lines_shows_advertised_module_names`
- `detail_lines_falls_back_to_module_count_when_names_truncated`

In `main.rs::tests`:
- `key_action_maps_arrow_and_vim_navigation`
- Extended `key_action_returns_none_for_unmapped_keys`
  with `Char('J')` / `Char('K')` to pin case sensitivity.

37 blit-tui unit tests (was 21). Workspace passing serially.

## Known gaps

1. **No router yet.** The operator chooses the screen at
   process start via `--screen`. F1 ↔ F2 switching lands in
   a1-6 along with the routing UI for F3 / F4.

2. **Discovery task is not cancelled on F1 exit.** When the
   event loop returns Ok(()), the discovery task's mpsc
   Sender is dropped. The task exits on its next send. With
   a 5s interval that's at most ~6.5s of latency before the
   task notices. Harmless for normal exit (process is
   terminating anyway); a future tighter slice could plumb
   a CancellationToken.

3. **No mDNS departure events.** mdns-sd reports
   departures sporadically; the design's pragmatic
   alternative is "what each rescan returns is truth." A
   daemon that goes away mid-scan disappears on the next
   tick (~5s). Acceptable today — the F1 detail pane
   surfaces the last-scan-Xs-ago marker so operators can
   spot stale rows.

4. **No render test against TestBackend.** Same as a1-2
   gap. Format helpers + detail_lines are covered; the
   layout is visual.

5. **Detail block is read-only.** Design specifies
   `[enter] browse  [t] trigger transfer  [d] diagnostics`
   hotkeys; those are F3 / future-trigger / F4 entries
   handled by their own slices.

## Out of scope (next A.1 slices)

- **a1-4-f3-browse**: F3 Browse via List/Find/DiskUsage.
- **a1-5-f4-profile**: F4 reads `~/.config/blit/perf_local.jsonl`.
- **a1-6-screen-router**: F-keys to navigate between panes,
  replacing the `--screen` flag.

## Round 2 (sha filled by sentinel)

Reviewer raised three findings:

### 1. Local endpoint missing + GetState detail not wired (Medium)

The A.1 plan (TUI_DESIGN.md:938 + §10 owner-signoff) calls
for two F1 commitments:

1. `Local` appears as a first-class row so the operator
   can interact with the host machine even with no LAN
   daemons.
2. The detail block surfaces `GetState`-derived counters
   (uptime / active / version) per selected daemon.

Round 1 shipped neither. Round 2 splits the commitment:

- **Land in this round**: synthetic Local row (1).
- **Defer to a follow-up sentinel**: GetState-driven detail
  block (2) → new open finding `a1-3b-f1-getstate-detail`,
  added to `REVIEW.md`.

**Local row**: `daemons.rs` gains an `EndpointKind { Local,
Remote }` discriminator + `LOCAL_INSTANCE_NAME` constant.
`DaemonRow::local()` constructs the synthetic row;
`DaemonsState::new()` pre-populates it so the operator
sees Local before mDNS even returns. `replace_from_discovery`
re-injects Local at index 0 on every rescan — discovered
remotes follow sorted by name.

`screens/f1.rs` special-cases Local: the table cell shows
`(this machine)` in the address column and `—` in the
remaining columns to keep the layout stable. The detail
block emits a Local-specific header + a "(pending GetState
integration — a1-3b)" placeholder + a hint that downstream
F2/F3 slices treat it symmetrically with remote daemons.

The findings doc and `REVIEW.md` Open findings now list
the `a1-3b-f1-getstate-detail` follow-up explicitly.

### 2. Selected row moved out of the visible table (Low)

Round 1's `render_table` painted every row with an inline
style and no viewport offset. Past the first page of
daemons, the highlighted row scrolled off and only the
detail block reflected the off-screen selection.

Fix: switch to ratatui's `TableState` + stateful render.
`Table::row_highlight_style(...)` paints the cyan
highlight; `TableState::with_selected(Some(idx))` carries
the selected index AND maintains an offset for
auto-scrolling. The widget guarantees the highlighted row
stays in the viewport.

Verified by a new TestBackend test
(`selected_row_stays_visible_when_list_exceeds_viewport`):
20 discovered daemons + Local in a 12-line terminal,
cursor at index 15 — assert the daemon's name is in the
rendered buffer.

### 3. Selection clamp didn't match the documented behavior (Low)

Round 1's reducer used `unwrap_or(0)` when the prior
selected name disappeared, jumping back to row 0 even when
the prior index was still valid. With a 5s mDNS rescan
cadence and unreliable departure events, this triggered
frequently.

Fix: when the prior name is gone, fall back to
`min(prior_index, rows.len()-1)` so the cursor stays near
where the operator left it.

+2 regression tests in `daemons::tests`:
- `replace_from_discovery_keeps_index_near_prior_when_name_lost`:
  4 daemons → cursor on `charlie` (index 3) → rescan
  removes charlie → cursor on `delta` (index 3), not row 0.
- `replace_from_discovery_clamps_to_last_row_when_tail_disappears`:
  cursor on last row → rescan removes it → cursor on new
  last row.

### Files changed (round 2)

- `crates/blit-tui/src/daemons.rs`: `EndpointKind`,
  `LOCAL_INSTANCE_NAME`, `DaemonRow::local()` +
  `is_local()`, `DaemonsState::new()` seeds Local row,
  `replace_from_discovery` re-injects Local + new
  index-near-prior clamp.
- `crates/blit-tui/src/screens/f1.rs`: `daemon_to_row`
  + `detail_lines` special-case Local; `render_table`
  switches to `TableState` stateful widget; +3 tests
  (Local detail copy, Local placeholder cells, viewport
  visibility).
- `.review/findings/a1-3b-f1-getstate-detail.md` (new
  follow-up finding doc).
- `REVIEW.md`: adds `a1-3b-f1-getstate-detail` to Open
  findings.

### Tests

7 new unit tests (5 daemons + 3 screens):

In `daemons::tests`:
- `new_state_has_local_row`
- `replace_from_discovery_keeps_local_at_index_zero`
- `replace_from_discovery_preserves_local_selection_across_rescan`
- `replace_from_discovery_keeps_index_near_prior_when_name_lost`
- `replace_from_discovery_clamps_to_last_row_when_tail_disappears`
- `from_service_tags_row_as_remote`

In `screens::f1::tests`:
- `detail_lines_for_local_row_uses_local_specific_copy`
- `daemon_to_row_for_local_uses_placeholders`
- `selected_row_stays_visible_when_list_exceeds_viewport`

Updated tests to account for the always-present Local
row in row-count assertions (`replace_from_discovery_sorts_by_instance_name`
becomes `…_keeps_local_at_index_zero`, etc.). The
behaviour is preserved; the indices and counts shifted by
one to include Local.

44 blit-tui unit tests (was 37). Workspace passes serially.

## Reviewer comments

(empty — pending grade)
