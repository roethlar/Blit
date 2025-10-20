## Remote pull coverage (2025-10-19)
- Added async integration tests inside `crates/blit-daemon/src/main.rs` that start `BlitService`, pull entire modules and single files, exercise `--force-grpc-data`, and assert traversal/missing-path errors.
- Enabled `tokio-stream`'s `net` feature and cleaned Windows FFI lint warnings (`#[allow(non_camel_case_types, non_snake_case)]`, removed unused mut).
- Docs updated: TODO (Phase 3 tests), DEVLOG entry, Phase 3 workflow + project assessment now note the coverage.
- Local validation: `cargo test -p blit-daemon`, `cargo test -p blit-cli`, `cargo test -p blit-core`.
- WingPT & MacGPT asked to rerun platform suites + manual fallback pulls; expect logs in `logs/wingpt/windows-pull-<timestamp>/` and `logs/macgpt/`.