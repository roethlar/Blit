## 2025-10-28 – Remote push throughput progress
- Added `RemotePushProgress` + `ProgressEvent` channel so the client emits manifest batches and per-payload byte counters.
- `DataPlaneSession` and control-plane fallback both report bytes/files as they stream; CLI `-p/--progress` (and `-v`) now prints rolling files transferred, total MiB, and average/current MiB/s.
- Supports diagnosing long enumerations where data is already flowing.
- Tests: `cargo fmt -p blit-core -p blit-cli`, `cargo check -p blit-core`, `cargo check -p blit-cli`, `timeout 120s cargo test -p blit-cli remote_tcp_fallback -- --nocapture`.