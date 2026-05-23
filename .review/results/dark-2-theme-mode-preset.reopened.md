Reviewed sha: `e0b631b0a2f6a537fca6fa72825bbc7b01294737`

# Reopened: invalid explicit base color falls through to the mode preset

`resolved_base_colors()` currently resolves each field as:

```rust
let bg = self.parse_background().or(preset.map(|(b, _)| b));
let fg = self.parse_foreground().or(preset.map(|(_, f)| f));
```

That makes an invalid non-empty explicit value indistinguishable from an
unset value. With this config:

```toml
[theme]
mode = "dark"
background = "blurple"
```

startup warns that `[theme] background` is not recognized and is "using
the terminal default", but rendering actually falls back to the dark
preset background (`black`). This conflicts with the dark-1 warning /
schema contract and with the dark-2 "explicit background/foreground
override the preset per-field" contract.

Please make the policy explicit and test it. Either:

- invalid explicit bg/fg overrides the preset to `None` for that field,
  matching the current warning text ("terminal default"), or
- invalid explicit bg/fg is ignored and the preset applies, but then the
  warning/schema text must say it is falling back to the mode preset when
  one exists.

I recommend the first policy because it preserves dark-1 semantics and
keeps an operator typo from silently changing to a preset color after
they explicitly set that field.

Validation run:

- `cargo fmt --all -- --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `cargo test --workspace` passed (`blit-tui`: 615 tests)
