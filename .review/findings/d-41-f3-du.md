# d-41-f3-du: subtree disk-usage on the F3 cursor

**Severity**: Feature (designed — TUI_DESIGN §5.3 "subtree" line)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `d804f22`

## What

TUI_DESIGN §5.3 specifies an F3 Stats line showing the subtree
size for the cursor row ("subtree: 14.2 GiB across 8,442 files")
and a `du` hotkey. The Stats block had the Selected / View /
Pull lines (a1-4, d-33) but no du. d-41 wires the daemon's
existing `DiskUsage` RPC to a `u` hotkey and renders the result:

```
┌─ Stats ──────────────────────────────────────────┐
│ Selected: photos/ · dir · —                       │
│ View: home/photos · 12 entries                    │
│ Pull: nas:/home/photos                            │
│ Subtree: 14.2 GiB across 8,442 files   ← d-41 `u` │
└───────────────────────────────────────────────────┘
```

## Approach

### RPC plumbing (mirrors the F3 pull pattern)

`blit_app::admin::du::stream` already exists and is documented
for TUI use ("the TUI forwards to an event channel"). `u`
resolves the cursor's `RemoteEndpoint` (the same
`browse::pull_source_endpoint` the pull preview uses), then
`spawn_f3_du` runs the RPC with `max_depth = 0` — the daemon
streams a single aggregate root entry — and posts an
`F3DuReply { request_id, result: Result<(bytes, files), String> }`
back to the event loop.

The accumulation is a pure, unit-tested
`du_total_from_entries(acc, bytes, files)` that keeps the
max-byte entry. With `max_depth = 0` there's only the root row,
but folding by max-bytes is robust if the daemon ever emits
children too (the root subtree contains every child, so it's
always the largest). This is the same "extract the fold so it's
testable" lesson from d-37 R2 / d-39.

### State (`f3du.rs`)

`F3DuState` mirrors the F3 pull machine: `Idle` / `Running` /
`Done` / `Error`, generation-guarded by `request_id` so pressing
`u` again (or on another row) supersedes an in-flight query and
the stale reply is dropped. Every non-`Idle` variant carries the
`path` it pertains to.

### Path-bound rendering — no TTL

The du result is tied to the path it was computed for. The
bridge `f3_du_to_display(status, current_path)` returns `Hidden`
unless the status path equals the cursor's current canonical
spec. So moving the cursor hides a now-stale total *without any
timer or explicit clearing* — a `Done` for `home/photos` simply
stops rendering once the operator navigates away, and reappears
if they return. This is simpler than the d-38 TTL machinery
because the staleness signal (cursor position) is already
available at render time.

### Key choice: `u`, not `d`

TUI_DESIGN §5.3 lists `d` for du. But `key_action` is a global
key→action map (one `UserAction` per key regardless of pane),
and `d` is already `ProfileDisable` on F4. Making `key_action`
screen-aware would be a broad refactor with wide test churn. The
codebase already set a precedent for exactly this conflict — it
rebound diagnostics dump from the design's `d` to `s`(napshot)
and documented why (see the `KeyCode::Char('s')` comment). d-41
follows that precedent: `u`(sage). While the F3 filter or
pull-dest prompt is open, the text-input handlers absorb `u`
first, so it only triggers du in normal F3 nav mode.

## Files changed

- `crates/blit-tui/src/f3du.rs` (new): `F3DuState` machine +
  8 unit tests.
- `crates/blit-tui/src/main.rs`:
  - `mod f3du`; AppState `f3_du` + `f3_du_reply_tx` + channel.
  - `F3DuReply`, `spawn_f3_du`, `run_f3_du_total`, pure
    `du_total_from_entries`.
  - `f3_du_to_display` bridge (path-match gating).
  - `UserAction::F3DuBegin`, F3 dispatch arm, `u` key mapping.
  - du reply select arm.
  - 5 test-fixture AppState constructors updated.
  - 7 new tests (key map, accumulator ×3, bridge gating ×3).
- `crates/blit-tui/src/screens/f3.rs`:
  - `F3DuDisplay` enum; Stats block grew 5→6 rows for the
    `Subtree:` line; module-doc layout sketch updated.
- `crates/blit-tui/src/help.rs`:
  - `u` keymap row; modal height 38→39; keymap test asserts
    the new row.

## Tests

+15 tests (413 → 428):

- `f3du::tests` (8): idle/begin/generation-bump, apply_done /
  apply_error with path preservation, stale-request drop, idle
  no-op, second-begin-supersedes.
- `main::tests` (7): `key_action` maps `u` → F3DuBegin;
  `du_total_from_entries` keeps the max-byte entry (alone /
  larger-replaces / equal-keeps-existing); `f3_du_to_display`
  shows Done only for the matching path, Idle always hidden,
  Running/Error gate on path.

The RPC streaming + spawn is exercised manually (needs a live
daemon emitting `DiskUsage` rows); the pure fold and the state
transitions are fully unit-tested.

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

## Known gaps

1. **Aggregate only, no per-child breakdown.** `max_depth = 0`
   gives the subtree total, matching the design's single-line
   Stats display. A drill-down du (per-child sizes) would be a
   separate, larger feature (its own pane or modal).

2. **No caching across re-entry.** TUI_DESIGN §5.3 mentions
   "cached for re-entry"; d-41 re-queries on each `u`. Caching
   is a clean follow-on (key the cache by canonical path), kept
   out to stay atomic. Re-querying is cheap and always correct.

3. **No spinner cadence.** While `Running`, the Stats line shows
   a static "computing..." — the reply arrives via the channel
   and wakes the loop directly, so no `needs_live_tick` coupling
   is needed (same as the d-37 pull progress).

## Out of scope

- Per-child du drill-down.
- du result caching.
- Making `key_action` screen-aware to honor the design's `d`.

## Reviewer comments

(empty — pending grade)
