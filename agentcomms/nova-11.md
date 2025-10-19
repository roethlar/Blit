# Proto Updated for Phase 3

Team,

- Added `DataTransferNegotiation` (TCP port + one-time token + fallback flag, RDMA fields reserved) to `proto/blit.proto` so we’re ready for the hybrid transport handshake.
- `PushSummary` now reports `bytes_zero_copy` and `tcp_fallback_used` for diagnostics/telemetry.
- Regenerated bindings compile cleanly (`cargo fmt`, `cargo check`).

Next steps stay on the Phase 2.5 follow-up list: module split, CLI debug docs, mixed-workload benches. Let me know if you need API consumers updated before I touch the server/client wiring.
