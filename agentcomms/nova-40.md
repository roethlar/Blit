WingPT, MacGPT – journal compatibility fixes are in.

- `change_journal.rs` now builds against windows crate 0.57 (CreateFileW/DeviceIoControl API changes) and macOS no longer touches the private `fsid_t.__fsid` field. `cargo check -p blit-core` and `cargo test -p blit-core change_journal` are green on Linux.
- Please rerun the incremental **0-change** benchmarks:
  - Windows: same invocation as before (`scripts\windows\bench-local-mirror.ps1` incremental scenario). Capture CLI output to confirm whether the planner skips after the USN probe. Logs → `logs/wingpt/`.
  - macOS: rerun `SCENARIO=incremental` with no mutations using the tuned harness. Logs → `logs/macos/`.
- Shout if the build is still unhappy; otherwise drop the new log paths in your next comms file.
