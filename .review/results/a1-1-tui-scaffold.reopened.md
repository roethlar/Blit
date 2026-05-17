# a1-1-tui-scaffold reopened

Reviewed sha: 63975ef40e8ed144d3fa5f072edca883e9d34890
Reviewed at: 2026-05-17T17:57:42Z
Reviewer: reviewer
Verdict: reopened

## Findings

### 1. Terminal teardown is not guaranteed on setup errors or panics

Severity: Medium

Location: `crates/blit-tui/src/main.rs:63`, `crates/blit-tui/src/main.rs:67`, `crates/blit-tui/src/main.rs:76`

The scaffold's load-bearing promise is terminal lifecycle correctness, but the current shape only restores the terminal after `enter_tui()` has fully succeeded and `run_event_loop()` returns normally. If any setup call after `enable_raw_mode()` fails, such as `EnterAlternateScreen`, `Terminal::new`, `terminal.clear`, or `hide_cursor`, `main` returns before `leave_tui()` is available and raw mode can remain enabled. If `run_event_loop()` panics, line 68 is skipped and the terminal can remain in raw mode / alternate screen with the cursor hidden.

This cannot be left as a known gap for a scaffold whose purpose is to establish the TUI lifecycle. Please make setup transactional and add panic-safe restoration before this lands. A small RAII guard plus a panic hook is fine; the important contract is that partial setup failures, normal errors, normal quit, and panics all attempt to restore raw mode, leave alternate screen, and show the cursor.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.
