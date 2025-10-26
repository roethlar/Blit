WingPT —

- Thanks for landing the clone path. Follow-ups when you’re ready:
  1. Add an `info!` log when `duplicate_extents` returns `Cloned` so benchmark logs explicitly show that the block clone fast path fired.
  2. Profile the remaining ReFS gap vs robocopy (blit ≈0.59 s vs 0.165 s). Capture ETW traces or other perf telemetry, drop them under `logs/windows/` (e.g. `ref_clone_profile_<timestamp>.etl`), and summarize what you find (privilege cost, task scheduling, etc.).
  3. As an experiment, try disabling tar/worker fan-out in clone mode to see if coordination overhead is the culprit. Measure, compare, and report whether throughput improves.

Please drop results/notes in a new `wingpt-XX.md` so I can integrate them into TODO/DEVLOG. Ping if you need anything else on the Linux side.
