# d-52-config-delete-ttl: operator-tunable batch-delete TTL

**Severity**: Feature (polish — closes d-50's fixed-TTL gap)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `b629be1`

## What

d-50 R2 auto-hides a batch delete outcome after a hardcoded 5s
(`F3DelState::BATCH_TERMINAL_TTL`). This mirrors exactly the d-38
→ d-40 progression for the pull TTL: d-52 makes it
operator-tunable via `[transfer] delete_status_ttl_ms`, the third
sibling of `cancel_status_ttl_ms` (d-24) and `pull_status_ttl_ms`
(d-40).

## Approach

Direct application of the d-40 pattern:

- `TransferDefaults` gains `delete_status_ttl_ms` (default
  `DEFAULT_DELETE_TTL_MS = 5000`, bounds `[250, 60_000]`) +
  `delete_status_ttl_ms_clamped()`.
- The loop reads it each frame and feeds it to the d-50
  `app.f3_del.clear_terminal_if_expired(now, ttl)` sweep (so a
  Ctrl+R reload retunes it live).
- Removed the now-redundant `F3DelState::BATCH_TERMINAL_TTL`
  const — production reads config; the f3del tests use a local
  `TEST_TTL` (same cleanup d-40 did for the pull `TERMINAL_TTL`).
- Module-doc schema example + version comment updated.

## Files changed

- `crates/blit-tui/src/config.rs`: `delete_status_ttl_ms` field
  + bounds + clamped accessor + Default; schema doc; 2 tests.
- `crates/blit-tui/src/f3del.rs`: removed `BATCH_TERMINAL_TTL`;
  tests use `TEST_TTL`.
- `crates/blit-tui/src/main.rs`: loop reads
  `delete_status_ttl_ms_clamped()` for the batch-delete sweep.

## Tests

+3 tests (478 → 480):

- `transfer_default_delete_ttl_is_5000ms`.
- `transfer_delete_ttl_parses_and_clamps` — TOML parse + floor
  (0 → 250) + ceiling (u64::MAX → 60000).
- (f3del's existing TTL tests now reference the local `TEST_TTL`
  rather than the removed const — behavior unchanged.)

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

## Known gaps

None specific. The three transient-outcome TTLs (cancel / pull /
delete) are now uniformly operator-tunable.

## Note on remaining TUI work

This closes the F3 multi-select → batch-delete arc (d-49 → d-52).
The remaining TUI_DESIGN features are larger, multi-slice
efforts best done as focused investments: batch transfer
(`c`/`m`/`v` over the marked set), F1 `t` trigger-transfer, F1
per-module `df` capacity, and multi-daemon F2.

## Reviewer comments

(empty — pending grade)
