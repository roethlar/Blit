# d-40-config-pull-ttl: operator-tunable F3 pull-outcome TTL

**Severity**: Feature (polish — closes d-38 known gap #1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `c4aca4f`

## What

d-38 added a 5-second auto-hide on the F3 pull Done/Error
footer fragment, but the TTL was hardcoded
(`f3pull::F3PullState::TERMINAL_TTL`). Its own known gap #1
flagged the asymmetry: the F2 cancel-status fragment is
operator-tunable via `[transfer] cancel_status_ttl_ms` (d-24),
but the pull fragment was not. d-40 closes that — the pull TTL
is now a sibling config field:

```toml
[transfer]
cancel_status_ttl_ms = 5000   # d-24
pull_status_ttl_ms   = 5000   # d-40
```

## Approach

A separate field, not a shared one. d-38's known-gaps note
explicitly argued against overloading `cancel_status_ttl_ms`
("avoid muddying the cancel-specific config field's meaning") —
the two outcome fragments are conceptually distinct surfaces an
operator might want to tune independently (e.g. long pull
retention, short cancel retention). The new field mirrors the
d-24 shape exactly:

- `pull_status_ttl_ms: u64` on `TransferDefaults`, default
  `DEFAULT_PULL_TTL_MS = 5000`.
- Bounds `MIN_PULL_TTL_MS = 250` / `MAX_PULL_TTL_MS = 60_000`
  (same rationale as the cancel bounds).
- `pull_status_ttl_ms_clamped()` accessor — out-of-range values
  silently snap to the bounds rather than being refused, same
  as every other clamped TUI config knob.

### Wiring

The event loop's d-38 sweep changes from a hardcoded const to a
per-frame config read:

```rust
let pull_ttl =
    Duration::from_millis(tui_config.transfer.pull_status_ttl_ms_clamped());
app.f3_pull.clear_terminal_if_expired(now, pull_ttl);
```

Reading it each frame (not once at startup) means a `Ctrl+R`
hot-reload (d-36) retunes the TTL live, consistent with how the
accent color and other reloadable knobs already behave.

### Removed `TERMINAL_TTL`

`clear_terminal_if_expired` already took the TTL as a parameter
(d-38 designed it that way), so the const was only a default
holder. With production now sourcing the value from config, the
const became production-dead (tests-only → `dead_code` under
`-D warnings`). Removed it; `DEFAULT_PULL_TTL_MS` is now the
single source of truth for the 5s default. The f3pull tests use
a local `TEST_TTL` const since they only need a representative
fixed duration to exercise the boundary logic.

## Files changed

- `crates/blit-tui/src/config.rs`:
  - `TransferDefaults` gains `pull_status_ttl_ms` +
    `DEFAULT/MIN/MAX_PULL_TTL_MS` + `pull_status_ttl_ms_clamped`.
  - `Default` impl seeds it; module-doc schema example +
    version comment updated.
  - 5 new tests.
- `crates/blit-tui/src/main.rs`:
  - Loop reads `pull_status_ttl_ms_clamped()` each frame and
    feeds it to `clear_terminal_if_expired`.
- `crates/blit-tui/src/f3pull.rs`:
  - Removed `TERMINAL_TTL`; tests use a local `TEST_TTL`.

## Tests

+5 tests (402 → 407):

- `transfer_default_pull_ttl_is_5000ms`.
- `transfer_pull_ttl_parses_from_toml`.
- `transfer_pull_ttl_clamp_floor` (0 → 250).
- `transfer_pull_ttl_clamp_ceiling` (u64::MAX → 60000).
- `transfer_pull_and_cancel_ttls_are_independent` — both keys
  in one `[transfer]` block resolve to distinct clamped values
  (the "separate field" guarantee).

The existing d-38 auto-hide tests are unchanged in behavior
(they now reference `TEST_TTL` instead of the removed const).

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

## Known gaps

1. **No min(tick, remaining) precision.** Same as d-38 known
   gap #2: the fragment can linger up to one `live_tick`
   interval past its TTL because the sweep runs per-frame, not
   on a TTL-exact timer. Acceptable for a 250ms–60s retention
   window. (The d-24 precision fix mattered only because the
   cancel TTL drives the *sleep budget*; the pull TTL does not,
   so there's no correctness issue here, only ±one-tick
   cosmetic lag.)

## Out of scope

- Folding the pull TTL into the sleep-budget computation (d-24
  style) — unnecessary; `needs_live_tick` already wakes the
  loop while a terminal fragment shows.

This closes the d-38 known-gaps list. The F3 pull feature
(d-33 → d-40) is now fully config-aware: preview, resolved
destination, live progress with throughput, and an
operator-tunable self-cleaning outcome.

## Reviewer comments

(empty — pending grade)
