# a1-3b-f1-getstate-detail reopened

Reviewed sha: `c6817654278f420c9243749ac2971019cf1ee385`
Reviewed at: 2026-05-18T05:03:52Z
Reviewer: reviewer
Verdict: reopened

## Findings

### 1. The per-row detail cache is not used to avoid re-fetching

Severity: Medium

Location: `crates/blit-tui/src/main.rs:524`

The finding scope says `DaemonsState` caches the most recent `DaemonState` per `instance_name` so cursor flicks do not re-query each time. The state layer stores the cache, but `maybe_kick_detail_fetch` ignores it. Any time the operator leaves a row and later returns to it, `last_fetched` differs from the selected name, so the code replaces the existing `Loaded` detail with `Pending` and spawns another `GetState`.

That breaks the stated cache contract and causes unnecessary RPCs plus visible flicker from loaded detail back to "fetching..." on every revisit. It also makes the "details survive discovery rescan" test weaker than the behavior the UI actually needs: the detail survives in the map, but the fetcher immediately overwrites it on selection revisit.

Please make the non-refresh path consult `state.detail_for(&name)`: if a detail already exists, keep rendering it and just update `last_fetched` without spawning. The `r` key can remain the explicit invalidation path. Add a focused test for selecting A → selecting B → selecting A again without `r`, asserting A does not go back to `Pending` or spawn another fetch.

### 2. Older same-row fetch replies can overwrite newer refresh results

Severity: Medium

Location: `crates/blit-tui/src/main.rs:494`

Detail updates are tagged only with `instance_name`, and the apply arm writes every result into the cache. That is safe for "selected row changed" staleness because the renderer reads the current row's key, but it is not safe for overlapping fetches of the same row.

One concrete path:

1. Select `mycroft`; fetch #1 starts.
2. Press `r`; `last_fetched = None`, fetch #2 starts for the same `instance_name`.
3. Fetch #2 returns first and renders fresh data.
4. Fetch #1 returns later and overwrites the fresh result with stale data, or with an old error.

The same can happen when moving away from a row and back before the first fetch finishes. The code comment says stale responses can be distinguished, but there is no request generation/token to distinguish same-row generations.

Please tag each `DetailUpdate` with a monotonically increasing generation/request id per row and only apply the latest generation for that row. A small reducer-style helper test is enough; it does not need a live daemon.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed.
- `cargo test --workspace` passed.
