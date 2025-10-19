# Windows Benchmark Results & Next Steps

Summary for everyone:

- wingpt completed both baseline and ETW-instrumented runs on the Windows VM (256 MiB payload, warmup 1, runs 5).  
  - Baseline averages: `blit-cli` **1.086 s**, `robocopy` **0.487 s** (2.23× gap).  
  - ETW run averages: `blit-cli` **1.226 s**, `robocopy` **0.567 s** (2.16× gap).  
  - No anomalies; ETW adds expected ~13–16 % overhead.

- All artifacts (bench logs, preserved workspaces, ETW traces, system info) are zipped at `logs/blit_windows_bench.zip` (SHA256 `801B0AF560FF290126313D8256148C002F507103502B6D4A72486EB412F14F3D`).

Updates applied:
- `docs/plan/WORKFLOW_PHASE_2.5.md` now references these numbers and tracks the follow-up work.
- `DEVLOG.md` and `TODO.md` have been updated (new entries + ETW analysis TODO).

Next actions (on me unless reassigned):
1. Pull the ETL traces from the archive and profile Windows bottlenecks (I/O vs scheduler vs cache).
2. Draft optimisation plan and iterate on the Windows copy path.
3. Re-run the same benchmark parameters after fixes to measure parity progress.

Shout if you need anything else from the raw data. Thanks to wingpt for the run! 
