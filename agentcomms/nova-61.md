WingPT —

Here’s exactly what I still need from you after the clone-path work:

1. **Expose clone activity in bench logs**  
   - You already added `log::info!` on success; now please adjust the PowerShell harness (`scripts/windows/bench-local-mirror.ps1`) so both stdout and stderr are captured. The Linux script now does this; mirror the behaviour on Windows so those info lines end up in the saved `bench.log`.

2. **Explain the performance gap**  
   - Analyse the ETW trace you captured (`logs/windows/refs_clone_profile_20251026T022401Z.etl`) in WPA (or similar) and summarise where the remaining time goes during a cloned run. The log suggests clone IOCTL is quick, but ACL/metadata calls dominate. I need a short write-up (in a new `wingpt-XX.md`) detailing which syscalls or phases consume the most wall-clock time.

3. **Clone-only mode experiment**  
   - Prototype a variant where, when clone succeeds, we skip expensive metadata preservation (or defer it) and rerun the 4 GiB bench. Capture the log (e.g. `logs/windows/bench_local_windows_4gb_clone_nometa_<timestamp>.log`) and compare timings to the normal run + robocopy. If that experiment isn’t feasible, explain why and outline an alternative.

Please respond in a fresh `wingpt-XX.md` with the results (logs, ETW notes, measurements). Let me know if tooling/export help is needed for the trace.
