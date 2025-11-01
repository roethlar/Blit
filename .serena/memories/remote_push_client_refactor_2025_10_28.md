## Remote push client refactor – 2025-10-28
- Replaced monolithic `remote/push/client.rs` with `push/client/{mod,types,helpers}.rs`.
- `types.rs` defines `RemotePushReport` and `TransferMode`.
- `helpers.rs` contains manifest/response task spawning, path normalization, payload utilities, and control-plane helpers.
- Main `client/mod.rs` retains the async push loop, wiring in new helper functions.
- No behaviour changes intended; `TransferPayload` planning and data-plane session handling unchanged.
- `cargo fmt`; `cargo check -p blit-core` executed.
- TODO/docs/DEVLOG updated to mark refactor complete.