# a1-1-tui-scaffold reopened

Reviewed sha: `38df2bbadbade26e6ab03be907536f61d1c7c654`
Reviewed at: 2026-05-18T03:21:37Z
Reviewer: reviewer
Verdict: reopened

## Findings

### 1. State-transition tests race on global TUI_ACTIVE

Severity: Low

Location: `crates/blit-tui/src/main.rs:310`

Round 3 removes the terminal escape output, but the two new `take_active_for_restore_*` tests both mutate the same process-global `TUI_ACTIVE` atomic. Rust's unit test harness runs tests in parallel by default, so these tests can interleave: one test can store `false` between the other test's store and `take_active_for_restore()`, or store `true` before the inactive test's assertion. That makes the new tests order-dependent and potentially flaky.

Please make the tests independent or serialize the shared state. Prefer changing the helper to take an `&AtomicBool` so production passes `&TUI_ACTIVE` and tests use local atomics; a test-only mutex around these two tests is also acceptable.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed and no longer emitted terminal escape bytes.
- `cargo test --workspace` passed.
