# e-4-config-tab-strip-counts: opt-out for tab-strip counts

**Severity**: Feature (polish — schema growth on the
e-3 config scaffold)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Operators who find the d-15 right-edge counts
("3 daemons · 1 active · 47 recent · ? help")
distracting — or who run the TUI on a narrow terminal
where the counts crowd out tab labels — can opt out
via `tui.toml`:

```toml
[tab_strip]
show_counts = false
```

Default is `true`, preserving the d-15 behavior.

When disabled, the tab strip renders only the F1..F4
labels with no right-edge column. Tabs always get full
labels (no responsive shrinkage to short labels) since
they're the only content competing for width.

## Approach

### Config schema

`TuiConfig` gains a `tab_strip: TabStripDefaults` section:

```rust
pub struct TabStripDefaults {
    pub show_counts: bool, // default true
}
```

Honors the same `#[serde(default, deny_unknown_fields)]`
contract as the existing `VerifyDefaults` — unknown
field names in `[tab_strip]` warn rather than silently
no-op, and partial configs keep defaults for unspecified
fields.

### Render path

`screens::render_tab_strip` gains a `show_counts: bool`
parameter. The responsive layout (d-15) is unchanged
internally; when `show_counts` is false the
`full_counts_width` and `short_counts_width` locals are
forced to zero, which the existing
"if area.width >= tabs + counts" branches consume
naturally. The render-counts step is also gated on the
bool so the right column truly doesn't paint.

### Apply

Main draw call passes `tui_config.tab_strip.show_counts`.

## Files changed

- `crates/blit-tui/src/config.rs`:
  - `TabStripDefaults` struct (default `show_counts: true`).
  - `TuiConfig` gains `tab_strip` field.
- `crates/blit-tui/src/screens/mod.rs`:
  - `render_tab_strip` gains `show_counts: bool`
    parameter.
  - Width math + render-counts step gated on the bool.
- `crates/blit-tui/src/main.rs`:
  - Draw call site passes
    `tui_config.tab_strip.show_counts`.

## Tests

+2 unit tests (209 → 211):

In `screens::tests`:
- `render_tab_strip_with_counts_shown_renders_counts`
  — renders into a 120-col `TestBackend` with
  `show_counts=true`, asserts the full counts string
  ("3 daemons · ...") appears.
- `render_tab_strip_with_counts_hidden_omits_counts` —
  same fixture with `show_counts=false`, asserts no
  daemons/active/recent strings appear AND the tabs
  still render with full labels.

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No partial opt-out.** Operator can't say "show
   daemons count but hide help hint". The whole counts
   column is on or off. A future polish could split the
   column into individually-configurable parts, but the
   string is already short — partial display would have
   to drop the `·` separators inconsistently.

2. **No runtime toggle.** Setting only applies at TUI
   start; flipping `H`-style at runtime isn't wired.
   Same gap as the d-6 / d-7 toggles — a future polish
   slice could add a hotkey + writeback.

## Out of scope (next slices)

- **Runtime tab-strip toggle hotkey.**
- **Color themes** (`[theme]`).
- **Refresh-interval config** (`[live_tick]`).

## Reviewer comments

(empty — pending grade)
