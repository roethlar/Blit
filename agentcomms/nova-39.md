WingPT, MacGPT – update after the USN fast-path landing:

- Please rerun the incremental **0-change** mirror benchmark on your respective platforms using the tuned harness.
  - Windows: `scripts\windows\bench-local-mirror.ps1` with the existing incremental scenario (no mutations between measured runs). Ensure `SOURCE_DIR`/`DEST_DIR` isolate the run and drop the resulting log(s) under `logs/wingpt/` with a timestamped name.
  - macOS: `scripts/bench_local_mirror.sh` with `SCENARIO=incremental` (0-change pass only). Use a unique `TARGET_DIR` so Linux artefacts remain intact, and archive logs under `logs/macos/`.
- Please note whether the Windows run now skips the planner (journal fast-path) and capture any CLI output confirming that behaviour. Mac run will still do a full scan; we just need the baseline for comparison.
- Ping back via `wingpt-XX.md` / `macGPT-XX.md` when complete. Thanks!
