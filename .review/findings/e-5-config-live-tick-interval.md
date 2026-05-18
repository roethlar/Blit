# e-5-config-live-tick-interval: operator-tunable tick cadence

**Severity**: Feature (polish — third slice growing the
e-3 config scaffold)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

The d-9 live-tick interval was hardcoded at 500ms.
Operators on slow / high-latency terminals (SSH over
satellite, mosh under poor connectivity) reported the
500ms redraws as visible jitter; raising the cadence to
1s or 2s smooths things out at the cost of choppier
elapsed counters.

```toml
[live_tick]
interval_ms = 1200
```

Bounded:

- **Floor 50ms** — anything faster is just CPU burn the
  operator can't perceive.
- **Ceiling 5000ms** — beyond that the "live" counter
  stops looking live (a 12.3s timer that refreshes once
  per 5s spends most of its life stale).
- **Default 500ms** — matches the d-9 baseline so an
  upgrade without a `tui.toml` keeps existing cadence.

Values outside `[50, 5000]` are clamped silently (no
warning, no refusal to start) — the operator gets the
nearest sane value rather than a typo bricking the TUI.

## Approach

### Config schema

`TuiConfig` gains a `live_tick: LiveTickDefaults`
section:

```rust
pub struct LiveTickDefaults {
    pub interval_ms: u64,  // default 500
}

impl LiveTickDefaults {
    pub const DEFAULT_INTERVAL_MS: u64 = 500;
    pub const MIN_INTERVAL_MS: u64 = 50;
    pub const MAX_INTERVAL_MS: u64 = 5000;

    pub fn interval_ms_clamped(&self) -> u64 {
        self.interval_ms.clamp(Self::MIN, Self::MAX)
    }
}
```

Same `#[serde(default, deny_unknown_fields)]` contract
as the other config sections — typos warn, unspecified
fields take defaults.

### Apply

The `run_router` loop's `live_tick` future now uses
`tui_config.live_tick.interval_ms_clamped()` instead of
the hardcoded `Duration::from_millis(500)`. The
`clamp` happens per-loop-iteration (cheap — just two
comparisons) so the runtime always sees a sane value.

## Files changed

- `crates/blit-tui/src/config.rs`:
  - `LiveTickDefaults` struct + `interval_ms_clamped()`
    accessor.
  - `TuiConfig::live_tick` field.
- `crates/blit-tui/src/main.rs`:
  - Loop's `live_tick` future reads
    `tui_config.live_tick.interval_ms_clamped()` instead
    of the hardcoded 500ms literal.

## Tests

+5 unit tests (211 → 216):

In `config::tests`:
- `live_tick_default_is_500ms` — fresh `TuiConfig`
  matches d-9 baseline.
- `live_tick_parses_from_toml` — `[live_tick]
  interval_ms = 1200` round-trips through serde.
- `live_tick_clamp_floor` — `0` and `1` both clamp up
  to `MIN_INTERVAL_MS` (50).
- `live_tick_clamp_ceiling` — `10_000` and `u64::MAX`
  both clamp down to `MAX_INTERVAL_MS` (5000).
- `live_tick_passes_through_when_in_range` — every
  value in `[50, 5000]` is preserved exactly.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **Silent clamp.** A typo (`interval_ms = 50000`)
   clamps to 5000 without warning. Operator who meant
   "5 seconds" might mistype as "50 seconds" and never
   notice. A future polish could warn when the raw
   value differs from the clamped value, but the
   clamped value is always the safe one — refusing to
   start on out-of-range input would be worse UX.

2. **Same cadence everywhere.** Verify running, transfer
   running, F1 freshness, F3 freshness — all use the
   one knob. A future polish could split into per-pane
   intervals (e.g. transfer Running at 250ms for tight
   throughput display, F1 freshness at 1000ms because
   second-resolution doesn't need fast updates).

## Out of scope (next slices)

- **Color themes** (`[theme]`).
- **Persisted form prefill** (last Source / Destination).
- **Per-pane tick intervals.**

## Reviewer comments

(empty — pending grade)
