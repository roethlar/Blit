# d-1-f4-profile-lifecycle reopened

Reviewed sha: `c380e0d7ff8338d83475bc02d6909fa8155c3d06`

Verdict: reopened

## Finding

### 1. Medium — Mutation failures are immediately hidden by the follow-up fetch

The new F4 lifecycle actions call the mutating helper and then unconditionally start a profile fetch:

- clear path: `crates/blit-tui/src/main.rs:595`
- disable path: `crates/blit-tui/src/main.rs:602`
- enable path: `crates/blit-tui/src/main.rs:607`

The helpers do call `ProfileState::note_fetch_error` on failure:

- `apply_profile_clear`: `crates/blit-tui/src/main.rs:621`
- `apply_profile_set_enabled`: `crates/blit-tui/src/main.rs:630`

but the next line in each action calls `begin_fetch()`, and `begin_fetch` sets the status back to `Pending`. If the read itself succeeds, `apply_report` then changes the status to `Loaded`. That means a failed clear/disable/enable can be completely hidden from the operator even though the code comment says mutation errors are surfaced in the F4 footer.

This is a persistent-state action, so the TUI must not paper over failures. Make the mutation helper return success/failure and only kick the follow-up fetch on success, or keep a separate operation-error surface that survives the refresh.

## Gates

- `cargo fmt --all -- --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `cargo test -p blit-tui` passed
- `cargo test --workspace` passed
