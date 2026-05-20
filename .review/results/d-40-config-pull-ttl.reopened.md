# d-40-config-pull-ttl reopened

Reviewed commit: `c4aca4f6c6cea7b8430602f327adbaed6552bdfb`
Reviewed at: `2026-05-20T16:36:31Z`
Reviewer: `reviewer`

Validation:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

Findings:

1. The configured pull TTL can still be delayed by the live tick budget.

   `crates/blit-tui/src/config.rs:228` makes the F3 pull outcome TTL operator-tunable, and `crates/blit-tui/src/config.rs:233` allows a 250ms floor. The event loop reads that TTL and clears expired pull terminal state at `crates/blit-tui/src/main.rs:604`, but the loop only wakes to do that work on the normal live tick. The tick budget at `crates/blit-tui/src/main.rs:727` still only considers `cancel_remaining`; it does not include the remaining F3 pull terminal deadline.

   That means `[transfer] pull_status_ttl_ms = 250` can remain visible until the next `[live_tick] interval_ms` wake. Since `crates/blit-tui/src/config.rs:127` permits a 5000ms live tick ceiling, a 250ms pull result can linger for about 5 seconds. This is the same class of bug already fixed for cancel status: `crates/blit-tui/src/main.rs:2274` explicitly documents that short TTLs must not be silently bounded by longer live ticks.

   Please add the F3 pull terminal remaining deadline to the tick-budget calculation, either with a helper analogous to `cancel_status_remaining_ttl` or an equivalent state API. Add coverage where `live_tick_interval` is 5s, pull TTL is 250ms, F3 has a terminal status, and the computed sleep budget is no greater than the remaining pull TTL.
