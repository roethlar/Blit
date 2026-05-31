# e-3-config-scaffold: tui.toml loader + Verify defaults

**Severity**: Feature (first slice of the E
"themes / refresh rates / config" milestone)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Establishes the `~/.config/blit/tui.toml` config-file
pattern that future polish slices can grow:

```toml
[verify]
default_use_checksum = true
default_one_way = false
```

Initial schema is deliberately tiny — just the two
Verify mode toggles introduced in d-6 / d-7. Operators
who prefer checksum verification by default no longer
have to hit `H` on every TUI launch. Future slices add
fields (color themes, refresh interval, persisted form
prefill) without breaking older configs because every
field gets `#[serde(default)]`.

**Missing file is the happy default** — the loader
returns `TuiConfig::default()` silently. Parse errors
print a warning to stderr (visible after the TUI
exits) and use defaults. The TUI never refuses to
start on a misconfigured `tui.toml`.

## Approach

### `crates/blit-tui/src/config.rs` (new module)

```rust
pub struct TuiConfig {
    pub verify: VerifyDefaults,
}

pub struct VerifyDefaults {
    pub default_use_checksum: bool,
    pub default_one_way: bool,
}

pub fn load(on_warn: impl FnOnce(String)) -> TuiConfig
pub fn load_from_path(path, on_warn) -> TuiConfig
```

`load` resolves the path via
`blit_core::config::config_dir().join("tui.toml")`
(shared with the rest of the workspace; honors
`set_config_dir()` overrides used by tests).

`load_from_path` is path-explicit for unit tests that
need to stage a tempfile.

### Failure modes (all graceful)

- **`config_dir()` fails** → silent fallback to
  defaults (not user-actionable; no platform home dir).
- **File missing or unreadable** → silent fallback to
  defaults (expected on a fresh install).
- **TOML parse error** → warn to stderr, defaults.
- **Unknown field** → `deny_unknown_fields` makes this
  a parse error, so the typo'd `defalut_use_checksum`
  warns instead of silently being ignored.

### Apply path

`VerifyState::with_defaults(use_checksum, one_way)`
new constructor takes the booleans. `VerifyState::new()`
still works — calls `with_defaults(false, false)` for
the rsync-compatible default.

`run_router` loads `TuiConfig` once at startup and
constructs `AppState::verify` via `with_defaults`.

## Files changed

- `crates/blit-tui/Cargo.toml`:
  - `serde` with derive feature (was indirect via
    serde_json).
  - `toml = "0.8"`.
- `crates/blit-tui/src/config.rs` (new):
  - `TuiConfig` + `VerifyDefaults` structs with serde
    derive.
  - `load` + `load_from_path` functions.
  - +6 unit tests covering the loader matrix.
- `crates/blit-tui/src/verify.rs`:
  - `with_defaults(use_checksum, one_way)` constructor.
  - `new()` delegates to `with_defaults(false, false)`.
  - +2 unit tests pinning the constructor contract.
- `crates/blit-tui/src/main.rs`:
  - `mod config;` declaration.
  - `run_router` loads `tui_config` at startup, warns on
    stderr for parse errors.
  - `AppState::verify` constructed via `with_defaults`.

## Tests

+8 unit tests (200 → 208):

In `config::tests`:
- `missing_file_returns_defaults_silently`
- `empty_file_returns_defaults`
- `populated_verify_section_overrides_defaults`
- `partial_verify_section_keeps_other_defaults`
- `malformed_toml_emits_warning_returns_defaults`
- `unknown_fields_emit_warning` — typo catch via
  `deny_unknown_fields`.

In `verify::tests`:
- `with_defaults_seeds_mode_flags_from_config`
- `new_uses_rsync_compatible_defaults`

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No way to view the loaded config from the TUI.**
   A future polish slice could add a `K` (config) key
   that surfaces the active config as a read-only
   overlay — useful when debugging "why is checksum
   on by default for this operator?"

2. **No write-back / persistence.** Toggles flipped at
   runtime via `H`/`O` are session-only and reset on
   next launch. Persisting the runtime state into
   `tui.toml` is a future polish slice (it changes
   the file the operator may be hand-editing, so it
   needs careful UX).

3. **Schema is tiny.** Future fields candidates:
   - `[tab_strip] show_counts: bool` — opt-out of d-15.
   - `[live_tick] interval_ms: u64` — operator-tunable
     refresh cadence.
   - `[theme] accent_color: String` — for accessibility
     / dark / high-contrast.
   Adding any of these is a one-field-at-a-time slice
   on top of this scaffold.

## Out of scope (next slices)

- **Color themes.**
- **Persisted runtime mode flips** (`H`/`O` writes back).
- **Per-file progress** events during local transfers.

## Reviewer comments

### Round 1 verdict — reopened (`.review/results/e-3-config-scaffold.reopened.md`)

One Medium-severity finding, addressed in round 2:

- **Parse warnings emitted inside the alternate screen.**
  Round 1 loaded the config inside `run_router`, which
  runs AFTER `TuiGuard::new()` enters raw mode + the
  alternate screen. `eprintln!` writes from inside that
  context either corrupt the rendered UI mid-draw or
  get swallowed by the alternate screen and never reach
  the operator. A typo like `defalut_use_checksum`
  silently fell back to defaults from the operator's
  perspective.

  Round 2 reorders: `main` loads the config BEFORE
  constructing the TuiGuard, accumulates any warnings
  into a `Vec<String>`, passes the loaded `TuiConfig`
  into `run_router`, then flushes the warnings to
  stderr AFTER the guard drops. The terminal is back
  to its normal state when the warning lands, so it's
  reliably visible.

### Round 2 file changes

- `crates/blit-tui/src/main.rs`:
  - `fn main` loads `TuiConfig` before `TuiGuard::new()`,
    accumulates warnings into a local Vec.
  - `run_router` signature gains `tui_config: TuiConfig`
    parameter; the inline load + `eprintln!` callback
    are gone.
  - Post-guard warning flush iterates the accumulated
    Vec.
- `crates/blit-tui/src/config.rs`:
  - New `warnings_route_through_callback_not_stderr`
    regression test capturing the buffer-then-flush
    contract.

### Round 2 tests

+1 test (208 → 209):

In `config::tests`:
- `warnings_route_through_callback_not_stderr` — drives
  the loader the same way `main` does (Vec collector
  callback) and asserts the warning landed in the
  buffer.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.
