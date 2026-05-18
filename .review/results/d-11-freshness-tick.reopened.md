# d-11-freshness-tick reopened

Reviewed sha: `eaf5881c4821807ef02133cf6c07557f8999dbb6`

Verdict: reopened

## Findings

### Low — F1 loaded detail timestamps still freeze after discovery degrades

`crates/blit-tui/src/main.rs:1414` only enables the live tick on F1 while `DiscoveryStatus::Live`. But F1 can still render a time-dependent detail line while discovery is degraded: `DaemonsState::note_discovery_error` preserves the existing row set and cached details, and `screens/f1.rs:241` / `screens/f1.rs:355` render loaded remote/local details as `as of {format_since(now, fetched_at)}` regardless of the discovery footer state.

That means the user can fetch daemon detail, then hit a later mDNS scan failure, and the footer correctly switches to `degraded: ...` while the visible detail `as of Xs ago` line stops ticking until another event wakes the loop. This leaves one of the same freshness timestamps this slice is meant to fix frozen on screen.

The gate needs to include F1 when the selected row has a cached `DaemonDetail::Loaded { .. }`, or otherwise expose a small `DaemonsState` predicate that captures "F1 currently renders a time-dependent detail/footer" and test the degraded-detail case.

## Gates

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed.
- `cargo test --workspace` passed.
