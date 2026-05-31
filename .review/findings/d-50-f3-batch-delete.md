# d-50-f3-batch-delete: D deletes the marked set

**Severity**: Feature (designed — TUI_DESIGN §5.3 batch delete)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `1349cfb`

## What

The consumer of d-49's `space` multi-select. `D` now deletes the
**marked set** in a single `Purge` (or the cursor row when
nothing is marked — the d-45 behavior, unchanged). The confirm
prompt shows `delete N item(s)? y/N` for a batch.

## Approach

### Delete machine: one path → many

`f3del::F3DelStatus` now carries `rel_paths: Vec<String>` + a
`label` + a `gate_path: Option<String>`:

- **Single** (no marks): `rel_paths = [cursor rel]`, `label =
  path spec`, `gate_path = Some(spec)` — the d-45 path-gated
  outcome (hides once the cursor leaves the path) is preserved
  exactly.
- **Batch**: `rel_paths = marked rels`, `label = "N item(s)"`,
  `gate_path = None` — the outcome shows until the next action
  (the marked rows are gone after the post-delete refresh
  anyway, so path-gating doesn't apply).

`confirm()` hands `(module_endpoint, rel_paths)` to one
`spawn_f3_del` → one `rm::purge(module, rel_paths)`. All targets
share a module (they come from one F3 view).

### Resolution + shaping (pure, tested)

- `browse::marked_endpoints(base)` resolves the marked rows to
  endpoints (reusing `pull_source_endpoint` per row, in display
  order).
- `build_delete_request(endpoints, batch)` filters out
  non-deletable targets (module roots — the d-45 guard), converts
  each to a canonical forward-slash wire rel-path (d-45 R2's
  `del_wire_path`), and shapes the `(module_endpoint, rel_paths,
  label, gate_path)` for single vs batch. Returns `None` when
  nothing is deletable.

### Dispatch + safety

`D` uses the marked set when `marked_count() > 0`, else the
cursor. The d-46 read-only gate still blocks the prompt up front;
the daemon's read-only + containment enforcement remains the
backstop. The d-45-R2 post-delete browse refresh still fires
(only on an applied reply), clearing the now-deleted rows — which
also clears the marks (a re-fetch resets the view).

## Files changed

- `crates/blit-tui/src/browse.rs`: `marked_endpoints`.
- `crates/blit-tui/src/f3del.rs`: `Vec` rel-paths + `label` +
  `gate_path` across the status variants + `begin`/`confirm`/
  `apply_*`; module doc; tests reworked for the new API + a batch
  test.
- `crates/blit-tui/src/main.rs`: `build_delete_request`;
  `spawn_f3_del`/`run_f3_del` take `Vec`; dispatch single/batch;
  bridge gates on `gate_path`; tests.
- `crates/blit-tui/src/screens/f3.rs`: `F3DelDisplay` carries
  `label`; footer shows `delete <label>? y/N` / `deleted <label>
  — N file(s)`.
- `crates/blit-tui/src/help.rs`: `D` row reworded to "cursor row
  or marked set".

## Tests

Net +~7 (466 → 473): `f3del` batch confirm + empty-rel-paths
no-op + reworked single tests; `main` `build_delete_request`
single/batch/module-root-filter + the bridge's single-gated /
batch-ungated outcome test.

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

## Known gaps

1. **No batch transfer (`c`/`m`/`v`) yet.** d-50 wires the marked
   set to delete only; batch copy/mirror/move over the selection
   is the next consumer (needs the transfer-options destination
   flow).
2. **Marked rows must share one module.** They always do today
   (marks are view-scoped, cleared on any view change), so the
   single-module `Purge` is correct. If cross-view selection ever
   lands, the batch would need grouping by module.
3. **Batch outcome has no auto-hide.** `gate_path = None` shows
   the "deleted N" line until the next action. The post-delete
   refresh changes the row set immediately, so it's brief in
   practice; a TTL (d-38 style) could be added if needed.

## Out of scope

- Batch `c`/`m`/`v` transfer over the marked set.
- Cross-module batch grouping.

## Reviewer comments

### Round 1 verdict — reopened (`.review/results/d-50-f3-batch-delete.reopened.md`)

One finding:

- **Batch Done/Error footer persisted indefinitely.** Batch sets
  `gate_path = None`, which the bridge treats as always-visible
  — but nothing ever cleared a batch terminal outcome, so
  `deleted N item(s)` stayed on screen through Down / filter /
  refresh / navigation. Single-row deletes self-hide on cursor
  move; batch had no equivalent.

### Round 2 fix

Added the d-38 pull-TTL pattern, scoped to batch outcomes:

- `F3DelStatus::Done`/`Error` carry `finished_at` (stamped by
  `apply_*`, which now take an `at: Instant`).
- `clear_terminal_if_expired(now, ttl)` sweeps **only**
  `gate_path = None` (batch) terminals to `Idle` after the TTL;
  single-row (`gate_path = Some`) outcomes are left to their
  cursor-move path gate, untouched.
- `is_batch_terminal()` gates `needs_live_tick`, so the loop
  wakes to expire a batch outcome (single outcomes are
  event-cleared and don't need ticking).
- `BATCH_TERMINAL_TTL = 5s` (the d-38 fixed baseline); the loop
  calls the clear each frame next to the pull-TTL clear.

### Round 2 tests

+2 tests (473 → 475):

- `batch_outcome_auto_hides_after_ttl` — batch Done stays within
  the TTL, clears to Idle at/after it; `is_batch_terminal`
  flips false.
- `single_outcome_is_not_swept_by_batch_ttl` — a single-row
  outcome survives the TTL sweep (relies on the path gate).

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

### Lesson restated

A "shows until the next action" outcome needs a concrete clearing
mechanism — there has to be an *action* that actually clears it.
Single delete had one (cursor-move path gate); batch's
`gate_path = None` removed the gate without substituting a timer,
so "until the next action" was really "forever." The d-38 TTL is
the substitute.
