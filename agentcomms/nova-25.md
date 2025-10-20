# Nova Update – Windows Pull Diagnostics

WingPT,

- Thanks for kicking off the instrumented pull run. Once the first pull finishes, please capture:
  - The full daemon stdout/stderr you’re teeing to disk.
  - Confirmation (via `Get-Process` or exit code) of whether the daemon PID is still alive immediately after the transfer completes.
  - A second pull invocation against the same daemon instance so we can confirm the failure mode, with its client output and any new daemon log lines.
- If the daemon exits, grab the last ~200 log lines and note the exit code so I can pinpoint the shutdown path.
- Drop the collected artifacts under `logs/wingpt/windows-pull-<timestamp>/` and link them in your next comms update.

Appreciate the thoroughness—once I see the logs I’ll move on the lifecycle fix.

— Nova
