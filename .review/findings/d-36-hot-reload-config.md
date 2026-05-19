# d-36-hot-reload-config: Ctrl+R hot-reload of tui.toml

**Severity**: Feature (polish — operator config iteration)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

`Ctrl+R` re-reads `tui.toml` and swaps the live config
without restarting the TUI. Theme accent, live-tick
interval, and the `[transfer]` knobs all take effect on
the next frame. A transient banner in the tab-strip line
confirms the outcome:

```
 F1 Daemons · F2 Transfers · ...        config reloaded   (green, 4s)
 F1 Daemons · F2 Transfers · ...   reload failed: … — kept previous  (red)
```

On a parse error the **current config is kept** (not
reset to defaults) — the operator's running settings
survive a typo'd edit.

## Approach

### Mutable config + per-frame derivation

`run_router` takes `mut tui_config`. The `Ctrl+R` arm
reassigns it. Two values that were computed once at
startup now derive per-frame from `tui_config` so a
reload is visible immediately:

- `accent_color` (was a single pre-loop binding) — moved
  into the loop body.
- the reload banner display (color + visibility).

### Reload logic

```rust
fn reload_tui_config(current, now) -> (TuiConfig, ReloadBanner) {
    let mut warning = None;
    let loaded = config::load(|m| warning = Some(m));
    classify_reload(loaded, warning, current, now)
}

fn classify_reload(loaded, warning, current, now) -> (TuiConfig, ReloadBanner) {
    match warning {
        Some(msg) => (current.clone(), error_banner(msg)),   // keep current
        None      => (loaded, ok_banner()),                  // adopt
    }
}
```

`classify_reload` is the pure, I/O-free decision core —
unit-testable without touching the process-global config
dir (which would race under parallel tests). `config::load`
returns defaults on a parse error AND calls the warn
callback; the `Some(warning)` arm is what distinguishes
"parse failed → keep current" from "parsed OK → adopt".
A missing file (no warning) legitimately adopts defaults.

### Banner

`ReloadBanner { message, ok, shown_at }` on `AppState`,
with a 4-second renderer-side TTL (`is_visible(now)`).
The loop clears an expired banner at the top of each
iteration, and `needs_live_tick` returns true while a
banner is set so the loop wakes to expire it (no idle
spin once it's gone). Rendered in the tab-strip's
right-edge column, taking precedence over the counts.

### Keymap

`Ctrl+R` → `UserAction::ReloadConfig`, checked in
`key_action` before the bare `Char('r') => Refresh` arm
so the Ctrl modifier disambiguates. Handled at the
top-level dispatch (it owns the run_router-scoped
`tui_config`), not in a per-pane dispatcher.

## Files changed

- `crates/blit-tui/src/main.rs`:
  - `mut tui_config`; `accent_color` + banner derived
    per-frame.
  - `ReloadBanner` struct (4s TTL) + AppState field.
  - `reload_tui_config` + pure `classify_reload`.
  - `UserAction::ReloadConfig` + `Ctrl+R` keymap +
    top-level dispatch.
  - Expired-banner clearing + `needs_live_tick` gate.
  - 5 tests + AppState fixtures.
- `crates/blit-tui/src/screens/mod.rs`:
  - `render_tab_strip` gains a `reload_banner` param;
    renders it in place of the counts when present.
  - 3 test call sites updated.
- `crates/blit-tui/src/help.rs`:
  - `Ctrl-R` row; modal height 37 → 38.
- `crates/blit-tui/src/config.rs`:
  - Module-doc note on runtime reloadability.

## Tests

+5 tests (381 → 386):

- `key_action_maps_ctrl_r_to_reload_config` — Ctrl+R →
  ReloadConfig; bare `r` stays Refresh.
- `classify_reload_success_adopts_new` — no warning →
  loaded config adopted, green banner.
- `classify_reload_parse_error_keeps_current` — warning
  → current config kept, red "reload failed" banner.
- `reload_banner_visibility_expires` — visible at 0s/3s,
  hidden at 5s (4s TTL).
- `needs_live_tick_true_while_reload_banner_set` — the
  loop ticks while the banner is up.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **Reload warnings only surface in the banner.** A
   parse error shows a one-line "reload failed: <msg>".
   The full multi-line warning isn't shown (the tab-strip
   line is single-line). The startup path still buffers
   warnings to stderr-on-exit; reload warnings don't join
   that buffer. Acceptable — the banner names the failure
   and the config is kept.

2. **Invalid accent on reload is silent.** A reloaded
   `[theme] accent_color` that doesn't parse falls back
   to the default at render (the `parse_accent().is_none()`
   warning only fires at startup). No banner mention.
   Minor — the theme just doesn't change.

3. **No file-watch / auto-reload.** Reload is manual
   (`Ctrl+R`). A future polish could watch the file via
   `notify` and reload on change.

## Out of scope

- File-watch auto-reload.
- Surfacing multi-line reload warnings.

## Reviewer comments

(empty — pending grade)
