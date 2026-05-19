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

### Round 1 verdict — reopened (`.review/results/d-35-f3-pull-execute.reopened.md`)

One finding:

- **The F3 dest skipped `resolve_destination`.** d-35
  round 1 passed the raw typed path straight to
  `PullSyncExecution.dest_root`. But `run_pull_sync`
  treats `dest_root` as already-resolved — a single-file
  pull expects the final FILE path, and a directory pull
  into an existing local dir must nest under the source
  basename. Without the CLI's `resolve_destination`
  step: "pull a file into an existing dir" tried to
  create the dir itself as the output file, and "pull a
  dir into an existing dir" merged its contents instead
  of nesting. The semantics are pinned by the CLI's
  `remote_pull_subpath.rs` tests.

### Round 2 fix

`begin_run` now applies `resolve_destination` (the same
`blit_app::transfers::resolution` entry the CLI uses)
before producing the `PullLaunch.dest_root`:

```rust
let raw_source = source.display();
let resolved = resolve_destination(
    &raw_source,
    &raw_dest,
    &Endpoint::Remote(source.clone()),
    Endpoint::Local(PathBuf::from(&raw_dest)),
);
let dest_root = match resolved {
    Endpoint::Local(p) => p,
    Endpoint::Remote(_) => PathBuf::from(&raw_dest), // unreachable: Local stays Local
};
```

Semantics now match the CLI:
- non-container dest (no trailing slash, doesn't exist)
  → used as-is (exact target / rename);
- trailing-slash or existing-dir dest → nest under the
  source basename (`dir` → `dest/dir`, `file.txt` →
  `dest/file.txt`).

The d-34 source `display()` drops the trailing slash for
directories, so a dir pull is always "the dir itself"
(non-contents) → nests under basename, never merges.

The existing begin_run tests use non-container dests
(non-existent, no trailing slash), so they're unchanged.

### Round 2 file changes

- `crates/blit-tui/src/f3pull.rs`:
  - `begin_run` resolves the dest via
    `resolve_destination`.
  - Imports `blit_app::endpoints::Endpoint` +
    `resolution::resolve_destination`.
  - 5 new resolution tests.

### Round 2 tests

+5 tests (376 → 381):

- `resolve_non_container_dest_used_as_is`.
- `resolve_trailing_slash_dest_nests_under_basename` —
  dir `2024` into `/x/` → `/x/2024`.
- `resolve_file_into_container_appends_filename` —
  `readme.txt` into `/x/` → `/x/readme.txt` (the final
  file path).
- `resolve_existing_dir_dest_nests_under_basename` —
  uses a real tempdir; dir nests under basename.
- `resolve_file_into_existing_dir_appends_filename` —
  tempdir; file → `<dir>/readme.txt`.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

### Lesson restated

A library entry point that documents "treats this arg as
already resolved" means the caller owns the resolution.
The CLI and TUI are now both callers of
`run_pull_sync`; both must run `resolve_destination`
first. Reusing a shared execution function doesn't mean
reusing only the function — it means reusing the whole
call contract, pre-steps included.
