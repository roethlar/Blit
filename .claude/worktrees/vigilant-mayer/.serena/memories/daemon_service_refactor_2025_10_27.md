## Daemon module split – 2025-10-27
- Extracted `crates/blit-daemon/src/runtime.rs` to host Clap args/config loader (formerly in `main.rs`).
- Created `crates/blit-daemon/src/service.rs` for gRPC handlers, purge helpers, data-plane logic, and tests; re-exported `BlitServer` so `main.rs` stays thin.
- `crates/blit-daemon/src/main.rs` now only parses args, loads runtime config, sets up mDNS, and boots tonic server.
- Updated TODO and workflow assessment (Phase 2/3) to mark refactor complete; DEVLOG entry added.
- Formatting + targeted build: `cargo fmt`; `cargo check -p blit-daemon`.