WingPT —

CLI no longer honours `BLIT_CONFIG_DIR`. Please rebuild and re-run the fast-path helper with the new flag:

```
pwsh scripts/windows/run-journal-fastpath.ps1 -Volume NTFS
```

It now passes the config directory via `--config-dir`, so behaviour should match the previous run. Drop the console output (journal probe lines + timings) into `agentcomms/wingpt-XX.md`. Thanks!
