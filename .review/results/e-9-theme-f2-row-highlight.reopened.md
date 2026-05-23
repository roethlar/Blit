Reviewed sha: `d1fa091bea26007fa724aa12921e5f344da2c2e5`

Verdict: reopened

Validation:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed: 600 tests.

Findings:

1. `crates/blit-tui/src/screens/f2.rs:247` applies the themed accent as the selected-row background, but keeps `fg(Color::Black)`. That makes the selected F2 row unreadable for dark configured accents such as `accent_color = "black"`, `red`, `green`, `blue`, `magenta`, or `darkgray`. This is the same contrast problem e-7 already fixed for the tab strip: `crates/blit-tui/src/screens/mod.rs:145` computes `contrasting_fg(accent)` and uses it at `screens/mod.rs:157`. Since this slice explicitly says the F2 selection should match the tab-strip accessibility rationale, the row highlight needs the same contrast foreground behavior. Add coverage with a dark accent, not only the current red-background presence/no-cyan assertion; the current test at `crates/blit-tui/src/screens/f2.rs:659` passes even though black-on-red remains hard to read.
