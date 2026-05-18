# e-5-config-live-tick-interval review

Reviewed sha: `55f173362e2b6cafdc0d6bc36df446f7c6c488a8`

## Findings

1. Low - source docs still describe the pre-e-5 live-tick contract.

   The runtime change is fine, but a few comments now contradict the code: `crates/blit-tui/src/main.rs:499` still says the select arm is a "500ms live-tick wakeup" even though `tui_config.live_tick.interval_ms_clamped()` drives the sleep; `crates/blit-tui/src/config.rs:76` names `[MIN_TICK_MS, MAX_TICK_MS]`, which are not the constants introduced by this slice; and `crates/blit-tui/src/config.rs:10` still presents the config as the initial verify-only schema despite the new `[tab_strip]` and `[live_tick]` sections.

   Please update these comments to match the current config contract. This matters because the next E-slice is also config-growth work, and stale schema/timing comments will send future agents to the wrong baseline.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed (216 tests).
- `cargo test --workspace` passed.
