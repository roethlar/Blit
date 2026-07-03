## blit-utils modularisation – 2025-10-27
- Split `crates/blit-utils/src/main.rs` (~840 lines) into:
  - `cli.rs` (Clap definitions) and `util.rs` (shared remote/path helpers).
  - Command modules: `scan`, `list_modules`, `ls`, `find`, `du`, `df`, `completions`, `rm`, `profile`.
- `main.rs` now just parses CLI and dispatches to module functions.
- Updated TODO/workflow/state docs plus DEVLOG entry.
- Formatting/build: `cargo fmt`; `cargo check -p blit-utils`.