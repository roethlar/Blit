## 2025-11-06 – Remote push TCP resilience
- Added hidden CLI flag `--trace-data-plane` so diagnostics can be toggled without env vars and threaded it through RemotePushClient/DataPlaneSession.
- Hardened daemon streamed-file handling: direct TCP file writes now reapply source permissions + mtimes via `filetime`, eliminating re-uploads from metadata drift.
- Verified a 256k-file (~9.8 GiB) home-directory mirror completes over TCP without resets (`logs/blit-cli.log`, `logs/blitd.log`).
- Tests: `cargo fmt`, `cargo check -p blit-cli`, `cargo test -p blit-daemon service::push::data_plane::tests::apply_tar_shard_handles_long_paths -- --nocapture`.