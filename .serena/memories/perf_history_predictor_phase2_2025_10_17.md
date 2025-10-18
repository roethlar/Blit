# Phase 2 Performance History & Predictor (2025-10-17)

## Summary
- Added local performance history module (`perf_history.rs`) storing JSONL summaries with rotation and opt-out env `BLIT_DISABLE_PERF_HISTORY`.
- Integrated orchestrator logging for both fast-path and streaming runs; errors surface only under `--verbose`.
- Introduced predictor scaffold (`perf_predictor.rs`) with per-profile EMA coefficients saved to `perf_predictor.json`; currently updates after each run.
- Dependencies added: `serde`, `serde_json`, `directories`, and dev-only `tempfile`.
- macOS worker paths cleaned to avoid unused warnings.

## Tests
- `cargo test -p blit-core`

## Next Steps
- Use predictor predictions to influence routing decisions (fast-path vs streaming).
- Implement `blit diagnostics perf` to surface history snapshots.
- Extend performance history/predictor to remote operations per Phase 3 plan.
- Benchmark warm-up impact (1st/10th/100th runs) as captured in TODO/workflow docs.