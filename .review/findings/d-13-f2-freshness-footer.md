# d-13-f2-freshness-footer: F2 footer surfaces last-event age

**Severity**: Feature (closes the d-11 known gap —
"F2 still doesn't tick")
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

F2's footer now shows "last event Xs ago" alongside
the connection-status banner:

```
live  ·  last event 3s ago  ·  q/Esc quit  ·  r refresh
degraded: stream broken  ·  last event 47s ago  ·  q/Esc quit  ·  r refresh
```

Pre-d-13 F2 was the only pane without a freshness
indicator. Operators looking at a stale "live" footer
had no way to tell whether the daemon was still
actively reporting transfers or whether the Subscribe
stream had silently quiesced. The new line ticks every
500ms via the d-9 live-tick gate, which d-13 extends to
F2 as well.

## Approach

### State

New field on `TransfersState`:

```rust
last_event_at: Option<Instant>,
```

Stamped on:

1. **`replace_from_snapshot(state, fetched_at)`** —
   signature gained `fetched_at: Instant`. Records it as
   the last_event_at so the footer ticks against the
   snapshot's reconcile time.
2. **`apply_event(event, now)`** — signature gained
   `now: Instant`. The function delegates to a private
   `apply_event_inner` and only stamps `last_event_at`
   on mutation (no-op events for unknown IDs leave the
   timestamp alone, so a stale terminal-id event doesn't
   refresh the footer).

### Render

`render_into` gains `now: Instant`. `render_footer`
takes `last_event_at` and a local `format_since`
matches the helper already used by F1/F3 (seconds /
minutes / hours).

The line only renders when `last_event_at` is `Some` —
NoRemote / Connecting pre-first-event states still
show just the status banner + key hints.

### Live-tick gate

`needs_live_tick` extends to F2:

```rust
Screen::F2 => app.transfers.last_event_at().is_some(),
```

Once the first Subscribe event or GetState snapshot
lands, F2's footer ticks every 500ms until a pane
change or process exit.

## Files changed

- `crates/blit-tui/src/state.rs`:
  - `last_event_at: Option<Instant>` field +
    accessor.
  - `replace_from_snapshot(state, fetched_at)` —
    signature change; stamps `last_event_at`.
  - `apply_event(event, now)` — signature change;
    delegates to `apply_event_inner`; stamps only on
    mutation.
- `crates/blit-tui/src/screens/f2.rs`:
  - `render_into` gains `now: Instant`.
  - New `format_since` helper (same shape as F1/F3).
  - `render_footer` shows "last event Xs ago" when
    `last_event_at` is `Some`.
- `crates/blit-tui/src/main.rs`:
  - F2 render call site passes `now`.
  - `needs_live_tick` F2 arm reads
    `app.transfers.last_event_at().is_some()`.
  - Existing F2 test comment + assertion updated to
    explain the d-13 contract.
  - New positive test for F2 ticking after a snapshot.
  - `apply_event` / `replace_from_snapshot` call sites
    in the router thread `Instant::now()` through.

## Tests

+3 unit tests (179 → 182):

In `state::tests`:
- `last_event_at_none_until_first_mutation`
- `replace_from_snapshot_stamps_last_event_at`
- `apply_event_stamps_only_on_mutation` — Started for
  new id stamps; Progress for unknown id does NOT
  advance the timestamp.

The existing `needs_live_tick_covers_per_pane_freshness_footers`
test was extended (not a new test, but new assertions)
to cover the F2 case both pre-event (false) and
post-snapshot (true).

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No per-row "started Xs ago" in the Active
   table.** F2's active rows still show only the
   transfer_id / kind / peer / module path. A future
   polish slice could add a started-age column or
   surface throughput-since-start. Out of scope here.

2. **last_event_at advances on snapshot reconcile.** If
   the stream degrades and we fall back to periodic
   GetState, the footer ticks from each reconcile, not
   from the last live event. That's a deliberate choice
   — the operator gets "system is still polling" signal
   in the degraded state — but a "last LIVE event"
   variant could be added if the distinction matters.

## Out of scope (next slices)

- **e-3 themes / config** — `~/.config/blit/tui.toml`.
- **Per-row freshness in the F2 active table.**
- **Per-file progress** events during local transfers.

## Reviewer comments

(empty — pending grade)
