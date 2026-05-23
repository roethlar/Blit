Reviewed sha: `5e24918fa98e31012d906d3f553afc0acb17de7c`

# Reopened: m2f-9-f2-discovery-refan

## Findings

1. `crates/blit-tui/src/main.rs:1095` — discovery changes during an in-flight setup are dropped.

   The auto re-fan is gated on `transfers_event_rx.is_some()`. During startup, the initial F2 setup is pending and `transfers_event_rx` is still `None` (the startup setup was spawned from the pre-discovery watch set at lines 611-615). If mDNS discovers a new daemon during that pending window, the update changes `f2_watched_identities`, but this branch does not re-fan and does not record that the pending setup is stale. When the original setup completes, F2 is live on the old watch set; subsequent steady discovery updates compare equal to the already-updated daemon list, so no later auto re-fan happens. The newly discovered daemon remains unwatched until a manual `r`, which is the gap this slice is intended to close.

   Expected: a watch-set change while `transfers_setup_pending` should not be lost. Either restart the pending setup against the current watch set by bumping the generation and spawning the new set, or record a pending re-fan and execute it when the current setup reply lands. Add a regression test for: initial setup pending with only `parsed_remote`, discovery adds a daemon, initial stale setup completes, and F2 still re-fans/ends up watching the discovered daemon without manual refresh.

## Gates

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (590 passed)
