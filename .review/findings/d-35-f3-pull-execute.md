# d-35-f3-pull-execute: F3 pull destination prompt + execution

**Severity**: Feature (F3 transfer-from-cursor, slice 3 — completes the feature)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Completes F3 transfer-from-cursor. d-33/d-34 surfaced
the pull-source preview; d-35 makes it actionable.
Pressing `p` on a selectable F3 row opens a destination
prompt; the operator types a local path; `Enter` runs a
remote→local PullSync owned by the TUI process (the
daemon streams bytes to the TUI, like the CLI's
`blit <remote> <local>` pull). The F3 footer shows the
prompt → progress → outcome.

```
Pull: nas:/photos/2024/img001.jpg        (Stats, from d-34)
...
loaded · 3s ago · pull → /tmp/photos_         ← typing (cyan)
loaded · 3s ago · pulling → /tmp/photos...    ← Running (yellow)
loaded · 3s ago · pulled 12 file(s) · 4.2 MiB → /tmp/photos   ← Done (green)
loaded · 3s ago · pull failed: connection refused            ← Error (red)
```

It's a TUI-owned transfer (not a daemon job), so it does
NOT appear on F2 — same ownership model as F4's local
transfers.

## Approach

### State machine (new `f3pull` module)

`F3PullState` mirrors F4's `TransferState` shape:

```
Idle
 └─ p ─────────► EnteringDest { source, dest }
                   │ chars/Backspace edit `dest`
                   │ Esc → Idle
                   │ Enter (non-empty) ─► Running { dest, request_id }
                                            │ reply ─► Done { dest, files, bytes }
                                            └ reply ─► Error { message }
```

`begin_run` moves the `source` endpoint out (via
`mem::replace`, no clone) into a `PullLaunch` and bumps a
monotonic `request_id`. `apply_done` / `apply_error` are
generation-guarded — a stale reply (id ≠ current
`Running`) is dropped.

### Execution

`spawn_f3_pull` runs `blit_app::transfers::remote::run_pull_sync`
(the same entry the CLI uses) on a tokio task with
default `PullSyncOptions` — no mirror, no filter, no
progress monitor. A non-mirror pull needs only
`run_pull_sync` (the mirror-purge half is a no-op when
`mirror_mode = false`, so it's skipped). The flattened
`(files, bytes)` / error lands on `f3_pull_reply_tx` for
the select! loop to apply.

### Input routing

`p` (not editing) → `UserAction::F3PullBegin`, which
derives the source via `browse::pull_source_endpoint`
and opens the prompt. While editing,
`handle_f3_pull_keystroke` (mirroring
`handle_f3_filter_keystroke`) absorbs chars / Backspace /
Esc / Enter, bubbling through for Ctrl-c / F-keys / `?`.
The router intercepts it before the action dispatcher,
just like the d-26 filter prompt.

### Render

New `F3PullDisplay` enum in `screens/f3.rs`, bridged from
`F3PullStatus` by `main.rs::f3_pull_to_display` (keeps
the screens layer off the `f3pull` internals). The
footer renders the prompt/progress/outcome fragment and
gains a `p pull` hint.

### Help overlay

New row `p  pull selected → local dir (F3)`. Modal height
36 → 37; keymap grep test gains `p`.

## Files changed

- `crates/blit-tui/src/f3pull.rs` (new): `F3PullState`
  state machine + `PullLaunch` + 14 unit tests.
- `crates/blit-tui/src/main.rs`:
  - `mod f3pull;`
  - AppState gains `f3_pull` + `f3_pull_reply_tx`.
  - `F3PullReply` + `spawn_f3_pull` + `f3_pull_to_display`.
  - `UserAction::F3PullBegin` + `p` keymap + F3 dispatch.
  - `handle_f3_pull_keystroke` + router interception.
  - select! reply arm.
  - F3 render passes the pull display.
  - 8 keymap/handler tests + AppState test fixtures.
- `crates/blit-tui/src/screens/f3.rs`:
  - `F3PullDisplay` enum; footer renders the fragment +
    `p pull` hint; module-doc updated.
- `crates/blit-tui/src/help.rs`:
  - `p` row; modal 36 → 37; grep test + clamp test.

## Tests

+22 tests (354 → 376):

**`f3pull::tests` (14):** state machine — idle start,
prompt open, char/backspace edit, cancel, empty/whitespace
dest keeps prompt, launch+running transition, dest
trimming, done/error terminal states, stale-reply drop,
monotonic request ids, begin-noop-while-running.

**`main::tests` (8):** `p` keymap; pull keystroke handler
routes chars/Backspace/Esc/Enter; Enter-on-empty keeps
prompt; bubbles through for F-keys / `?` / Ctrl-c.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No live byte progress.** `spawn_f3_pull` passes
   `None` for the progress monitor, so the footer shows
   `pulling...` without a byte counter. A future polish
   could wire a `RemoteTransferProgress` channel like the
   CLI's `spawn_progress_monitor`.

2. **No auto-hide TTL on Done/Error.** Unlike the F2
   cancel fragment (d-23), the pull outcome persists
   until the next `p` or pane navigation. Acceptable
   (the operator sees the result), but a future polish
   could add a TTL.

3. **Default options only.** No mirror, no
   filter, no checksum, no resume from the TUI. The
   CLI remains the surface for those flags; the F3 pull
   is the "grab this path to here" quick action.

4. **Could not validate the pull end-to-end here.** The
   state machine + wiring are unit-tested, but the
   actual `run_pull_sync` round-trip needs a live daemon
   — not exercised in CI. The execution path reuses the
   CLI's verified entry point with default options.

## Out of scope

- Push (local→remote) from F3 — pull is the natural
  F3-cursor direction.
- Live progress / TTL / option flags (future polish).

## Reviewer comments

(empty — pending grade)
