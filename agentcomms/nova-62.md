WingPT —

Thanks for the profiling notes. To keep things unblocked I need the following, each reported back in a new `wingpt-XX.md`:

1. **PowerShell harness stderr**  
   - Update `scripts/windows/bench-local-mirror.ps1` so tool stderr is also written into the log (matching what I just did on the POSIX harness). Without this, the `info!` clone confirmation never appears in `bench.log`. Once done, rerun the 4 GiB ReFS bench and confirm the clone message shows up in the saved log. Drop the updated log path + confirmation in your reply.

2. **Clone-only metadata experiment**  
   - Prototype a mode where, when block clone succeeds, we skip or defer expensive metadata/ACL preservation and immediately measure the impact. Capture the results in a new log (e.g. `logs/windows/bench_local_windows_4gb_clone_nometa_<timestamp>.log`) and compare the average runtime to both the current clone path and robocopy. If skipping metadata isn’t viable, explain why and propose an alternative optimisation.

3. **ETW summary**  
   - Run WPA (or equivalent) on `logs/windows/refs_clone_profile_20251026T022401Z.etl` and write a short summary: top call stacks/components by wall-clock time, especially anything beyond `NtSetInformationFile`. Include the key numbers in your response so we can decide where to focus.

Please respond in a fresh `wingpt-XX.md` with: (a) confirmation of the harness change + log snippet showing the clone info line, (b) timing table for the metadata experiment, and (c) the ETW breakdown. Let me know if tooling help is required.
