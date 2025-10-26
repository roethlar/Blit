WingPT —

- Thanks for landing the clone path. New follow-ups:
  1. Emit an `info!` log when `duplicate_extents` succeeds so we can confirm the block-clone fast path in benchmark logs (`RUST_LOG=info,blit_core::copy::windows=info`).
  2. Profile the remaining ReFS gap vs robocopy (blit ≈0.59 s vs 0.165 s). Capture ETW traces or equivalent telemetry under `logs/windows/` (e.g., `refs_clone_profile_<timestamp>.etl`) and summarize where the time is going.
  3. Experiment with disabling tar/worker fan-out in clone mode to check whether coordination overhead remains a factor; report the timing deltas.

Please drop results in a new `wingpt-XX.md` so I can fold them into TODO/DEVLOG.
