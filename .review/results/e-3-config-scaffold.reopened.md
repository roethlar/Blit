# e-3-config-scaffold review

Reviewed sha: `985b047709bab1a68f20abfc1f69df5fd152b075`

## Findings

1. Medium - malformed-config warnings are emitted after the TUI has entered the alternate screen.

   `main` constructs `TuiGuard` before calling `run_router` (`crates/blit-tui/src/main.rs:222`), and `TuiGuard::new()` enables raw mode and enters the alternate screen (`crates/blit-tui/src/main.rs:1074`, `crates/blit-tui/src/main.rs:1081`). The new config load happens later inside `run_router` with `config::load(|msg| eprintln!("[blit-tui] {msg}"))` at `crates/blit-tui/src/main.rs:252`.

   That means TOML parse warnings are written while the alternate screen is active. The finding handoff says parse errors should warn on stderr and be visible after the TUI exits; in this ordering they are not reliably visible after exit and can also corrupt the TUI screen before the first draw. The practical result is that a typo such as `defalut_use_checksum` can silently fall back to defaults from the operator's perspective.

   Please move config loading before `TuiGuard::new()` and pass the loaded `TuiConfig` into `run_router`, or otherwise buffer the warning and print it after the guard restores the terminal. A small regression test around startup ordering or warning delivery would be useful because the loader unit tests currently exercise only `load_from_path`, not where the warning is emitted in the TUI lifecycle.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed (208 tests).
- `cargo test --workspace` passed.
