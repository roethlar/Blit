# d-24-config-cancel-ttl review

Reviewed sha: `517a3aef64ab3df40c06667d807055f2a70a353b`

## Findings

1. Low - Configured cancel TTL is still bounded by the unrelated live-tick cadence.

   The new value is read at render time in `crates/blit-tui/src/main.rs:537`, but the only automatic redraw timer is still `live_tick.interval_ms` from `crates/blit-tui/src/main.rs:581`. `needs_live_tick` also remains keyed only on `TransfersState::last_event_at()` at `crates/blit-tui/src/main.rs:1748`; it does not schedule a wakeup for the cancel status deadline. This means an operator can set `[transfer] cancel_status_ttl_ms = 250`, but if `[live_tick] interval_ms = 5000`, a terminal cancel fragment can stay visible for roughly five seconds after the reply instead of clearing near the configured 250 ms. That contradicts the new config contract documented in `crates/blit-tui/src/config.rs:180` and the finding doc's "near-instant auto-clean" motivation.

   Please make the event-loop sleep account for the cancel-status expiry while F2 is visible, for example by using the smaller of the live-tick interval and the remaining cancel TTL when `cancel_status` is Done/Error. Add a regression test that combines a short cancel TTL with a longer live-tick interval so the configured TTL cannot silently be delayed by the freshness-footer cadence.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.
