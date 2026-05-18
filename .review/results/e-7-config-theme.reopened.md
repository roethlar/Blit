# e-7-config-theme review

Reviewed sha: `2b910dc6e0386208ed0af273c21b9e8260e880c3`

## Findings

1. Low - `accent_color = "black"` renders the active tab as black-on-black.

   `crates/blit-tui/src/config.rs:196` accepts `black` as a supported accent color, but the active tab renderer always uses `fg(Color::Black)` and only swaps the background to the configured accent at `crates/blit-tui/src/screens/mod.rs:139`. With `[theme] accent_color = "black"`, the selected tab text becomes invisible. This is directly in the advertised supported palette, so it should either choose a contrasting foreground for dark accents or reject/fallback for accents that cannot keep the active tab readable. Please add a renderer-level regression test for at least the black accent case.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed (225 tests).
- `cargo test --workspace` passed.
