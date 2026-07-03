## CLI module refactor – 2025-10-27
- Split `crates/blit-cli/src/main.rs` (~1k lines) into focused modules:
  - `cli.rs` holds Clap structs/enums for all verbs and options.
  - `context.rs` encapsulates `AppContext` loading perf-history settings.
  - `diagnostics.rs`, `scan.rs`, `list.rs`, and `transfers.rs` host the respective command handlers (with local-transfer tests moved into `transfers.rs`).
- `main.rs` now wires modules: parse CLI, configure config-dir, dispatch commands.
- Updated TODO + workflow/state docs; DEVLOG entry added.
- Formatting/build: `cargo fmt`; `cargo check -p blit-cli`.