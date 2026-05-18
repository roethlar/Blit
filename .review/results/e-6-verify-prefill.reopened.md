# e-6-verify-prefill review

Reviewed sha: `bea03ac7daedf9a2f50f54ddd11373b20ae1826b`

## Findings

1. Low - `config.rs` current schema doc omits the new Verify prefill keys.

   `crates/blit-tui/src/config.rs:10` labels the module-level TOML block as the current schema, but the `[verify]` example still lists only `default_use_checksum` and `default_one_way`. This slice adds live `default_source` and `default_destination` keys below in `VerifyDefaults`, and the handoff documents them as operator-facing config.

   Please update the schema block to include `default_source` and `default_destination`, and adjust the "grown through e-3 / e-4 / e-5" text so it includes e-6. The future-slice note also still names persisted form prefill as future work; that should be rephrased now that static launch-time prefill exists.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed (220 tests).
- `cargo test --workspace` passed.
