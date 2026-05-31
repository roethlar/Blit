# d-43-du-cache: cache F3 subtree totals for re-entry

**Severity**: Feature (polish — closes d-41 known gap #2)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `617733b`

## What

TUI_DESIGN §5.3 says du data is "cached for re-entry." d-41
shipped the `u` subtree-total query but re-ran the RPC every
time — pressing `u` twice on a row, or returning to a row queried
earlier, paid a fresh round-trip. d-43 adds a per-path cache so a
known total is served instantly.

## Approach

`F3DuState` gains a `cache: HashMap<String, (u64, u64)>` keyed by
the canonical cursor spec (the same `path` string the render
bridge already gates on). `begin` now returns a `DuBegin` enum:

- **`DuBegin::Cached`** — the path's total is known; status goes
  straight to `Done` and no RPC is spawned.
- **`DuBegin::Fetch(id)`** — cache miss; status goes to `Running`
  and the caller spawns the RPC stamped with `id` (unchanged
  from d-41).

`apply_done` populates the cache when it accepts a reply. A cache
hit does **not** bump `request_seq` (it consumes no request id),
so generation-guarding of in-flight fetches is unaffected.

The dispatch arm changes from "always spawn" to "spawn only on
`Fetch`":

```rust
if let f3du::DuBegin::Fetch(id) = app.f3_du.begin(target.display()) {
    spawn_f3_du(id, target, app.f3_du_reply_tx.clone());
}
```

The render bridge and path-match gating are unchanged — a cached
`Done` renders exactly like a freshly-fetched one.

## Files changed

- `crates/blit-tui/src/f3du.rs`:
  - `DuBegin` enum; `cache` field; `begin` returns `DuBegin`;
    `apply_done` populates the cache.
  - 3 new tests + existing tests adapted to the new return via a
    `fetch_id` helper.
- `crates/blit-tui/src/main.rs`:
  - F3 du dispatch arm spawns only on `DuBegin::Fetch`.

## Tests

+3 tests (433 → 436):

- `begin_serves_cached_total_without_fetching` — query a path,
  apply the reply, re-`begin` the same path → `DuBegin::Cached`,
  status is `Done` with the cached total.
- `begin_fetches_for_uncached_path_even_after_a_cache_hit` — a
  different path still fetches.
- `cached_begin_does_not_consume_a_request_id` — a cache hit
  doesn't burn a generation id (the next real fetch reuses the
  next sequential id).

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

## Known gaps

1. **Cache is never invalidated within a session.** du totals are
   treated as stable for the life of the `F3DuState`. If files
   change on the remote mid-session, a cached total goes stale
   until the TUI restarts. Acceptable: du is an at-a-glance
   estimate, not a live gauge, and a stale-by-minutes figure is
   the same tradeoff `blit du` makes. A future slice could clear
   the cache on a browse refetch (`r`) if liveness matters.

2. **Unbounded cache growth.** One entry per distinct path
   queried. In practice an operator queries a handful of rows per
   session; the entries are tiny `(u64, u64)` tuples. Not worth an
   LRU bound at this scale.

## Out of scope

- Cache invalidation on browse refetch.
- LRU / size-bounded cache.

## Reviewer comments

(empty — pending grade)
