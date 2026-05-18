# d-24-config-cancel-ttl: operator-tunable cancel TTL

**Severity**: Feature (polish — closes d-23's known gap #1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

d-23 shipped a hardcoded 5-second TTL for the F2 cancel
fragment. d-23's "Known gaps" §1 called out the polish
opportunity:

> **No config-tunable TTL.** Hardcoded 5s. A future
> polish could add `[transfer] cancel_status_ttl_ms`
> for operators who want more / less time to read.

d-24 lands that polish. Operators with slower reading
speeds, multi-screen setups, or wall-mounted dashboards
can crank the TTL up; operators who want a near-instant
auto-clean can drop it.

```toml
[transfer]
cancel_status_ttl_ms = 10000   # 10 seconds
```

The value is silently clamped to `[250, 60000]` (0.25s
floor, 60s ceiling) — same pattern as e-5's
`[live_tick] interval_ms`.

## Approach

### New config struct

`TransferDefaults` in `crates/blit-tui/src/config.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferDefaults {
    pub cancel_status_ttl_ms: u64,
}

impl TransferDefaults {
    pub const DEFAULT_CANCEL_TTL_MS: u64 = 5_000;
    pub const MIN_CANCEL_TTL_MS: u64 = 250;
    pub const MAX_CANCEL_TTL_MS: u64 = 60_000;

    pub fn cancel_status_ttl_ms_clamped(&self) -> u64 {
        self.cancel_status_ttl_ms
            .clamp(Self::MIN_CANCEL_TTL_MS, Self::MAX_CANCEL_TTL_MS)
    }
}
```

`#[serde(default, deny_unknown_fields)]` matches the
project pattern set in e-3..e-7: forward-compat on
missing fields, typo-detection on extras (the d-24
config tests exercise the typo case explicitly).

Wired into `TuiConfig` next to `live_tick`, `theme`,
etc. — the existing config-warn pipeline already routes
through caller-provided callbacks, so no plumbing
churn.

### main.rs

Removed:

```rust
const CANCEL_STATUS_TTL: std::time::Duration =
    std::time::Duration::from_secs(5);
```

Changed `cancel_status_to_display` to take `ttl:
Duration` instead of reading the constant:

```rust
fn cancel_status_to_display(
    status: &F2CancelStatus,
    now: Instant,
    ttl: std::time::Duration,
) -> screens::f2::F2CancelDisplay {
    /* >= ttl checks for Done/Error */
}
```

Draw call site reads the clamped TTL from config each
frame:

```rust
&cancel_status_to_display(
    &app.cancel_status,
    now,
    std::time::Duration::from_millis(
        tui_config.transfer.cancel_status_ttl_ms_clamped(),
    ),
),
```

The clamp is a 2-op `u64::clamp` per frame —
negligible. Reading `tui_config` is a `&self` borrow
that's already in scope.

### Source-doc sync (the recurring lesson)

Updated three documentation surfaces in the same slice
— this is the e-5/e-6/d-20/d-22-R2 lesson:

1. `config.rs` module-doc schema block now lists the
   `[transfer]` section with the d-24 row.
2. `screens/f2.rs` module-doc:
   - Layout-sketch header now says
     `d-14 / d-15 / d-20 / d-21 / d-22 / d-23 / d-24`.
   - The "d-23 self-cleaning" paragraph is split into
     a d-23 paragraph (TTL-driven hide) and a d-24
     paragraph (the TTL is operator-tunable).
3. `main.rs` rustdoc on `F2CancelStatus::Done.finished_at`
   already references `[transfer] cancel_status_ttl_ms`
   instead of the old constant.

## Files changed

- `crates/blit-tui/src/config.rs`:
  - New `TransferDefaults` struct with clamped
    accessor.
  - `TuiConfig` gains `pub transfer: TransferDefaults`.
  - Module-doc schema block lists the new section.
- `crates/blit-tui/src/main.rs`:
  - `const CANCEL_STATUS_TTL` removed.
  - `cancel_status_to_display` signature gains
    `ttl: Duration`.
  - Draw call site passes
    `Duration::from_millis(tui_config.transfer.cancel_status_ttl_ms_clamped())`.
  - Rustdoc on `Done.finished_at` updated.
- `crates/blit-tui/src/screens/f2.rs`:
  - Module-doc layout-header includes d-24.
  - TTL paragraph split into d-23 (mechanism) + d-24
    (operator-tunable).

## Tests

+8 tests (249 → 257):

**`config::tests` (new — 7):**
- `transfer_default_cancel_ttl_is_5000ms` — default
  matches d-23's old constant.
- `transfer_cancel_ttl_parses_from_toml` — happy-path
  parse of `[transfer] cancel_status_ttl_ms = 2500`.
- `transfer_cancel_ttl_clamp_floor` — 0 + 100 → 250.
- `transfer_cancel_ttl_clamp_ceiling` — u64::MAX +
  120_000 → 60_000.
- `transfer_cancel_ttl_passes_through_when_in_range` —
  250 / 1k / 5k / 10k / 30k / 60k unchanged.
- `transfer_cancel_ttl_round_trips_through_toml` — 750
  parses + clamps to 750.
- `transfer_unknown_field_warns` — typo'd
  `cancel_status_ttl` (no `_ms` suffix) warns via the
  callback.

**`main::tests` (new — 1):**
- `cancel_status_respects_caller_supplied_ttl` —
  passing a 1-second custom TTL to
  `cancel_status_to_display` hides a 1001ms-old Done
  fragment AND keeps a 500ms-old one visible. The
  existing d-23 tests now use a local
  `TEST_CANCEL_TTL` constant sourced from
  `TransferDefaults::DEFAULT_CANCEL_TTL_MS`, so they
  pin the default-value behavior; this new test pins
  the override-value behavior.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No hot-reload of `tui.toml`.** Same as e-3..e-7
   and d-24: changes take effect on next TUI launch. A
   future polish could add a `SIGHUP` reload (or `R`
   key) that re-parses `tui.toml` and updates the
   live `TuiConfig`.

2. **Clamp is silent.** An operator who sets
   `cancel_status_ttl_ms = 999999` gets 60s and no
   warning. The e-5 `[live_tick]` clamp is silent for
   the same reason — clamping to a sensible default
   is less disruptive than refusing to start.

## Out of scope (next slices)

- **Hot-reload tui.toml.**
- **Cancel confirmation prompt.**
- **TiB/s throughput tier on F2.**
- **F3 module filter.**

## Reviewer comments

(empty — pending grade)
