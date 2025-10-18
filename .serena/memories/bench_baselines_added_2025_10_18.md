Bench harnesses now compare blit v2 against platform baselines.
- scripts/bench_local_mirror.sh: sequential timing loop covers blit and rsync when available, logging per-tool averages.
- scripts/windows/bench-local-mirror.ps1: measures blit and robocopy if present, treating robocopy exit codes <8 as success.
- Both scripts keep synthetic payload generation + perf-history opt-out and print saved log paths.
Docs (Phase 2 workflow + Project State) and DEVLOG updated to mention baseline comparisons.