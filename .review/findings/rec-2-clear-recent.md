# rec-2-clear-recent: `ClearRecent` RPC — wipe recents without touching planner telemetry

**Severity**: Feature (recent-persistence, step 2)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `9c2955e`

## What

Second slice of the recent-persistence feature. Adds a `ClearRecent`
RPC so the TUI (rec-3) can wipe the `GetState.recent[]` list. Clears
both the in-memory recent-runs ring and its persisted backing store
(`recents.jsonl`, from rec-1).

**Core safety property (owner constraint):** clearing recents must
**never** touch the planner/predictor's historical telemetry
(`perf_local.jsonl`). `ClearRecent` only ever references the recents
ring and `recents.jsonl`; `perf_local.jsonl` is a separate store and is
left byte-for-byte intact. A test asserts exactly this.

## Approach

- **proto** (`proto/blit.proto`): `rpc ClearRecent(ClearRecentRequest)
  returns (ClearRecentResponse)` next to `CancelJob`; empty
  `ClearRecentRequest` (operator clears the whole list);
  `ClearRecentResponse { uint32 cleared }` (count removed). Regenerated
  at build time via `tonic-build` (`generated.rs` is not tracked).
- **`ActiveJobs::clear_recent() -> usize`**: locks the ring, records its
  length, clears it; then signals the rec-1 persistence writer
  (`OnceLock<Sender>`) so it reads the now-empty ring and atomically
  rewrites `recents.jsonl` empty. The signal is non-blocking and a no-op
  when persistence isn't armed (test/default tables). **Never references
  `perf_local.jsonl`.**
- **daemon handler** (`service/core.rs`): `clear_recent` returns the
  count. Empty request, so no validation; clearing an empty list is a
  well-defined no-op (returns 0 — idempotent).
- **trait-impl ripple**: adding an RPC adds a `Blit` trait method, so
  the three hand-written test mocks
  (`blit-core/tests/pull_sync_with_spec_wire.rs` `SpyServer`;
  `blit-cli/tests/remote_remote.rs` `UnimplementedBlit` +
  `RejectingPullSyncBlit`) each gained a `clear_recent` stub matching
  their existing style (`unimplemented!()` / `Status::unimplemented`).

## Why this design

- **Clear via the rec-1 writer, not a separate file-delete path**:
  reuses the established atomic-rewrite mechanism (empty ring → empty
  file), so there's one code path that owns `recents.jsonl` and no
  blocking file I/O in the RPC handler. The in-memory clear is
  synchronous, so `GetState.recent[]` reflects the clear immediately;
  the on-disk flush follows asynchronously (durability, not
  correctness of the live view).
- **Empty request**: clearing is all-or-nothing for the operator's
  view; no per-id selectivity needed (and per-id would invite confusion
  with `CancelJob`, which targets *active* transfers).

## Files changed

- `proto/blit.proto`: `ClearRecent` rpc + messages.
- `crates/blit-daemon/src/active_jobs.rs`: `clear_recent()` + tests.
- `crates/blit-daemon/src/service/core.rs`: handler + import + test.
- `crates/blit-core/tests/pull_sync_with_spec_wire.rs`,
  `crates/blit-cli/tests/remote_remote.rs`: trait-mock stubs.

## Tests

`blit-daemon` 140 (was 137; +3):

- `active_jobs::clear_recent_empties_store_but_not_perf_local` — the
  **core safety test**: with persistence armed and a sibling
  `perf_local.jsonl` present, a recorded completion lands in the ring +
  store; `clear_recent` empties the ring (synchronously) and the store
  (polled async flush), and `perf_local.jsonl` is asserted byte-for-byte
  unchanged.
- `active_jobs::clear_recent_unarmed_empties_ring` — clears the ring +
  returns the count with no persistence armed (no panic, no file).
- `service::core::clear_recent_empties_recent_and_reports_count` — RPC
  returns the count, `GetState.recent[]` is empty afterward, and a
  second clear returns 0 (idempotent).

## Scope / next

rec-2 is daemon + proto only. Next:
- **rec-3**: TUI F2 "clear recent" action — `UserAction::ClearRecent`, a
  key binding wired through `KeysDefaults::resolved()`'s collision
  policy (+ warn + tests, per the keymap-collisions memory), an F2
  footer hint, and dispatch → spawn the `ClearRecent` RPC (mirroring the
  existing `spawn_cancel_transfer`). After rec-3 verifies, the feature
  is complete.

## Reviewer comments

(empty — pending review)
