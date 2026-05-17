# a1-1-tui-scaffold reopened

Reviewed sha: `a88055994a91e7a5a5610081ab089d8164f70cee`
Reviewed at: 2026-05-17T18:11:28Z
Reviewer: reviewer
Verdict: reopened

## Findings

### 1. Restore idempotency test emits real terminal control sequences

Severity: Low

Location: `crates/blit-tui/src/main.rs:308`

Round 2 fixes the original lifecycle issue, but the new `restore_terminal_idempotent_across_repeated_calls` test sets `TUI_ACTIVE=true` and calls the real `restore_terminal()` without actually entering the TUI. That makes the test emit real crossterm escape sequences (`Show` and `LeaveAlternateScreen`) during `cargo test`; the passing test output included `\x1b[?25h\x1b[?1049l` before the `blit-tui` test names.

Unit tests should not manipulate the developer's terminal or pollute CI logs with terminal control bytes. Please refactor the idempotency check so it exercises the state transition without writing terminal escape sequences, for example by extracting a pure `take_active_for_restore()` helper or injecting a test sink for the terminal commands.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed, but emitted terminal escape bytes from the new `blit-tui` restore test.
