# Phase 2 Fast-Path Routing (2025-10-17)

## Summary
- Implemented tiny/huge manifest fast-path routing in `TransferOrchestrator`.
- Tiny manifests (<=8 files, <=100 MiB, no mirror/checksum/force-tar) now trigger sequential direct copy.
- Single-file workloads >=1 GiB route directly to the large-file worker.
- Early exit logic stops scanning once thresholds exceeded; fallback remains streaming planner.
- Exposed `copy_paths_blocking`/`copy_large_blocking` for orchestrator reuse.

## Tests
- `cargo test -p blit-core`.

## Next Steps
- Build adaptive predictor + telemetry store (Phase 2.2) to inform routing.
- Implement CLI progress UI / flag cleanup (Phase 2.3).
- Add unit/integration coverage for fast-path heuristics and upcoming predictor (Phase 2.4).
- Expand benchmarks capturing fast-path vs. streaming hit rates (Phase 2.4/2.5).