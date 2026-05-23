# audit-6-test-gaps: Test coverage and quality gaps across the codebase

**Severity**: Test Gap
**Status**: Open
**Branch**: (none yet)

## What

Ground-up audit identified structural test coverage gaps across multiple
crates:

1. **`crates/blit-app/` has zero inline tests.** This crate contains critical
   orchestration glue (transfers/dispatch.rs, endpoints.rs, check.rs,
   diagnostics, profile, and admin/ subdirectory). Every consumer (CLI, TUI)
   depends on it, yet there are no `#[cfg(test)]` blocks anywhere in the
   crate. Only end-to-end CLI integration tests provide any coverage.

2. **`crates/blit-tui/` has zero rendering tests.** All existing tests verify
   state transitions (apply_done, apply_error, etc.) but never run the
   `render_*` functions that produce ratatui widgets. A typo in a label,
   layout regression, or incorrect data binding in the render path has no
   automated protection.

3. **`crates/blit-prometheus-bridge/` has no integration tests.** No test
   spins up a real HTTP server and scrapes the `/metrics` endpoint. Format
   regressions are only caught after deployment.

4. **`crates/blit-core/src/path_safety.rs`** has no tests for non-UTF-8 byte
   sequences, decomposed Unicode (NFD vs NFC), bidirectional text, or
   zero-width joiners in path components. This is the primary security
   boundary for path traversal prevention.

5. **`crates/blit-cli/tests/`** has no dedicated test for remote→local or
   local→remote move operations. Only local-to-local and remote-to-remote
   moves are tested. The four cardinal move directions have incomplete
   coverage (data-loss risk for pull-move/push-move).

6. **`crates/blit-daemon/src/delegation_gate.rs`** has no test for DNS
   rebinding scenario where hostname resolves to different IP on second call.
   The `ScriptedResolver` test infrastructure exists but isn't used for this.

7. **`crates/blit-core/src/copy/file_copy/clone.rs`** has no test forcing
   fast-path failure to verify fallback chain (clonefile → fcopyfile →
   sendfile → copy_file_range → buffered copy).

## Approach

- Add unit tests to `blit-app` for endpoint parsing, transfer dispatch
- Add snapshot/regression tests for TUI render output (or at minimum, a test
  that render functions don't panic on edge-case state)
- Add HTTP integration test for prometheus-bridge
- Add Unicode edge case tests for path_safety
- Add DNS rebinding test using existing ScriptedResolver
- Add fast-path failure tests for copy fallback chain

## Files changed

TBD by coder. Multiple crate test modules.

## Known gaps

- TUI render testing may require a terminal emulator harness or ratatui
  buffer-based assertions. This is a non-trivial test infrastructure addition
  and should be scoped carefully.
