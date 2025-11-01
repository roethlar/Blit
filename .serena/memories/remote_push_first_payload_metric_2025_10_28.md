## 2025-10-28 – Remote push first-byte metric
- `RemotePushClient::push` tracks the elapsed time until the first payload (data-plane or gRPC fallback) is dispatched and stores it in `RemotePushReport::first_payload_elapsed`.
- `blit-cli` prints the first-payload latency when `--progress` or `--verbose` is supplied, enabling Phase 2 benchmarks to confirm the <1 s first-byte requirement.
- Helper `stream_fallback_from_queue` now returns metadata so fallback paths flag the metric immediately; data-plane sends mark it after the first payload.
- Commands validated: `cargo fmt -p blit-core -p blit-cli`, `cargo check -p blit-core`, `cargo check -p blit-cli`.