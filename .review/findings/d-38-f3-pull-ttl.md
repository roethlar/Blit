# d-38-f3-pull-ttl: auto-hide TTL on the F3 pull outcome

**Severity**: Feature (polish — closes d-35 known gap #2)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

The last d-35 gap. The F3 pull Done/Error fragment
(`pulled N file(s) · X → <dest>` / `pull failed: <msg>`)
lingered in the footer forever until the operator
started another pull or navigated away. d-38 adds a
5-second auto-hide TTL, mirroring the d-23 cancel-status
auto-clear — so the footer self-cleans after the
operator has had time to read the outcome.

## Approach

State-level expiry (the d-36 reload-banner pattern),
not renderer-side hiding (the d-23 cancel pattern). The
`Done` / `Error` variants gain `finished_at: Instant`;
once `now - finished_at >= TTL`, the loop clears the
state back to `Idle`. Two consequences:

- The renderer bridge stays trivial (`Done → Done`,
  `Error → Error`, `Idle → Hidden`) — no `now` threading.
- `is_terminal()` flips false once cleared, so
  `needs_live_tick` stops ticking for it (no idle spin).

```rust
pub fn is_terminal(&self) -> bool { Done | Error }

pub fn clear_terminal_if_expired(&mut self, now, ttl) {
    let finished_at = match &self.status {
        Done { finished_at, .. } | Error { finished_at, .. } => *finished_at,
        _ => return,
    };
    if now.saturating_duration_since(finished_at) >= ttl {
        self.status = Idle;
    }
}

pub const TERMINAL_TTL: Duration = Duration::from_secs(5);
```

Event loop:
- `apply_done` / `apply_error` stamp `finished_at = now`.
- Before each draw: `clear_terminal_if_expired(now, TERMINAL_TTL)`
  (right after the d-36 reload-banner clear).
- `needs_live_tick` returns true while `is_terminal()`,
  so the loop wakes to expire the fragment.

`Running` is immune to the sweep (only Done/Error carry
`finished_at`), so a long pull never auto-clears.

## Files changed

- `crates/blit-tui/src/f3pull.rs`:
  - `Done` / `Error` gain `finished_at`.
  - `apply_done` / `apply_error` take `at: Instant`.
  - `is_terminal` + `clear_terminal_if_expired` +
    `TERMINAL_TTL`.
  - 5 new tests; existing reply-test calls pass
    `Instant::now()`.
- `crates/blit-tui/src/main.rs`:
  - Reply arm stamps `at` into apply_done/apply_error.
  - Loop clears expired terminal fragment before render.
  - `needs_live_tick` gates on `is_terminal()`.

## Tests

+5 tests (394 → 399):

- `done_is_terminal_running_and_idle_are_not`.
- `error_is_terminal`.
- `clear_terminal_hides_done_after_ttl` — within TTL
  stays, past TTL → Idle.
- `clear_terminal_at_exact_boundary_hides` — the `>=`
  boundary clears on the dot.
- `clear_terminal_is_noop_on_running_and_idle` — a
  long-running pull is never swept.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **Fixed 5s TTL, not config-tunable.** The F2 cancel
   TTL is operator-tunable via `[transfer]
   cancel_status_ttl_ms` (d-24); the F3 pull TTL is
   hardcoded. A future polish could route it through the
   same (or a sibling) config knob. Kept fixed here to
   avoid muddying the cancel-specific config field's
   meaning.

2. **No min(tick, remaining) precision (d-24 style).**
   `needs_live_tick` ticks at `live_tick.interval_ms`
   cadence while terminal, so the fragment can linger up
   to one tick past 5s. Acceptable — the d-24 precision
   fix mattered for short cancel TTLs; 5s ± one tick is
   imperceptible.

## Out of scope

- Config-tunable pull TTL.
- min(tick, remaining) precision.

This closes the d-35 known-gaps list; the F3
transfer-from-cursor feature (d-33 → d-38) is now
feature-complete with preview, execution, live progress,
and self-cleaning outcomes.

## Reviewer comments

(empty — pending grade)
