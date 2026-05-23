# rec-3-tui-clear-recent: F2 "clear recent" action

**Severity**: Feature (recent-persistence, step 3 — final)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `00d2ba5`

## What

The operator-facing half of the recent-persistence feature. An `E` key
on F2 clears the recent-transfers list. Completes the chain: rec-1
persists recents, rec-2 added the `ClearRecent` RPC, rec-3 wires the TUI
to it.

Pressing `E` on F2 empties the local recent view **immediately** (for
responsiveness) and fans a `ClearRecent` RPC to **every watched daemon**
(F2 is multi-daemon) so the rows don't reappear on the next snapshot.
Active transfers are untouched, and — per the owner constraint — the
planner's `perf_local.jsonl` telemetry is never affected (`ClearRecent`
only touches the recents ring + `recents.jsonl`, proven by the rec-2
safety test).

## Approach

- **blit-app** (`admin/jobs.rs`): `clear_recent(remote) -> Result<u32>`
  client wrapper for the `ClearRecent` RPC, alongside the existing
  `cancel` / `query` / `subscribe`. Returns the count cleared; errors
  only on transport/unexpected status.
- **TUI**:
  - `UserAction::ClearRecent`.
  - `key_action` maps `E` (Shift+e) → `ClearRecent`. Capital chosen
    because lowercase `e` is `ProfileEnable` (F4); `E` was previously
    free. **Fixed key, not configurable** — the user didn't ask for
    remappability, and a fixed key avoids growing the
    `KeysDefaults::resolved()` collision matrix (the niche surface
    deferred in keys-4). It joins the other F2 command keys (`K`, `X`)
    which are likewise fixed.
  - F2 `handle_pane_action`: `TransfersState::clear_recent(now)` empties
    the local recent `VecDeque` (active rows untouched; `last_event_at`
    bumped only when something was removed), then `spawn_clear_recent`
    per `f2_watched_endpoints` — mirroring the m2f-8 batch-cancel
    fan-out (one RPC per watched daemon).
  - `spawn_clear_recent`: fire-and-forget `ClearRecent` task.
  - F2 footer gains an `E clear recent` hint (+ module-doc footer line).

## Why fire-and-forget

Recents are a **view**, not data, and are cleared locally before the
RPCs spawn. So a transiently-unreachable daemon must not block clearing
the others, and there's no per-daemon outcome worth a status fragment
(unlike `CancelJob`, where each transfer's cancel result matters). Worst
case on a failed daemon clear: its old rows reappear on a later manual
refresh, and the operator presses `E` again. This keeps rec-3 a bounded,
single-purpose slice. (If error surfacing is wanted, it's a clean
follow-up — but it would pull in the F2CancelStatus-style
status/TTL machinery, which felt disproportionate here.)

## Files changed

- `crates/blit-app/src/admin/jobs.rs`: `clear_recent` client +
  `ClearRecentRequest` import.
- `crates/blit-tui/src/state.rs`: `TransfersState::clear_recent` + tests.
- `crates/blit-tui/src/main.rs`: `UserAction::ClearRecent`; `E` binding;
  F2 dispatch; `spawn_clear_recent`; tests; updated the d-1 profile-keys
  test (E is no longer unmapped).
- `crates/blit-tui/src/screens/f2.rs`: footer hint + doc.

## Tests

`blit-tui` 622 (was 619; +3):

- `state::clear_recent_empties_recent_keeps_active` — clears the recent
  view, returns the count, leaves active transfers intact.
- `state::clear_recent_empty_is_zero_and_leaves_last_event_unset` — a
  no-op clear returns 0 and doesn't fabricate footer activity.
- `key_action_maps_shift_e_to_clear_recent` — `E` → `ClearRecent`,
  lowercase `e` still `ProfileEnable` (case-sensitive).

The daemon-side guarantees (ring + `recents.jsonl` cleared,
`perf_local.jsonl` untouched) are covered by rec-2's
`clear_recent_empties_store_but_not_perf_local`. The blit-app
`clear_recent` wrapper is a thin RPC call (like `cancel`), exercised
end-to-end at runtime; no unit test stands up a tonic server for it,
matching the existing `cancel`/`query` convention.

## Scope

Final slice of the feature. After this verifies, recent-persistence +
clear-recent is complete: recents survive restarts (rec-1), a daemon RPC
clears them without touching planner telemetry (rec-2), and the TUI
exposes it on F2 (rec-3). Next step is opening a PR for `phase5/a1` —
pending the user's go-ahead (no push without it).

## Reviewer comments

(empty — pending review)
