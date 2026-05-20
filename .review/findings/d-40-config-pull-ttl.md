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

_(Round 1 listed "no min(tick, remaining) precision" as an
acceptable gap. The reviewer correctly rejected that reasoning —
see Round 2. There are no remaining known gaps.)_

This closes the d-38 known-gaps list. The F3 pull feature
(d-33 → d-40) is now fully config-aware: preview, resolved
destination, live progress with throughput, and an
operator-tunable self-cleaning outcome.

## Reviewer comments

### Round 1 verdict — reopened (`.review/results/d-40-config-pull-ttl.reopened.md`)

One finding:

- **The configured pull TTL can still be delayed by the live
  tick budget.** The loop reads `pull_status_ttl_ms` and clears
  expired pull terminal state, but only on the normal live-tick
  wake. The tick-budget computation considered only
  `cancel_remaining`, not the F3 pull terminal deadline. So
  `pull_status_ttl_ms = 250` could stay visible until the next
  `live_tick.interval_ms` wake — and since the live tick ceiling
  is 5000ms, a 250ms result could linger ~5s. This is the exact
  class of bug d-24 already fixed for cancel status. The
  reviewer asked for the pull deadline to be folded into the
  budget (a helper analogous to `cancel_status_remaining_ttl`)
  plus coverage of the 5s-tick / 250ms-TTL / F3-terminal
  scenario.

  My round-1 "Out of scope" note had it backwards: I claimed
  `needs_live_tick` waking the loop was sufficient, but that
  only guarantees a wake *at the live-tick cadence*, which is
  precisely the thing that's too coarse for a short TTL.

### Round 2 fix

- New `F3PullState::terminal_remaining(now, ttl) -> Option<Duration>`
  in `f3pull.rs` — the wall-clock remaining before the d-38
  auto-hide fires on a Done/Error fragment, mirroring
  `cancel_status_remaining_ttl`. `None` when no fragment shows
  or it has already expired.
- New pure `min_opt(a, b)` helper in `main.rs` — the shorter of
  two optional deadlines.
- The loop now computes `pull_remaining` (gated on `Screen::F3`)
  and feeds `min_opt(cancel_remaining, pull_remaining)` into the
  unchanged `compute_tick_budget`, so the budget collapses to
  `min(live_tick, cancel_remaining, pull_remaining)`.

### Round 2 tests

+6 tests (407 → 413):

- `f3pull::tests::terminal_remaining_some_within_ttl_none_after`
  — remaining shrinks within the window; `None` at/after the
  `>=` boundary.
- `f3pull::tests::terminal_remaining_none_on_idle_and_running`.
- `f3pull::tests::terminal_remaining_some_for_error_fragment`.
- `min_opt_picks_the_shorter_deadline` (incl. the `None` arms).
- `short_pull_ttl_overrides_long_live_tick` — the reviewer's
  exact scenario: `compute_tick_budget(true, 5s, min_opt(None,
  Some(250ms))) == Some(250ms)`, asserting the budget never
  exceeds the pull deadline.
- `budget_picks_nearer_of_cancel_and_pull_deadlines` — when both
  are pending, the nearer wins.

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

### Lesson restated

Any auto-hide fragment with an operator-tunable TTL must feed
its remaining deadline into the loop's sleep budget — otherwise
a long `live_tick.interval_ms` silently bounds a short TTL. d-24
established this for cancel status; d-40 R1 reintroduced the bug
for the pull fragment by reasoning that "the loop already ticks"
was enough. It isn't: ticking at the live-tick cadence is the
problem, not the solution. The budget must collapse to the
nearest deadline across *all* pending fragments.
