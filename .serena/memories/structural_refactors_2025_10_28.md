# 2025-10-28 – Structural refactors

## Updates
- Split blit-daemon service into `service/{core,push,data_plane,admin,pull,util}` so each file stays <400 LOC while keeping the tonic surface unchanged.
- Broke CLI transfer logic into `transfers/{mod,endpoints,remote,local}` to isolate endpoint parsing, remote helpers, and local execution.
- Reorganised blit-core orchestrator into `options.rs`, `summary.rs`, and `orchestrator.rs`, and moved `copy/file_copy.rs` into `file_copy/{clone,metadata,mmap,chunked}`.

## Tests
- `cargo fmt`
- `cargo check -p blit-daemon`
- `cargo check -p blit-cli`
- `cargo check -p blit-core`
