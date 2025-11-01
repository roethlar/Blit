## 2025-10-28 – Remote push streaming update
- `RemotePushClient` now biases response handling and streams gRPC fallback payloads as soon as files are requested; no longer buffers the entire manifest before sending data.
- Added helper to drain pending headers into control-plane transfers mid-manifest and finalize data-plane sessions after the control loop exits.
- Requirement: capture cross-platform benchmarks verifying the <1s first-byte guarantee and document in Phase 2 workflow; TODOs/docs updated.
- Commands run: `cargo fmt -p blit-core`, `cargo check -p blit-core`, `timeout 120s cargo test -p blit-cli remote_tcp_fallback -- --nocapture`.